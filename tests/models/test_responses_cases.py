import json
import pytest

from pytest_cases import parametrize

from sili_telegram_bot.modules.config import config as CONFIG


class VoicelineResource:
    """
    Wrapper for lazy access to the voiceline resource.
    """

    _resource_dict = None

    @classmethod
    def get_resource_dict(cls) -> dict:
        if cls._resource_dict is None:
            entity_response_file = CONFIG["voicelines"]["resource_file"]

            with open(entity_response_file, "r") as f:
                cls._resource_dict = json.load(f)

        return cls._resource_dict


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


def first_voiceline(entity) -> str:
    """
    Retrieve the first voiceline for a given entity.
    """
    res_dict = VoicelineResource.get_resource_dict()

    try:
        voiceline = res_dict[entity][0]["text"]
    except IndexError as e:
        raise ValueError(
            f"Error while retrieving first voiceline for {entity}: {e}. "
            f"Likely the entity has no responses in the file."
        )

    return voiceline


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

    class TestFirstVoicelineCases:
        def case_default(self):
            # FIXME This conversion from hero to entity name is pretty hacky, but
            # adding the whole lookup logic here seems silly as .
            return (default_hero(), first_voiceline(default_hero() + "/Responses"))

        @pytest.mark.slow
        @parametrize("hero_name", remaining_voiced_heroes())
        def case_remaining_heroes(self, hero_name):
            # FIXME This conversion from hero to entity name is pretty hacky, but
            # adding the whole lookup logic here seems silly.
            return (hero_name, first_voiceline(hero_name + "/Responses"))
