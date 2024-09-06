import pytest

from pytest_cases import parametrize

import test_infrastructure.common_case_infra as case_infra


class TestResponsesCases:

    class TestCrummyWizardCases:
        def case_default(self):
            """
            Generate a case just for Keeper of the Light as a (almost) random hero.
            """
            return case_infra.default_hero()

        @pytest.mark.slow
        @parametrize("hero_name", case_infra.remaining_voiced_heroes())
        def case_remaining_voiced_heroes(self, hero_name: str):
            """
            Generate cases involving all known hero names.
            """
            return hero_name

    class TestFirstVoicelineCases:
        def case_default(self):
            # FIXME This conversion from hero to entity name is pretty hacky, but
            # adding the whole lookup logic here seems silly as .
            return (
                case_infra.default_hero(),
                case_infra.first_voiceline(case_infra.default_hero() + "/Responses"),
            )

        @pytest.mark.slow
        @parametrize("hero_name", case_infra.remaining_voiced_heroes())
        def case_remaining_heroes(self, hero_name):
            # FIXME This conversion from hero to entity name is pretty hacky, but
            # adding the whole lookup logic here seems silly.
            return (hero_name, case_infra.first_voiceline(hero_name + "/Responses"))
