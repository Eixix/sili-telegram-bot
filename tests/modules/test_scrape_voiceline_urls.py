import bs4
import pytest

from pytest_cases import parametrize_with_cases

import test_scrape_voiceline_urls_cases as case_module

from sili_telegram_bot.modules import voiceline_scraping


class TestProcessResponseText:
    @parametrize_with_cases(
        "input,expected",
        cases=case_module.TestProcessResponseTextCases,
    )
    def test_success(self, input, expected):
        res = voiceline_scraping.process_response_text(input)

        assert res == expected


class TestResponseFromLinkTag:
    @parametrize_with_cases(
        "link_tag,should_be_dict",
        cases=case_module.TestResponseFromLinkTagCases,
    )
    def test_success(self, link_tag: bs4.element.Tag, should_be_dict: bool) -> None:
        res = voiceline_scraping.response_from_link_tag(link_tag)

        if should_be_dict:
            assert isinstance(res, dict)
            assert isinstance(res["text"], str)
            assert isinstance(res["urls"], list)
            assert isinstance(res["urls"][0], str | None)
        else:
            assert res is None


class TestExtractResponseUrlsFromTitles:

    @pytest.mark.slow
    @parametrize_with_cases(
        "input",
        cases=case_module.TestExtractResponseUrlsFromTitlesCases,
    )
    def test_success(self, input):
        """
        Test basic functionality.
        """
        res = voiceline_scraping.extract_response_urls_from_titles(input)

        assert isinstance(res, dict)
        assert input == [*res.keys()]
