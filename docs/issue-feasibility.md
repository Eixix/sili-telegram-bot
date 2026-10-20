# Open issue feasibility — 20 October 2026

Reviewed all ten open issues, their descriptions/discussions, and the deployed
Rust implementation. None is obsolete just because of the rewrite. No requested
features below were implemented as part of this review. Effort is relative:
small means a local change; medium adds durable state or an integration; large
requires both product decisions and several interacting components.

| Issue | Assessment | Effort | Suggested order |
| --- | --- | --- | --- |
| [#36 Match links](https://github.com/Eixix/sili-telegram-bot/issues/36) | Ready: summaries already have a match ID; append an OpenDota/Dotabuff link once per match. No new API requests. | Small | 1 |
| [#4 Hero punchlines](https://github.com/Eixix/sili-telegram-bot/issues/4) | Ready: use `hero_id` and existing performance buckets; add optional hero-specific text with the generic pool as fallback. | Small | 2 |
| [#3 Player punchlines](https://github.com/Eixix/sili-telegram-bot/issues/3) | Ready once the desired lines are supplied: key optional overrides by stable Steam32 ID and hero ID, rather than display names. Decide priority relative to #4. | Small | 3 |
| [#20 Debug match data](https://github.com/Eixix/sili-telegram-bot/issues/20) | Partly covered by fixture constructors and local HTTP integration tests. A straightforward developer replay command is still missing. Extract a pure summary renderer and add offline fixture replay with no Telegram credentials/network. | Small–medium | 4 |
| [#56 Audio at five acceptances](https://github.com/Eixix/sili-telegram-bot/issues/56) | Feasible: existing polls are non-anonymous, but their IDs are discarded and incoming votes are not requested/handled. Persist bot-owned poll/chat IDs, positive option, per-user votes, and a one-shot threshold marker; handle vote changes/retractions and restart/replay. Supply the requested audio clip. | Medium | 5 |
| [#27 Rampage mention](https://github.com/Eixix/sili-telegram-bot/issues/27) | Feasible with parsed match details. Current `Match` only retains basic K/D/A; total kills cannot identify a rampage. Fetch/cache one detail response per new shared match, inspect each tracked player's multikills, and retry missing parse data separately without blocking ordinary summaries. | Medium | 6 |
| [#37 Personal records](https://github.com/Eixix/sili-telegram-bot/issues/37) | Feasible after choosing metrics and scope. K/D/A records can use current history; GPM/XPM/damage need projected fields or details. Persist per-player maxima and announced record IDs; define all-time vs since tracking, ranked/Turbo filters, hero-specific vs overall records, ties and backfill. Baseline silently to avoid initial spam. | Medium–large | 7 |
| [#10 Thursday challenge](https://github.com/Eixix/sili-telegram-bot/issues/10) | Feasible: reuse the timezone-aware daily scheduler and completion markers. Need challenge text/rules, eligible players, fair selection/rotation and expiry. Posting a challenge is simple; automatically verifying completion is separate work and depends on measurable match data. | Small–medium for posting | 8 |
| [#13 MemeTeamGenerator](https://github.com/Eixix/sili-telegram-bot/issues/13) | Feasible with a curated strategy catalog: choose five distinct heroes with role/strategy constraints, save the generated draft, and offer rerolls. Public output fits current chat authorization. Captain DMs require private-chat onboarding and scoped authorization because commands currently accept only the configured chat. | Medium for public MVP; large with private captain flow | 9 |
| [#16 User interactions](https://github.com/Eixix/sili-telegram-bot/issues/16) | Technically feasible but too broad to estimate responsibly: no behavior or acceptance criteria are described. Choose one interaction (e.g. challenge/accept, team signup, or duel), then specify participants, authorization, expiry and persistence. Could overlap #56/#10/#13. | Needs definition | After a concrete use case |

## Code findings and API checks

- `src/dota.rs`: `Dota::summary` builds one shared-match message and currently
  uses generic performance text; it has all the IDs needed for #36/#4/#3.
  `CrawlState` persists snapshots, an outbox and announced match IDs. New
  record/detail notifications should preserve that deduplication behavior.
- `src/main.rs`: `Bot::poll` sends a non-anonymous poll but discards its result.
  `Bot::update` handles messages/inline queries only. `src/telegram.rs` explicitly
  requests only `message` and `inline_query`, so #56 needs update-subscription
  changes as well as a handler. Telegram documents
  [poll answers](https://core.telegram.org/bots/api#pollanswer) for non-anonymous
  polls sent by the bot. Persist the poll-to-chat mapping; poll answers alone
  are not a trusted group-command context.
- OpenDota's official [match schema](https://github.com/odota/core/blob/master/svc/api/responses/MatchResponse.ts)
  exposes `multi_kills`; the current bot neither fetches detailed matches nor
  retains this field. Parsed data may arrive later or be absent. Missing data
  must not be treated as proof that no rampage occurred.
- The official [player-match route](https://github.com/odota/core/blob/master/svc/api/spec.ts)
  supports field projection, while its [default columns](https://github.com/odota/core/blob/master/svc/api/playerFields.ts)
  omit GPM/XPM/damage/multikills. #37 needs an explicit data choice rather than
  assuming those values are in current snapshots. OpenDota's `/records/{field}`
  is not a per-player personal-record endpoint.
- Existing tests in `src/dota.rs`, `src/live_tests.rs` and `src/test_support.rs`
  already allow synthetic matches, failed requests and deduplication tests.
  That addresses part of #20, but does not provide the requested convenient
  developer-facing replay workflow.

Keep all ten issues open. Prioritize #36, #4 and #3 for small useful changes,
then #20 to make the stateful features easier to develop. Resolve #16's scope
before treating it as a separate implementation task.
