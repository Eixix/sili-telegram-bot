# Match messages and community features

Shared-match messages include OpenDota and Dotabuff links. Hero punchlines ship
for Lion, Omniknight and Riki; other heroes retain the generic performance pool.
Optional personal punchlines use stable Steam32 IDs, so renaming a player does
not change their rules. Set these in your private `config.json`:

```json
{
  "match_messages": {
    "hero_punchlines": {
      "32": {"2": ["verschwindend wenig Impact gehabt"], "inf": ["die Gegner überrascht"]}
    },
    "player_punchlines": {
      "YOUR_MICHI_STEAM32_ID": {
        "111": ["die Flexibilität einer Tomate bewiesen"]
      },
      "YOUR_CORMO_STEAM32_ID": {
        "64": ["– üüüüberraaaschung – wieder Feuer und Eis verteilt"],
        "84": ["bewiesen: Ogre ist doch auch nur ein Jakiro"]
      }
    }
  }
}
```

Replace the placeholder IDs with identifiers from `accounts.account_list`.
Private account IDs and mappings are deliberately absent from public defaults.
Precedence is player+hero, player `default`, configured hero, built-in hero,
generic performance text. Text is a **verb phrase after “hat mit HERO”**, without
HTML. A rule can be a string array or performance buckets: numeric keys are
exclusive upper bounds for `(kills + assists) / max(deaths, 1)`; `inf` covers the
highest bucket. Empty pools fall back to the next rule. An optional `default`
array covers missing buckets. Player/hero names and phrases are escaped.

## Records and rampages

Both features are enabled by default. Disable individually with
`match_features.personal_records: false` or `match_features.rampages: false`.
All state remains in the existing persistent volume; no new service is needed.

Personal records track maximum kills, assists and KDA across the completed
matches available in each tracked account's OpenDota history. The first crawl
silently establishes a baseline. These are **observed records**, not guaranteed
all-time records: private or unavailable upstream history cannot be counted.
They include solo games and use the API's existing default match filters.
Only strict improvements are announced; ties are silent. Fresh games are handled
chronologically. The existing catch-up policy suppresses notifications for a
burst of five or more new games per account, while still updating the maxima.
Snapshots, maxima and queued messages are saved together. A failed Telegram
send is retried from the durable outbox. As with the original outbox, a crash
after Telegram accepts a message but before local acknowledgement can duplicate
that message; the Bot API has no send-message idempotency key.

Rampages are announced separately for new shared matches (at least two tracked
players). Details are fetched after the ordinary summary. Only parsed
`multi_kills` counts of five or more count; total kills alone never suffice.
Missing parse data retries with backoff, at most twelve attempts or 24 hours,
without requesting an OpenDota replay parse. Each minute processes at most three
due matches. Successful details are cached by match ID and reused for late
players. Missing/unavailable parsed data can mean no rampage announcement.
Old games are not backfilled. Removed accounts' names fall back to their ID.

## Community commands

- `/challenge`: show this ISO week's Thursday challenge. The first call assigns
  it; later calls and the Thursday job reuse the saved assignment. Each eligible
  account gets a turn before anyone is selected again; ties are random and
  immediate repeats are avoided when possible. Assignment uses configured
  display names, without assuming a Telegram-user mapping. Thursday delivery
  uses the existing local-time daily scheduler and completion markers.
- `/memestrats`: list curated strategies (`global`, `invisible`, `push`).
- `/memeteam [strategy]`: generate a public five-hero draft with distinct heroes
  and strategy-specific candidates for positions 1–5. Omit the strategy for a
  random choice; repeat the command for a new draft. Unknown strategies return
  the catalog. The latest draft is saved in `memeteam.json`. Drafts are independent
  and do not reserve heroes across separate teams or calls.

These commands retain the configured-chat restriction. Captain DMs, automatic
challenge-completion verification and competitive lineup balancing are outside
this implementation. Challenges and drafts need no additional API or LLM.
Set `community.challenges_enabled: false` to disable challenge assignment and
Thursday posting. Optional `community.challenges` is an array of custom text;
empty/missing content uses the built-in pool. Changes apply next week, or after
an operator removes `challenges.json` while the bot is stopped.

## Offline match replay

```sh
cargo run --locked -- --replay-match tests/test_data/match_replay.json --seed 42
```

This dispatches before configuration loading: no Telegram token, chat ID,
private config, network or running service is required. Output is Telegram HTML
printed to stdout; nothing is sent. The fixture contains `players` entries with
`account` and `game`, optional `heroes`, `match_messages` overrides and optional
`punlines`. It renders one completed match with a deterministic random seed
(default zero), using exactly the live summary renderer. Copy the example to
experiment with stats, heroes, names and personal rules. The replay command does
not modify crawl state or simulate the scheduler; HTTP integration tests cover
those paths.
