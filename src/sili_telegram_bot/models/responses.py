import json
import logging
import os
import regex
import requests

from dataclasses import dataclass
from difflib import get_close_matches

from sili_telegram_bot.modules.voiceline_scraping import (
    save_entity_table,
    save_resource,
)
from sili_telegram_bot.models.exceptions import MissingResponseUrlException
from sili_telegram_bot.modules.config import config

VL_CONFIG = config["voicelines"]

LOGGER = logging.getLogger(__name__)


@dataclass
class ResponseArgs:
    entity: str
    line: str
    type: str = "hero"
    level: int = 0


def parse_voiceline_args(args: list[str]) -> dict:
    """
    Parse the arguments to a voiceline request into a dict of args to be accepted
    by `Responses.get_link()`.
    """
    basic_help_text = (
        "The format should be "
        "'/voiceline Entity Name (entity_type): Voice line (level)'.\n"
        'Enclose line in "double quotes" to use regex as described in the `regex` '
        "module."
    )
    if len(args) <= 1:
        help_txt = "Not enough arguments. " + basic_help_text

        raise ValueError(help_txt)

    else:
        # To separate out hero and voice line (both may contain whitespaces),
        # we first concatenate all args to a string and then split it on the
        # colon to get hero and voiceline
        arg_string = " ".join(args)

        # pattern for each arg. A continuous string of any character besides parens or
        # a colon.
        single_arg_pattern = r"[^():]+"
        ap = single_arg_pattern

        # Parse out arguments as described in the help text above.
        arg_pattern = (
            r"^(" + ap + r")(\(" + ap + r"\))?:\w*(" + ap + r")(\(" + ap + r"\))?"
        )

        matches = regex.search(arg_pattern, arg_string)

        try:
            entity, type, line, level = matches.groups()
        except Exception as e:
            raise ValueError(f"Could not parse args: {arg_string}. " + basic_help_text)

        if not entity:
            raise ValueError(
                f"Could not parse out the name of the entity from '{arg_string}'. "
                + basic_help_text
            )

        if not line:
            raise ValueError(
                f"Could not parse out the voiceline from '{arg_string}'. "
                + basic_help_text
            )

        args = {"entity": entity.strip(), "line": line.strip()}

        if type:
            args["type"] = type.strip("()")

        if level:
            # Convert to int, and remove one level to account for zero indexing.
            args["level"] = int(level.strip("()")) - 1

    return ResponseArgs(**args)


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

    entity_data = {}
    entity_responses = []

    def __init__(
        self,
        entity_data_file: str = VL_CONFIG["entity_data_file"],
        resource_file: str = VL_CONFIG["resource_file"],
    ):
        self.entity_data_file = entity_data_file
        self.resource_file = resource_file

        try:
            with open(entity_data_file, "r") as infile:
                self.entity_data = json.load(infile)

        except FileNotFoundError as e:
            raise FileNotFoundError(
                f"Error when attempting to read entity data file at "
                f"'{entity_data_file}': {e}. The resources have likely not finished "
                f"downloading yet."
            )

        except json.JSONDecodeError as e:
            LOGGER.error(
                f"Error when attempting to read entity data file at "
                f"'{entity_data_file}': {e}. Likely the file got corrupted after an "
                f"ill-timed bot shutdown. The file was removed, and will be "
                f"created again."
            )
            save_entity_table(entity_data_file)

        try:
            with open(resource_file, "r") as infile:
                self.entity_responses = json.load(infile)

        except FileNotFoundError as e:
            raise FileNotFoundError(
                f"Error when attempting to read responses file at "
                f"'{resource_file}': {e}. The resources have likely not finished "
                f"downloading yet."
            )

        except json.JSONDecodeError as e:
            LOGGER.error(
                f"Error when attempting to read entity data file at "
                f"'{resource_file}': {e}. Likely the file got corrupted after an "
                f"ill-timed bot shutdown. The file was removed, and will be "
                f"created again."
            )
            save_resource(resource_file)

    def _get_type_data(self, entity_type: str):
        """
        Get data on response entities of one type, with type being one of hero,
        voice_pack, announcer, legacy, and other.
        """
        if not entity_type in self.entity_type_lookup:
            raise ValueError(f"Unknown entity type: '{entity_type}'")

        key = self.entity_type_lookup[entity_type]

        return self.entity_data[key]

    def _get_response_list(
        self, name: str, fuzzy_match: bool = False, *args, **kwargs
    ) -> list | None:
        type_data = self._get_type_data(*args, **kwargs)

        if fuzzy_match:
            all_names = [entity["name"] for entity in type_data.values()]
            match_names = get_close_matches(name, all_names, n=int(1e9), cutoff=0.2)

            matches = [type_data[name] for name in match_names]

        else:
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
        name_match = self._get_response_list(
            name, entity_type=entity_type, fuzzy_match=True
        )

        if not name_match:
            raise Exception(f"Could not find responses for '{name}'")

        matched_name = name_match["name"]

        name_responses = self.entity_responses[name_match["title"]]

        match_responses = [
            resp for resp in name_responses if regex.search(re_pattern, resp["text"])
        ]

        if not match_responses:
            response_url = name_match["url"]
            raise Exception(
                f"Could not find line for '{matched_name}'. Check the responses page to see "
                f"if you typed it correctly: {response_url}"
            )

        # FIXME In the future, return all matched responses and let the user choose
        # which they want.
        response = match_responses[0]

        response_urls = response["urls"]

        available_urls = [
            # Convert back to 1 based indexing.
            ": ".join([str(i + 1), url])
            for i, url in enumerate(response_urls)
            if url is not None
        ]

        if len(response_urls) < level + 1:
            exception_text = f"Entity {matched_name} does not have as many levels."

            if available_urls:
                concat_urls = ", ".join(available_urls)
                alternative_text = f"Available levels are: {concat_urls}."
                exception_text += " " + alternative_text

            raise MissingResponseUrlException(exception_text)

        if response_urls[level] is None:
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
