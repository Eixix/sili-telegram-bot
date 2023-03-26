import logging
import os

from telegram import Update
from telegram.constants import ParseMode
from telegram.ext import ContextTypes

from src import dota_api
from src.models.birthdays import Birthdays
from src.models.message import Message
from src.models.voiceline import Voiceline
from src.repositories.env_variables import CHAT_ID
from src.telegram_service.telegram_jobs import send_dota_matches, poll

logger = logging.getLogger(__name__)


async def voiceline(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:  # List[Tuple[Any, MessageType]]
    logger.info("Getting voiceline...")

    if len(context.args) <= 1:
        logger.info("... not enough arguments, sending help msg.")

        help_txt = "Not enough arguments, format should be '/voiceline " \
                   "Hero Name: Voice line'...\n" \
                   "Enclose line in \"double quotes\" to use regex as described in " \
                   "the `regex` module."

        await update.message.reply_text(text=help_txt)

    else:
        # To separate out hero and voice line (both may contain whitespaces),
        # we first concatenate all args to a string and then split it on the
        # colon to get hero and voiceline
        arg_string = " ".join(context.args)

        hero, line = arg_string.split(":")

        vl = Voiceline(hero)

        vl_link = vl.get_link(line.strip())

        if vl_link is None:
            await update.message.reply_text(text=("Could not find line... "
                                                  "Check here if you typed it right: "
                                                  f"{vl.response_url}"))

            logger.info("... delivery failed, could not find voiceline.")

        else:
            vl_file_path = vl.download_mp3(vl_link)

            try:
                # Delete /voiceline to make conversation more seamless
                await context.bot.delete_message(
                    CHAT_ID=CHAT_ID, message_id=update.message.message_id)
                await update.message.reply_text(text=update.message.chat.username + ":")
                await update.message.reply_voice(voice=open(vl_file_path, "rb"))

                logger.info("... voiceline delivered.")

            finally:
                os.remove(vl_file_path)


async def crawl(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    if update.effective_chat.id == int(CHAT_ID):
        await send_dota_matches(context)


async def dodo(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    if update.effective_chat.id == int(CHAT_ID):
        await poll(context)


async def player_infos(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    if update.effective_chat.id == int(CHAT_ID):
        message = Message(None, None, dota_api.get_playerinfos())
        messages = await message.get_message_for_playerinfos()

        if messages:
            await update.message.reply_text(text=messages,
                                            parse_mode=ParseMode.HTML)


async def last_game(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    if update.effective_chat.id == int(CHAT_ID):
        time = dota_api.get_lastgame()

        if time:
            await update.message.reply_text(text=time,
                                            parse_mode=ParseMode.HTML)


async def all_birthdays(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    if update.effective_chat.id == int(CHAT_ID):
        await update.message.reply_text(text=Birthdays().GetBirthdays(),
                                        parse_mode=ParseMode.HTML)


async def message_handler(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    if update.effective_chat.id == int(CHAT_ID):
        message_text = await update.message.text.lower()
        if "doubt" in message_text or "daud" in message_text or "daut" in message_text:
            await context.bot.send_animation(animation=open('resources/i_daut_it.gif', 'rb'))
