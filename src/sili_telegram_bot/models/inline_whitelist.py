"""
Whitelist for authenticating users for the inline voicelines. This is necessary,
since inline queries can't be filtered by chat id, only by user id. This is a hacky
solution, by which users in the authorized group need to use a command to whitelist
themselves for use of the inline voiceline queries.

Currently this is NOT thread safe.
"""

from pathlib import Path

from sili_telegram_bot.modules.config import config

INLINE_WHITELIST_PATH = config["inline_authentication"]["user_whitelist_path"]


class InlineWhitelist:

    def __init__(self, whitelist_path: str = INLINE_WHITELIST_PATH) -> None:
        self.wl_path = whitelist_path
        Path(self.wl_path).touch()

    def get_whitelist(self) -> list[str]:
        with open(self.wl_path, "r") as infile:
            return set(infile.readlines())

    def add_to_whitelist(self, user_id: str) -> None:
        with open(self.wl_path, "a", newline="\n") as infile:
            infile.write(user_id + "\n")


default_whitelist = InlineWhitelist()
