# sili-telegram-bot

A Rust Telegram bot for the Sili Dota 2 group. It posts shared match results,
player stats, birthday reminders, Dota patch alerts, polls, and searchable
voicelines. It uses outbound long polling; no public port or reverse proxy is needed.

Copy `example_config.json` to `config.json`, set your Telegram bot token and chat
ID, and replace the example accounts with Steam32 IDs. The existing Python bot's
JSON configuration is supported. See [migration and behavior notes](docs/rewrite.md)
and [homelab deployment](docs/homelab.md).

```sh
cp example_config.json config.json
# Edit config.json before starting.
cargo run --locked -- --check-config
cargo run --locked --release
```

Rust 1.90 is pinned in `rust-toolchain.toml`. Configuration precedence is built-in
defaults, `config.json`, then environment variables. `CONFIG_FILE` selects another
JSON file. `BOT_TOKEN` / `CHAT_ID` override JSON secrets; the old lowercase
`bot_token` / `chat_id` names still work. `BOT_TOKEN_FILE` takes precedence over
token environment variables and reads a mounted secret. Tokens are never build arguments.
`TZ` defaults to `Europe/Berlin`; `RUST_LOG` controls logging.

To use Docker:

```sh
docker compose build
docker compose up -d
docker compose logs -f sili-bot
```

Compose reads optional runtime credentials from `.env` (see `.env.example`).
It mounts `config.json` read-only and reuses the named `silibotvolume` at
`/bot/resources/dynamic`. A new volume works automatically. When reusing a legacy
root-owned volume, back it up and update its ownership before starting the new
non-root container; instructions are in the migration notes.

| Command | Behavior |
| --- | --- |
| `/crawl` | Check for new shared matches immediately |
| `/playerinfos` | Profile, wins/losses, win percentage, last game |
| `/lastgame` | Most recent game across tracked accounts |
| `/birthdays` | List upcoming birthday dates |
| `/dodo` | Send the Dota voice clip and group poll |
| `/voiceline Legion Commander: Ten hut! [2]` | Find and send a voiceline; level defaults to 1 |
| `/enable_inline_voicelines` | Allow the calling group member to use inline search |
| `/stopbot` | Exit the process (Docker restart policies can restart it) |

Only the configured chat can execute commands or trigger the doubt/daud/daut GIF.
Inline search checks a persistent user whitelist. Enable inline mode using
BotFather's `/setinline`, then query `@your_bot Legion Commander: Ten hut`.
Quoted voiceline text uses Rust regex syntax, for example
`/voiceline Legion Commander: "^Ten hut!$"`. Unquoted text tolerates one edit.

Voiceline caches from the Python version are read directly. Scraping is enabled
by default and runs in the background at startup and weekly, with at least the
configured delay between requests. The user agent includes the application name
and repository URL, with no email address. Set `voicelines.refresh_enabled` to
`false` to use only cached data. You can also run
`cargo run --locked --release -- --refresh-voicelines`.
Page parsing defaults to one request per minute (`secs_between_parse_requests`),
with a hard minimum of 30 seconds, following
[Liquipedia's API limits](https://liquipedia.net/api-terms-of-use); other calls
wait at least two seconds. HTTP 429/server errors retry up to five times with
exponential backoff and respect `Retry-After`. A full catalog can take about
four hours at the default pace. A failed refresh retains the previous catalog
and is retried after an hour; successful refreshes repeat weekly.
If Liquipedia returns its IP-block/CAPTCHA page, the bot stops immediate retries
and checks again after a day, keeping cached voices available. Complete the
wiki's manual unblock flow through the same outgoing connection as the bot,
then restart the service to refresh immediately. See the
[wiki troubleshooting notes](docs/homelab.md#liquipedia-access-blocks).

Validation:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
docker build --target test -t sili-bot-test .
docker run --rm --network none sili-bot-test
```

Tests use local HTTP mocks and the original response/scraper fixtures; no Telegram
credentials or live wiki access are needed. Opt-in live tests and their verified
coverage are documented in [live functional verification](docs/live-tests.md).
The standalone GitHub Actions deploy
keeps WireGuard, SSH, rsync, Docker, and the persistent volume, but now runs only
via **workflow_dispatch**. It requires the existing `WIREGUARD_CONF`,
`DEPLOY_SECRET_KEY`, `SSH_HOST`, `SSH_USER`, `BOT_TOKEN`, and `CHAT_ID` secrets,
plus `SSH_KNOWN_HOSTS` containing the verified server host key. Provision the
server's `~/sili-telegram-bot/config.json` privately before first deployment.
Use the homelab deployment workflow instead once the service belongs to that stack;
run only one instance for each bot token.
