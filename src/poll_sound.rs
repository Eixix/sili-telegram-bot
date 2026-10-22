//! A durable one-shot cheer when a bot-owned DoDo poll reaches five yes votes.
use crate::{challenges, config::Config, storage, telegram::Telegram};
use anyhow::{Context, Result, ensure};
use chrono::DateTime;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, btree_map::Entry};

#[derive(Default, Deserialize, Serialize)]
struct State {
    polls: BTreeMap<String, Poll>,
}
#[derive(Deserialize, Serialize)]
struct Poll {
    chat: i64,
    #[serde(default)]
    reached: bool,
    #[serde(default)]
    sent: bool,
    #[serde(default)]
    attempts: u32,
    #[serde(default)]
    next_attempt: i64,
}
fn enabled(config: &Config) -> bool {
    config.0["community"]["five_yes_sound_enabled"].as_bool() == Some(true)
}
fn load(config: &Config) -> Result<State> {
    Ok(storage::read(&config.state_dir().join("poll-sounds-v1.json"))?.unwrap_or_default())
}
fn save(config: &Config, state: &State) -> Result<()> {
    storage::write(&config.state_dir().join("poll-sounds-v1.json"), state)
}
fn register_id(config: &Config, chat: i64, id: &str, yes: u64) -> Result<()> {
    let mut state = load(config)?;
    let changed = match state.polls.entry(id.into()) {
        Entry::Vacant(entry) => {
            entry.insert(Poll {
                chat,
                reached: yes >= 5,
                sent: false,
                attempts: 0,
                next_attempt: 0,
            });
            true
        }
        Entry::Occupied(mut entry) => {
            let poll = entry.get_mut();
            ensure!(poll.chat == chat, "Poll is registered to a different chat");
            if yes >= 5 && !poll.reached {
                poll.reached = true;
                true
            } else {
                false
            }
        }
    };
    if changed {
        save(config, &state)?;
    }
    Ok(())
}
/// Call only with the response to this bot's sendPoll, never an incoming poll.
pub fn register(config: &Config, chat: i64, message: &Value) -> Result<()> {
    if !enabled(config) {
        return Ok(());
    }
    let id = message["poll"]["id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("Missing sent poll ID")?;
    register_id(config, chat, id, 0)
}
pub fn observe(config: &Config, chat: i64, update: &Value) -> Result<()> {
    if !enabled(config) {
        return Ok(());
    }
    let Some(poll) = update.get("poll") else {
        return Ok(());
    };
    let Some(id) = poll["id"].as_str() else {
        return Ok(());
    };
    let mut state = load(config)?;
    let Some(saved) = state
        .polls
        .get_mut(id)
        .filter(|p| p.chat == chat && !p.reached)
    else {
        return Ok(());
    };
    let options = poll["options"].as_array().context("Missing poll options")?;
    ensure!(options.len() == 2, "Unexpected DoDo poll options");
    let yes = options[0]["voter_count"]
        .as_u64()
        .context("Missing yes vote count")?;
    if yes >= 5 {
        // Latch the crossing even if a subsequent update retracts a vote before upload.
        saved.reached = true;
        save(config, &state)?;
    }
    Ok(())
}
pub async fn deliver(config: &Config, telegram: &Telegram, now: DateTime<Tz>) -> Result<()> {
    if !enabled(config) {
        return Ok(());
    }
    // Upgrade compatibility: #98 already persisted today's open Thursday poll.
    // Only that trusted registry can supply an existing poll ID; never adopt arbitrary updates.
    if let Some((id, yes)) = challenges::open_poll(config, now.date_naive(), telegram.chat)? {
        register_id(config, telegram.chat, &id, yes)?;
    }
    let mut state = load(config)?;
    let Some((id, poll)) = state.polls.iter_mut().find(|(_, p)| {
        p.chat == telegram.chat && p.reached && !p.sent && p.next_attempt <= now.timestamp()
    }) else {
        return Ok(());
    };
    let id = id.clone();
    poll.attempts = poll.attempts.saturating_add(1);
    poll.next_attempt = now.timestamp() + (30_i64 * (1_i64 << poll.attempts.min(7))).min(3600);
    save(config, &state)?;
    telegram
        .upload(
            "sendVoice",
            "voice",
            &config.path("static_resources", "five_yes_voiceline_path"),
        )
        .await?;
    state.polls.get_mut(&id).unwrap().sent = true;
    save(config, &state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Server;
    use chrono::TimeZone;
    use reqwest::Client;
    use serde_json::json;
    fn config(dir: &std::path::Path) -> Config {
        Config(
            json!({"community":{"five_yes_sound_enabled":true},"dynamic_resources":{"hero_data_path":dir.join("heroes.json")},"static_resources":{"five_yes_voiceline_path":std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/static/sache.mp3")}}),
        )
    }
    fn now() -> DateTime<Tz> {
        chrono_tz::Europe::Berlin
            .with_ymd_and_hms(2026, 10, 22, 19, 0, 0)
            .unwrap()
    }
    fn update(id: &str, yes: u64, no: u64) -> Value {
        json!({"poll":{"id":id,"options":[{"voter_count":yes},{"voter_count":no}],"total_voter_count":yes+no}})
    }
    #[tokio::test]
    async fn four_five_six_retractions_and_restart_deliver_only_once() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        register(&config, -123, &json!({"poll":{"id":"owned"}})).unwrap();
        observe(&config, -123, &update("owned", 4, 20)).unwrap();
        deliver(&config, &telegram, now()).await.unwrap();
        assert!(server.requests.lock().unwrap().is_empty());
        observe(&config, -123, &update("owned", 5, 20)).unwrap();
        observe(&config, -123, &update("owned", 4, 21)).unwrap();
        deliver(&config, &telegram, now()).await.unwrap();
        observe(&config, -123, &update("owned", 6, 19)).unwrap();
        register(&config, -123, &json!({"poll":{"id":"owned"}})).unwrap();
        deliver(&config, &telegram, now() + chrono::Duration::hours(1))
            .await
            .unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].0.starts_with("POST /sendVoice"));
        let bytes =
            std::fs::read(config.path("static_resources", "five_yes_voiceline_path")).unwrap();
        assert!(
            server.bodies.lock().unwrap()[0]
                .windows(bytes.len())
                .any(|w| w == bytes)
        );
        assert!(load(&config).unwrap().polls["owned"].sent);
    }
    #[tokio::test]
    async fn unknown_polls_other_chats_and_no_votes_do_not_trigger_but_first_six_does() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        register(&config, -123, &json!({"poll":{"id":"owned"}})).unwrap();
        observe(&config, -123, &update("foreign", 6, 0)).unwrap();
        observe(&config, 999, &update("owned", 6, 0)).unwrap();
        observe(&config, -123, &update("owned", 0, 6)).unwrap();
        deliver(&config, &telegram, now()).await.unwrap();
        assert!(server.requests.lock().unwrap().is_empty());
        observe(&config, -123, &update("owned", 6, 0)).unwrap();
        deliver(&config, &telegram, now()).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn failed_upload_retries_with_backoff_and_then_stays_sent() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        let server = Server::new(vec![
            (503, json!({"ok":false,"error_code":503})),
            (200, json!({"ok":true,"result":{}})),
        ]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        register(&config, -123, &json!({"poll":{"id":"owned"}})).unwrap();
        observe(&config, -123, &update("owned", 5, 0)).unwrap();
        assert!(deliver(&config, &telegram, now()).await.is_err());
        assert!(!load(&config).unwrap().polls["owned"].sent);
        deliver(&config, &telegram, now() + chrono::Duration::seconds(59))
            .await
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        deliver(&config, &telegram, now() + chrono::Duration::seconds(60))
            .await
            .unwrap();
        deliver(&config, &telegram, now() + chrono::Duration::hours(1))
            .await
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 2);
    }
    #[tokio::test]
    async fn adopts_open_thursday_poll_from_existing_challenge_state_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = config(dir.path());
        config.0["community"]["challenges_enabled"] = json!(true);
        // Fixture represents the trusted #98 state already on production at upgrade.
        let votes: serde_json::Map<String, Value> = (1..=5)
            .map(|id| {
                (
                    format!("user:{id}"),
                    json!({"update_id":id,"option":0,"name":"Test"}),
                )
            })
            .collect();
        storage::write(&dir.path().join("dodo-challenges-v1.json"),&json!({"events":{"2026-10-22":{"poll_id":"existing","message_id":99,"chat":-123,"votes":votes}}})).unwrap();
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        deliver(&config, &telegram, now()).await.unwrap();
        deliver(&config, &telegram, now()).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        let state: Value = storage::read(&dir.path().join("dodo-challenges-v1.json"))
            .unwrap()
            .unwrap();
        assert_eq!(
            state["events"]["2026-10-22"]["votes"]
                .as_object()
                .unwrap()
                .len(),
            5
        );
    }
    #[tokio::test]
    async fn later_votes_in_adopted_poll_latch_without_an_aggregate_update() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = config(dir.path());
        config.0["community"]["challenges_enabled"] = json!(true);
        let path = dir.path().join("dodo-challenges-v1.json");
        let mut state = json!({"events":{"2026-10-22":{"poll_id":"existing","message_id":99,"chat":-123,"votes":{}}}});
        storage::write(&path, &state).unwrap();
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        deliver(&config, &telegram, now()).await.unwrap();
        assert!(server.requests.lock().unwrap().is_empty());
        for id in 1..=5 {
            state["events"]["2026-10-22"]["votes"][format!("user:{id}")] =
                json!({"update_id":id,"option":0});
        }
        storage::write(&path, &state).unwrap();
        deliver(&config, &telegram, now()).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }
    #[tokio::test]
    async fn disabled_feature_neither_registers_nor_sends_audio() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = config(dir.path());
        config.0["community"]["five_yes_sound_enabled"] = json!(false);
        let server = Server::new(vec![]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        register(&config, -123, &json!({"poll":{"id":"owned"}})).unwrap();
        observe(&config, -123, &update("owned", 5, 0)).unwrap();
        deliver(&config, &telegram, now()).await.unwrap();
        assert!(server.requests.lock().unwrap().is_empty());
        assert!(!dir.path().join("poll-sounds-v1.json").exists());
    }
    #[tokio::test]
    async fn closed_historical_poll_is_not_adopted_and_new_polls_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = config(dir.path());
        config.0["community"]["challenges_enabled"] = json!(true);
        storage::write(&dir.path().join("dodo-challenges-v1.json"),&json!({"events":{"2026-10-22":{"poll_id":"old","message_id":99,"chat":-123,"final_counts":[5,0]}}})).unwrap();
        let ok = json!({"ok":true,"result":{}});
        let server = Server::new(vec![(200, ok.clone()), (200, ok)]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        deliver(&config, &telegram, now()).await.unwrap();
        assert!(server.requests.lock().unwrap().is_empty());
        assert!(load(&config).unwrap().polls.is_empty());
        for id in ["one", "two"] {
            register(&config, -123, &json!({"poll":{"id":id}})).unwrap();
            observe(&config, -123, &update(id, 5, 0)).unwrap();
        }
        for _ in 0..3 {
            deliver(&config, &telegram, now()).await.unwrap();
        }
        assert_eq!(server.requests.lock().unwrap().len(), 2);
        assert!(load(&config).unwrap().polls.values().all(|p| p.sent));
    }
}
