from pytest import fixture
from sili_telegram_bot.models.mediawiki_api import APIWrapper


@fixture(scope="module")
def mediawiki_api():
    """
    Provide the MediawikiAPI instance, the same across tests.
    """
    return APIWrapper.get_or_create_mediawiki_api()
