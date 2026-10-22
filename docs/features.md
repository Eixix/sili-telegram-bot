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

- `/challenge`: show the current Thursday's challenge or the waiting status. It
  never draws early or rerolls the selected member. The Thursday poll opens as
  before; at **20:00 in the configured `TZ`** the bot closes it and draws from
  the actual Telegram members who voted yes. No linking command or Dota mapping
  is required to participate. Retractions and changed answers replace old votes.
  Selection rotates fairly among the current yes voters, keyed by Telegram ID.
  The selected member is mentioned directly. The existing challenge text and
  task pool are retained.
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
20:00 poll closure. Optional `community.challenges` is an array of custom text;
empty/missing content uses the built-in pool. Text changes apply to the next
unassigned event; a saved assignment is never rerolled by a restart.

### Optional Dota-to-Telegram mapping

Keep the mapping in private `config.json`. Keys are configured Steam32 account
identifiers; values are numeric Telegram **user IDs**, not usernames or the
negative group chat ID. For example:

```json
{
  "community": {
    "player_members": {
      "12345": 111111111,
      "678910": 222222222
    }
  }
}
```

Each Dota account and Telegram member may appear once. Unknown Dota accounts,
nonpositive Telegram IDs, and duplicate member assignments fail configuration
validation. This mapping gives a yes voter their configured Dota display name;
an unmapped yes voter remains fully eligible and uses their Telegram name.
There are no `/linkplayer` or `/unlinkplayer` commands. Config changes take effect
on restart, following the existing private-config deployment workflow. Do not
commit real member IDs or guess associations from similar display names.

### Closing, delivery and migration

The bot tracks only its own registered, non-anonymous Thursday poll. Repeating
`/dodo` points back to that poll instead of replacing the participant list. A
late start after 20:00 does not open a new signup. Other weekdays retain normal
poll behavior. A new challenge is never published before 20:00, including if
someone closes the poll early.

Telegram provides voter identities through
[`poll_answer` updates](https://core.telegram.org/bots/api#pollanswer). At the
cutoff, the bot uses [`stopPoll`](https://core.telegram.org/bots/api#stoppoll),
then waits for the closed-poll update, matching final per-option vote counts,
and five seconds without another vote update before selecting. This can delay
publication slightly beyond 20:00. Missing votes delay the draw rather than
selecting from an incomplete roster. Anonymous chat/channel votes count toward
synchronization but cannot identify an individual for the draw.

Poll metadata, votes, assignment, delivery markers and participant rotation are
saved in `dodo-challenges-v1.json`. Telegram's processed-update offset is saved
in `telegram-offset.json`; polling acknowledges updates after they are handled,
so queued votes survive restart. Failed message delivery retries the same saved
assignment. A crash after Telegram accepts the send but before acknowledgement
can still duplicate the notification, as with the existing outbox.

Old `challenges.json` assignments are deliberately ignored. Rotation counts are
migrated only where the private config identifies the corresponding Telegram
member. An old poll cannot be adopted retrospectively: its earlier individual
votes may already have been discarded by the previous bot version. After an
upgrade on Thursday, create a fresh tracked `/dodo` poll before 20:00. If the bot
is offline past Telegram's update-retention window, it cannot reconstruct the
missing votes and will leave the challenge pending. No historic challenges are
sent after the event day.

## Sound at five yes votes

Every new bot-created `/dodo` poll can play `resources/static/sache.mp3` once when
its **positive option reaches at least five votes**. Five negative votes do not
count. A first update showing six or more yes votes also triggers it. The clip
is delivered as a voice message on the background job's next tick (normally
within ten seconds), independently of the Thursday challenge's 20:00 schedule.
Polls from other chats and unregistered poll IDs cannot trigger delivery.

The threshold crossing and successful delivery are persisted per poll in
`poll-sounds-v1.json`. Repeated updates, restarts, and dropping below five then
crossing again do not replay an acknowledged delivery. Once reached, the cheer
remains queued even if someone retracts a vote before upload. Upload failures
retry with exponential backoff from one minute up to one hour; the same queued
cheer is retained. As with the existing message outbox, a crash after Telegram
accepts the upload but before local acknowledgement can cause a duplicate.

Today's **open tracked Thursday poll** can be adopted from #98's existing
`dodo-challenges-v1.json`, including its recorded yes votes, without replacing
the signup poll. Future updates from that registry also count. Closed old polls
are not backfilled. Other weekday polls created before this feature was enabled
cannot be adopted; new `/dodo` polls are registered automatically.

Defaults enable the feature with `community.five_yes_sound_enabled: true`.
Set it to `false` to disable registration and delivery. Override the file with
`static_resources.five_yes_voiceline_path` if needed. The supplied MP3 is bundled
in the existing Docker image; no transcoder, extra service or new deployment
secret is required. The existing `poll` subscription supplies aggregate counts;
individual Dota/member mappings are not needed for the sound.

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
