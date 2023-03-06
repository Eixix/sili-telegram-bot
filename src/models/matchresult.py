class MatchResult:
    name = ""
    kills = 0
    assists = 0
    deaths = 0
    meme_constant = 0.00
    hero = ""

    def __init__(self, name: str, kills: int, assists: int, deaths: int, hero: str) -> None:
        self.name: str = name
        self.kills: int = kills
        self.assists: int = assists
        self.deaths: int = deaths
        self.meme_constant: float = int(kills) + int(assists) \
            if self.deaths == 0 \
            else (int(kills) + int(assists)) / int(deaths)
        self.hero: str = hero
