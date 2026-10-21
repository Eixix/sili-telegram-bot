use crate::{
    config::{Account, Config},
    storage,
    telegram::Telegram,
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
mod details;
mod messages;
mod records;
use rand::prelude::IndexedRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

#[derive(Clone, Deserialize, Serialize)]
pub struct Match {
    pub match_id: u64,
    #[serde(default)]
    pub start_time: i64,
    #[serde(default)]
    pub duration: i64,
    pub radiant_win: Option<bool>,
    pub player_slot: u8,
    pub hero_id: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
}
impl Match {
    fn won(&self) -> bool {
        self.radiant_win == Some(self.player_slot < 128)
    }
    fn ratio(&self) -> f64 {
        (self.kills as f64 + self.assists as f64) / self.deaths.max(1) as f64
    }
}
#[derive(Default, Deserialize, Serialize)]
struct CrawlState {
    accounts: HashMap<String, Vec<Match>>,
    pending: VecDeque<String>,
    #[serde(default)]
    announced: std::collections::BTreeSet<u64>,
    #[serde(default)]
    records: HashMap<String, records::Records>,
    #[serde(default)]
    details: BTreeMap<u64, details::Task>,
    #[serde(default)]
    detail_checked: std::collections::BTreeSet<String>,
}
pub struct Dota {
    pub client: Client,
    pub config: Config,
    pub punlines: Value,
}
impl Dota {
    async fn get(&self, path: &str) -> Result<Value> {
        let root = self.config.string("opendota_api", "api_root");
        Ok(self
            .client
            .get(format!("{}/{path}", root.trim_end_matches('/')))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
    async fn player(&self, account: &Account, endpoint: &str) -> Result<Value> {
        self.get(&format!(
            "{}/{}/{}",
            self.config.string("opendota_api", "player_segment"),
            account.identifier,
            endpoint
        ))
        .await
    }
    async fn matches(&self, account: &Account, limit: Option<usize>) -> Result<Vec<Match>> {
        let mut endpoint = self
            .config
            .string("opendota_api", "account_matches_endpoint");
        if let Some(limit) = limit {
            endpoint.push_str(&format!("?limit={limit}"));
        }
        serde_json::from_value(self.player(account, &endpoint).await?)
            .context("Invalid OpenDota match response")
    }
    pub async fn update_heroes(&self) -> Result<()> {
        let heroes = self
            .get(&self.config.string("opendota_api", "hero_endpoint"))
            .await?;
        ensure!(
            heroes.as_array().is_some_and(|a| !a.is_empty()),
            "Empty hero list"
        );
        storage::write(
            &self.config.path("dynamic_resources", "hero_data_path"),
            &heroes,
        )
    }
    pub async fn crawl(&self, telegram: &Telegram) -> Result<()> {
        let path = self.config.state_dir().join("crawl.json");
        let mut state: CrawlState = storage::read(&path)?.unwrap_or_default();
        // Drain durable messages before fetching new matches. Failures stay queued.
        drain(&mut state, &path, telegram).await?;
        let accounts = self.config.accounts()?;
        let mut fetched = HashMap::new();
        let mut new_ids = HashSet::new();
        let mut fresh_by_account = HashMap::new();
        for account in &accounts {
            let current = self.matches(account, None).await?;
            let legacy = self
                .config
                .path("dynamic_resources", "match_data_dir")
                .join(format!("{}.json", account.identifier));
            let previous = match state.accounts.get(&account.identifier) {
                Some(matches) => Some(matches.clone()),
                None => storage::read::<Vec<Match>>(&legacy)?,
            };
            let fresh: HashSet<_> = new_match_ids(previous.as_deref(), &current)
                .into_iter()
                .collect();
            new_ids.extend(fresh.iter().copied());
            fresh_by_account.insert(account.identifier.clone(), fresh);
            fetched.insert(account.identifier.clone(), current);
        }
        let heroes: Vec<Value> =
            storage::read(&self.config.path("dynamic_resources", "hero_data_path"))?
                .unwrap_or_default();
        let mut groups: BTreeMap<u64, Vec<(&Account, &Match)>> = BTreeMap::new();
        for account in &accounts {
            for game in &fetched[&account.identifier] {
                if new_ids.contains(&game.match_id) && game.radiant_win.is_some() {
                    groups
                        .entry(game.match_id)
                        .or_default()
                        .push((account, game));
                }
            }
        }
        for (id, players) in groups.iter().filter(|(_, p)| p.len() > 1) {
            if state.announced.insert(*id) {
                state.pending.push_back(self.summary(players, &heroes));
            }
            if self.config.0["match_features"]["rampages"].as_bool() == Some(true) {
                details::schedule(&mut state, *id, players);
            }
        }
        if self.config.0["match_features"]["personal_records"].as_bool() == Some(true) {
            for account in &accounts {
                let (records, messages) = records::update(
                    account,
                    &fetched[&account.identifier],
                    &fresh_by_account[&account.identifier],
                    state.records.remove(&account.identifier),
                );
                state.records.insert(account.identifier.clone(), records);
                state.pending.extend(messages);
            }
        }
        state.accounts = fetched;
        // Save snapshots and outbox together, only after all upstream requests succeed.
        storage::write(&path, &state)?;
        drain(&mut state, &path, telegram).await
    }
    fn summary(&self, players: &[(&Account, &Match)], heroes: &[Value]) -> String {
        messages::summary(
            players,
            heroes,
            &self.punlines,
            &self.config.0["match_messages"],
            &mut rand::rng(),
        )
    }
    pub fn replay_summary(
        players: &[(&Account, &Match)],
        heroes: &[Value],
        punlines: &Value,
        overrides: &Value,
        seed: u64,
    ) -> String {
        use rand::SeedableRng;
        messages::summary(
            players,
            heroes,
            punlines,
            overrides,
            &mut rand::rngs::StdRng::seed_from_u64(seed),
        )
    }
    pub async fn lastgame(&self) -> Result<String> {
        let mut latest = None;
        for account in self.config.accounts()? {
            if let Some(game) = self.matches(&account, Some(1)).await?.first() {
                latest = Some(latest.unwrap_or(0).max(game.start_time + game.duration));
            }
        }
        Ok(match latest {
            Some(t) => self.date(t)?,
            None => "Noch keine Spiele gefunden.".into(),
        })
    }
    fn date(&self, timestamp: i64) -> Result<String> {
        Ok(DateTime::from_timestamp(timestamp, 0)
            .context("Invalid match timestamp")?
            .with_timezone(&self.config.timezone()?)
            .format("%d.%m.%Y %H:%M:%S")
            .to_string())
    }
    pub async fn playerinfos(&self) -> Result<Vec<String>> {
        let mut messages = Vec::new();
        for account in self.config.accounts()? {
            let profile = self.player(&account, "").await?;
            let wl = self.player(&account, "wl").await?;
            let win = wl["win"].as_u64().context("Missing win count")?;
            let lose = wl["lose"].as_u64().context("Missing lose count")?;
            let last = self.matches(&account, Some(1)).await?;
            let last = if let Some(game) = last.first() {
                let end = game.start_time + game.duration;
                let day = DateTime::from_timestamp(end, 0)
                    .context("Invalid match timestamp")?
                    .with_timezone(&self.config.timezone()?)
                    .date_naive();
                format!(
                    "{} ({} Tag(e) vergangen)",
                    self.date(end)?,
                    (Utc::now()
                        .with_timezone(&self.config.timezone()?)
                        .date_naive()
                        - day)
                        .num_days()
                )
            } else {
                "Keine Spiele".into()
            };
            messages.push(format!("<b>{}</b>\nSteamname: {}\nAnzahl Spiele: {}\nSiege: {}\nNiederlagen: {}\nWin rate: {:.2}%\nLetztes Spiel: {}", storage::html(&account.name), storage::html(profile["profile"]["personaname"].as_str().unwrap_or("Unbekannt")), win+lose, win, lose, win as f64 * 100.0 / (win+lose).max(1) as f64, last));
        }
        Ok(messages)
    }
    pub async fn patches(&self, telegram: &Telegram) -> Result<()> {
        let data: Value = self
            .client
            .get("https://www.dota2.com/datafeed/patchnoteslist?language=english")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let patches: Vec<String> = data["patches"]
            .as_array()
            .context("Missing patches")?
            .iter()
            .filter_map(|p| p["patch_name"].as_str().map(str::to_owned))
            .collect();
        ensure!(!patches.is_empty(), "Empty patch list");
        let path = self.config.state_dir().join("patches.json");
        let known: Option<Vec<String>> = storage::read(&path)?;
        let mut saved = known.clone().unwrap_or_else(|| patches.clone());
        if let Some(known) = known {
            for patch in patches.iter().filter(|p| !known.contains(p)) {
                telegram.text(&format!("Es gibt ein neues Dota2 Update! Gameplay Update {}\nhttps://www.dota2.com/patches/{}", storage::html(patch), storage::html(patch))).await?;
                saved.push(patch.clone());
                storage::write(&path, &saved)?;
            }
        }
        storage::write(&path, &saved)
    }
}
fn new_match_ids(previous: Option<&[Match]>, current: &[Match]) -> Vec<u64> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    let known: HashSet<_> = previous.iter().map(|m| m.match_id).collect();
    let fresh: Vec<_> = current
        .iter()
        .filter(|m| !known.contains(&m.match_id))
        .map(|m| m.match_id)
        .collect();
    // Preserve the old bot's catch-up limit and silent initial baseline.
    if fresh.len() < 5 { fresh } else { Vec::new() }
}
async fn drain(state: &mut CrawlState, path: &std::path::Path, telegram: &Telegram) -> Result<()> {
    while let Some(message) = state.pending.front() {
        telegram.text(message).await?;
        state.pending.pop_front();
        storage::write(path, state)?;
    }
    Ok(())
}
pub fn choose(value: &Value) -> String {
    value
        .as_array()
        .and_then(|a| a.choose(&mut rand::rng()))
        .and_then(Value::as_str)
        .unwrap_or("Dota!")
        .into()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn game(id: u64) -> Match {
        serde_json::from_value(serde_json::json!({"match_id":id,"radiant_win":true,"player_slot":0,"hero_id":1,"kills":1,"deaths":0,"assists":2})).unwrap()
    }
    #[test]
    fn detects_new_ids_when_api_length_does_not_change() {
        assert_eq!(
            new_match_ids(Some(&[game(2), game(1)]), &[game(3), game(2)]),
            vec![3]
        );
        assert!(new_match_ids(None, &[game(3)]).is_empty());
        assert!(new_match_ids(Some(&[]), &(1..=5).map(game).collect::<Vec<_>>()).is_empty());
    }
    #[test]
    fn sides_and_zero_deaths() {
        let mut game = game(1);
        assert!(game.won());
        assert_eq!(game.ratio(), 3.0);
        game.player_slot = 128;
        assert!(!game.won());
        game.radiant_win = Some(false);
        assert!(game.won());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_support::Server;
    use serde_json::json;
    fn games(id: u64) -> Value {
        json!([{"match_id":id,"radiant_win":true,"player_slot":0,"hero_id":1,"kills":3,"deaths":0,"assists":8}])
    }
    #[tokio::test]
    async fn migrates_old_snapshots_and_retries_durable_outbox() {
        let temp = tempfile::tempdir().unwrap();
        let server = Server::new(vec![
            (200, games(2)),
            (200, games(2)),
            (503, json!({"ok":false,"error_code":503})),
            (200, json!({"ok":true,"result":{}})),
            (200, games(2)),
            (200, games(2)),
        ]);
        let config = Config(json!({
            "accounts":{"account_list":[{"identifier":"1","name":"One & Two"},{"identifier":"2","name":"Second"}]},
            "dynamic_resources":{"hero_data_path":temp.path().join("heroes.json"),"match_data_dir":temp.path().join("matchdata")},
            "opendota_api":{"api_root":server.url,"player_segment":"players","account_matches_endpoint":"matches"}
        }));
        for id in ["1", "2"] {
            storage::write(&temp.path().join(format!("matchdata/{id}.json")), &games(1)).unwrap();
        }
        let client = Client::new();
        let telegram = Telegram::mock(client.clone(), server.url.clone());
        let dota = Dota {
            client,
            config,
            punlines: json!({}),
        };
        assert!(dota.crawl(&telegram).await.is_err());
        let path = temp.path().join("crawl.json");
        let failed: CrawlState = storage::read(&path).unwrap().unwrap();
        assert_eq!(failed.pending.len(), 1);
        assert_eq!(failed.accounts["1"][0].match_id, 2);
        assert!(failed.pending[0].contains("One &amp; Two"));
        dota.crawl(&telegram).await.unwrap();
        let recovered: CrawlState = storage::read(&path).unwrap().unwrap();
        assert!(recovered.pending.is_empty());
        assert_eq!(recovered.announced.len(), 1);
        let requests = server.requests.lock().unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|(line, _)| line.starts_with("POST /sendMessage"))
                .count(),
            2
        );
    }
    #[tokio::test]
    async fn delayed_third_player_does_not_repeat_an_announced_match() {
        let temp = tempfile::tempdir().unwrap();
        let server = Server::new(vec![
            (200, games(2)),
            (200, games(2)),
            (200, games(1)),
            (200, json!({"ok":true,"result":{}})),
            (200, games(2)),
            (200, games(2)),
            (200, games(2)),
        ]);
        let config = Config(json!({
            "accounts":{"account_list":[{"identifier":"1","name":"One"},{"identifier":"2","name":"Two"},{"identifier":"3","name":"Three"}]},
            "dynamic_resources":{"hero_data_path":temp.path().join("heroes.json"),"match_data_dir":temp.path().join("matchdata")},
            "opendota_api":{"api_root":server.url,"player_segment":"players","account_matches_endpoint":"matches"}
        }));
        for id in ["1", "2", "3"] {
            storage::write(&temp.path().join(format!("matchdata/{id}.json")), &games(1)).unwrap();
        }
        let client = Client::new();
        let telegram = Telegram::mock(client.clone(), server.url.clone());
        let dota = Dota {
            client,
            config,
            punlines: json!({}),
        };
        dota.crawl(&telegram).await.unwrap();
        dota.crawl(&telegram).await.unwrap();
        assert_eq!(
            server
                .requests
                .lock()
                .unwrap()
                .iter()
                .filter(|(line, _)| line.starts_with("POST /sendMessage"))
                .count(),
            1
        );
    }
    #[tokio::test]
    async fn upstream_failure_does_not_advance_snapshots() {
        let temp = tempfile::tempdir().unwrap();
        let server = Server::new(vec![
            (200, games(2)),
            (429, json!({"error":"rate limited"})),
        ]);
        let config = Config(json!({
            "accounts":{"account_list":[{"identifier":"1","name":"One"},{"identifier":"2","name":"Two"}]},
            "dynamic_resources":{"hero_data_path":temp.path().join("heroes.json"),"match_data_dir":temp.path().join("matchdata")},
            "opendota_api":{"api_root":server.url,"player_segment":"players","account_matches_endpoint":"matches"}
        }));
        let client = Client::new();
        let telegram = Telegram::mock(client.clone(), server.url.clone());
        let dota = Dota {
            client,
            config,
            punlines: json!({}),
        };
        assert!(dota.crawl(&telegram).await.is_err());
        assert!(!temp.path().join("crawl.json").exists());
    }
    #[tokio::test]
    async fn new_features_baseline_silently_then_queue_records_and_rampages_once() {
        let temp = tempfile::tempdir().unwrap();
        let matches = |id, slot| json!([{"match_id":id,"radiant_win":true,"player_slot":slot,"hero_id":1,"kills":id*3,"deaths":1,"assists":id*4}]);
        let ok = json!({"ok":true,"result":{}});
        let server = Server::new(vec![
            (200, matches(1, 0)),
            (200, matches(1, 1)),
            (200, matches(2, 0)),
            (200, matches(2, 1)),
            (200, ok.clone()),
            (200, ok.clone()),
            (200, ok.clone()),
            (
                200,
                json!({"match_id":2,"players":[{"player_slot":0,"account_id":1,"multi_kills":{"5":1}},{"player_slot":1,"account_id":2,"multi_kills":{}}]}),
            ),
            (200, ok),
            (200, matches(2, 0)),
            (200, matches(2, 1)),
        ]);
        let config = Config(json!({
            "dynamic_resources":{"hero_data_path":temp.path().join("heroes.json"),"match_data_dir":temp.path().join("legacy")},
            "accounts":{"account_list":[{"identifier":"1","name":"One"},{"identifier":"2","name":"Two"}]},
            "opendota_api":{"api_root":server.url,"player_segment":"players","account_matches_endpoint":"matches"},
            "match_features":{"rampages":true,"personal_records":true}
        }));
        let client = Client::new();
        let telegram = Telegram::mock(client.clone(), server.url.clone());
        let dota = Dota {
            client,
            config,
            punlines: json!({}),
        };
        dota.crawl(&telegram).await.unwrap();
        dota.details(&telegram).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 2);
        dota.crawl(&telegram).await.unwrap();
        dota.details(&telegram).await.unwrap();
        dota.crawl(&telegram).await.unwrap();
        dota.details(&telegram).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 11);
        assert!(requests[4].1["text"].as_str().unwrap().contains("Dotabuff"));
        assert!(
            requests[5].1["text"]
                .as_str()
                .unwrap()
                .contains("persönlicher Rekord")
        );
        assert!(requests[8].1["text"].as_str().unwrap().contains("RAMPAGE"));
        let state: CrawlState = storage::read(&temp.path().join("crawl.json"))
            .unwrap()
            .unwrap();
        assert!(state.pending.is_empty());
        assert!(state.details.is_empty());
        assert_eq!(state.records.len(), 2);
        assert_eq!(state.detail_checked.len(), 2);
    }
}
