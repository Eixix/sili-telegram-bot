# sili-telegram-bot 🚀

A telegram bot that evaluates DOTA2 matches of people and notifies about their results.

# Local development 🏗️

- Copy the `accounts_file.json.example` to `matchdata/accounts_file.json`
- Find your [Steam32 ID with this website and update your name](https://steamid.xyz/) in the `accounts_file.json`
- Create a new bot with the [@botfather](https://t.me/botfather)
- Find out your corresponding chat ID, e.g. [@userinfobot](https://t.me/userinfobot)

## Without docker ⚙️

Change the two variables `bot_token` and `chat_id` in `bot.py` accordingly. Instead of using inline environment variables you can use a `.env` file in VSCode.

### For Linux 🐧

```bash
pip install -r requirements.txt
bot_token="<bot_token>" chat_id="<chat_id>" src/bot.py
```

### For Windows 💩

```powershell
pip install -r requirements.txt
cd src
set bot_token="<bot_token>"
set chat_id="<chat_id>"
python src/bot.py
```

## With docker 🐋

```bash
docker rm sili-bot
docker build -t sili-bot --build-arg bot_token="<BOT_TOKEN>" --build-arg chat_id="<CHAT_ID>" .
docker run --name sili-bot sili-bot
```

## WIP: Port to Rust
