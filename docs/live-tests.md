# Live functional verification

On 20 October 2026, six opt-in live tests passed against an explicitly authorized
Telegram test bot, the user's confirmed private test chat, and real upstream APIs.
The supplied production configuration contained ten accounts. Test runs overrode
the bot token/chat ID and all mutable paths; production chat and state were unused.
No credentials or real chat IDs are embedded in the test source.

| Live test | Verified behavior |
| --- | --- |
| `telegram_test_chat_functionality` | Real bot/chat lookup, messages, empty-history response, birthday listing, daily birthday/reminder delivery and saved deduplication markers, persistent whitelist, GIF upload, audio and non-anonymous poll, invalid voiceline input, fixture audio download and upload, deletion of a bot-owned test marker, match-summary delivery and duplicate suppression, player-info edge cases, addressed command, and stop signal |
| `telegram_cached_wiki_voice` | Original cached metadata, real wiki MP3 download, and voice delivery through the Rust command handler |
| `voices::tests::live_wiki_gzip_and_sniper_responses` | Real gzip-compressed wiki API fetch and parsing of 573 Sniper response rows |
| `live_opendota_heroes` | Live response parsing and caching of 127 heroes |
| `live_patch_feed_parsing_and_deduplication` | Real patch-feed baseline and unchanged-feed silence; simulated new-patch notification sent only to a local Telegram mock, then saved |
| `telegram_configured_accounts` | Birthdays from the supplied config, live latest-game/profile/win-loss queries for all ten accounts delivered to the private test chat, and a silent first crawl saving all ten account snapshots |

The first test constructs incoming command updates locally and sends their replies
through the real Telegram API. The actual polling executable was also run in a
separate interactive window; the user confirmed replies worked. The supervisor
ended that window with SIGTERM. The handler test verifies the `/stopbot` return
value; this does not establish that `/stopbot` ended the interactive process.

Live testing found an integration defect: OpenDota rejected unidentified requests
with Cloudflare HTTP 403/error 1010. A proper application User-Agent resolved it.
The production HTTP client now identifies the application with name, version, and
project URL. The live hero and account tests passed with the same identification.

## Remaining limits

- Inline mode was disabled on the test bot. Whitelist behavior and result generation
  are tested, but real inline query/selection is unverified. Enable `/setinline`
  through BotFather before testing it.
- A complete fresh wiki scrape was not part of the initial test run. Real cached
  wiki audio retrieval succeeded. Automatic scraping now works without an email;
  the live wiki test also checks discovery through the current navigation page.
  Production discovery found 221 pages. Its first crawl hit HTTP 429 at the
  legacy four-second interval; parse requests now enforce Liquipedia's 30-second
  minimum, with bounded retries for HTTP 429/server errors and `Retry-After`.
- Daily work was invoked directly. No future midnight or weekly event was awaited.
- Group-specific permissions were not exercised in this private-chat run.

## Repeating the tests

Every test in `src/live_tests.rs` is ignored by default and is compiled only for
testing. Ordinary `cargo test` and offline Docker CI never run them. These tests
send real messages and media, so use a dedicated test bot and an explicit test chat.

Set these environment variables privately:

- `SILI_LIVE_TOKEN_FILE`: private file containing the test token.
- `SILI_LIVE_CHAT_ID` and `SILI_LIVE_USER_ID`: confirmed test chat and user.
- `SILI_LIVE_REPORT_DIR`: existing temporary directory for the first test's marker.

For `telegram_configured_accounts`, also set `BOT_TOKEN_FILE` to the same test-token
file and `CONFIG_FILE` to an isolated config. Override `secrets.chat_id`, all
`dynamic_resources` paths, the whitelist path and both voice-cache paths; never
point this test at production data. Disable wiki refresh in the test configuration.
The test also checks that the configured chat equals the explicit test chat.

Run each intended test explicitly:

```sh
cargo test --locked live_tests::telegram_test_chat_functionality -- --ignored --nocapture
cargo test --locked live_tests::telegram_cached_wiki_voice -- --ignored --nocapture
cargo test --locked voices::tests::live_wiki_gzip_and_sniper_responses -- --ignored --nocapture
cargo test --locked live_tests::live_opendota_heroes -- --ignored --nocapture
cargo test --locked live_tests::live_patch_feed_parsing_and_deduplication -- --ignored --nocapture
cargo test --locked live_tests::telegram_configured_accounts -- --ignored --nocapture
```

The hero and patch-feed tests require network access but no Telegram credentials.
Account queries consume OpenDota's API allowance; space repeated runs apart.
