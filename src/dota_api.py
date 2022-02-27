import json, requests

def api_crawl():
    messages = []
    heroes = requests.get("https://api.opendota.com/api/heroes").json()

    accounts_file = []
    with open("accounts_file.json", 'r') as f:
        accounts_file = json.load(f)

    for account in accounts_file:

        account_id = list(account)[0]
        account_name = list(account.values())[0]

        # Get all matches
        api_matches = requests.get(f"https://api.opendota.com/api/players/{account_id}/matches").json()

        # Get all old matches
        local_matches = []
        with open(f"matchdata/{account_id}.json", 'r') as f:
            local_matches = json.load(f)

        # The amount of new games
        diff = len(api_matches) - len(local_matches)

        if diff < 5:
            # Create a new punline for every new game
            for i in range(diff):
                kills = api_matches[i]['kills']
                assists = api_matches[i]['assists']
                deaths = api_matches[i]['deaths']
                hero_id = api_matches[i]['hero_id']
                hero_name = heroes[hero_id-2]['localized_name']

                meme_constant = 0
                verb = ""
                try:
                    meme_constant = (int(kills) + int(assists)) / int(deaths)

                    if meme_constant < 0.5:
                        verb = "mächtigst abgestunken und wird hier der öffentlichen Schmach preisgegeben"
                    elif meme_constant < 1:
                        verb = "hat schlecht, aber nicht absolut erbärmlich gespielt"
                    elif meme_constant < 2:
                        verb = "immerhin neutral oder leicht besser gespielt"
                    elif meme_constant < 5:
                        verb = "mehr impact gehabt als erwartet, da kann man nicht meckern"
                    else:
                        verb = "total rasiert"
                except ZeroDivisionError:
                    verb = "total abartig rasiert oder ist einfach nur nicht gestorben weil er afk war"

                messages.append(f"{account_name} hat mit {hero_name} {verb} mit {kills} Kills, {deaths} Toden und {assists} Assists")


        with open(f"matchdata/{account_id}.json", 'w', encoding='utf-8') as f:
            json.dump(api_matches, f, ensure_ascii=False, indent=2)
    
    return messages