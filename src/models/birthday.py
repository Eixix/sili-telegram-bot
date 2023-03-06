import datetime


class Birthday:
    def __init__(self, name: str, date: str) -> None:
        self.name = name
        self.date = datetime.date(
            datetime.date.today().year, int(date[2:]), int(date[:2]))
        if datetime.date.today() > self.date:
            self.date = datetime.date(
                datetime.date.today().year + 1, int(date[2:]), int(date[:2]))
