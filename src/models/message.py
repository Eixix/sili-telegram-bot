from models.matches import Matches
import random
import logging
from models.match import Match
from models.playerinfo import Playerinfo
import pytz

class Message:
    matches = Matches
    punlines = {}
    playerinfos = []

    logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                    level=logging.INFO)
    logger = logging.getLogger(__name__)

    def __init__(self, matches, punlines,playerinfos):
        self.matches = matches
        self.punlines = punlines
        self.playerinfos = playerinfos
        
    def get_messages_for_matches(self):
        messages = []
        for match in self.matches.get_matches():
            messages.append(self._create_message_for_match(match))
        return messages

    def get_message_for_playerinfos(self):
        messages = []
        for playerinfo in self.playerinfos:
            messages.append(self._create_message_for_playerinfos(playerinfo))
        return '\n\n'.join(messages)

    def _create_message_for_playerinfos(self, playerinfo: Playerinfo):
        messages = []
        messages.append(f"<b>{playerinfo.name}</b>")
        messages.append(f"Steamname: {playerinfo.steamname}")
        messages.append(f"Anzahl Spiele: {playerinfo.count_games}")
        messages.append(f"Siege: {playerinfo.wins}")
        messages.append(f"Niederlagen: {playerinfo.loses}")
        messages.append(f"Win rate: {playerinfo.win_rate}")
        messages.append(f"Letztes Spiel: {playerinfo.last_game.strftime('%d.%m.%Y %H:%M:%S')} ({playerinfo.days_since_last_game} Tag(e) vergangen)")
        return '\n'.join(messages)

    def _create_message_for_match(self, match: Match):
        messages = []

        if match.win:
            messages.append(f"<b>W: {random.choice(self.punlines['win'])}</b>")
        else:
            messages.append(f"<b>L: {random.choice(self.punlines['lose'])}</b>")

        for matchresult in match.matchresults:
            messages.append(f"{matchresult.name} hat mit {matchresult.hero} {self._generate_verb(matchresult)} mit {matchresult.kills} Kills, {matchresult.deaths} Toden und {matchresult.assists} Assists")

        return '\n\n'.join(messages)

    def _generate_verb(self, matchresult):
        verb = ""
        if matchresult.meme_constant < 0.5:
            verb = random.choice(self.punlines["<0.5"])
        elif matchresult.meme_constant < 1:
            verb = random.choice(self.punlines["<1"])
        elif matchresult.meme_constant < 2:
            verb = random.choice(self.punlines["<2"])
        elif matchresult.meme_constant < 5:
            verb = random.choice(self.punlines["<5"])
        elif matchresult.meme_constant < 10:
            verb = random.choice(self.punlines["<10"])
        else:
            verb = random.choice(self.punlines[">10"])

        return verb