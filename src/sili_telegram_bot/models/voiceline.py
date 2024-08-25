import requests
import regex
import os

from sili_telegram_bot.models.responses import Responses
from sili_telegram_bot.modules.config import config

DYN_RESOURCE_CONFIG = config["dynamic_resources"]
VL_CONFIG = config["voicelines"]


class Voiceline:
    hero = ""
    responses = None

    def __init__(self, hero_string: str) -> None:
        self.responses = Responses()

        self.hero = hero_string

    def get_link(self, line):
        fuzzy_rules = "{e<=1}"

        if regex.search(r"^\".+\"", line):
            line = line.strip('"')
            line_re = regex.compile(line, flags=regex.IGNORECASE)

        else:
            line_re = regex.compile(
                f"(?:{regex.escape(line)}){fuzzy_rules}", flags=regex.IGNORECASE
            )

        return self.responses.get_response_url(name=self.hero, pattern=line_re)

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
