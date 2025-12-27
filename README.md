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

## Managing dependency versions

We track the current version of all dependencies in the `requirements.txt` file
to ensure a reasonable level or reproducibility of the software environment. If
you wish to work in this environment, create an empty virtual environment,
install the dependencies and then the package itself from source:

```bash
VENV="./.venv-req"

python3 -m venv "$VENV"
source "$VENV/bin/activate"

pip3 install -r requirements.txt
pip3 install .
```

In turn, the requirements txt file can be updated to match your venv like so:

```bash
pip3 freeze --local --exclude sili-telegram-bot > requirements.txt
```

Currently, the container does not use the requirements file, but rather installs
whatever versions meet the specifications in `pyproject.toml`. However, a
requirements file generated from the versions used in the image can be generated
as follows:

```bash
# Start the container without running the bot, mounting the current dir to later
# retrieve the requirements file.
docker run -dt --rm -v $(pwd):/mnt/host --entrypoint sleep \
  --name sili-bot sili-bot infinity

# Generate a requirements file with the currently installed package versions and
# save it to the current dir on the docker host.
docker exec sili-bot bash -c "pip3 freeze \
  --exclude sili-telegram-bot > /mnt/host/requirements.txt"

docker stop sili-bot
```

Similarly, the deployment action produces the requirements file generated this
way as an artefact, allowing the reproduction of the software environment of the
deployed image.

## WIP: Port to Rust

# Inline mode

The bot has the capability for inline response search (see
<https://core.telegram.org/bots/inline> for more information). To enable this,
send `/setinline` to the botfather and, when prompred, set the placeholder
message along the lines of "Search for responses...".
