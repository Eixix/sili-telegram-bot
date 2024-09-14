import bs4
import pytest

from pytest_cases import parametrize

import test_infrastructure.common_case_infra as case_infra


def str_to_li_tag(x: str) -> bs4.element.Tag:
    return bs4.BeautifulSoup(
        x,
        features="html.parser",
    ).contents[0]


class TestProcessResponseTextCases:
    def case_basic(self):
        return ("Link▶️ Abaddon.", "Abaddon.")

    def case_unused(self):
        return ("Link▶️ u Deathcoil.", "Deathcoil.")

    def case_hero_icon(self):
        return (
            "Link▶️  You gave up too much for your honor, Dragonus.",
            "You gave up too much for your honor, Dragonus.",
        )

    def case_non_repeating(self):
        return (
            "Link▶️  r Tell me where to walk and not to walk, Techies.",
            "Tell me where to walk and not to walk, Techies.",
        )

    def case_icon_non_repeating(self):
        return ("Link▶️  r Ah, scepter.", "Ah, scepter.")

    def case_missing_file(self):
        return (" Prepare for repairs!", "Prepare for repairs!")

    def case_missing_file_unused(self):
        return (" u Easy mode.", "Easy mode.")

    def case_missing_file_non_repeating(self):
        return (" r Deathmatch.", "Deathmatch.")

    def case_cooldown(self):
        return ("Link▶️ 300 Ho!", "Ho!")

    def case_parens(self):
        return ("Link▶️ (Humming)", "(Humming)")

    def case_removed(self):
        return ("Link▶️ rem The battle begins!", "The battle begins!")

    def case_followup(self):
        return ("Link▶️ Welcome to Dota. followup", "Welcome to Dota.")

    def case_ellipsis(self):
        return ("Link▶️ Uh…ah…a chilling assault!", "Uh...ah...a chilling assault!")


class TestResponseFromLinkTagCases:
    def case_basic(self):
        return (
            str_to_li_tag(
                """<li><span><audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                <source src="https://liquipedia.net/commons/images/8/89/Vo_abaddon_abad_spawn_01.mp3" type="audio/mpeg"><a
                    href="https://liquipedia.net/commons/images/8/89/Vo_abaddon_abad_spawn_01.mp3">Link</a>
            </audio><a class="ext-audiobutton" data-state="play" title="Play/Pause"></a></span> Abaddon.</li>"""
            ),
            True,
        )

    def case_basic_with_link(self):
        return (
            str_to_li_tag(
                """<li><span><audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                            <source src="https://liquipedia.net/commons/images/a/a5/Vo_abaddon_abad_begin_02.mp3" type="audio/mpeg"><a
                                href="https://liquipedia.net/commons/images/a/a5/Vo_abaddon_abad_begin_02.mp3">Link</a>
                        </audio><a class="ext-audiobutton" data-state="play" title="Play/Pause"></a></span> The <a
                        href="/dota2game/Fog_of_war" class="mw-redirect" title="Fog of war">fog of war</a> is no match for the mist of
                    fate.</li>"""
            ),
            True,
        )

    def case_basic_unused(self):
        return (
            str_to_li_tag(
                """<li><span><audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                        <source src="https://liquipedia.net/commons/images/8/8b/Vo_abaddon_abad_deathcoil_01.mp3" type="audio/mpeg">
                        <a href="https://liquipedia.net/commons/images/8/8b/Vo_abaddon_abad_deathcoil_01.mp3">Link</a>
                    </audio><a class="ext-audiobutton" data-state="play" title="Play/Pause"></a></span> <i><small><abbr
                            title="Unused response">u</abbr></small></i> Deathcoil.</li>"""
            ),
            True,
        )

    def case_basic_non_repeating(self):
        return (
            str_to_li_tag(
                """<li><span><audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                        <source src="https://liquipedia.net/commons/images/3/3c/Vo_abaddon_abad_ally_05.mp3" type="audio/mpeg"><a
                            href="https://liquipedia.net/commons/images/3/3c/Vo_abaddon_abad_ally_05.mp3">Link</a>
                    </audio><a class="ext-audiobutton" data-state="play" title="Play/Pause"></a></span> <a href="/dota2game/Dazzle"
                    title="Dazzle"><img alt=""
                        src="/commons/images/thumb/8/84/Dazzle_mapicon_dota2_gameasset.png/16px-Dazzle_mapicon_dota2_gameasset.png"
                        decoding="async" width="16" height="16" class="pixelart"
                        srcset="/commons/images/thumb/8/84/Dazzle_mapicon_dota2_gameasset.png/24px-Dazzle_mapicon_dota2_gameasset.png 1.5x, /commons/images/8/84/Dazzle_mapicon_dota2_gameasset.png 2x"></a>
                <i><small><abbr title="Not-repeating response">r</abbr></small></i> The powers of ill reprieve are joined in us,
                Dazzle.</li>"""
            ),
            True,
        )

    def case_basic_with_ability_icon(self):
        return (
            str_to_li_tag(
                """<li><span><audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                        <source src="https://liquipedia.net/commons/images/5/5f/Vo_abaddon_abad_deathcoil_08.mp3" type="audio/mpeg">
                        <a href="https://liquipedia.net/commons/images/5/5f/Vo_abaddon_abad_deathcoil_08.mp3">Link</a>
                    </audio><a class="ext-audiobutton" data-state="play" title="Play/Pause"></a></span> <a href="/dota2game/Abaddon"
                    title="Abaddon"><img alt=""
                        src="/commons/images/thumb/9/93/Abaddon_mapicon_dota2_gameasset.png/16px-Abaddon_mapicon_dota2_gameasset.png"
                        decoding="async" width="16" height="16" class="pixelart"
                        srcset="/commons/images/thumb/9/93/Abaddon_mapicon_dota2_gameasset.png/24px-Abaddon_mapicon_dota2_gameasset.png 1.5x, /commons/images/9/93/Abaddon_mapicon_dota2_gameasset.png 2x"></a>
                <a href="/dota2game/Mist_Coil" title="Mist Coil"><img alt=""
                        src="/commons/images/thumb/b/be/Mist_Coil_abilityicon_dota2_gameasset.png/16px-Mist_Coil_abilityicon_dota2_gameasset.png"
                        decoding="async" width="16" height="16"
                        srcset="/commons/images/thumb/b/be/Mist_Coil_abilityicon_dota2_gameasset.png/24px-Mist_Coil_abilityicon_dota2_gameasset.png 1.5x, /commons/images/thumb/b/be/Mist_Coil_abilityicon_dota2_gameasset.png/32px-Mist_Coil_abilityicon_dota2_gameasset.png 2x"></a>
                By my right.</li>"""
            ),
            True,
        )

    def case_malformed_vl_li_tag(self):
        return (
            str_to_li_tag(
                """<li>Infusing Aghanim's Scepter into an ally triggers their "thanks" response.</li>"""
            ),
            False,
        )

    def case_missing_file(self):
        return (
            str_to_li_tag(
                """<li><a class="ext-audiobutton" data-state="error" title="File not found"></a> Prepare for repairs!</li>"""
            ),
            True,
        )

    def case_basic_with_cooldown(self):
        return (
            str_to_li_tag(
                """<li><span><audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                    <source src="https://liquipedia.net/commons/images/0/03/Vo_abaddon_abad_item_rare_01.mp3" type="audio/mpeg">
                    <a href="https://liquipedia.net/commons/images/0/03/Vo_abaddon_abad_item_rare_01.mp3">Link</a>
                </audio><a class="ext-audiobutton" data-state="play" title="Play/Pause"></a></span> <i><small><abbr
                        title="300 seconds cooldown">300</abbr></small></i> I claim this prize for the House Avernus.</li>"""
            ),
            True,
        )

    def case_with_arcana(self):
        return (
            str_to_li_tag(
                """<li class="">
                <span>
                    <audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                        <source src="https://liquipedia.net/commons/images/a/ad/Vo_phantom_assassin_phass_win_03.mp3"
                            type="audio/mpeg">
                        <a href="https://liquipedia.net/commons/images/a/ad/Vo_phantom_assassin_phass_win_03.mp3">Link</a>
                    </audio>
                    <a class="ext-audiobutton" data-state="play" title="Play/Pause"></a>
                </span>
                <span>
                    <audio hidden="" class="ext-audiobutton" preload="metadata" data-volume="1">
                        <source src="https://liquipedia.net/commons/images/6/68/Vo_phantom_assassin_phass_arc_win_03.mp3"
                            type="audio/mpeg">
                        <a href="https://liquipedia.net/commons/images/6/68/Vo_phantom_assassin_phass_arc_win_03.mp3">Link</a>
                    </audio>
                    <a class="ext-audiobutton" data-state="play" title="Play/Pause"></a>
                </span>
                Yes.
            </li>"""
            ),
            True,
        )

    def case_with_arcana_one_file_missing(self):
        return (
            str_to_li_tag(
                """<li class=""><a class="ext-audiobutton" data-state="error" title="File not found"></a> <span><audio hidden=""
                        class="ext-audiobutton" preload="metadata" data-volume="1">
                        <source src="https://liquipedia.net/commons/images/a/a9/Vo_phantom_assassin_phass_arc_move_13.mp3"
                            type="audio/mpeg"><a
                            href="https://liquipedia.net/commons/images/a/a9/Vo_phantom_assassin_phass_arc_move_13.mp3">Link</a>
                    </audio><a class="ext-audiobutton" data-state="play" title="Play/Pause"></a></span> I'm but a shadow.</li>"""
            ),
            True,
        )


class TestExtractResponseUrlsFromTitlesCases:

    def case_default_hero(self):
        return [case_infra.hero_name_to_tile(case_infra.default_hero())]

    @pytest.mark.skip(
        reason=(
            "I'm not sure pytest is set up to always use just one instance of "
            "the mediawiki API."
        )
    )
    @parametrize("hero_name", case_infra.remaining_heroes())
    def case_remaining_heroes(self, hero_name):
        return [case_infra.hero_name_to_tile(hero_name)]

    @parametrize("entity_title", case_infra.non_hero_entity_page_titles())
    def case_remaining_heroes(self, entity_title):
        return [entity_title]
