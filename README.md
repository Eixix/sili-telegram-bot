# sili-telegram-bot

A telegram bot that evaluates DOTA2 matches of people and notifies about their results.

# Local development
- Copy the `accounts_file.json.example` to `accounts_file.json`
- Find your [Steam32 ID with this website and update your name](https://steamid.xyz/) in the `accounts_file.json`
- Create a new bot with the [@botfather](https://t.me/botfather)
- Find out your corresponding chat ID, e.g. [@userinfobot](https://t.me/userinfobot)
- `docker build -t sili-bot --build-arg bot_token=<BOT_TOKEN> --build-arg chat_id=<CHAT_ID> .`
