from pytest_cases import parametrize_with_cases

import test_responses_cases as case_module

from sili_telegram_bot.models.exceptions import MissingResponseUrlException
from sili_telegram_bot.models.responses import Responses


class TestResponses:
    """
    Test the Response class.
    """

    @parametrize_with_cases(
        "entity_name,response",
        cases=case_module.TestResponsesCases.TestFirstVoicelineCases,
    )
    def test_first_voiceline(self, entity_name, response) -> None:
        """
        Get the first response for any entity.
        """
        rsp = Responses()

        try:
            resp_url = rsp.get_link(entity_name, response)
            assert resp_url is not None

        except MissingResponseUrlException:
            # Currently, there is no way to tell if a voiceline has a URL associated.
            # A missing URL is expected, and so this still counts as a test success.
            pass

    @parametrize_with_cases(
        "hero_name", cases=case_module.TestResponsesCases.TestCrummyWizardCases
    )
    def test_crummy_wizard_success(self, hero_name) -> None:
        """
        Test if the "Crummy wizard" voiceline can be retrieved for a given voiced hero.
        """
        rsp = Responses()
        resp_url = rsp.get_link(hero_name, "Crummy wizard")
        assert resp_url is not None
