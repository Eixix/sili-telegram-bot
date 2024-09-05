import json
import pytest

from pytest_cases import parametrize


def all_hero_names() -> set[str]:
    """
    Provide a list of all known hero names.
    """
    hero_info_path = "resources/dynamic/heroes.json"

    with open(hero_info_path, "r") as f:
        known_heroes = json.load(f)

    return {hero["localized_name"] for hero in known_heroes}


def unvoiced_heroes() -> set[str]:
    """
    Provide a list of all the heroes that aren't actually voiced.
    """
    return {"Io", "Phoenix", "Marci", "Primal Beast"}


def voiced_heroes() -> set[str]:
    """
    Provide a list of all heroes with actual voicelines.
    """
    return set(all_hero_names()) - set(unvoiced_heroes())


def default_hero() -> str:
    """
    Provide some default hero_name.
    """
    return "Keeper of the Light"


def remaining_heroes() -> set[str]:
    """
    Provide a list of all the heroes except the default one to avoid duplication.
    """
    return set(all_hero_names()) - set(default_hero())


def remaining_voiced_heroes() -> set[str]:
    """
    Provide a list of all the voiced heroes except the default one to avoid duplication.
    """

    return set(voiced_heroes()) - set(default_hero())


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
