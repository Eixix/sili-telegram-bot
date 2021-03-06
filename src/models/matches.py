from models.match import Match
import logging
from models.matchresult import MatchResult

class Matches:
    matches = []

    logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                    level=logging.INFO)
    logger = logging.getLogger(__name__)

    def __init__(self):
        self.matches = []

    def add_result(self, match_result, heroes, account_name):
        match_id = match_result['match_id']
        kills = match_result['kills']
        assists = match_result['assists']
        deaths = match_result['deaths']
        hero_id = match_result['hero_id']
        hero_name = ""

        win = False
        if (match_result['radiant_win'] and match_result['player_slot'] <= 127) or (not match_result['radiant_win'] and not match_result['player_slot'] <= 127):
            win = True
        try:
            hero_name = heroes[str(hero_id)]['localized_name']
        except IndexError:
            hero_name = f"Unbekannter Held Nr. {hero_id}"

        existing_match = next((m for m in self.matches if m.id == match_id), None)
        matchresult = MatchResult(account_name, kills, assists, deaths, hero_name)

        if existing_match:
            existing_match.add_matchresult(matchresult)
        else:
            self.matches.append(Match(match_id,win,matchresult))

    def get_matches(self):
        return self.matches
        



