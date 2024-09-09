#!/usr/bin/env python3

import datetime
from telegram import Update, ParseMode, error, user
from telegram.ext import (
    CallbackContext,
    CommandHandler,
    Filters,
    MessageHandler,
    Updater,
)
import sili_telegram_bot.dota_api as dota_api
import json
import logging
import random
import regex
import os

from sili_telegram_bot.modules.config import config
from sili_telegram_bot.models.message import Message
from sili_telegram_bot.models.patch_checker import PatchChecker
from sili_telegram_bot.models.responses import Responses
from sili_telegram_bot.models.birthdays import Birthdays
from sili_telegram_bot.modules.voiceline_scraping import get_response_data

updater = Updater(config["secrets"]["bot_token"])
RESOURCE_CONFIG = config["static_resources"]


logger = logging.getLogger(__name__)

punlines = {}
patch_checker = PatchChecker()

with open(RESOURCE_CONFIG["punline_path"], "r", encoding="utf8") as f:
    punlines = json.load(f)


def get_dota_matches(context: CallbackContext) -> None:
    message = Message(dota_api.api_crawl(), punlines, None)
    messages = message.get_messages_for_matches()

    if messages:
        for m in messages:
            context.bot.send_message(
                chat_id=config["secrets"]["chat_id"], text=m, parse_mode=ParseMode.HTML
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


def poll(context: CallbackContext) -> None:
    logger.info(f"DODO")

    questions = [
        random.choice(punlines["dodo_poll"]["ja"]),
        random.choice(punlines["dodo_poll"]["nein"]),
    ]

    context.bot.send_voice(
        chat_id=config["secrets"]["chat_id"],
        voice=open(RESOURCE_CONFIG["dodo_voiceline_path"], "rb"),
    )

    weekday = _weekdaynumber_to_weekday(datetime.datetime.today().weekday())

    context.bot.send_poll(
        config["secrets"]["chat_id"],
        f"Do{weekday}",
        questions,
        is_anonymous=False,
        allows_multiple_answers=False,
    )


def user_to_representation(user: user.User):
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


def update_response_resource(context: CallbackContext) -> None:
    """
    Update the JSON file containing all voiceline URLs.
    """
    get_response_data()


def _parse_voiceline_args(args: list[str]) -> dict:
    """
    Parse the arguments to a voiceline request into a dict of args to be accepted
    by `Responses.get_link()`.
    """
    basic_help_text = (
        "The format should be "
        "'/voiceline Entity Name (entity_type): Voice line (level)'.\n"
        'Enclose line in "double quotes" to use regex as described in the `regex` '
        "module."
    )
    if len(args) <= 1:
        help_txt = "Not enough arguments. " + basic_help_text

        raise ValueError(help_txt)

    else:
        # To separate out hero and voice line (both may contain whitespaces),
        # we first concatenate all args to a string and then split it on the
        # colon to get hero and voiceline
        arg_string = " ".join(args)

        # pattern for each arg. A continuous string of any character besides parens or
        # a colon.
        single_arg_pattern = r"[^():]+"
        ap = single_arg_pattern

        # Parse out arguments as described in the help text above.
        arg_pattern = (
            r"^(" + ap + r")(\(" + ap + r"\))?:\w*(" + ap + r")(\(" + ap + r"\))?"
        )

        matches = regex.search(arg_pattern, arg_string)

        try:
            entity, type, line, level = matches.groups()
        except Exception as e:
            raise ValueError(f"Could not parse args: {arg_string}. " + basic_help_text)

        if not entity:
            raise ValueError(
                f"Could not parse out the name of the entity from '{arg_string}'. "
                + basic_help_text
            )

        if not line:
            raise ValueError(
                f"Could not parse out the voiceline from '{arg_string}'. "
                + basic_help_text
            )

        if type:
            type = type.strip("()")

        if level:
            level = int(level.strip("()"))

    return {
        "entity": entity.strip(),
        "type": type,
        "line": line.strip(),
        "level": level,
    }


def voiceline(update: Update, context: CallbackContext) -> None:
    if update.effective_chat.id == int(config["secrets"]["chat_id"]):
        logger.info("Getting voiceline...")

        try:
            voiceline_args = _parse_voiceline_args(context.args)

        except ValueError as e:
            logger.warning(f"Error while parsing voiceline args: {str(e)}")
            context.bot.send_message(chat_id=config["secrets"]["chat_id"], text=str(e))

            return None

        try:
            responses = Responses()

            vl_link = responses.get_link(**voiceline_args)

        except Exception as e:
            entity = voiceline_args["entity"]
            logger.error(f"Error while attempting to get voiceline for {entity}: {e}")

            context.bot.send_message(
                chat_id=config["secrets"]["chat_id"],
                text=str(e),
            )

            return None

        vl_file_path = responses.download_mp3(vl_link)

        try:
            # Delete /voiceline to make conversation more seamless
            try:
                context.bot.delete_message(
                    chat_id=config["secrets"]["chat_id"],
                    message_id=update.message.message_id,
                )
            except error.BadRequest as e:
                logger.warning(
                    f"Error attempting to delete message: {e}. Likely "
                    f"insufficient permissions for the bot in this chat. Try "
                    f"giving it the `can_delete_messages` permission. See "
                    f"<https://docs.python-telegram-bot.org/en/stable/telegram.bot.html#telegram.Bot.delete_message> "
                    f"for more info."
                )

            sender_name = user_to_representation(update.message.from_user)

            context.bot.send_message(
                chat_id=config["secrets"]["chat_id"], text=sender_name + ":"
            )
            context.bot.send_voice(
                chat_id=config["secrets"]["chat_id"],
                voice=open(vl_file_path, "rb"),
            )

            logger.info("... voiceline delivered.")

        finally:
            os.remove(vl_file_path)


def crawl(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(config["secrets"]["chat_id"]):
        get_dota_matches(context)


def dodo(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(config["secrets"]["chat_id"]):
        poll(context)


def playerinfos(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(config["secrets"]["chat_id"]):
        message = Message(None, None, dota_api.get_playerinfos())
        messages = message.get_message_for_playerinfos()

        if messages:
            context.bot.send_message(
                chat_id=config["secrets"]["chat_id"],
                text=messages,
                parse_mode=ParseMode.HTML,
            )


def lastgame(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(config["secrets"]["chat_id"]):
        time = dota_api.get_lastgame()

        if time:
            context.bot.send_message(
                chat_id=config["secrets"]["chat_id"],
                text=time,
                parse_mode=ParseMode.HTML,
            )


def birthdays(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(config["secrets"]["chat_id"]):
        context.bot.send_message(
            chat_id=config["secrets"]["chat_id"],
            text=Birthdays().GetBirthdays(),
            parse_mode=ParseMode.HTML,
        )


def upcomingBirthdays(context: CallbackContext):
    upcomingBirthdays = Birthdays().GetUpcomingBirthdays()
    if upcomingBirthdays != None:
        context.bot.send_message(
            chat_id=config["secrets"]["chat_id"],
            text=upcomingBirthdays,
            parse_mode=ParseMode.HTML,
        )


def todayBirthdays(context: CallbackContext):
    todayBirthdays = Birthdays().GetTodayBirthdays()
    if todayBirthdays != None and todayBirthdays != "":
        context.bot.send_message(
            chat_id=config["secrets"]["chat_id"],
            text=todayBirthdays,
            parse_mode=ParseMode.HTML,
        )


def stopbot(update: Update, context: CallbackContext):
    if (
        update.effective_chat.id == int(config["secrets"]["chat_id"])
        and updater.running
    ):
        updater.stop()


def message_handler(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(config["secrets"]["chat_id"]):
        message_text = update.message.text.lower()
        if "doubt" in message_text or "daud" in message_text or "daut" in message_text:
            context.bot.send_animation(
                chat_id=config["secrets"]["chat_id"],
                animation=open(RESOURCE_CONFIG["daut_gif_path"], "rb"),
            )


def get_if_new_patch(context: CallbackContext) -> None:
    new_patch_exists, new_patch_number = patch_checker.get_if_new_patch()
    if new_patch_exists:
        context.bot.send_message(
            chat_id=config["secrets"]["chat_id"],
            text=f"Es gibt ein neues Dota2 Update! Gameplay Update {new_patch_number} \n https://www.dota2.com/patches/{new_patch_number}",
            parse_mode=ParseMode.HTML,
        )


def update_heroes(context: CallbackContext) -> None:
    """
    Update heroes json with latest version from the opendota api.
    """
    dota_api.update_heroes()


def main():
    dispatcher = updater.dispatcher
    job_queue = updater.job_queue

    # Right after startup, get all dynamic resources.
    job_queue.run_once(update_heroes, when=datetime.datetime.now())
    job_queue.run_once(update_response_resource, when=datetime.datetime.now())

    dispatcher.add_handler(CommandHandler("dodo", dodo))
    dispatcher.add_handler(CommandHandler("crawl", crawl))
    dispatcher.add_handler(CommandHandler("playerinfos", playerinfos))
    dispatcher.add_handler(CommandHandler("lastgame", lastgame))
    dispatcher.add_handler(CommandHandler("birthdays", birthdays))
    dispatcher.add_handler(CommandHandler("stopbot", stopbot))
    dispatcher.add_handler(CommandHandler("voiceline", voiceline))
    dispatcher.add_handler(
        MessageHandler(Filters.text & (~Filters.command), message_handler)
    )

    job_queue.run_repeating(get_dota_matches, interval=600, first=10)

    # Reduced the interval heavily, as cloudflare caching should prevent bans completely according to @maakep
    job_queue.run_repeating(get_if_new_patch, interval=30, first=10)
    job_queue.run_daily(poll, datetime.time(0, 0, 0), days=(3,))
    job_queue.run_daily(update_response_resource, datetime.time(0, 0, 0), days=(6,))

    # Trying to catch a new patch, assuming it is out on the night from thursday to
    # friday at 2AM.
    job_queue.run_daily(update_heroes, datetime.time(2, 0, 0), days=(4,))

    job_queue.run_daily(upcomingBirthdays, datetime.time(0, 0, 0))
    job_queue.run_daily(todayBirthdays, datetime.time(0, 0, 0))

    updater.start_polling()
    updater.idle()
