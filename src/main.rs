mod birthdays;
mod config;
mod dota;
#[cfg(test)]
mod live_tests;
mod storage;
mod telegram;
#[cfg(test)]
mod test_support;
mod voices;

use anyhow::{Context, Result};
use chrono::{Datelike, Utc};
use config::Config;
use dota::Dota;
use reqwest::Client;
use serde_json::{Value, json};
use std::{collections::BTreeSet, time::Duration};
use telegram::Telegram;
use tokio::time::{Instant, MissedTickBehavior};
use voices::Voices;

const HTTP_USER_AGENT: &str = concat!(
    "sili-telegram-bot/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/Eixix/sili-telegram-bot)"
);

struct Bot {
    config: Config,
    telegram: Telegram,
    dota: Dota,
    voices: Voices,
    whitelist: BTreeSet<String>,
}
impl Bot {
    async fn poll(&self) -> Result<()> {
        self.telegram
            .upload(
                "sendVoice",
                "voice",
                &self.config.path("static_resources", "dodo_voiceline_path"),
            )
            .await?;
        let weekdays = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
        let weekday = Utc::now()
            .with_timezone(&self.config.timezone()?)
            .weekday()
            .num_days_from_monday() as usize;
        self.telegram.call("sendPoll", json!({"chat_id":self.telegram.chat,"question":format!("Do{}",weekdays[weekday]),
            "options":[{"text":dota::choose(&self.dota.punlines["dodo_poll"]["ja"])},{"text":dota::choose(&self.dota.punlines["dodo_poll"]["nein"])}],"is_anonymous":false,"allows_multiple_answers":false})).await?;
        Ok(())
    }
    async fn update(&mut self, update: &Value) -> Result<bool> {
        if let Some(query) = update.get("inline_query") {
            let user = query["from"]["id"]
                .as_i64()
                .map(|n| n.to_string())
                .unwrap_or_default();
            let results = if self.whitelist.contains(&user) {
                let text = query["query"].as_str().unwrap_or("");
                if text.chars().count() > 256 {
                    vec![]
                } else {
                    self.voices
                        .catalog
                        .read()
                        .await
                        .as_ref()
                        .map(|c| c.inline(text))
                        .unwrap_or_default()
                }
            } else {
                vec![]
            };
            self.telegram.call("answerInlineQuery", json!({"inline_query_id":query["id"],"results":results,"is_personal":true,"cache_time":0})).await?;
            return Ok(false);
        }
        let Some(message) = update.get("message") else {
            return Ok(false);
        };
        if message["chat"]["id"].as_i64() != Some(self.telegram.chat) {
            return Ok(false);
        }
        let Some(text) = message["text"].as_str() else {
            return Ok(false);
        };
        let (command, args) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
        let (command, addressed_to) = command
            .split_once('@')
            .map_or((command, None), |(c, b)| (c, Some(b)));
        if let Some(addressed_to) = addressed_to {
            let me = self.telegram.call("getMe", json!({})).await?;
            if !me["username"]
                .as_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(addressed_to))
            {
                return Ok(false);
            }
        }
        match command {
            "/dodo" => self.poll().await?,
            "/crawl" => self.dota.crawl(&self.telegram).await?,
            "/playerinfos" => {
                for text in self.dota.playerinfos().await? {
                    self.telegram.text(&text).await?;
                }
            }
            "/lastgame" => self.telegram.text(&self.dota.lastgame().await?).await?,
            "/birthdays" => {
                self.telegram
                    .text(
                        &birthdays::messages(
                            &self.config.accounts()?,
                            Utc::now()
                                .with_timezone(&self.config.timezone()?)
                                .date_naive(),
                        )?
                        .0,
                    )
                    .await?
            }
            "/stopbot" => {
                self.telegram.text("Sili-Bot wird gestoppt.").await?;
                return Ok(true);
            }
            "/enable_inline_voicelines" => {
                if let Some(user) = message["from"]["id"].as_i64() {
                    let mut whitelist = self.whitelist.clone();
                    whitelist.insert(user.to_string());
                    let path = self
                        .config
                        .path("inline_authentication", "user_whitelist_path");
                    storage::write_bytes(
                        &path,
                        format!(
                            "{}\n",
                            whitelist.iter().cloned().collect::<Vec<_>>().join("\n")
                        )
                        .as_bytes(),
                    )?;
                    self.whitelist = whitelist;
                    self.telegram
                        .text("You can now use the inline responses!")
                        .await?;
                }
            }
            "/voiceline" => {
                let link = match self.voices.link(args.trim()).await {
                    Ok(link) => link,
                    Err(error) => {
                        self.telegram
                            .text(&storage::html(&error.to_string()))
                            .await?;
                        return Ok(false);
                    }
                };
                let data = self
                    .dota
                    .client
                    .get(link)
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?;
                if let Err(error) = self
                    .telegram
                    .call(
                        "deleteMessage",
                        json!({"chat_id":self.telegram.chat,"message_id":message["message_id"]}),
                    )
                    .await
                {
                    tracing::warn!(%error, "Could not delete voiceline command");
                }
                let user = &message["from"];
                let name = user["username"]
                    .as_str()
                    .or_else(|| user["first_name"].as_str())
                    .unwrap_or("Unknown user");
                self.telegram
                    .text(&format!("{}:", storage::html(name)))
                    .await?;
                self.telegram
                    .upload_bytes("sendVoice", "voice", "response.mp3".into(), data.to_vec())
                    .await?;
            }
            _ if !text.starts_with('/') => {
                let text = text.to_lowercase();
                if ["doubt", "daud", "daut"].iter().any(|s| text.contains(s)) {
                    self.telegram
                        .upload(
                            "sendAnimation",
                            "animation",
                            &self.config.path("static_resources", "daut_gif_path"),
                        )
                        .await?;
                }
            }
            _ => {}
        }
        Ok(false)
    }
    async fn daily(&self) -> Result<()> {
        let today = Utc::now()
            .with_timezone(&self.config.timezone()?)
            .date_naive();
        let path = self.config.state_dir().join("daily.json");
        let mut sent: BTreeSet<String> = storage::read(&path)?.unwrap_or_default();
        let (_, upcoming, now) = birthdays::messages(&self.config.accounts()?, today)?;
        for (name, message) in [("upcoming", upcoming), ("birthday", now)] {
            let key = format!("{today}:{name}");
            if !sent.contains(&key) {
                if let Some(message) = message {
                    self.telegram.text(&message).await?;
                }
                sent.insert(key);
                storage::write(&path, &sent)?;
            }
        }
        if today.weekday() == chrono::Weekday::Thu {
            let key = format!("{today}:poll");
            if !sent.contains(&key) {
                self.poll().await?;
                sent.insert(key);
                storage::write(&path, &sent)?;
            }
        }
        sent.retain(|k| k.starts_with(&today.to_string()));
        storage::write(&path, &sent)
    }
}
async fn run(mut bot: Bot) -> Result<()> {
    let telegram = bot.telegram.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let poller = tokio::spawn(async move {
        let mut offset = 0;
        loop {
            match telegram.poll_updates(offset).await {
                Ok(updates) => {
                    for update in updates {
                        let next = update["update_id"].as_i64().unwrap_or(0) + 1;
                        if tx.send(update).await.is_err() {
                            return;
                        }
                        offset = next;
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "Telegram polling failed");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });
    let voices = bot.voices.clone();
    let refresher = tokio::spawn(async move {
        if voices.refresh_enabled() {
            loop {
                match voices.refresh().await {
                    Ok(()) => tracing::info!("Voiceline cache refreshed"),
                    Err(error) => {
                        tracing::warn!(%error, "Voiceline refresh failed; retaining cache")
                    }
                }
                tokio::time::sleep(Duration::from_secs(7 * 24 * 3600)).await;
            }
        }
    });
    let mut tick = tokio::time::interval(Duration::from_secs(10));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut crawl_at = Instant::now() + Duration::from_secs(10);
    let mut patches_at = crawl_at;
    let mut heroes_at = Instant::now();
    let mut daily_at = Instant::now();
    tracing::info!("Sili bot started");
    loop {
        tokio::select! {
            update = rx.recv() => {
                let Some(update) = update else {break};
                match bot.update(&update).await {
                    Ok(true) => break,
                    Ok(false) => {},
                    Err(error) => {
                        tracing::warn!(%error, "Command failed");
                        if update.get("message").is_some() {
                            // Only the authorized chat can execute commands.
                            if let Err(e) = bot.telegram.text("Die Anfrage ist fehlgeschlagen. Bitte später erneut versuchen; /voiceline nutzt Entity: Text [level].").await {tracing::warn!(error=%e,"Error reply failed");}
                        }
                    }
                }
            },
            _ = tick.tick() => {
                if Instant::now() >= heroes_at {
                    let result = bot.dota.update_heroes().await;
                    let success = result.is_ok(); report("heroes",result);
                    heroes_at = Instant::now()+Duration::from_secs(if success {7*24*3600} else {600});
                }
                if Instant::now() >= crawl_at { report("matches", bot.dota.crawl(&bot.telegram).await); crawl_at = Instant::now()+Duration::from_secs(600); }
                if Instant::now() >= patches_at { report("patches", bot.dota.patches(&bot.telegram).await); patches_at = Instant::now()+Duration::from_secs(30); }
                if Instant::now() >= daily_at { report("daily", bot.daily().await); daily_at = Instant::now()+Duration::from_secs(60); }
            }
        }
    }
    poller.abort();
    refresher.abort();
    Ok(())
}
fn report(job: &str, result: Result<()>) {
    if let Err(error) = result {
        tracing::warn!(job,%error,"Scheduled job failed");
    }
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! {_ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {}}
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    let level = config.string("logging", "level").to_lowercase();
    let level = match level.as_str() {
        "critical" => "error",
        "warning" => "warn",
        "" => "info",
        other => other,
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(format!("sili_telegram_bot={level}")))
        .context("Invalid logging level")?;
    tracing_subscriber::fmt().with_env_filter(filter).init();
    if std::env::args().any(|a| a == "--check-config") {
        println!("Configuration is valid");
        return Ok(());
    }
    let client = Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(35))
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let telegram = Telegram::new(
        client.clone(),
        &config.string("secrets", "bot_token"),
        config.chat_id()?,
    );
    let voices = Voices::new(config.clone(), client.clone())?;
    if std::env::args().any(|a| a == "--refresh-voicelines") {
        return voices.refresh().await;
    }
    let punlines = storage::read(&config.path("static_resources", "punline_path"))?
        .context("Missing punlines file")?;
    let whitelist_path = config.path("inline_authentication", "user_whitelist_path");
    let whitelist = match std::fs::read_to_string(whitelist_path) {
        Ok(data) => data
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeSet::new(),
        Err(e) => return Err(e.into()),
    };
    let bot = Bot {
        dota: Dota {
            config: config.clone(),
            client,
            punlines,
        },
        config,
        telegram,
        voices,
        whitelist,
    };
    tokio::select! { result = run(bot) => result, _ = shutdown() => {tracing::info!("Shutdown requested"); Ok(())} }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Server;
    #[tokio::test]
    async fn restricts_chat_and_inline_access_and_persists_separate_user_ids() {
        let temp = tempfile::tempdir().unwrap();
        let server = Server::new(vec![
            (200, json!({"ok":true,"result":true})),
            (200, json!({"ok":true,"result":{}})),
            (200, json!({"ok":true,"result":{}})),
        ]);
        let config = Config(json!({
            "inline_authentication":{"user_whitelist_path":temp.path().join("whitelist.txt")},
            "dynamic_resources":{"hero_data_path":temp.path().join("heroes.json")},
            "voicelines":{"entity_data_file":temp.path().join("entities.json"),"resource_file":temp.path().join("responses.json")}
        }));
        let client = Client::new();
        let mut bot = Bot {
            config: config.clone(),
            telegram: Telegram::mock(client.clone(), server.url.clone()),
            voices: Voices::new(config.clone(), client.clone()).unwrap(),
            dota: Dota {
                config,
                client,
                punlines: json!({}),
            },
            whitelist: BTreeSet::new(),
        };
        assert!(
            !bot.update(&json!({"message":{"chat":{"id":999},"text":"/stopbot"}}))
                .await
                .unwrap()
        );
        bot.update(&json!({"inline_query":{"id":"test","from":{"id":77},"query":"Legion"}}))
            .await
            .unwrap();
        for id in [77, 88] {
            bot.update(&json!({"message":{"chat":{"id":-123},"from":{"id":id},"text":"/enable_inline_voicelines"}})).await.unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(temp.path().join("whitelist.txt")).unwrap(),
            "77\n88\n"
        );
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].1["results"], json!([]));
    }
}
