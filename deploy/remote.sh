#!/usr/bin/env bash
set -euo pipefail
umask 077
cd "$(dirname "$0")/.."
test -f config.json
# Use the deploy user's identity to read private mounted configuration.
printf 'BOT_UID=%s\nBOT_GID=%s\n' "$(id -u)" "$(id -g)" >> .deploy.env
docker compose --env-file .deploy.env config --quiet
docker compose --env-file .deploy.env build --pull
docker compose --env-file .deploy.env run --rm --no-deps sili-bot --check-config
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
backup_container=""
rollback() {
  result=$?
  if [ "$result" -ne 0 ] && [ -n "$backup_container" ]; then
    echo 'Startup failed; restoring the previous container.' >&2
    docker rm -f sili-bot >/dev/null 2>&1 || true
    docker rename "$backup_container" sili-bot
    docker start sili-bot
  elif [ "$result" -ne 0 ] && docker inspect sili-bot >/dev/null 2>&1; then
    docker start sili-bot || true
  fi
  exit "$result"
}
trap rollback EXIT
if docker inspect sili-bot >/dev/null 2>&1; then
  docker stop sili-bot
  candidate="sili-bot-rollback-$timestamp"
  if [ -n "$(docker inspect -f '{{if index .Config.Labels "com.docker.compose.project"}}compose{{end}}' sili-bot)" ]; then
    old_image="$(docker inspect -f '{{.Image}}' sili-bot)"
    old_user="$(docker inspect -f '{{.Config.User}}' sili-bot)"
    # An unlabelled rollback copy cannot be adopted/removed by Compose.
    docker create --name "$candidate" --user "${old_user:-0:0}" \
      --env-file .deploy.env -e CONFIG_FILE=/bot/config.json \
      --mount "type=bind,src=$PWD/config.json,dst=/bot/config.json,readonly" \
      --mount source=silibotvolume,target=/bot/resources/dynamic "$old_image" >/dev/null
    backup_container="$candidate"
    docker rm sili-bot
  else
    docker rename sili-bot "$candidate"
    backup_container="$candidate"
  fi
fi
# Snapshot stopped state before changing ownership for the Rust runtime.
install -d -m 700 backups
cp -p config.json "backups/config-$timestamp.json"
# Stream the archive to the host shell: a private host bind may be inaccessible
# to container root, especially with capability drops or user-namespace remapping.
backup="backups/dynamic-$timestamp.tar.gz"
docker compose --env-file .deploy.env run --rm -T --no-deps --user 0:0 \
  --cap-add DAC_OVERRIDE --entrypoint tar sili-bot \
  -czf - -C /bot/resources/dynamic . > "$backup"
gzip -t "$backup"
docker compose --env-file .deploy.env run --rm -T --no-deps --user 0:0 \
  --cap-add CHOWN --cap-add DAC_OVERRIDE --entrypoint chown sili-bot \
  -R "$(id -u):$(id -g)" /bot/resources/dynamic
docker compose --env-file .deploy.env run --rm --no-deps --entrypoint sh sili-bot -ec \
  'test -r /bot/config.json && test -w /bot/resources/dynamic'
docker compose --env-file .deploy.env up -d --force-recreate
sleep 15
test "$(docker inspect -f '{{.State.Running}}' sili-bot)" = true
test "$(docker inspect -f '{{.RestartCount}}' sili-bot)" = 0
if docker logs --since 1m sili-bot 2>&1 | grep -Eq 'Permission denied|Invalid state file|Invalid config|Error:'; then
  echo 'Startup validation found a configuration or state error.' >&2
  exit 1
fi
docker exec sili-bot sili-telegram-bot --check-config < /dev/null
echo "Rust bot running; backup snapshot: backups/dynamic-$timestamp.tar.gz"
if [ -n "$backup_container" ]; then
  echo "Previous container retained stopped as $backup_container"
fi
