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
                "hero",
            )

        @pytest.mark.slow
        @parametrize("hero_name", case_infra.remaining_voiced_heroes())
        def case_remaining_heroes(self, hero_name):
            return (
                hero_name,
                case_infra.first_voiceline(case_infra.hero_name_to_tile(hero_name)),
                "hero",
            )

        @pytest.mark.slow
        @parametrize(
            "entity_name,entity_page_title,entity_type",
            zip(
                map(
                    case_infra.entity_title_to_name,
                    case_infra.non_hero_entity_page_titles(),
                ),
                case_infra.non_hero_entity_page_titles(),
                map(
                    case_infra.entity_title_to_type,
                    case_infra.non_hero_entity_page_titles(),
                ),
            ),
            ids=lambda input: input[0],
        )
        def case_remaining_entities(self, entity_name, entity_page_title, entity_type):
            return (
                entity_name,
                case_infra.first_voiceline(entity_page_title),
                entity_type,
            )
