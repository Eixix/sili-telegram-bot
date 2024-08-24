from datetime import datetime
import pytz


class Playerinfo:
    name = ""
    steamname = ""
    count_games = 0
    wins = 0
    loses = 0
    win_rate = 0.00
    last_game = datetime
    days_since_last_game = 0

    def __init__(self, name, steamname, count_games, wins, loses, win_rate, last_game):
        self.name = name
        self.steamname = steamname
        self.count_games = count_games
        self.wins = wins
        self.loses = loses
        self.win_rate = win_rate
        self.last_game = last_game
        self.days_since_last_game = abs(
            (
                last_game.date() - datetime.now(pytz.timezone("Europe/Berlin")).date()
            ).days
        )
