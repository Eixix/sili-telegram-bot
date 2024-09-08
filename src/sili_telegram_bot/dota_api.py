import json
import requests
import logging
import pytz
from ast import literal_eval
from datetime import datetime
from sili_telegram_bot.models.matches import Matches
from sili_telegram_bot.models.playerinfo import Playerinfo
from sili_telegram_bot.modules.config import config

STC_RESOURCE_CONFIG = config["static_resources"]
DYN_RESOURCE_CONFIG = config["dynamic_resources"]
ACCOUNTS_CONFIG = config["accounts"]
API_CONFIG = config["opendota_api"]
PLAYER_SEGMENT_URL = f"{API_CONFIG['api_root']}/{API_CONFIG['player_segment']}"
MATCHES_ENDPOINT = API_CONFIG["account_matches_endpoint"]


logger = logging.getLogger(__name__)


def update_heroes() -> None:
    """
    Retrive current hero list from the openDOTA API.
    """
    api_root = API_CONFIG["api_root"]
    heroes_endpoint = API_CONFIG["hero_endpoint"]

    heroes_json = requests.get(f"{api_root}/{heroes_endpoint}").json()

    with open(DYN_RESOURCE_CONFIG["hero_data_path"], "w") as f:
        json.dump(heroes_json, f, indent=4)


def _get_heroes():
    with open(DYN_RESOURCE_CONFIG["hero_data_path"], "r") as f:
        return json.load(f)


def _get_accounts():
    """
    De-serialize the string representation of the accounts list.
    """
    # FIXME Use some other config library that allows for deeper nesting.
    return literal_eval(ACCOUNTS_CONFIG["account_list"])


def _get_local_matches(account_id):
    try:
        with open(
            f"{DYN_RESOURCE_CONFIG['match_data_dir']}/{account_id}.json", "r"
        ) as f:
            return json.load(f)
    except FileNotFoundError:
        return []


def _get_api_matches(account_id):

    account_match_url = f"{PLAYER_SEGMENT_URL}/{account_id}/{MATCHES_ENDPOINT}"

    return requests.get(account_match_url).json()


def get_playerinfos():
    playerinfos = []

    accounts = _get_accounts()
    for account in accounts:
        account_id = account["identifier"]

        player = requests.get(f"{PLAYER_SEGMENT_URL}/{account_id}").json()
        last_match = requests.get(
            f"{PLAYER_SEGMENT_URL}/{account_id}/{MATCHES_ENDPOINT}?limit=1"
        ).json()
        wins_loses = requests.get(f"{PLAYER_SEGMENT_URL}/{account_id}/wl").json()

        playerinfos.append(
            Playerinfo(
                account["name"],
                player["profile"]["personaname"],
                wins_loses["win"] + wins_loses["lose"],
                wins_loses["win"],
                wins_loses["lose"],
                round(wins_loses["win"] / wins_loses["lose"], 2),
                pytz.utc.localize(
                    datetime.utcfromtimestamp(
                        last_match[0]["start_time"] + last_match[0]["duration"]
                    )
                ).astimezone(pytz.timezone("Europe/Berlin")),
            )
        )
    return playerinfos


def get_lastgame():
    lastgame = 0
    accounts = _get_accounts()
    for account in accounts:
        last_match = requests.get(
            f"{PLAYER_SEGMENT_URL}/{account['identifier']}/{MATCHES_ENDPOINT}?limit=1"
        ).json()
        match_end = last_match[0]["start_time"] + last_match[0]["duration"]
        if match_end > lastgame:
            lastgame = match_end

    return (
        pytz.utc.localize(datetime.utcfromtimestamp(lastgame))
        .astimezone(pytz.timezone("Europe/Berlin"))
        .strftime("%d.%m.%Y %H:%M:%S")
    )


def api_crawl():
    heroes = _get_heroes()
    accounts = _get_accounts()

    matches = Matches()

    for account in accounts:

        account_id = account["identifier"]
        account_name = account["name"]

        # Get all old matches
        local_matches = _get_local_matches(account_id)

        # Get all matches
        api_matches = _get_api_matches(account_id)

        # The amount of new games
        diff = len(api_matches) - len(local_matches)

        with open(
            f"{DYN_RESOURCE_CONFIG['match_data_dir']}/{account_id}.json",
            "w",
            encoding="utf-8",
        ) as f:
            json.dump(api_matches, f, ensure_ascii=False, indent=2)

        if diff < 5:
            for i in range(diff):
                matches.add_result(api_matches[i], heroes, account_name)

    return matches
