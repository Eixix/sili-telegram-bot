#!/usr/bin/env python3

from datetime import time
from telegram import Update
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
                                    text=m)

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


def doubt(update: Update, context: CallbackContext):
    if update.effective_chat.id == int(chat_id) and ("doubt" in update.message.text or "daud" in update.message.text) :
        context.bot.send_animation(
            chat_id=chat_id, animation=open('resources/i_daut_it.gif', 'rb'))


<<<<<<< Updated upstream
=======
# add challenge functionality
def challenge(update: Update, context: CallbackContext):

    # get users.id of requesting user
    user_id = update.effective_user.id

    logger.info(f"new challenge from user-id: {user_id}")

    # check if user who sent request is member of group
    if context.bot.get_chat_member(chat_id=chat_id, user_id=user_id).user.id == user_id:
        logger.info("Ist in Gruppe")
        # challenge_menu(update, context)
    else:
        context.bot.send_message(
            chat_id=update.effective_chat.id, text="Du bist nicht Teil der Gruppe")


def check_for_challenge(context: CallbackContext):
    challenges = {}

    with open("resources/communication.json", 'w') as f:
        challenges = json.load(f)
        json.dump({}, f)

    context.bot.send_message(
        chat_id=chat_id, text=challenges)


>>>>>>> Stashed changes
def main():

    updater = Updater(token)
    dispatcher = updater.dispatcher
    job_queue = updater.job_queue

    dispatcher.add_handler(CommandHandler('dodo', dodo))
    dispatcher.add_handler(CommandHandler('crawl', crawl))
    dispatcher.add_handler(MessageHandler(
        Filters.text & (~Filters.command), doubt))

    job_queue.run_repeating(get_dota_matches, interval=600, first=10)
    job_queue.run_daily(poll, time(0, 0, 0), days=(3,))

    updater.start_polling()
    updater.idle()


if __name__ == '__main__':
    main()
