import json
import requests
import logging
import pytz
from datetime import datetime
from models.matches import Matches
from models.playerinfo import Playerinfo

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


def get_playerinfos():
    playerinfos = []

    accounts_file = _get_accounts()
    for account in accounts_file:
        account_id = account['identifier']

        player = requests.get(f"https://api.opendota.com/api/players/{account_id}").json()
        last_match = requests.get(f"https://api.opendota.com/api/players/{account_id}/matches?limit=1").json()
        wins_loses = requests.get(f"https://api.opendota.com/api/players/{account_id}/wl").json()
 
        playerinfos.append(Playerinfo(account['name'], 
                                        player["profile"]["personaname"], 
                                        wins_loses["win"] + wins_loses["lose"], 
                                        wins_loses["win"], 
                                        wins_loses["lose"], 
                                        round(wins_loses["win"]/wins_loses["lose"], 2), 
                                        pytz.utc.localize(datetime.utcfromtimestamp(last_match[0]["start_time"] + last_match[0]["duration"])).astimezone(pytz.timezone('Europe/Berlin'))))
    return playerinfos

def get_lastgame():
    lastgame = 0
    accounts_file = _get_accounts()
    for account in accounts_file:
        last_match = requests.get(f"https://api.opendota.com/api/players/{account['identifier']}/matches?limit=1").json()
        match_end = last_match[0]["start_time"] + last_match[0]["duration"]
        if (match_end > lastgame):
            lastgame = match_end
   
    return pytz.utc.localize(datetime.utcfromtimestamp(lastgame)).astimezone(pytz.timezone('Europe/Berlin')).strftime('%d.%m.%Y %H:%M:%S')

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




