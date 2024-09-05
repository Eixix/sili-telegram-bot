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


def unvoiced_heroes() -> list[str]:
    """
    Provide a list of all the heroes that aren't actually voiced.
    """
    return ["Io", "Phoenix", "Marci", "Primal Beast"]


def voiced_heroes() -> list[str]:
    """
    Provide a list of all heroes with actual voicelines.
    """
    unvoiced_set = set(unvoiced_heroes())
    voiced_set = set(all_hero_names()) - unvoiced_set
    return [*voiced_set]


def default_hero() -> str:
    """
    Provide some default hero_name.
    """
    return "Keeper of the Light"


def remaining_heroes() -> list[str]:
    """
    Provide a list of all the heroes except the default one to avoid duplication.
    """
    remaining_set = set(all_hero_names()) - set(default_hero())
    return [*remaining_set]


def remaining_voiced_heroes() -> list[str]:
    """
    Provide a list of all the voiced heroes except the default one to avoid duplication.
    """
    remaining_voiced_heroes = set(voiced_heroes()) - set(default_hero())

    return [*remaining_voiced_heroes]


class TestResponsesCases:

    class TestCrummyWizardCases:
        def case_default(self):
            """
            Generate a case just for Keeper of the Light as a (almost) random hero.
            """
            return default_hero()

        @pytest.mark.slow
        @parametrize("hero_name", remaining_voiced_heroes())
        def case_remaining_voiced_heroes(self, hero_name: str):
            """
            Generate cases involving all known hero names.
            """
            return hero_name
