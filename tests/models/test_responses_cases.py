import json
import pytest

from pytest_cases import parametrize


def all_hero_names() -> list[str]:
    """
    Provide a list of all known hero names.
    """
    hero_info_path = "resources/dynamic/heroes.json"

    with open(hero_info_path, "r") as f:
        known_heroes = json.load(f)

    return [hero["localized_name"] for hero in known_heroes]


class TestResponsesCases:
    @pytest.mark.slow
    @parametrize("hero_name", all_hero_names())
    def case_hero_generator(self, hero_name: str):
        """
        Generate cases involving all known hero names.
        """
        return hero_name

    @pytest.mark.duplication
    def case_kotl(self):
        """
        Generate a case just for Keeper of the Light as a (almost) random hero.
        """
        return "Keeper of the Light"
