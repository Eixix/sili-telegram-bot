import requests
import bs4
import regex


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
        self.soup = bs4.BeautifulSoup(vl_pg_response.content,
                                      features="html.parser")

        # On the the wiki page all voiceline links are in tags which can be
        # selected using this css selector.
        # TODO: check if this looks as expected (i.e. has content)
        self.vl_tags = self.soup.select("#mw-content-text h2~ ul li")

    def get_line(self, line):
        line_tag = ""

        fuzzy_rules = "{e<=2}"
        line_re = regex.compile(f"(?:{regex.escape(line)}){fuzzy_rules}",
                                flags=regex.IGNORECASE)

        for tag in self.vl_tags:
            # Each of the tags contains the audiobutton with the link to the
            # audiofile as its first child and as its second child the text
            # of the voiceline. Here we find the tag containing the link to
            # the desired line.
            # Note: Like this it will only perform basic string matching and
            # stop at the first find. This may not be desirable if different
            # lines have the same text (different intonation).
            if regex.search(line_re, tag.contents[-1]):
                line_tag = tag
                break

        if not line_tag:
            message = "Sorry, konnte die line nicht finden..."
        else:
            vl_link = line_tag.find("source")["src"]
            message = vl_link

        return message
