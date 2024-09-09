import datetime


class Birthday:
    def __init__(self, name, date) -> None:
        self.name = name
        date_obj = datetime.datetime.strptime(date, "%d.%m.%Y")
        self.date = datetime.date(
            datetime.date.today().year, date_obj.month, date_obj.day
        )
        if datetime.date.today() > self.date:
            self.date = datetime.date(
                datetime.date.today().year + 1, date_obj.month, date_obj.day
            )
