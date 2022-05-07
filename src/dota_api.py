import json
import requests
import logging
from models.matches import Matches

logger = logging.getLogger(__name__)

def _get_heroes():
    with open("resources/heroes.json", 'r') as f:
        return json.load(f)

def _get_accounts():
    with open("matchdata/accounts_file.json", 'r') as f:
        return json.load(f)

def _get_local_matches(account_id):
    try:
        with open(f"matchdata/{account_id}.json", 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        return []


def _get_api_matches(account_id):
    return requests.get(f"https://api.opendota.com/api/players/{account_id}/matches").json()


def api_crawl():
    heroes = _get_heroes()
    accounts_file = _get_accounts()
    
    matches = Matches()

    for account in accounts_file:

        account_id = account['identifier']
        account_name = account['name']

        # Get all old matches
        local_matches = _get_local_matches(account_id)

        # Get all matches
        api_matches = _get_api_matches(account_id)

        # The amount of new games
        diff = len(api_matches) - len(local_matches)

        with open(f"matchdata/{account_id}.json", 'w', encoding='utf-8') as f:
            json.dump(api_matches, f, ensure_ascii=False, indent=2)

        if diff < 5:
            for i in range(diff):
                matches.add_result(api_matches[i], heroes, account_name)    
    
    return matches




