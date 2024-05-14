from pytest_cases import parametrize_with_cases

import test_voiceline_cases as case_module

from models.voiceline import Voiceline


class TestVoiceline:
    """
    Test the Voiceline class.
    """

    @parametrize_with_cases("hero_name", cases=case_module.TestVoicelineCases)
    def test_crummy_wizard_success(self, hero_name) -> None:
        """
        Test if the "Crummy wizard" voiceline can be retrieved for a given hero.
        (That voiceline exists for every hero *so far*.)
        """
        vl = Voiceline(hero_name)
        assert vl.get_link("Crummy wizard") is not None
