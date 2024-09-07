class Match:
    id = 0
    win = True
    matchresults = []

    def __init__(self, id, win, matchresult):
        self.id = id
        self.win = win
        self.matchresults = []
        self.matchresults.append(matchresult)

    def add_matchresult(self, matchresult):
        self.matchresults.append(matchresult)
