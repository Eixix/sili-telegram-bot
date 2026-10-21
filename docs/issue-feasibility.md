# Feature issue status — 21 October 2026

The defined feature batch is implemented in Rust with the existing JSON volume
and Docker deployment. See [configuration, commands and limits](features.md).

| Issue | Implementation |
| --- | --- |
| [#36](https://github.com/Eixix/sili-telegram-bot/issues/36) | One OpenDota and Dotabuff link per shared-match summary. |
| [#4](https://github.com/Eixix/sili-telegram-bot/issues/4) | Hero punchlines, optional performance buckets and generic fallback. Built-in Lion/Omniknight/Riki examples. |
| [#3](https://github.com/Eixix/sili-telegram-bot/issues/3) | Stable-ID player/hero overrides and player defaults take precedence over hero/generic rules. Private mappings must be configured; examples are documented. |
| [#20](https://github.com/Eixix/sili-telegram-bot/issues/20) | Deterministic `--replay-match FILE [--seed NUMBER]`, before credentials/config loading, using the live summary renderer. |
| [#27](https://github.com/Eixix/sili-telegram-bot/issues/27) | Cached parsed-match enrichment, bounded backoff, durable notifications and late-player deduplication. Best-effort when OpenDota has no parsed replay. |
| [#37](https://github.com/Eixix/sili-telegram-bot/issues/37) | Observed personal kills/assists/KDA maxima, silent baseline, strict improvements and durable outbox. Includes solo matches; preserves catch-up suppression. |
| [#10](https://github.com/Eixix/sili-telegram-bot/issues/10) | Persisted weekly challenge, fair configured-player rotation, Thursday posting and `/challenge`. Completion is discussed by the group, not automatically verified. |
| [#13](https://github.com/Eixix/sili-telegram-bot/issues/13) | Public `/memeteam` and `/memestrats`, curated role/strategy pools and five distinct heroes. Repeated calls generate independent drafts; no private-chat authorization expansion. |
| [#56](https://github.com/Eixix/sili-telegram-bot/issues/56) | **Skipped at the user's request.** Leave open until the audio clip is available. |
| [#16](https://github.com/Eixix/sili-telegram-bot/issues/16) | **Needs a defined interaction.** Leave open; no acceptance criteria are described. |

Verification covers summary precedence/escaping/links, credential-free replay,
silent baselines/ties/catch-up, the crawl-to-record-to-rampage flow, unavailable
parsed details/retries/caching/delivery failures, weekly fairness/year boundaries,
Thursday delivery retry/restart deduplication, strategy uniqueness/role constraints
and configured-chat restrictions. Existing voiceline/cache and deployment rollback
regressions remain part of the test suite. Live upstream tests are opt-in and
must not be represented as having run merely because mocked tests pass.
