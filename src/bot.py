#!/usr/bin/env python3

import datetime
from telegram import Update, ParseMode
from telegram.ext import CallbackContext, CommandHandler, Filters, MessageHandler, Updater
import dota_api
import json
import logging
import random
import os
from models.message import Message
from models.voiceline import Voiceline
from models.birthdays import Birthdays

# Environment variable
token = os.environ['bot_token']
chat_id = os.environ['chat_id']
updater = Updater(token)

logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                    level=logging.INFO)
logger = logging.getLogger(__name__)

punlines = {}
with open("resources/punlines.json", 'r', encoding="utf8") as f:
    punlines = json.load(f)


def get_dota_matches(context: CallbackContext) -> None:
    message = Message(dota_api.api_crawl(), punlines, None)
    messages = message.get_messages_for_matches()

    if messages:
        for m in messages:
            context.bot.send_message(chat_id=chat_id,
                                     text=m,
                                     parse_mode=ParseMode.HTML)


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

    questions = [random.choice(punlines["dodo_poll"]["ja"]),
                 random.choice(punlines["dodo_poll"]["nein"])]

    context.bot.send_voice(chat_id=chat_id, voice=open(
        'resources/lets_dota.mpeg', 'rb'))

    weekday = _weekdaynumber_to_weekday(datetime.datetime.today().weekday())

    context.bot.send_poll(
        chat_id,
        f"Do{weekday}",
        questions,
        is_anonymous=False,
        allows_multiple_answers=False,
    )


def voiceline(update: Update, context: CallbackContext) -> None:
    logger.info("Getting voiceline...")

    if len(context.args) <= 1:
        logger.info("... not enough arguments, sending help msg.")

        help_txt = "Not enough arguments, format should be '/voiceline " \
            "Hero Name: Voice line'...\n" \
            "Enclose line in \"double quotes\" to use regex as described in " \
            "the `regex` module."

        context.bot.send_message(chat_id=chat_id,
                                 text=help_txt)

    else:
        # To separate out hero and voice line (both may contain whitespaces),
        # we first concatenate all args to a string and then split it on the
        # colon to get hero and voiceline
        arg_string = " ".join(context.args)

        hero, line = arg_string.split(":")

        vl = Voiceline(hero)

        vl_link = vl.get_link(line.strip())

        if vl_link is None:
            context.bot.send_message(chat_id=chat_id,
                                     text=("Could not find line... "
                                           "Check here if you typed it right: "
                                           f"{vl.response_url}"))

            logger.info("... delivery failed, could not find voiceline.")

        else:
            vl_file_path = vl.download_mp3(vl_link)

            try:
                # Delete /voiceline to make conversation more seamless
                context.bot.delete_message(
                    chat_id=chat_id, message_id=update.message.message_id)
                context.bot.send_message(chat_id=chat_id,
                                         text=update.message.chat.username + ":")
                context.bot.send_voice(chat_id=chat_id,
                                       voice=open(vl_file_path, "rb"))

                logger.info("... voiceline delivered.")

            finally:
                os.remove(vl_file_path)


def crawl(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        get_dota_matches(context)


def dodo(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        poll(context)


def playerinfos(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        message = Message(None, None, dota_api.get_playerinfos())
        messages = message.get_message_for_playerinfos()

        if messages:
            context.bot.send_message(chat_id=chat_id,
                                     text=messages,
                                     parse_mode=ParseMode.HTML)


def lastgame(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        time = dota_api.get_lastgame()

        if time:
            context.bot.send_message(chat_id=chat_id,
                                     text=time,
                                     parse_mode=ParseMode.HTML)


def birthdays(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        context.bot.send_message(chat_id=chat_id,
                                 text=Birthdays().GetBirthdays(),
                                 parse_mode=ParseMode.HTML)


def upcomingBirthdays(context: CallbackContext):
    upcomingBirthdays = Birthdays().GetUpcomingBirthdays()
    if upcomingBirthdays != None:
        context.bot.send_message(chat_id=chat_id,
                                 text=upcomingBirthdays,
                                 parse_mode=ParseMode.HTML)


def todayBirthdays(context: CallbackContext):
    todayBirthdays = Birthdays().GetTodayBirthdays()
    if todayBirthdays != None and todayBirthdays != "":
        context.bot.send_message(chat_id=chat_id,
                                 text=todayBirthdays,
                                 parse_mode=ParseMode.HTML)


def stopbot(update: Update, context: CallbackContext):
    if (update.effective_chat.id == int(chat_id) and updater.running):
        updater.stop()


def message_handler(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        message_text = update.message.text.lower()
        if ("doubt" in message_text or "daud" in message_text or "daut" in message_text):
            context.bot.send_animation(
                chat_id=chat_id, animation=open('resources/i_daut_it.gif', 'rb'))


def main():
    dispatcher = updater.dispatcher
    job_queue = updater.job_queue

    dispatcher.add_handler(CommandHandler('dodo', dodo))
    dispatcher.add_handler(CommandHandler('crawl', crawl))
    dispatcher.add_handler(CommandHandler('playerinfos', playerinfos))
    dispatcher.add_handler(CommandHandler('lastgame', lastgame))
    dispatcher.add_handler(CommandHandler('birthdays', birthdays))
    dispatcher.add_handler(CommandHandler('stopbot', stopbot))
    dispatcher.add_handler(CommandHandler('voiceline', voiceline))
    dispatcher.add_handler(MessageHandler(
        Filters.text & (~Filters.command), message_handler))

    job_queue.run_repeating(get_dota_matches, interval=600, first=10)
    job_queue.run_daily(poll, datetime.time(0, 0, 0), days=(3,))

    job_queue.run_daily(upcomingBirthdays, datetime.time(0, 0, 0))
    job_queue.run_daily(todayBirthdays, datetime.time(0, 0, 0))

    updater.start_polling()
    updater.idle()


if __name__ == '__main__':
    main()
