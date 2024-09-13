#!/usr/bin/env python3

import datetime
from telegram import Update, User, error
from telegram.constants import ParseMode
from telegram.ext import (
    Application,
    CallbackContext,
    CommandHandler,
    MessageHandler,
    filters,
)
import sili_telegram_bot.dota_api as dota_api
import json
import logging
import random
import os

from apscheduler.schedulers.background import BackgroundScheduler
from apscheduler.jobstores.memory import MemoryJobStore
from apscheduler.triggers import date, interval
from dataclasses import asdict

from sili_telegram_bot.modules.config import config
from sili_telegram_bot.models.message import Message
from sili_telegram_bot.models.patch_checker import PatchChecker
from sili_telegram_bot.models.responses import parse_voiceline_args, Responses
from sili_telegram_bot.models.birthdays import Birthdays
from sili_telegram_bot.modules.voiceline_scraping import get_response_data

RESOURCE_CONFIG = config["static_resources"]
SECRETS = config["secrets"]


logger = logging.getLogger(__name__)

patch_checker = PatchChecker()


def get_app(bot_token: str) -> Application:
    """
    Build and return a telegram Application.
    """
    return Application.builder().token(bot_token).build()


def get_punlines(punline_path: str) -> dict:
    """
    Load punlines from file described in config.
    """
    with open(punline_path, "r", encoding="utf8") as f:
        return json.load(f)


PUNLINES = get_punlines(RESOURCE_CONFIG["punline_path"])
SILI_BOT_APP = get_app(SECRETS["bot_token"])


async def get_dota_matches(context: CallbackContext) -> None:
    message = Message(dota_api.api_crawl(), PUNLINES, None)
    messages = message.get_messages_for_matches()

    if messages:
        for m in messages:
            context.bot.send_message(
                chat_id=SECRETS["chat_id"], text=m, parse_mode=ParseMode.HTML
            )


def _weekdaynumber_to_weekday(weekdaynumber: int) -> str:
    match weekdaynumber:
        case 0:
            return "Mo"
        case 1:
            return "Di"
        case 2:
            return "Mi"
        case 3:
            return "Do"
        case 4:
            return "Fr"
        case 5:
            return "Sa"
        case 6:
            return "So"


async def poll(context: CallbackContext) -> None:
    logger.info(f"DODO")

    questions = [
        random.choice(PUNLINES["dodo_poll"]["ja"]),
        random.choice(PUNLINES["dodo_poll"]["nein"]),
    ]

    await context.bot.send_voice(
        chat_id=SECRETS["chat_id"],
        voice=open(RESOURCE_CONFIG["dodo_voiceline_path"], "rb"),
    )

    weekday = _weekdaynumber_to_weekday(datetime.datetime.today().weekday())

    await context.bot.send_poll(
        SECRETS["chat_id"],
        f"Do{weekday}",
        questions,
        is_anonymous=False,
        allows_multiple_answers=False,
    )


def user_to_representation(user: User):
    """
    Generate *some* string identifying a telegram user based on what information is
    available.

    username > first name (maybe last name, if available) > User ID > "Unknown User"
    """
    # All the fields we need for this
    id_field_names = ["username", "first_name", "last_name", "id"]
    user_dict = user.to_dict()

    representation_dict = {
        key: value for key, value in user_dict.items() if key in id_field_names
    }

    if len(representation_dict) == 0:
        return "Unknown user"

    else:
        if "username" in representation_dict:
            return representation_dict["username"]
        elif "first_name" in representation_dict:
            name = representation_dict["first_name"]
            if "last_name" in representation_dict:
                name += representation_dict["last_name"]

            return name
        elif "id" in representation_dict:
            return str(representation_dict["id"])


async def voiceline(update: Update, context: CallbackContext) -> None:
    if update.effective_chat.id == int(SECRETS["chat_id"]):
        logger.info("Getting voiceline...")

        try:
            voiceline_args = asdict(parse_voiceline_args(context.args))

        except ValueError as e:
            logger.error(f"Error while parsing voiceline args: {str(e)}")
            await context.bot.send_message(chat_id=SECRETS["chat_id"], text=str(e))

            return None

        try:
            responses = Responses()

            vl_link = responses.get_link(**voiceline_args)

        except Exception as e:
            entity = voiceline_args["entity"]
            logger.error(f"Error while attempting to get voiceline for {entity}: {e}")

            await context.bot.send_message(
                chat_id=SECRETS["chat_id"],
                text=str(e),
            )

            return None

        vl_file_path = responses.download_mp3(vl_link)

        try:
            # Delete /voiceline to make conversation more seamless
            try:
                await context.bot.delete_message(
                    chat_id=SECRETS["chat_id"],
                    message_id=update.message.message_id,
                )
            except error.BadRequest as e:
                logger.error(
                    f"Error attempting to delete message: {e}. Likely "
                    f"insufficient permissions for the bot in this chat. Try "
                    f"giving it the `can_delete_messages` permission. See "
                    f"<https://docs.python-telegram-bot.org/en/stable/telegram.bot.html#telegram.Bot.delete_message> "
                    f"for more info."
                )

            sender_name = user_to_representation(update.message.from_user)

            await context.bot.send_message(
                chat_id=SECRETS["chat_id"], text=sender_name + ":"
            )
            await context.bot.send_voice(
                chat_id=SECRETS["chat_id"],
                voice=open(vl_file_path, "rb"),
            )

            logger.info("... voiceline delivered.")

        finally:
            os.remove(vl_file_path)


async def crawl(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(SECRETS["chat_id"]):
        await get_dota_matches(context)


async def dodo(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(SECRETS["chat_id"]):
        await poll(context)


async def playerinfos(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(SECRETS["chat_id"]):
        message = Message(None, None, dota_api.get_playerinfos())
        messages = message.get_message_for_playerinfos()

        if messages:
            await context.bot.send_message(
                chat_id=SECRETS["chat_id"],
                text=messages,
                parse_mode=ParseMode.HTML,
            )


async def lastgame(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(SECRETS["chat_id"]):
        time = dota_api.get_lastgame()

        if time:
            await context.bot.send_message(
                chat_id=SECRETS["chat_id"],
                text=time,
                parse_mode=ParseMode.HTML,
            )


async def birthdays(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(SECRETS["chat_id"]):
        await context.bot.send_message(
            chat_id=SECRETS["chat_id"],
            text=Birthdays().GetBirthdays(),
            parse_mode=ParseMode.HTML,
        )


async def upcomingBirthdays(context: CallbackContext):
    upcomingBirthdays = Birthdays().GetUpcomingBirthdays()
    if upcomingBirthdays != None:
        await context.bot.send_message(
            chat_id=SECRETS["chat_id"],
            text=upcomingBirthdays,
            parse_mode=ParseMode.HTML,
        )


async def todayBirthdays(context: CallbackContext):
    todayBirthdays = Birthdays().GetTodayBirthdays()
    if todayBirthdays != None and todayBirthdays != "":
        await context.bot.send_message(
            chat_id=SECRETS["chat_id"],
            text=todayBirthdays,
            parse_mode=ParseMode.HTML,
        )


async def stopbot(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(SECRETS["chat_id"]) and SILI_BOT_APP.running:
        await SILI_BOT_APP.stop()


async def message_handler(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(SECRETS["chat_id"]):
        message_text = update.message.text.lower()
        if "doubt" in message_text or "daud" in message_text or "daut" in message_text:
            await context.bot.send_animation(
                chat_id=SECRETS["chat_id"],
                animation=open(RESOURCE_CONFIG["daut_gif_path"], "rb"),
            )


async def get_if_new_patch(context: CallbackContext) -> None:
    new_patch_exists, new_patch_number = patch_checker.get_if_new_patch()
    if new_patch_exists:
        await context.bot.send_message(
            chat_id=SECRETS["chat_id"],
            text=f"Es gibt ein neues Dota2 Update! Gameplay Update {new_patch_number} \n https://www.dota2.com/patches/{new_patch_number}",
            parse_mode=ParseMode.HTML,
        )


def get_and_config_scheduler() -> BackgroundScheduler:
    """
    Initialize Scheduler and add non-telegram jobs.
    """
    scheduler = BackgroundScheduler(jobstores={"non-telegram": MemoryJobStore()})

    # Right after startup, get all dynamic resources.
    scheduler.add_job(dota_api.update_heroes, trigger=date.DateTrigger())
    scheduler.add_job(get_response_data, trigger=date.DateTrigger())

    # And repeating, trying to catch a new patch, assuming it is out on the night from
    # thursday to friday at 2AM.
    # FIXME Use an event from get_if_new_patch for this?
    today_weekday = datetime.date.today().isoweekday()
    thursday_weekday = 4
    next_thursday = datetime.date.today() + datetime.timedelta(
        days=(thursday_weekday - today_weekday) % 7
    )

    scheduler.add_job(
        dota_api.update_heroes,
        trigger=interval.IntervalTrigger(weeks=1, start_date=next_thursday),
    )
    scheduler.add_job(
        get_response_data,
        trigger=interval.IntervalTrigger(weeks=1, start_date=next_thursday),
    )

    return scheduler


def main():
    job_queue = SILI_BOT_APP.job_queue

    SILI_BOT_APP.add_handler(CommandHandler("dodo", dodo))
    SILI_BOT_APP.add_handler(CommandHandler("crawl", crawl))
    SILI_BOT_APP.add_handler(CommandHandler("playerinfos", playerinfos))
    SILI_BOT_APP.add_handler(CommandHandler("lastgame", lastgame))
    SILI_BOT_APP.add_handler(CommandHandler("birthdays", birthdays))
    SILI_BOT_APP.add_handler(CommandHandler("stopbot", stopbot))
    SILI_BOT_APP.add_handler(CommandHandler("voiceline", voiceline))
    SILI_BOT_APP.add_handler(
        MessageHandler(filters.TEXT & (~filters.COMMAND), message_handler)
    )

    job_queue.run_repeating(get_dota_matches, interval=600, first=10)

    # Reduced the interval heavily, as cloudflare caching should prevent bans completely according to @maakep
    job_queue.run_repeating(get_if_new_patch, interval=30, first=10)
    job_queue.run_daily(poll, datetime.time(0, 0, 0), days=(3,))

    job_queue.run_daily(upcomingBirthdays, datetime.time(0, 0, 0))
    job_queue.run_daily(todayBirthdays, datetime.time(0, 0, 0))

    non_telegram_scheduler = get_and_config_scheduler()
    non_telegram_scheduler.start()

    SILI_BOT_APP.run_polling()
    SILI_BOT_APP.shutdown()
    non_telegram_scheduler.shutdown(wait=False)
