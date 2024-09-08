from sili_telegram_bot.models.birthday import Birthday
from sili_telegram_bot.modules.config import config
import datetime
from ast import literal_eval

ACCOUNTS_CONFIG = config["accounts"]


class Birthdays:
    def __init__(self) -> None:
        self.birthdays = []

        accounts = literal_eval(ACCOUNTS_CONFIG["account_list"])
        for account in accounts:
            self.birthdays.append(Birthday(account["name"], account["birthday"]))

    def GetBirthdays(self):
        message = "Geburtstage:"
        for birthday in self.birthdays:
            message += (
                "\n" + str(birthday.name) + ": " + birthday.date.strftime("%d.%m.%Y")
            )
            if birthday.date == datetime.date.today():
                message += " (Happy Birthday!)"
        return message

    def GetUpcomingBirthdays(self):
        message = ""
        for birthday in self.birthdays:
            if birthday.date == datetime.date.today() + datetime.timedelta(days=10):
                if message == "":
                    message += "Meine Damen und Herren der Sili-Gemeinde, darf ich Sie höflich darauf hinweisen, dass in der nächsten Zeit ein Geburtstag ansteht. Bitte beraten Sie sich zeitnah, um ein adäquates Geschenk bereit zu haben."
                message += (
                    "\n\nGeburtstagskind: "
                    + birthday.name
                    + " - "
                    + birthday.date.strftime("%d.%m.%Y")
                )
        return message + "\n\nIhr freundlicher Sili-Bot" if message != "" else None

    def GetTodayBirthdays(self):
        message = ""
        for birthday in self.birthdays:
            if birthday.date == datetime.date.today():
                if message != "":
                    message += "\n"
                message += (
                    "Alles Gute zu deinem Geburtstag "
                    + birthday.name
                    + ". Du bist geil!\n\n@Rest: Jetzt noch schnell das Geschenk (Dota+) kaufen! Hopp Hopp!"
                )
        return message + "\n\nDein geliebter Sili-Bot" if message != "" else None
