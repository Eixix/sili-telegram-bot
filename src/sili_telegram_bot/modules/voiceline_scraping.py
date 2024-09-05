"""
Functions to scrape & parse URL links into a JSON dict for expedient & extensive
voiceline retrieval. These depend on the page layout of the wiki though,
so this will need to be adapted to changes there.
"""

import bs4
import json
import logging
import re
import unicodedata

from sili_telegram_bot.modules.config import config
from sili_telegram_bot.modules.mediawiki_api import APIWrapper
from sili_telegram_bot.modules.response_types import EntityData, EntityResponse

VL_CONFIG = config["voicelines"]

LOGGER = logging.getLogger(__name__)


TEXT_PROCESS_RE_PREFIX = re.compile(
    r"^(\s*)?((Link(\u25B6.)\s)+)?\s*(u\s)?(r\s)?(rem\s)?(\d+\s)?\s*"
)


TEXT_PROCESS_RE_SUFFIX = re.compile(r"(\s+followup)?$")


def parse_link_row(
    link_row: bs4.element, base_url: str = VL_CONFIG["base_url"]
) -> dict[str, EntityData]:
    """
    Parse out entitiy names, urls, and titles from individual table row.
    """
    entity_data_dict = {}
    for tag in link_row:
        ref_sep = "/"
        entity_name = tag.string.replace("\u00a0", " ")

        # These tags are relative to the root of the entire wiki, so we need to
        # prune the wiki specifier off of them if we want to glue it onto the base_url.
        pruned_ref = ref_sep + ref_sep.join(
            tag["href"].strip(ref_sep).split(ref_sep)[1:]
        )

        data = EntityData(
            # For some reason, some entity names contain non-breaking spaces (\u00a0),
            # which we don't want.
            name=entity_name,
            url=base_url + pruned_ref,
            title=tag["title"],
        )
        entity_data_dict[entity_name] = data

    return entity_data_dict


def process_response_text(text: str) -> str:
    """
    Process the text related to a voice response to remove unneeded info (Emoji, tool
    tips, etc.).
    """
    de_prefixed_text = re.sub(string=text, pattern=TEXT_PROCESS_RE_PREFIX, repl="")
    stripped_text = re.sub(
        string=de_prefixed_text, pattern=TEXT_PROCESS_RE_SUFFIX, repl=""
    )
    processed_text = unicodedata.normalize("NFKD", stripped_text)
    return processed_text


def audio_button_has_url(button_tag: bs4.element) -> bool:
    """
    Check if a audio button tag has an associated audio url.
    """
    return button_tag["data-state"] == "play"


def get_audio_button_url(button_tag: bs4.element) -> str:
    """
    Get the associated audio URL from an audio-button by extracing it from its parent.
    """
    return button_tag.parent.source["src"]


def response_from_link_tag(link_tag: bs4.element) -> EntityResponse | None:
    """
    Extract information on the response contained in a link tag. If no information
    is present, return None.
    """
    # For heros with voice *altering* arcanas (e.g. Monkey King), there may be more
    # than one audio-button, each of which may or may not be working.
    audio_button_tags = link_tag.find_all("a", class_="ext-audiobutton")

    # If there are no audio buttons (because someone put only text into a <li> tag)
    # we can skip the curren tag.
    if audio_button_tags:
        resp_audio_urls = []

        for button_tag in audio_button_tags:
            if audio_button_has_url(button_tag):
                resp_audio_urls.append(get_audio_button_url(button_tag))
            else:
                resp_audio_urls.append(None)

        resp_text = process_response_text(link_tag.text)

        return EntityResponse({"text": resp_text, "urls": resp_audio_urls})

    else:
        return None


def extract_entity_response_urls(entity_page_html: str) -> list[EntityResponse]:
    """
    Extract all response urls (to audio files) of an entity from the html of its
    responses pageand return them as a dict oflists. The first item will (almost)
    always be the basic voiceline URL, but if the entity has an altered voice
    (i.e. an arcana) the second item of the list will be for that. The list may
    contain None in the case of missing files.
    """
    entity_soup = bs4.BeautifulSoup(entity_page_html, features="html.parser")

    # All li tags in bullet points.
    bullet_li_tags = entity_soup.select(".mw-parser-output > ul li")

    # Li tags inside tables (found in Announcers.)
    table_li_tags = entity_soup.select(".wikitable li")
    response_tags = bullet_li_tags + table_li_tags

    responses = []

    for tag in response_tags:
        optional_response = response_from_link_tag(tag)
        if optional_response:
            responses.append(optional_response)

    return responses


def extract_response_urls_from_titles(
    page_titles: list[str],
) -> dict[str, list[EntityResponse]]:
    """
    Retrieve page html and extract response urls for a list of page titles.
    """
    out = {}

    mediawiki_api = APIWrapper.get_or_create_mediawiki_api()

    for page_title in page_titles:
        LOGGER.info(f"Getting responses for page '{page_title}'...")
        page_html = mediawiki_api.page(page_title, auto_suggest=False).html
        out[page_title] = extract_entity_response_urls(page_html)

    return out


def extract_entity_table(
    navbar_title: str = "Template:VoiceNavSidebar",
) -> dict[str, dict[str, EntityData]]:
    """
    Parse data for every response entity from the responses navbar on the wiki.
    """
    mediawiki_api = APIWrapper.get_or_create_mediawiki_api()
    navbar_page = mediawiki_api.page(navbar_title)
    navbar_soup = bs4.BeautifulSoup(navbar_page.html, features="html.parser")
    url_table = navbar_soup.find(class_="nowraplinks")

    table_headers = [
        tag.string.strip() for tag in url_table.find_all(class_="navbox-odd")
    ]
    path_tag_list = [
        tag.find_all("a") for tag in url_table.find_all(class_="navbox-even")
    ]

    entity_data_lists = [parse_link_row(link_row) for link_row in path_tag_list]

    return {
        table_header: entity_dict
        for table_header, entity_dict in zip(table_headers, entity_data_lists)
    }


def save_entity_table(
    output_file: str = VL_CONFIG["entity_data_file"], *args, **kwargs
):
    """
    Get table with response entity data and save to output_file.
    """
    LOGGER.info(f"Getting response entity data, will be saved to '{output_file}'.")
    entity_table = extract_entity_table(*args, **kwargs)
    with open(output_file, "w") as outfile:
        json.dump(entity_table, outfile, indent=4)


def extract_voiceline_urls() -> dict[str, dict[str, EntityData]]:
    """
    Extract the URLs for all entities with responses and save to JSON file.
    """
    mediawiki_api = APIWrapper.get_or_create_mediawiki_api()
    response_titles = mediawiki_api.categorymembers("Responses", results=None)[0]
    return extract_response_urls_from_titles(response_titles)


def save_resource(
    output_file: str = VL_CONFIG["resource_file"],
) -> None:
    """
    Get table with response entity data and save to output_file.
    """
    LOGGER.info(f"Getting responses, will be saved to '{output_file}'.")
    entity_resp_dict = extract_voiceline_urls()

    with open(output_file, "w") as outfile:
        json.dump(
            obj=entity_resp_dict,
            fp=outfile,
            indent=2,
        )


def get_response_data() -> None:
    """
    Retrieve data on responses and save to configured paths.
    """
    LOGGER.info("Getting response data...")
    save_entity_table(output_file=VL_CONFIG["entity_data_file"])
    save_resource(output_file=VL_CONFIG["resource_file"])