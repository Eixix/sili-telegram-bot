import requests
import bs4

class Voiceline:
    hero = ""
    response_url = ""

    # TODO: Should this be initialised like that?
    soup = bs4.BeautifulSoup()

    def __init__(self, hero_string):
        base_url = "https://dota2.fandom.com/wiki"

        # On the fandom wiki the pages for heroes follow the pattern of 
        # "base_url/Capitalized_Hero/subpage", so we need to ensure the hero
        # name follows that pattern   
        hero_list = hero_string.split(" ")
        self.hero = "_".join([x.capitalize() for x in hero_list])

        self.response_url = f"{base_url}/{self.hero}/Responses"

        vl_pg_response = requests.get(self.response_url)
        self.soup = bs4.BeautifulSoup(vl_pg_response.content, "lxml")

        # On the the wiki page all voiceline links are in tags which can be 
        # selected using this css selector.
        # TODO: check if this looks as expected (i.e. has content)
        self.vl_tags = self.soup.select(".mw-parser-output > ul li")

    def get_line(self, line):
        line_tag = ""

        for tag in self.vl_tags:
            # Each of the tags contains the audiobutton with the link to the
            # audiofile as its first child and as its second child the text 
            # of the voiceline. Here we find the tag containing the link to 
            # the desired line.
            # Note: Like this it will only perform basic string matching and
            # stop at the first find. This may not be desirable if different 
            # lines have the same text (different intonation).
            if line in tag.contents[1]:
                line_tag = tag
                break

        # TODO: Handle this properly
        if not line_tag:
            raise("Line not found")

        vl_link = line_tag.find("source")["src"]

        print(vl_link)

        pass