from models.matchresult import MatchResult


class Match:
    def __init__(self, id: int, win: bool, matchresult: MatchResult) -> None:
        self.id: int = id
        self.win: bool = win
        self.matchresults: list[MatchResult] = [matchresult]

    def add_matchresult(self, matchresult: MatchResult) -> None:
        self.matchresults.append(matchresult)
