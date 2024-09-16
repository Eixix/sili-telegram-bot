"""
Provide in-line searching for responses.
"""

import logging
import regex

from hashlib import md5
from itertools import islice
from telegram import InlineQueryResultVoice, Update
from telegram.error import BadRequest
from telegram.ext import Application, CallbackContext, InlineQueryHandler


from sili_telegram_bot.models.responses import Responses, parse_voiceline_args

LOGGER = logging.getLogger(__name__)


class LazyResponseDict:

    _RESPONSES = None
    _FULL_RESPONSE_DICT = None

    @classmethod
    def _get_or_create_response_obj(cls) -> Responses:
        if cls._RESPONSES is None:
            cls._RESPONSES = Responses()

        return cls._RESPONSES

    @classmethod
    def _create_full_response_dict(
        cls, entity_data: dict, response_data: dict
    ) -> dict[str, str]:
        """
        Create a dict mapping full response info to the response url. The info
        consists of: Entity name (entity_type): Response text (url_level)
        This corresponds to the format of the voiceline command.
        """
        full_response_dict = {}

        # FIXME Switch to SQLite DB and remove these ghastly loops.
        for type_data in entity_data.values():
            for entity_name, entity_dict in type_data.items():
                entity_title = entity_dict["title"]
                for response_dict in response_data[entity_title]:
                    response_text = response_dict["text"]

                    for url in response_dict["urls"]:
                        full_response = f"{entity_name}: {response_text}"

                        if url:
                            full_response_dict[full_response] = url

        return full_response_dict

    @classmethod
    def get_or_create_full_resp_dict(cls):
        if cls._FULL_RESPONSE_DICT is None:
            responses = cls._get_or_create_response_obj()
            cls._FULL_RESPONSE_DICT = cls._create_full_response_dict(
                responses.entity_data, responses.entity_responses
            )

        return cls._FULL_RESPONSE_DICT


def get_substring_matches(
    query: str, match_pool: list[str], tolerance: int = 1, max_n: int = 50
) -> list[str]:
    """
    Search for substring matches of query in a pool of potential matches with some
    tolerance for errors. Since this is intended for retrieving inline results, which
    can't exceed more than some number of results, the search terminates early if that
    many results are found to speed up running time.
    """
    fuzzy_rules = f"{{e<={tolerance}}}"
    pattern = regex.compile(f"{query}{fuzzy_rules}", flags=regex.IGNORECASE)

    matches = []
    n_found = 0

    for candidate in match_pool:
        search_res = regex.search(pattern, candidate)

        if search_res:
            n_found += 1
            matches.append(candidate)

        if n_found >= max_n:
            break

    return matches


def create_voice_result(full_response: str, url: str) -> InlineQueryResultVoice:
    """
    Create a Voiceline result to be sent of to a chat requesting responses.
    """
    # Silly hack to get around retrieving all the info again from the Response object.
    # Would probably not be needed if the response data was kept in a relational DB.
    parsed_response_data = parse_voiceline_args(full_response.split(" "))
    response_hash = md5(bytes(full_response, encoding="utf-8")).hexdigest()

    response_title = f"{parsed_response_data.entity}: {parsed_response_data.line}"

    return InlineQueryResultVoice(id=response_hash, voice_url=url, title=response_title)


def matched_response_dict_to_voice_results(
    response_dict: dict,
) -> list[InlineQueryResultVoice]:
    return [
        create_voice_result(response, url) for response, url in response_dict.items()
    ]


async def handle_inline_vl_query(update: Update, context: CallbackContext) -> None:
    """
    Inline query handler to provide responses.
    """
    max_matches = 50
    full_resp_dict = LazyResponseDict.get_or_create_full_resp_dict()
    query = update.inline_query
    query_text = query.query

    LOGGER.info(f"Recieved query '{query_text}'")

    if len(query_text) > 0:
        LOGGER.info(f"Matching responses for '{query_text}'")
        matching_responses = get_substring_matches(
            query_text, full_resp_dict.keys(), max_n=max_matches
        )

    else:
        LOGGER.info(f"Query has no length, returning first {max_matches} responses...")
        n_keys = min(max_matches, len(full_resp_dict.keys()))
        matching_responses = islice(full_resp_dict.keys(), n_keys)

    LOGGER.info(f"Back-searching response URLs for '{query_text}'")
    matched_response_dict = {
        matching_response: full_resp_dict[matching_response]
        for matching_response in matching_responses
    }

    LOGGER.info(f"Compiling and sending VL results for '{query_text}'")

    try:
        await query.answer(
            results=matched_response_dict_to_voice_results(matched_response_dict)
        )

    except BadRequest as e:
        LOGGER.error(
            f"Error when attempting to send inline response results for "
            f"'{query_text}': {e}"
        )


def add_inline_handlers(application: Application) -> None:
    """
    Add handlers related to voiceline parsing to the application.
    """
    application.add_handler(InlineQueryHandler(handle_inline_vl_query))
