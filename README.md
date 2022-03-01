# sili-telegram-bot

A telegram bot that evaluates DOTA2 matches of people and notifies about their results.

# Execute with docker
- Create a new bot with the [@botfather](https://t.me/botfather)
- Find out your corresponding chat ID, e.g. [@userinfobot](https://t.me/userinfobot)
- `docker build -t sili-bot --build-arg bot_token=<BOT_TOKEN> --build-arg chat_id=<CHAT_ID> .`