import requests
import bs4
import json
import regex
import os

from sili_telegram_bot.modules.config import config

DYN_RESOURCE_CONFIG = config["dynamic_resources"]
VL_CONFIG = config["voicelines"]


class Voiceline:
    hero = ""
    response_url = ""

    # TODO: Should this be initialised like that?
    soup = bs4.BeautifulSoup()

    def __init__(self, hero_string: str) -> None:

        with open(DYN_RESOURCE_CONFIG["hero_data_path"], "r") as f:
            known_heroes = json.load(f)

        # For comparing the input hero name to known heroes, we throw away
        # capitalization, allowing for some fuzzyness in the input.
        hero_lookup_string = hero_string.lower()

        hero_name_lookup = {
            hero["localized_name"].lower(): hero["localized_name"]
            for hero in known_heroes
        }

        if not hero_lookup_string in hero_name_lookup:
            base_url = VL_CONFIG["base_url"]
            all_hero_path = VL_CONFIG["all_hero_path"]
            all_hero_url = f"{base_url}/{all_hero_path}"

            raise ValueError(
                f"Unknown hero {hero_string}. "
                f"(Check the hero name against the hero list at {all_hero_url})"
            )

        # On the fandom wiki the pages for heroes follow the pattern of
        # "base_url/Capitalized_Hero/subpage", so we need to ensure the hero
        # name follows that pattern
        self.hero = hero_name_lookup[hero_lookup_string].replace(" ", "_")

        self.response_url = (
            f"{VL_CONFIG['base_url']}/{self.hero}/{VL_CONFIG['hero_response_path']}"
        )

        vl_pg_response = requests.get(self.response_url)
        self.soup = bs4.BeautifulSoup(vl_pg_response.content, features="html.parser")

        # On the the wiki page all voiceline links are in tags which can be
        # selected using this css selector.
        # TODO: check if this looks as expected (i.e. has content)
        self.vl_tags = self.soup.select("#mw-content-text h2~ ul li")

    def get_link(self, line):
        line_tag = ""
        fuzzy_rules = "{e<=1}"

        if regex.search(r"^\".+\"", line):
            line = line.strip('"')
            line_re = regex.compile(line, flags=regex.IGNORECASE)

        else:
            line_re = regex.compile(
                f"(?:{regex.escape(line)}){fuzzy_rules}", flags=regex.IGNORECASE
            )

        for tag in self.vl_tags:
            # Each of the tags contains the audiobutton with the link to the
            # audiofile as its first child and as its second child the text
            # of the voiceline (possibly broken up by link tags). Here we find
            # the tag containing the link to
            # the desired line. The selector is not perfect so for some heroes
            # tags that are not voicelines can get read too. The outer if
            # statement helps against that. The voiceline may contain links, so
            # extracting all literal text is necessary.
            #
            # TODO: add ways to find other occurrences than only the first
            if tag.contents[0].name:
                tag_text = "".join([*map(lambda content: content.text, tag.contents)])
                if regex.search(line_re, tag_text.strip()):
                    line_tag = tag
                    break

        if not line_tag:
            vl_link = None
        else:
            vl_link = line_tag.find("source")["src"]

        return vl_link

    def download_mp3(self, link):
        file_name_match = regex.search(r"[^\/]*.mp3", link)

        if not file_name_match:
            raise (
                f"Could not extract file name from link. Is the link "
                f"correct? {link}"
            )

        file_path = os.path.join(
            VL_CONFIG["temp_download_dir"], file_name_match.group()
        )

        dl_response = requests.get(link)

        if not dl_response.status_code == 200:
            raise (f"Could not get a positive response from {link}")

        dl_file_handle = open(file_path, "wb")
        dl_file_handle.write(dl_response.content)

        return file_path
