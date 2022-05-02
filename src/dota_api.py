import json
import requests
import random
import logging

logger = logging.getLogger(__name__)
punlines = {}
used_verbs = []
verb_decay = 5

with open("../resources/punlines.json", 'r') as f:
    punlines = json.load(f)


def _get_heroes():
    with open("../resources/heroes.json", 'r') as f:
        return json.load(f)


def _get_accounts():
    with open("../matchdata/accounts_file.json", 'r') as f:
        return json.load(f)


def _get_local_matches(account_id):
    try:
        with open(f"../matchdata/{account_id}.json", 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        return []


def _get_api_matches(account_id):
    return requests.get(f"https://api.opendota.com/api/players/{account_id}/matches").json()


def _check_verb(verb):
    if not verb in used_verbs:
        used_verbs = used_verbs[1:verb_decay - 1] + [verb]
        queue.put(verb)
        return True
    else:
        return False


def _generate_verb(kills, assists, deaths):
    meme_constant = 0

    if deaths == 0:
        meme_constant = (int(kills) + int(assists))
    else:
        meme_constant = (int(kills) + int(assists)) / int(deaths)

    verb = ""

    if meme_constant < 0.5:
        verb = random.choice(punlines[0]["<0.5"])
    elif meme_constant < 1:
        verb = random.choice(punlines[0]["<1"])
    elif meme_constant < 2:
        verb = random.choice(punlines[0]["<2"])
    elif meme_constant < 5:
        verb = random.choice(punlines[0]["<5"])
    elif meme_constant < 10:
        verb = random.choice(punlines[0]["<10"])
    else:
        verb = random.choice(punlines[0][">10"])

    return verb


def _get_messages(account_name, api_matches, diff, heroes):

    messages = []

    # Only for less then 5 matches, for new files
    if diff < 5:
        # Create a new punline for every new game
        for i in range(diff):
            kills = api_matches[i]['kills']
            assists = api_matches[i]['assists']
            deaths = api_matches[i]['deaths']
            hero_id = api_matches[i]['hero_id']
            hero_name = ""

            win = "L"
            if (api_matches[i]['radiant_win'] and api_matches[i]['player_slot'] <= 127) or (not api_matches[i]['radiant_win'] and not api_matches[i]['player_slot'] <= 127):
                win = "W"

            try:
                hero_name = heroes[str(hero_id)]['localized_name']
                logger.info(hero_name)
            except IndexError:
                hero_name = f"Unbekannter Held Nr. {hero_id}"

            verb = _generate_verb(kills, assists, deaths)

            # FIXME: check if this is the most efficient way to reroll something
            while not _check_verb:
                verb = _generate_verb(kills, assists, deaths)

            messages.append(
                f"{win}: {account_name} hat mit {hero_name} {verb} mit {kills} Kills, {deaths} Toden und {assists} Assists")

    return messages


def api_crawl():
    heroes = _get_heroes()
    accounts_file = _get_accounts()
    messages = []

    for account in accounts_file:

        account_id = list(account)[0]
        account_name = list(account.values())[0]

        # Get all old matches
        local_matches = _get_local_matches(account_id)

        # Get all matches
        api_matches = _get_api_matches(account_id)

        # The amount of new games
        diff = len(api_matches) - len(local_matches)

        with open(f"../matchdata/{account_id}.json", 'w', encoding='utf-8') as f:
            json.dump(api_matches, f, ensure_ascii=False, indent=2)

        messages.extend(_get_messages(account_name, api_matches, diff, heroes))

    return messages
