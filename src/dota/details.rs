use super::{CrawlState, Dota, Match, drain};
use crate::{config::Account, storage, telegram::Telegram};
use anyhow::{Result, ensure};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize)]
pub(super) struct Task {
    players: BTreeMap<String, u8>,
    attempts: u32,
    next_try: i64,
    created_at: i64,
}
fn key(id: u64, account: &str) -> String {
    format!("{id}:{account}")
}
pub(super) fn schedule(state: &mut CrawlState, id: u64, players: &[(&Account, &Match)]) {
    for (account, game) in players {
        if state.detail_checked.contains(&key(id, &account.identifier)) {
            continue;
        }
        let now = Utc::now().timestamp();
        state
            .details
            .entry(id)
            .or_insert_with(|| Task {
                players: BTreeMap::new(),
                attempts: 0,
                next_try: now,
                created_at: now,
            })
            .players
            .insert(account.identifier.clone(), game.player_slot);
    }
}
fn parsed<'a>(data: &'a Value, task: &Task) -> Option<Vec<(&'a Value, &'a Value)>> {
    let players = data["players"].as_array()?;
    task.players
        .iter()
        .map(|(account, slot)| {
            let player = players
                .iter()
                .find(|p| p["player_slot"].as_u64() == Some(*slot as u64))?;
            // Anonymous upstream players can still be identified by the known match slot.
            if let Some(id) = player["account_id"].as_u64()
                && id.to_string() != *account
            {
                return None;
            }
            let kills = player.get("multi_kills")?;
            kills.as_object()?;
            Some((player, kills))
        })
        .collect()
}
fn rampage(kills: &Value) -> bool {
    kills.as_object().is_some_and(|values| {
        values.iter().any(|(count, times)| {
            count.parse::<u32>().is_ok_and(|n| n >= 5) && times.as_u64().is_some_and(|n| n > 0)
        })
    })
}
impl Dota {
    /// Best-effort enrichment runs independently of normal match announcements.
    pub async fn details(&self, telegram: &Telegram) -> Result<()> {
        if self.config.0["match_features"]["rampages"].as_bool() != Some(true) {
            return Ok(());
        }
        let path = self.config.state_dir().join("crawl.json");
        let mut state: CrawlState = storage::read(&path)?.unwrap_or_default();
        drain(&mut state, &path, telegram).await?;
        let now = Utc::now().timestamp();
        let due: Vec<_> = state
            .details
            .iter()
            .filter(|(_, t)| t.next_try <= now)
            .take(3)
            .map(|(id, _)| *id)
            .collect();
        let accounts = self.config.accounts()?;
        for id in due {
            let mut task = state.details.remove(&id).expect("scheduled task");
            let cache = self
                .config
                .state_dir()
                .join(format!("match-details/{id}.json"));
            let result: Result<Value> = async {
                if let Some(data) = storage::read::<Value>(&cache)?
                    && data["match_id"].as_u64() == Some(id)
                    && parsed(&data, &task).is_some()
                {
                    return Ok(data);
                }
                let data = self.get(&format!("matches/{id}")).await?;
                ensure!(
                    data["match_id"].as_u64() == Some(id),
                    "Mismatched match details"
                );
                ensure!(
                    parsed(&data, &task).is_some(),
                    "Match details not parsed yet"
                );
                storage::write(&cache, &data)?;
                Ok(data)
            }
            .await;
            match result {
                Ok(data) => {
                    for ((account_id, _), (_, kills)) in task
                        .players
                        .iter()
                        .zip(parsed(&data, &task).expect("validated details"))
                    {
                        if state.detail_checked.insert(key(id, account_id)) && rampage(kills) {
                            let name = accounts
                                .iter()
                                .find(|a| a.identifier == *account_id)
                                .map(|a| a.name.as_str())
                                .unwrap_or(account_id);
                            state.pending.push_back(format!("<b>RAMPAGE!</b> {} hat das ganze Team abgeräumt!\nhttps://www.opendota.com/matches/{id}", storage::html(name)));
                        }
                    }
                }
                Err(error) => {
                    task.attempts += 1;
                    if task.attempts >= 12 || now - task.created_at >= 24 * 3600 {
                        tracing::warn!(match_id=id, %error, "Giving up unavailable rampage details");
                        state
                            .detail_checked
                            .extend(task.players.keys().map(|a| key(id, a)));
                    } else {
                        task.next_try = now + (60_i64 * (1 << task.attempts.min(6))).min(3600);
                        tracing::debug!(match_id=id, %error, "Retrying match enrichment later");
                        state.details.insert(id, task);
                    }
                }
            }
            storage::write(&path, &state)?;
        }
        drain(&mut state, &path, telegram).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, test_support::Server};
    use serde_json::json;
    #[tokio::test]
    async fn retries_unparsed_details_caches_late_players_and_retries_delivery() {
        let temp = tempfile::tempdir().unwrap();
        let data = json!({"match_id":42,"players":[{"account_id":1,"player_slot":0,"multi_kills":{"5":1}},{"account_id":2,"player_slot":1,"multi_kills":{}},{"account_id":3,"player_slot":2,"multi_kills":{"6":1}}]});
        let server = Server::new(vec![
            (200, json!({"match_id":42,"players":[]})),
            (200, data),
            (503, json!({"ok":false,"error_code":503})),
            (200, json!({"ok":true,"result":{}})),
            (200, json!({"ok":true,"result":{}})),
        ]);
        let config = Config(
            json!({"dynamic_resources":{"hero_data_path":temp.path().join("heroes.json")},"accounts":{"account_list":[{"identifier":"1","name":"A & B"}]},"match_features":{"rampages":true},"opendota_api":{"api_root":server.url}}),
        );
        let client = reqwest::Client::new();
        let telegram = Telegram::mock(client.clone(), server.url.clone());
        let dota = Dota {
            client,
            config,
            punlines: json!({}),
        };
        let path = temp.path().join("crawl.json");
        let mut state = CrawlState::default();
        state.details.insert(
            42,
            Task {
                players: BTreeMap::from([("1".into(), 0), ("2".into(), 1)]),
                attempts: 0,
                next_try: 0,
                created_at: Utc::now().timestamp(),
            },
        );
        storage::write(&path, &state).unwrap();
        dota.details(&telegram).await.unwrap();
        let mut state: CrawlState = storage::read(&path).unwrap().unwrap();
        assert_eq!(state.details[&42].attempts, 1);
        state.details.get_mut(&42).unwrap().next_try = 0;
        storage::write(&path, &state).unwrap();
        assert!(dota.details(&telegram).await.is_err());
        let state: CrawlState = storage::read(&path).unwrap().unwrap();
        assert!(state.details.is_empty());
        assert_eq!(state.pending.len(), 1);
        assert!(state.pending[0].contains("A &amp; B"));
        dota.details(&telegram).await.unwrap();
        let mut state: CrawlState = storage::read(&path).unwrap().unwrap();
        state.details.insert(
            42,
            Task {
                players: BTreeMap::from([("3".into(), 2)]),
                attempts: 0,
                next_try: 0,
                created_at: Utc::now().timestamp(),
            },
        );
        storage::write(&path, &state).unwrap();
        dota.details(&telegram).await.unwrap();
        dota.details(&telegram).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|(p, _)| p.starts_with("GET"))
                .count(),
            2
        );
        assert_eq!(requests.len(), 5);
    }
    #[test]
    fn ignores_four_kills_and_zero_counts() {
        assert!(!rampage(&json!({"4":3,"5":0})));
        assert!(rampage(&json!({"5":2})));
        assert!(!rampage(&Value::Null));
    }
    #[tokio::test]
    async fn bounded_retries_expire_without_false_rampages() {
        let temp = tempfile::tempdir().unwrap();
        let server = Server::new(vec![(429, json!({"error":"rate limited"}))]);
        let config = Config(
            json!({"dynamic_resources":{"hero_data_path":temp.path().join("heroes.json")},"accounts":{"account_list":[]},"match_features":{"rampages":true},"opendota_api":{"api_root":server.url}}),
        );
        let client = reqwest::Client::new();
        let telegram = Telegram::mock(client.clone(), server.url.clone());
        let dota = Dota {
            client,
            config,
            punlines: json!({}),
        };
        let path = temp.path().join("crawl.json");
        let mut state = CrawlState::default();
        state.details.insert(
            42,
            Task {
                players: BTreeMap::from([("1".into(), 0)]),
                attempts: 11,
                next_try: 0,
                created_at: Utc::now().timestamp(),
            },
        );
        storage::write(&path, &state).unwrap();
        dota.details(&telegram).await.unwrap();
        dota.details(&telegram).await.unwrap();
        let state: CrawlState = storage::read(&path).unwrap().unwrap();
        assert!(state.details.is_empty());
        assert!(state.pending.is_empty());
        assert!(state.detail_checked.contains("42:1"));
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }
}
