"""
Subset entity data and responses to build test data.
Run as `PYTHONPATH="tests" python3 tests/test_infrastructure/generate_test_data.py`
"""

import json
import os
import random
import logging


from common_case_infra import VoicelineResource
from sili_telegram_bot.modules.config import config
from sili_telegram_bot.modules import voiceline_scraping as vl_scrape

logger = logging.getLogger(__name__)

VL_CONFIG = config["voicelines"]

ENTITY_DATA_FILE = VL_CONFIG["entity_data_file"]
RESOURCE_FILE = VL_CONFIG["resource_file"]

TEST_DATA_DIR = "tests/test_data"
TEST_ENTITY_DATA_OUTPUT = f"{TEST_DATA_DIR}/entity_data.json"
TEST_RESPONSES_OUTPUT = f"{TEST_DATA_DIR}/responses.json"


def ensure_response_data() -> None:
    if not os.path.exists(ENTITY_DATA_FILE):
        logger.info(
            f"No entity data file found at {ENTITY_DATA_FILE}, retrieving it..."
        )
        vl_scrape.save_entity_table()

    if not os.path.exists(VL_CONFIG["resource_file"]):
        logger.info(f"No resource file found at {ENTITY_DATA_FILE}, retrieving it...")
        vl_scrape.save_resource()


def subset_entity_data(entity_data: dict[str, dict], max_n_per_group: int = 5) -> dict:
    """
    Randomly subset entity data across types.
    """
    logger.info("Generating test entities from overall entity data...")
    subset_data_dict = {}

    for type, type_dict in entity_data.items():
        logger.info(f"... for type {type}...")
        n_per_group = min(len(type_dict), max_n_per_group)
        rand_entities = random.sample([*type_dict.keys()], n_per_group)

        subset_data_dict[type] = {entity: type_dict[entity] for entity in rand_entities}

    logger.info(f"... Done.")

    return subset_data_dict


def entity_data_to_titles(entity_data: dict) -> list[dict]:
    """
    Extract all titles from an entity data dict into a list.
    """
    title_list = []

    for type_dict in entity_data.values():
        title_list += [entity["title"] for entity in type_dict.values()]

    return title_list


def extract_responses_by_title(
    responses: dict[str, dict], titles: list[str]
) -> list[dict]:
    """
    Extract all response entries
    """
    logger.info("Extracting test entities from response data...")
    return {title: responses[title] for title in titles}


def main():
    logger.info("Generating test data from complete responses data...")
    vl_resource = VoicelineResource(RESOURCE_FILE, ENTITY_DATA_FILE)
    ensure_response_data()
    entity_data = vl_resource.get_entity_dict()
    test_entity_data = subset_entity_data(entity_data)

    logger.info(f"Saving subset entity data to {TEST_ENTITY_DATA_OUTPUT}...")
    with open(TEST_ENTITY_DATA_OUTPUT, "w") as outfile:
        json.dump(test_entity_data, outfile, indent=4)

    test_titles = entity_data_to_titles(test_entity_data)
    response_dict = vl_resource.get_resource_dict()
    test_responses = extract_responses_by_title(response_dict, test_titles)

    logger.info(f"Saving subset responses to {TEST_ENTITY_DATA_OUTPUT}...")
    with open(TEST_RESPONSES_OUTPUT, "w") as outfile:
        json.dump(test_responses, outfile, indent=4)

    logger.info("... All done.")


if __name__ == "__main__":
    main()
