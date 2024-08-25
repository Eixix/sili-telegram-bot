import json
import regex

from sili_telegram_bot.modules.config import config

VL_CONFIG = config["voicelines"]


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

        if match_responses:
            # FIXME In the future, return all matched responses and let the user choose
            # which they want.
            return match_responses[0]["urls"][level]
        else:
            response_url = name_match["url"]
            raise Exception(
                f"Could not find line for '{name}'. Check the responses page to see "
                f"if you typed it correctly: {response_url}"
            )
