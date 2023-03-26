import datetime
from enum import Enum

from telegram.ext import Application, CommandHandler, MessageHandler, filters

from src.repositories.env_variables import TOKEN
# src.telegram_service.telegram_administration import stop_bot
from src.telegram_service.telegram_handler import (voiceline,
                                                   crawl,
                                                   dodo,
                                                   player_infos,
                                                   last_game,
                                                   all_birthdays,
                                                   message_handler)
from src.telegram_service.telegram_jobs import (get_if_new_patch,
                                                send_dota_matches,
                                                poll,
                                                upcoming_birthdays,
                                                today_birthdays)

application = Application.builder().token(TOKEN).build()


def initialize_telegram_bot() -> None:
    _initialize_handler()
    _initialize_administration_handler()
    _initialize_jobs()

    application.run_polling()
    application.idle()


def _initialize_handler() -> None:
    application.add_handler(CommandHandler('voiceline', voiceline))
    application.add_handler(CommandHandler('crawl', crawl))
    application.add_handler(CommandHandler('dodo', dodo))

    application.add_handler(CommandHandler('playerinfos', player_infos))
    application.add_handler(CommandHandler('lastgame', last_game))
    application.add_handler(CommandHandler('birthdays', all_birthdays))

    application.add_handler(MessageHandler(filters.TEXT & (~filters.COMMAND),
                                           message_handler))


def _initialize_administration_handler() -> None:
    pass
    # application.add_handler(CommandHandler('stopbot', stop_bot))


def _initialize_jobs() -> None:
    job_queue = application.job_queue
    job_queue.run_repeating(send_dota_matches, interval=600, first=10)

    # Reduced the interval heavily, as cloudflare caching should prevent bans completely according to @maakep
    job_queue.run_repeating(get_if_new_patch, interval=30, first=10)
    job_queue.run_daily(poll, datetime.time(0, 0, 0), days=(3,))

    job_queue.run_daily(upcoming_birthdays, datetime.time(0, 0, 0))
    job_queue.run_daily(today_birthdays, datetime.time(0, 0, 0))


class MessageType(Enum):
    Text: 0
    Voice: 1
    Poll: 2


async def message_handler_wrapper(func, *args, **kwargs) -> None:
    pass
