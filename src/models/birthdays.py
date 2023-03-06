from models.birthday import Birthday
import datetime
import json


class Birthdays:
    def __init__(self) -> None:
        self.birthdays = []
        with open("matchdata/accounts_file.json", 'r') as f:
            birthdays = json.load(f)
            for birthday in birthdays:
                self.birthdays.append(
                    Birthday(birthday["name"], birthday["birthday"]))

    def GetBirthdays(self) -> str:
        message = "Geburtstage:"
        for birthday in self.birthdays:
            message += '\n' + str(birthday.name) + ": " + \
                birthday.date.strftime("%d.%m.%Y")
            if birthday.date == datetime.date.today():
                message += " (Happy Birthday!)"
        return message

    def GetUpcomingBirthdays(self) -> str:
        message = ""
        for birthday in self.birthdays:
            if birthday.date == datetime.date.today() + datetime.timedelta(days=10):
                if message == "":
                    message += "Meine Damen und Herren der Sili-Gemeinde, darf ich Sie höflich darauf hinweisen," \
                               " dass in der nächsten Zeit ein Geburtstag ansteht. " \
                               "Bitte beraten Sie sich zeitnah, um ein adäquates Geschenk bereit zu haben."
                message += "\n\nGeburtstagskind: " + birthday. name + \
                    " - " + birthday.date.strftime("%d.%m.%Y")
        return message + "\n\nIhr freundlicher Sili-Bot" if message != "" else None

    def GetTodayBirthdays(self) -> str:
        message = ""
        for birthday in self.birthdays:
            if birthday.date == datetime.date.today():
                if message != "":
                    message += "\n"
                message += "Alles Gute zu deinem Geburtstag " + birthday. name + \
                    ". Du bist geil!\n\n@Rest: Jetzt noch schnell das Geschenk (Dota+) kaufen! Hopp Hopp!"
        return message + "\n\nDein geliebter Sili-Bot" if message != "" else None
