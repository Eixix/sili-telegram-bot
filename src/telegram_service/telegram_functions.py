from telegram.ext import Application, CommandHandler, MessageHandler

from src.repositories.ENV_VARIABLES import token

application = Application.builder().token(token).build()






def initialize_telegram_bot():
    _initalize_handler()
    _initalize_jobs()

    application.run_polling()
    application.idle()


def _initalize_handler():
    application.add_handler(CommandHandler('dodo', dodo))
    application.add_handler(CommandHandler('crawl', crawl))
    application.add_handler(CommandHandler('playerinfos', player_infos))
    application.add_handler(CommandHandler('lastgame', last_game))
    application.add_handler(CommandHandler('birthdays', all_birthdays))
    application.add_handler(CommandHandler('stopbot', stop_bot))
    application.add_handler(CommandHandler('voiceline', voiceline))
    application.add_handler(MessageHandler(
        filters.TEXT & (~filters.COMMAND), message_handler))


def _initalize_jobs():
    job_queue = application.job_queue
    job_queue.run_repeating(send_dota_matches, interval=600, first=10)

    # Reduced the interval heavily, as cloudflare caching should prevent bans completely according to @maakep
    job_queue.run_repeating(get_if_new_patch, interval=30, first=10)
    job_queue.run_daily(poll, datetime.time(0, 0, 0), days=(3,))

    job_queue.run_daily(upcoming_birthdays, datetime.time(0, 0, 0))
    job_queue.run_daily(today_birthdays, datetime.time(0, 0, 0))



