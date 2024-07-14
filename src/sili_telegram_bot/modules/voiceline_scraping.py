"""
Functions to scrape & parse URL links into a JSON dict for expedient & extensive
voiceline retrieval. These depend on the page layout of the wiki though,
so this will need to be adapted to changes there.
"""

import bs4
import json
import requests


def parse_link_row(link_row: bs4.element, base_url: str) -> dict:
    """
    Parse out entitiy names & urls from individual table row.
    """
    # For some reason, some entity names contain non-breaking spaces (\u00a0),
    # which we don't want.
    return {
        tag.string.replace("\u00a0", " "): base_url + tag["href"] for tag in link_row
    }


def parse_response_table(table: bs4.element, base_url: str) -> dict:
    """
    Parse out URLs for inidividual response entities (hero, arcana, etc.) from
    table html.
    """
    table_headers = [tag.string.strip() for tag in table.find_all(class_="navbox-odd")]
    path_tag_list = [tag.find_all("a") for tag in table.find_all(class_="navbox-even")]
    entity_dicts = [parse_link_row(path_tags, base_url) for path_tags in path_tag_list]
    return {
        header: entity_dict for header, entity_dict in zip(table_headers, entity_dicts)
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
