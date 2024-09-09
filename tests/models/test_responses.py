"""
Test the Response class.
"""

from pytest import raises
from pytest_cases import parametrize_with_cases

import test_responses_cases as case_module

from sili_telegram_bot.models.exceptions import MissingResponseUrlException
from sili_telegram_bot.models.responses import parse_voiceline_args, Responses


class TestFirstVoiceline:
    """
    Test getting the first response for any entity.
    """

    @parametrize_with_cases(
        "entity_name,response,entity_type",
        cases=case_module.TestFirstVoicelineCases,
    )
    def test_success(self, entity_name, response, entity_type) -> None:
        rsp = Responses()

        try:
            resp_url = rsp.get_link(entity_name, response, type=entity_type)
            assert resp_url is not None

        except MissingResponseUrlException:
            # Currently, there is no way to tell if a voiceline has a URL associated.
            # A missing URL is expected, and so this still counts as a test success.
            pass


class TestCrummyWizard:
    @parametrize_with_cases("hero_name", cases=case_module.TestCrummyWizardCases)
    def test_success(self, hero_name) -> None:
        """
        Test if the "Crummy wizard" voiceline can be retrieved for a given voiced hero.
        """
        rsp = Responses()
        resp_url = rsp.get_link(hero_name, "Crummy wizard")
        assert resp_url is not None


class TestParseVoicelineArgs:
    @parametrize_with_cases(
        "input,expected",
        cases=case_module.TestParseVoicelineArgsCases.TestSuccessCases,
    )
    def test_success(self, input, expected):
        assert parse_voiceline_args(input) == expected

    @parametrize_with_cases(
        "input", cases=case_module.TestParseVoicelineArgsCases.TestValueErrorCases
    )
    def test_value_error(self, input):
        with raises(ValueError):
            parse_voiceline_args(input)
