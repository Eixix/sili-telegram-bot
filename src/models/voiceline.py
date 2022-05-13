import requests
import bs4 as bs

class Voiceline:
    hero = ""
    response_url = ""

    def __init__(self, hero_string):
        base_url = "https://dota2.fandom.com/wiki/"

        # On the fandom wiki the pages for heroes follow the pattern of 
        # "base_url/Capitalized_Hero/subpage", so we need to ensure the hero
        # name follows that pattern   
        hero_list = hero_string.split(" ")
        self.hero = "_".join([x.upper() for x in hero_list])

        self.response_url = f"{self.base_url}/{self.hero}/Responses"

    def get_line(self, line):
        # TODO: finish this function
        pass