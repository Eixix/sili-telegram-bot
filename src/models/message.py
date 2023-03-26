import random
import numpy as np
import logging
from models.match import Match
from models.playerinfo import Playerinfo
from models.matches import Matches
from models.matchresult import MatchResult


class Message:
    logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                        level=logging.INFO)
    logger = logging.getLogger(__name__)

    def __init__(self, matches: Matches, punlines: list[str], player_infos: Playerinfo) -> None:
        if matches is not None:
            self.matches: Matches = matches

        if punlines is not None:
            self.punlines = punlines
            self.verb_numbers = {
                key: len(value) for key, value in punlines['performance_verbs'].items()}
            self.meme_const_cats = [
                key for key in punlines['performance_verbs'].keys()]
            self.meme_const_cats_flt = np.array(
                [float(mc_cat) for mc_cat in self.meme_const_cats])
            self.meme_const_cats_sort = np.sort(self.meme_const_cats_flt)
            self._reset_used_verbs()

        if player_infos is not None:
            self.player_infos = player_infos

    def get_messages_for_matches(self) -> list[str]:
        messages = []
        for match in self.matches.get_matches():
            # Only send messages with more than 1 player from sili
            if len(match.matchresults) > 1:
                messages.append(self._create_message_for_match(match))
        return messages

    def get_message_for_player_infos(self) -> str:
        messages = []
        for player_info in self.playerinfos:
            messages.append(self._create_message_for_playerinfos(player_info))
        return '\n\n'.join(messages)

    def _create_message_for_player_infos(self, playerinfo: Playerinfo) -> str:
        messages = [f"<b>{playerinfo.name}</b>",
                    f"Steamname: {playerinfo.steamname}",
                    f"Anzahl Spiele: {playerinfo.count_games}",
                    f"Siege: {playerinfo.wins}",
                    f"Niederlagen: {playerinfo.loses}",
                    f"Win rate: {playerinfo.win_rate}",
                    f"Letztes Spiel: {playerinfo.last_game.strftime('%d.%m.%Y %H:%M:%S')} "
                    f"({playerinfo.days_since_last_game} Tag(e) vergangen)"]
        return '\n'.join(messages)

    def _create_message_for_match(self, match: Match) -> str:
        messages = []

        if match.win:
            messages.append(
                f"<b>W: {random.choice(self.punlines['match_outcome']['win'])}</b>")
        else:
            messages.append(
                f"<b>L: {random.choice(self.punlines['match_outcome']['lose'])}</b>")

        for match_result in match.matchresults:
            messages.append(
                f"{match_result.name} hat mit {match_result.hero} {self._generate_verb(match_result)} "
                f"mit {match_result.kills} Kills, {match_result.deaths} Toden und {match_result.assists} Assists")

        self._reset_used_verbs()

        return '\n\n'.join(messages)

    def _reset_used_verbs(self, cat: str = None) -> None:
        if cat is None:
            self.used_verbs = {key: [] for key in self.verb_numbers.keys()}
        else:
            if type(cat) is str:
                if cat not in self.meme_const_cats:
                    raise f"'{cat}' is no a valid category."
                self.used_verbs[cat] = []
            elif type(cat) is list:
                # FIXME: make the error say which categories are invalid
                if not all([x in self.meme_const_cats for x in cat]):
                    raise f"'{cat}' contains invalid categories."
                for cat_i in cat:
                    self.used_verbs[cat_i] = []

    def _generate_verb(self, match_result: MatchResult) -> str:
        verb = ""

        # In case meme constant categories are not sorted in increasing order,
        # the index based finding of the appropriate meme constant category will
        # not work as intended. That is why we first get a lookup array mapping
        # the orderd array back to the order the categories are provided in the
        # punlines file. We then get the number giving the nth largest category
        # and use the lookup array to find out to which index that corresponds.
        #
        # All of this could be avoided by converting the category strings to
        # floats and back again, but I ran into the trouble that naive
        # conversion left trailing zeroes in the strings. This meant that the
        # dict lookup didn´t work.
        mc_cat_lookup = self.meme_const_cats_flt.argsort()

        order_idx_arr = np.where(
            self.meme_const_cats_sort > match_result.meme_constant)[0]
        mc_cat_idx = mc_cat_lookup[order_idx_arr[0]]

        cat = self.meme_const_cats[mc_cat_idx]
        cat = str(cat)

        # In order to ensure verbs are not used again if necessary, get how
        # many verbs there are for each category and randomly choose an index
        # used for selecting each new verb from the punlines dict. These indices
        # are recorded and are not elegible to be chosen again until the record
        # is reset.
        cat_verb_n = self.verb_numbers[cat]

        if len(self.used_verbs[cat]) >= cat_verb_n:
            self.logger.info(f"Out of unique verbs for category {cat}, "
                             "resetting list of used verbs...")
            self._reset_used_verbs(cat)

        verb_idx_list = [* range(0, cat_verb_n)]
        verb_idx_set_unused = set(verb_idx_list) - set(self.used_verbs[cat])

        verb_idx = random.choice([* verb_idx_set_unused])
        self.used_verbs[cat].append(verb_idx)

        verb = self.punlines["performance_verbs"][cat][verb_idx]

        return verb

    def get_message_for_playerinfos(self):
        pass
