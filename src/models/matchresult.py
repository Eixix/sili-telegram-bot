class MatchResult:
    name = ""
    kills = 0
    assists = 0
    deaths = 0
    meme_constant = 0.00
    hero = ""

    def __init__(self, name, kills, assists, deaths, hero):
        self.name = name
        self.kills = kills
        self.assists = assists
        self.deaths = deaths
        if self.deaths == 0:
            self.meme_constant = (int(kills) + int(assists)/2)
        else:
            self.meme_constant = (int(kills) + int(assists)/2) / int(deaths)
        self.hero = hero