#!/usr/bin/env python3

from datetime import time
from telegram import Update
from telegram.ext import CallbackContext, CommandHandler, Updater
import dota_api
import logging
import os

# Environment variable
token = os.environ['bot_token']
chat_id = os.environ['chat_id']


logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                    level=logging.INFO)
logger = logging.getLogger(__name__)


def get_dota_matches(context: CallbackContext):
    messages = dota_api.api_crawl()
    print(f"messages: {messages}")

    if messages:
        messages = '\n\n'.join(messages)
        context.bot.send_message(chat_id=chat_id,
                                 text=messages)
    else:
        context.bot.send_message(chat_id=chat_id,
                                 text="Crawled but found nothing")


def poll(context: CallbackContext) -> None:
    # TODO: Add dumb phrases here
    questions = ["Ja", "Nein"]
    message = context.bot.send_poll(
        chat_id,
        "DoDo?",
        questions,
        is_anonymous=False,
        allows_multiple_answers=False,
    )


def crawl(update: Update, context: CallbackContext):
    get_dota_matches()


def main():

    updater = Updater(token)
    dispatcher = updater.dispatcher
    job_queue = updater.job_queue

    dispatcher.add_handler(CommandHandler('dodo', poll))
    dispatcher.add_handler(CommandHandler('crawl', crawl))

    job_queue.run_repeating(get_dota_matches, interval=600, first=10)
    job_queue.run_daily(poll, time(0, 0, 0), days=(3,))

    updater.start_polling()
    updater.idle()


if __name__ == '__main__':
    main()
