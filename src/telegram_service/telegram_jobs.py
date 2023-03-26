import logging
from datetime import datetime
from random import random

from telegram.constants import ParseMode
from telegram.ext import ContextTypes

from src import dota_api
from src.models.birthdays import Birthdays
from src.models.helper import _weekdaynumber_to_weekday
from src.models.message import Message
from src.models.patch_checker import PatchChecker
from src.repositories.env_variables import CHAT_ID
from src.repositories.punlines import get_punlines, PunlinesTypes

logger = logging.getLogger(__name__)


async def send_dota_matches(context: ContextTypes.DEFAULT_TYPE) -> None:
    message = Message(dota_api.api_crawl(), None)
    messages = message.get_messages_for_matches()

    if messages:
        for m in messages:
            await context.bot.send_message(chat_id=CHAT_ID,
                                           text=m,
                                           parse_mode=ParseMode.HTML)


async def get_if_new_patch(context: ContextTypes.DEFAULT_TYPE) -> None:
    patch_checker = PatchChecker.get_instance()
    new_patch_exists, new_patch_number = patch_checker.get_if_new_patch()
    if new_patch_exists:
        await context.bot.send_message(chat_id=CHAT_ID,
                                       text=f"Es gibt ein neues Dota2 Update! Gameplay Update {new_patch_number} "
                                            f"\n https://www.dota2.com/patches/{new_patch_number}",
                                       parse_mode=ParseMode.HTML)


async def poll(context: ContextTypes.DEFAULT_TYPE) -> None:
    logger.info("DODO")

    dodo_poll_punlines = get_punlines(PunlinesTypes.Dodo_Poll)
    questions = [random.choice(dodo_poll_punlines["ja"]),
                 random.choice(dodo_poll_punlines["nein"])]

    await context.bot.send_voice(chat_id=CHAT_ID,
                                 voice=open('resources/lets_dota.mpeg', 'rb'))

    weekday = _weekdaynumber_to_weekday(datetime.datetime.today().weekday())

    await context.bot.send_poll(
        CHAT_ID,
        f"Do{weekday}",
        questions,
        is_anonymous=False,
        allows_multiple_answers=False,
    )


async def upcoming_birthdays(context: ContextTypes.DEFAULT_TYPE) -> None:
    birthdays = Birthdays().GetUpcomingBirthdays()
    if birthdays is not None:
        await context.bot.send_message(chat_id=CHAT_ID,
                                       text=birthdays,
                                       parse_mode=ParseMode.HTML)


async def today_birthdays(context: ContextTypes.DEFAULT_TYPE) -> None:
    birthdays = Birthdays().GetTodayBirthdays()
    if birthdays is not None and birthdays != "":
        await context.bot.send_message(chat_id=CHAT_ID,
                                       text=birthdays,
                                       parse_mode=ParseMode.HTML)
