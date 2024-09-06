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
            return (
                case_infra.default_hero(),
                case_infra.first_voiceline(
                    case_infra.hero_name_to_tile(case_infra.default_hero())
                ),
            )

        @pytest.mark.slow
        @parametrize("hero_name", case_infra.remaining_voiced_heroes())
        def case_remaining_heroes(self, hero_name):
            return (
                hero_name,
                case_infra.first_voiceline(case_infra.hero_name_to_tile(hero_name)),
            )

        @pytest.mark.slow
        @pytest.mark.xfail(reason="Entity line getting not yet implemented.")
        @parametrize(
            "entity_name,entity_page_title",
            zip(
                case_infra.non_hero_entity_names(),
                case_infra.non_hero_entity_page_titles(),
            ),
            ids=lambda input: input[0],
        )
        def case_remaining_entities(self, entity_name, entity_page_title):
            return (
                entity_name,
                case_infra.first_voiceline(entity_page_title),
            )
