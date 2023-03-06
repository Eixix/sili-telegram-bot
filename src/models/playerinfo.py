from datetime import datetime
import pytz


class Playerinfo:

    def __init__(self,
                 name: str,
                 steam_name: str,
                 count_games: int,
                 wins: int,
                 loses: int,
                 win_rate: float,
                 last_game: datetime) -> None:
        self.name: str = name
        self.steam_name: str = steam_name
        self.count_games: int = count_games
        self.wins: int = wins
        self.loses: int = loses
        self.win_rate: float = win_rate
        self.last_game: datetime = last_game
        self.days_since_last_game: int = abs(
            (last_game.date() - datetime.now(pytz.timezone('Europe/Berlin')).date()).days)
