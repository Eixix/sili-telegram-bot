#!/usr/bin/env python3

from datetime import time
from telegram import Update, ParseMode
from telegram.ext import CallbackContext, CommandHandler, Filters, MessageHandler, Updater
import dota_api
import json
import logging
import random
import os
from models.message import Message

# Environment variable
token = os.environ['bot_token']
chat_id = os.environ['chat_id']
updater = Updater(token)

logging.basicConfig(format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
                    level=logging.INFO)
logger = logging.getLogger(__name__)

punlines = {}
with open("resources/punlines.json", 'r') as f:
    punlines = json.load(f)


def get_dota_matches(context: CallbackContext) -> None:
    message = Message(dota_api.api_crawl(),punlines)
    messages = message.get_messages()

    if messages:
        for m in messages:
            context.bot.send_message(chat_id=chat_id,
                                     text=m,
                                     parse_mode=ParseMode.HTML)

def poll(context: CallbackContext) -> None:
    logger.info(f"DODO")

    questions = [random.choice(punlines["ja"]),
                 random.choice(punlines["nein"])]

    context.bot.send_voice(chat_id=chat_id, voice=open(
        'resources/lets_dota.mpeg', 'rb'))

    context.bot.send_poll(
        chat_id,
        "DoDo?",
        questions,
        is_anonymous=False,
        allows_multiple_answers=False,
    )

def crawl(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        get_dota_matches(context)

def dodo(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        poll(context)

def message_handler(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id):
        message_text = update.message.text.lower()
        if ("doubt" in message_text or "daud" in message_text or "daut" in message_text) :
            context.bot.send_animation(chat_id=chat_id, animation=open('resources/i_daut_it.gif', 'rb'))
        if ("/stopbot" in message_text and updater.running):
            updater.stop()
        

def main():
    dispatcher = updater.dispatcher
    job_queue = updater.job_queue

    dispatcher.add_handler(CommandHandler('dodo', dodo))
    dispatcher.add_handler(CommandHandler('crawl', crawl))
    dispatcher.add_handler(MessageHandler(
        Filters.text & (~Filters.command), message_handler))

    job_queue.run_repeating(get_dota_matches, interval=600, first=10)
    job_queue.run_daily(poll, time(0, 0, 0), days=(3,))

    updater.start_polling()
    updater.idle()


if __name__ == '__main__':
    main()
