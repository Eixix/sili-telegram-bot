import json
import os
import regex
import requests

from dataclasses import dataclass

from sili_telegram_bot.models.exceptions import MissingResponseUrlException
from sili_telegram_bot.modules.config import config

VL_CONFIG = config["voicelines"]


@dataclass
class ResponseArgs:
    entity: str
    line: str
    type: str = "hero"
    level: int = 0


class Responses:
    """
    Interface with the comprehensive response json to retrieve individual lines.
    """

    entity_type_lookup = {
        "hero": "Hero responses",
        "voice_pack": "Voice Packs",
        "announcer": "Announcer Packs",
        "legacy": "Archived",
        "other": "Other responses",
    }
    entity_data_file = VL_CONFIG["entity_data_file"]
    resource_file = VL_CONFIG["resource_file"]
    entity_data = {}
    entity_responses = []

    def __init__(self):
        with open(self.entity_data_file, "r") as infile:
            self.entity_data = json.load(infile)

        with open(self.resource_file, "r") as infile:
            self.entity_responses = json.load(infile)

    def _get_type_data(self, entity_type: str):
        """
        Get data on response entities of one type, with type being one of hero,
        voice_pack, announcer, legacy, and other.
        """
        if not entity_type in self.entity_type_lookup:
            raise ValueError(f"Unknown entity type: '{entity_type}'")

        key = self.entity_type_lookup[entity_type]

        return self.entity_data[key]

    def _get_response_list(self, name: str, *args, **kwargs) -> list | None:
        type_data = self._get_type_data(*args, **kwargs)

        matches = [
            value for key, value in type_data.items() if key.lower() == name.lower()
        ]

        if matches:
            # Ignore potential multi-matches for now.
            # FIXME No longer do that.
            return matches[0]

        else:
            return None

    def get_response_url(
        self,
        name: str,
        pattern: str | regex.Pattern,
        entity_type: str = "hero",
        level: int = 0,
    ) -> str:
        """Retrieve the response url for a particular response."""
        re_pattern = regex.compile(pattern)
        name_match = self._get_response_list(name, entity_type=entity_type)

        if not name_match:
            raise Exception(f"Could not find responses for '{name}'")

        name_responses = self.entity_responses[name_match["title"]]

        match_responses = [
            resp for resp in name_responses if regex.search(re_pattern, resp["text"])
        ]

        if not match_responses:
            response_url = name_match["url"]
            raise Exception(
                f"Could not find line for '{name}'. Check the responses page to see "
                f"if you typed it correctly: {response_url}"
            )

        # FIXME In the future, return all matched responses and let the user choose
        # which they want.
        response = match_responses[0]

        response_urls = response["urls"]

        if response_urls[level] is None:
            available_urls = [
                ": ".join([str(i), url])
                for i, url in enumerate(response_urls)
                if url is not None
            ]
            exception_text = (
                f"The requested response URL is not available (missing file on the "
                f"wiki)."
            )
            if available_urls:
                concat_urls = ", ".join(available_urls)
                alternative_text = (
                    f"Alternative response levels are available: {concat_urls}."
                )
                exception_text += " " + alternative_text
            raise MissingResponseUrlException(exception_text)

        return response_urls[level]

    def get_link(self, entity, line, type="hero", level=0):
        fuzzy_rules = "{e<=1}"

        if regex.search(r"^\".+\"", line):
            line = line.strip('"')
            line_re = regex.compile(line, flags=regex.IGNORECASE)

        else:
            line_re = regex.compile(
                f"(?:{regex.escape(line)}){fuzzy_rules}", flags=regex.IGNORECASE
            )

        return self.get_response_url(
            name=entity, entity_type=type, pattern=line_re, level=level
        )

    def download_mp3(self, link):
        file_name_match = regex.search(r"[^\/]*.mp3", link)

        if not file_name_match:
            raise (
                f"Could not extract file name from link. Is the link "
                f"correct? {link}"
            )

        file_path = os.path.join("resources", file_name_match.group())

        dl_response = requests.get(link)

        if not dl_response.status_code == 200:
            raise (f"Could not get a positive response from {link}")

        dl_file_handle = open(file_path, "wb")
        dl_file_handle.write(dl_response.content)

        return file_path
