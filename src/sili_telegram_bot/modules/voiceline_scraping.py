"""
Functions to scrape & parse URL links into a JSON dict for expedient & extensive
voiceline retrieval. These depend on the page layout of the wiki though,
so this will need to be adapted to changes there.
"""

import bs4
import json
import re
import requests
import unicodedata

from typing import TypedDict


class EntityResponse(TypedDict):
    text: str
    urls: list[str, None]


TEXT_PROCESS_RE_PREFIX = re.compile(
    r"^(\s*)?((Link(\u25B6.)\s)+)?\s*(u\s)?(r\s)?(rem\s)?(\d+\s)?\s*"
)


TEXT_PROCESS_RE_SUFFIX = re.compile(r"(\s+followup)?$")


def parse_link_row(link_row: bs4.element, base_url: str) -> dict:
    """
    Parse out entitiy names & urls from individual table row.
    """
    # For some reason, some entity names contain non-breaking spaces (\u00a0),
    # which we don't want.
    return {
        tag.string.replace("\u00a0", " "): base_url + tag["href"] for tag in link_row
    }


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


def scrape_entity_response_urls(entity_base_response_url: str) -> list[EntityResponse]:
    """
    Scrape all response urls (to audio files) of an entity and return them as a dict of
    lists. The first item will (almost) always be the basic voiceline URL, but if the
    entity has an altered voice (i.e. an arcana) the second item of the list will be for
    that. The list may contain None in the case of missing files.
    """
    entity_url_resp = requests.get(entity_base_response_url)
    entity_soup = bs4.BeautifulSoup(entity_url_resp.content, features="html.parser")

    # All li tags in bullet points.
    bullet_li_tags = entity_soup.select("#mw-content-text h2~ ul li")

    # Li tags inside tables (found in Announcers.)
    table_li_tags = entity_soup.select(".wikitable li")
    response_tags = bullet_li_tags + table_li_tags

    responses = []

    for tag in response_tags:
        optional_response = response_from_link_tag(tag)
        if optional_response:
            responses.append(optional_response)

    return responses


def scrape_response_url_dict(
    response_dict: dict[str, str]
) -> dict[str, list[EntityResponse]]:
    """
    Scrape the response urls for all entities in a dict.
    """
    out = {}

    for entity, response_url in response_dict.items():
        out[entity] = scrape_entity_response_urls(response_url)

    return out


def parse_response_table(table: bs4.element, base_url: str) -> dict:
    """
    Parse out URLs for inidividual response entities (hero, arcana, etc.) from
    table html.
    """
    table_headers = [tag.string.strip() for tag in table.find_all(class_="navbox-odd")]
    path_tag_list = [tag.find_all("a") for tag in table.find_all(class_="navbox-even")]
    entity_base_dicts = [
        parse_link_row(path_tags, base_url) for path_tags in path_tag_list
    ]

    full_dict = [
        scrape_response_url_dict(entity_base_dict)
        for entity_base_dict in entity_base_dicts
    ]

    return {
        header: entity_dict for header, entity_dict in zip(table_headers, full_dict)
    }


def scrape_voiceline_urls(
    output_file: str = "resources/entity_responses.json",
    base_url: str = "https://liquipedia.net",
    navbar_path: str = "/dota2game/Template:VoiceNavSidebar",
) -> None:
    """
    Scrape the URLs for all entities with responses and save to JSON file.
    """
    navbar_response = requests.get(base_url + navbar_path)
    response_soup = bs4.BeautifulSoup(navbar_response.content, features="html.parser")
    url_table = response_soup.find(class_="nowraplinks")

    json.dump(
        obj=parse_response_table(url_table, base_url=base_url),
        fp=open(output_file, "w"),
        indent=2,
    )
