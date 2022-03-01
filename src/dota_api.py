import json
import requests


def _get_heroes():
    return requests.get("https://api.opendota.com/api/heroes").json()


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


def _generate_verb(kills, assists, deaths):
    meme_constant = 0

    if deaths == 0:
        meme_constant = (int(kills) + int(assists))
    else:
        meme_constant = (int(kills) + int(assists)) / int(deaths)

    verb = ""

    if meme_constant < 0.5:
        verb = "mächtigst abgestunken und wird hier der öffentlichen Schmach preisgegeben"
    elif meme_constant < 1:
        verb = "schlecht, aber nicht absolut erbärmlich gespielt"
    elif meme_constant < 2:
        verb = "immerhin neutral oder leicht besser gespielt"
    elif meme_constant < 5:
        verb = "mehr impact gehabt als erwartet, da kann man nicht meckern"
    elif meme_constant < 10:
        verb = "total rasiert"
    else:
        verb = "absolut gottartig gespielt und darf hier nicht mehr angezweifelt werden"

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

            try:
                hero_name = heroes[hero_id-2]['localized_name']
            except IndexError:
                hero_name = f"Unbekannter Held Nr. {hero_id}"

            verb = _generate_verb(kills, assists, deaths)

            messages.append(
                f"{account_name} hat mit {hero_name} {verb} mit {kills} Kills, {deaths} Toden und {assists} Assists")

    return messages


def api_crawl():
    heroes = _get_heroes()
    accounts_file = _get_accounts()

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

    return _get_messages(account_name, api_matches, diff, heroes)
