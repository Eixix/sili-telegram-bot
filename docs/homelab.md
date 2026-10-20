# Integrating with ../homelab

`deploy/homelab.yaml` follows the sibling repository's Compose includes, deployment
UID/GID, private `secrets/`, persistent `data/`, and manual deployment convention.
The sibling repository is unchanged.

1. Stage source at `homelab/apps/sili-telegram-bot/`, excluding `.git`, `target`,
   private `config.json`, `.env*`, and `resources/dynamic`.
2. Copy `deploy/homelab.yaml` to `homelab/compose/apps/sili-telegram-bot.yaml` and add
   `- ./compose/apps/sili-telegram-bot.yaml` to the root Compose `include` list.
3. Privately provision `homelab/secrets/sili-telegram-bot/config.json` with accounts
   and production `secrets.bot_token` / `secrets.chat_id`. Keep the default dynamic
   paths, which resolve inside `/bot/resources/dynamic`.
4. Create `homelab/data/sili-telegram-bot/`, owned by `DEPLOY_UID`/`DEPLOY_GID` from
   `.env` (the include defaults to 1003:1003). The deployment UID must also be able
   to read the private config file.
5. For migration, stop the old bot, back up the `silibotvolume` contents, copy them
   into the new data directory, and set its ownership to the deployment UID/GID.
   Retain the old volume for rollback.
6. Validate and deploy from the homelab root:

   ```sh
   docker compose --env-file .env --profile external config --quiet
   docker compose --env-file .env --profile external build sili-bot
   docker compose --env-file .env --profile external up -d sili-bot
   docker compose logs --tail=100 sili-bot
   ```

The Compose include was validated together with an isolated copy of the full
homelab stack, using public example/dummy environment values. No production
configuration or services were changed.

Once the source and include belong to homelab, its existing manual **Deploy Homelab**
workflow can deploy with `services=sili-bot`. Provision this bot's secrets/data on
the server first; that workflow preserves these directories but does not create
the bot's private configuration. Use that workflow instead of this repository's
standalone deploy after migration. Never run both with the same Telegram token.

The bot publishes no ports and disables Traefik discovery. It uses the existing
`internal_network` for outbound requests and a read-only root filesystem with
capabilities dropped. Confirm that the bot's data directory is included in the
homelab backup policy. Docker `test` and `prod` targets remain available.
