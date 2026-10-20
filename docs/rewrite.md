# Rust migration notes

The Rust replacement preserves the JSON account configuration, chat restriction,
commands, local media/punlines, voiceline categories and selectable audio levels.
`CONFIG_FILE` selects user JSON; environment credentials override JSON. Both the
uppercase and original lowercase token/chat variables work. `BOT_TOKEN_FILE` takes
precedence for the token. `TZ` defaults to Europe/Berlin. The Python-specific
`metaconfig` settings are no longer interpreted.

The original bot used synchronous HTTP calls in asynchronous command handlers,
compared match-list lengths, saved match snapshots before delivering messages,
kept patch history only in memory, and appended whitelist IDs without newlines.
The rewrite uses Tokio and reqwest/Rustls, match IDs, a durable outbox and atomic
state replacement. Live testing also established that OpenDota requires a proper
application User-Agent from this environment; the shared HTTP client now sets it.

Existing dynamic files can remain in the mounted directory:

- `heroes.json`, `matchdata/<account>.json`: old account snapshots are imported when
  the new crawler has no snapshot for that account.
- `entity_data.json`, `entity_responses.json`: loaded as the legacy voice catalog.
- `whitelist.txt`: still newline-delimited. IDs concatenated by the Python bug need
  manual correction because their boundaries cannot reliably be recovered.

New files beside the hero cache are `crawl.json`, `patches.json`, `daily.json`, and
`voicelines.json`. Back up the whole dynamic directory. Corrupt JSON is reported,
not silently discarded. Run only one process per bot token/data directory.

The first crawl establishes a silent baseline. Later crawls suppress backlogs of
five or more new games per account, preserving the old catch-up limit. Summaries
require at least two tracked players. Announced IDs prevent duplicates when a
third account's results arrive late. Fetch failures do not advance snapshots; send
failures leave messages queued. Delivery remains at least once: a successful send
followed by a lost acknowledgment can produce a duplicate after retry.

Match checks run every ten minutes, patch checks every thirty seconds, and hero/wiki
refreshes at startup and weekly. Daily birthdays and Thursday polls use local `TZ`
and persist completion markers. Restarting can catch up the current day's work.
February 29 birthdays use February 28 in non-leap years; ten-day reminders cross
year boundaries. Win rate is wins divided by total games, expressed as a percentage.
HTML from names is escaped. Quoted voice patterns use Rust regex syntax, without
Python-specific backreferences/lookaround; ambiguous fuzzy matches may rank differently.

Wiki scraping is enabled by default (`voicelines.refresh_enabled`) and uses the
application name and repository URL in its user agent, without an email address.
Legacy `user_agent_email` values are ignored. A failed refresh retains the previous catalog;
a complete new catalog is atomically published in one `voicelines.json` file.

## Existing Docker installation

The runtime image now uses UID/GID 1000 rather than root. Back up and stop the old
bot before changing ownership or starting the new one:

```sh
docker run --rm --user 0:0 --entrypoint chown \
  --mount source=silibotvolume,target=/data debian:bookworm-slim -R 1000:1000 /data
```

Make the mounted config readable by UID 1000. Remove the stopped legacy container
before Compose takes over its name. The named volume remains `silibotvolume`, mounted
at `/bot/resources/dynamic`. Credentials are supplied at runtime, never as build
arguments. The standalone deployment retains WireGuard, SSH, rsync and Docker, but
is manually triggered and uses verified `SSH_KNOWN_HOSTS`.

For rollback, stop both versions and restore the pre-migration volume backup:
the Python version does not read the new Rust state and its old snapshots will
otherwise be stale. Production deployment has not been performed.

See [live test results and limitations](live-tests.md) and [homelab setup](homelab.md).

The standalone deployment workflow creates a private config backup and compressed
snapshot of the stopped dynamic volume before ownership migration. It runs the
service using the deploy user's UID/GID (overriding the image's default UID 1000),
checks config/data access, and verifies startup without restarts. The previous
container is retained stopped with a timestamped rollback name; a failed cutover
restores it automatically. Production backups and rollback containers are not
removed by the workflow. Wiki requests support gzip, as required by Liquipedia.
