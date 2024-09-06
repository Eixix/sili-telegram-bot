import json

from sili_telegram_bot.modules.config import config as CONFIG


class VoicelineResource:
    """
    Wrapper for lazy access to the voiceline resource.
    """

    _resource_dict = None
    _entity_dict = None

    @classmethod
    def get_resource_dict(cls) -> dict:
        if cls._resource_dict is None:
            entity_response_file = CONFIG["voicelines"]["resource_file"]

            with open(entity_response_file, "r") as f:
                cls._resource_dict = json.load(f)

        return cls._resource_dict

    @classmethod
    def get_entity_dict(cls) -> dict:
        if cls._entity_dict is None:
            entity_data_file = CONFIG["voicelines"]["entity_data_file"]

            with open(entity_data_file, "r") as f:
                cls._entity_dict = json.load(f)

        return cls._entity_dict


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
    return all_hero_names() - unvoiced_heroes()


def default_hero() -> str:
    """
    Provide some default hero_name.
    """
    return "Keeper of the Light"


def remaining_heroes() -> set[str]:
    """
    Provide a list of all the heroes except the default one to avoid duplication.
    """
    return all_hero_names() - set(default_hero())


def remaining_voiced_heroes() -> set[str]:
    """
    Provide a list of all the voiced heroes except the default one to avoid duplication.
    """
    return voiced_heroes() - set(default_hero())


def hero_name_to_tile(hero_name: str) -> str:
    """
    Look up a hero's title in the entity data.
    """
    entity_data = VoicelineResource.get_entity_dict()

    return entity_data["Hero responses"][hero_name]["title"]


def non_hero_entity_dict() -> dict:
    """
    Provide the entity dict without heroes.
    """
    entity_dict = VoicelineResource.get_entity_dict()

    return {
        name: section
        for name, section in entity_dict.items()
        if name != "Hero responses"
    }


def non_hero_entity_page_titles() -> set[str]:
    """
    Provide a set of all the wiki page titles of all non-hero response entities.
    """
    entity_dict = non_hero_entity_dict()

    page_titles = set()

    for section in entity_dict.values():
        section_titles = [item["title"] for item in section.values()]
        page_titles = page_titles.union(set(section_titles))

    return page_titles


def non_hero_entity_names() -> set[str]:
    """
    Provide the set of names of all non-hero response entities.
    """
    entity_dict = non_hero_entity_dict()

    entity_names = set()

    for section in entity_dict.values():
        section_names = [item["name"] for item in section.values()]
        entity_names = entity_names.union(set(section_names))

    return entity_names


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
