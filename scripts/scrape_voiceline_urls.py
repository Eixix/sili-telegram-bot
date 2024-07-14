"""
Scrape & parse URL links into a JSON dict for expedient & extensive voiceline retrieval.
To be run after response additions. Depends on the page layout of the wiki though,
so this will need to be adapted to changes there.
"""

from sili_telegram_bot.modules.voiceline_scraping import scrape_voiceline_urls

if __name__ == "__main__":
    # TODO Add argparser for output_dir.
    scrape_voiceline_urls()
