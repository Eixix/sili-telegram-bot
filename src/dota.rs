use crate::{
    config::{Account, Config},
    storage,
    telegram::Telegram,
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
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
            new_ids.extend(new_match_ids(previous.as_deref(), &current));
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
        }
        state.accounts = fetched;
        // Save snapshots and outbox together, only after all upstream requests succeed.
        storage::write(&path, &state)?;
        drain(&mut state, &path, telegram).await
    }
    fn summary(&self, players: &[(&Account, &Match)], heroes: &[Value]) -> String {
        let win = players[0].1.won();
        let outcome = choose(&self.punlines["match_outcome"][if win { "win" } else { "lose" }]);
        let mut parts = vec![format!(
            "<b>{}: {}</b>",
            if win { "W" } else { "L" },
            storage::html(&outcome)
        )];
        let mut used = HashSet::new();
        for (account, game) in players {
            let hero = heroes
                .iter()
                .find(|h| h["id"].as_u64() == Some(game.hero_id as u64))
                .and_then(|h| h["localized_name"].as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Unbekannter Held Nr. {}", game.hero_id));
            let mut categories: Vec<_> = self.punlines["performance_verbs"]
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(k, v)| k.parse::<f64>().ok().map(|n| (n, v)))
                .collect();
            categories.sort_by(|a, b| a.0.total_cmp(&b.0));
            let category = categories
                .iter()
                .find(|(n, _)| *n > game.ratio())
                .or(categories.last());
            let verb = if let Some((_, values)) = category {
                let available: Vec<_> = values
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .filter(|s| !used.contains(*s))
                    .collect();
                available
                    .choose(&mut rand::rng())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| choose(values))
            } else {
                "gespielt".into()
            };
            used.insert(verb.clone());
            parts.push(format!(
                "{} hat mit {} {} mit {} Kills, {} Toden und {} Assists",
                storage::html(&account.name),
                storage::html(&hero),
                storage::html(&verb),
                game.kills,
                game.deaths,
                game.assists
            ));
        }
        parts.join("\n\n")
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
}
