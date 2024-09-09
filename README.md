# sili-telegram-bot 🚀

A telegram bot that evaluates DOTA2 matches of people and notifies about their
results.

# Local development 🏗️

- Copy the `config.json.example` to `config.json`
- Find your
  [Steam32 ID with this website and update your name](https://steamid.xyz/)
- Create a new bot with the [@botfather](https://t.me/botfather)
- Find out your corresponding chat ID, e.g.
  [@userinfobot](https://t.me/userinfobot)
- Update `config.json` with all the information.

## Without docker ⚙️

Change the two variables `bot_token` and `chat_id` in `bot.py` accordingly.
Instead of using inline environment variables you can use a `.env` file in
VSCode.

### For Linux 🐧

```bash
pip install .
bot_token="<bot_token>" chat_id="<chat_id>" run_bot
```

### For Windows 💩

```powershell
pip install .
cd src
set bot_token="<bot_token>"
set chat_id="<chat_id>"
run_bot
```

## With docker 🐋

```bash
docker rm sili-bot
docker build -t sili-bot --build-arg bot_token="<BOT_TOKEN>" --build-arg chat_id="<CHAT_ID>" .
docker run --name sili-bot sili-bot
```

Or, if the secrets are already contained in `config.json`:

```bash
docker rm sili-bot
docker build -t sili-bot .
docker run --name sili-bot sili-bot
```

By default, the container incorporates the state of `resources/dynamic` at
build time, but does not have persistence. As a consequence, the bot might
re-send match summaries after a restart. To mitigate this, use a volume
for the dynamic resources. This decouples the containers state from the build
repo's and adds persistence:

```bash
docker run --mount source=silibotvolume,target=/bot/resources/dynamic --name sili-bot -d sili-bot
```

## WIP: Port to Rust
