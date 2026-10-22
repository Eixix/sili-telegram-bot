mod birthdays;
mod challenges;
mod community;
mod config;
mod dota;
#[cfg(test)]
mod live_tests;
mod members;
mod poll_sound;
mod replay;
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
        self.poll_at(Utc::now().with_timezone(&self.config.timezone()?))
            .await
    }
    async fn poll_at(&self, now: chrono::DateTime<chrono_tz::Tz>) -> Result<()> {
        let day = now.date_naive();
        if challenges::enabled(&self.config) && challenges::thursday(day) {
            if let Some(message_id) = challenges::existing(&self.config, day, self.telegram.chat)? {
                self.telegram.call("sendMessage",json!({"chat_id":self.telegram.chat,"text":"Die DoDo-Umfrage für heute gibt es bereits. Die Challenge wird um 20:00 Uhr ausgelost.","reply_parameters":{"message_id":message_id,"allow_sending_without_reply":true}})).await?;
                return Ok(());
            }
            if challenges::started(now) {
                self.telegram.text("Die DoDo-Anmeldung für heute ist um 20:00 Uhr beendet. Eine neue Umfrage würde die Teilnehmerauswahl verändern.").await?;
                return Ok(());
            }
        }
        self.telegram
            .upload(
                "sendVoice",
                "voice",
                &self.config.path("static_resources", "dodo_voiceline_path"),
            )
            .await?;
        let weekdays = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
        let weekday = now.weekday().num_days_from_monday() as usize;
        let sent=self.telegram.call("sendPoll", json!({"chat_id":self.telegram.chat,"question":format!("Do{}",weekdays[weekday]),
            "options":[{"text":dota::choose(&self.dota.punlines["dodo_poll"]["ja"])},{"text":dota::choose(&self.dota.punlines["dodo_poll"]["nein"])}],"is_anonymous":false,"allows_multiple_answers":false})).await?;
        challenges::register(&self.config, day, self.telegram.chat, &sent)?;
        poll_sound::register(&self.config, self.telegram.chat, &sent)?;
        Ok(())
    }
    async fn update(&mut self, update: &Value) -> Result<bool> {
        if update.get("poll_answer").is_some() || update.get("poll").is_some() {
            challenges::update(
                &self.config,
                self.telegram.chat,
                update,
                Utc::now().with_timezone(&self.config.timezone()?),
            )?;
            poll_sound::observe(&self.config, self.telegram.chat, update)?;
            return Ok(false);
        }
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
            "/challenge" => {
                let now = Utc::now().with_timezone(&self.config.timezone()?);
                self.telegram
                    .text(&challenges::status(&self.config, now, self.telegram.chat)?)
                    .await?;
            }
            "/memeteam" => {
                self.telegram
                    .text(&community::memeteam(&self.config, args)?)
                    .await?
            }
            "/memestrats" => self.telegram.text(&community::strategies()).await?,
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
                let (link, caption) = match self.voices.link(args.trim()).await {
                    Ok(link) => link,
                    Err(error) => {
                        self.telegram
                            .text(&storage::html(&error.to_string()))
                            .await?;
                        return Ok(false);
                    }
                };
                let data = self.voices.audio(&link).await?;
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
                    .upload_bytes_caption(
                        "sendVoice",
                        "voice",
                        "response.mp3".into(),
                        data,
                        Some(&caption),
                    )
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
        self.daily_at(Utc::now().with_timezone(&self.config.timezone()?))
            .await
    }
    async fn daily_at(&self, now: chrono::DateTime<chrono_tz::Tz>) -> Result<()> {
        let today = now.date_naive();
        let path = self.config.state_dir().join("daily.json");
        let mut sent: BTreeSet<String> = storage::read(&path)?.unwrap_or_default();
        let (_, upcoming, birthday) = birthdays::messages(&self.config.accounts()?, today)?;
        for (name, message) in [("upcoming", upcoming), ("birthday", birthday)] {
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
            if !sent.contains(&key)
                && (!challenges::enabled(&self.config) || !challenges::started(now))
            {
                self.poll_at(now).await?;
                sent.insert(key);
                storage::write(&path, &sent)?;
            }
        }
        sent.retain(|k| k.starts_with(&today.to_string()));
        storage::write(&path, &sent)
    }
}
type IncomingUpdate = (Value, tokio::sync::oneshot::Sender<bool>);
async fn forward_updates(
    telegram: Telegram,
    tx: tokio::sync::mpsc::Sender<IncomingUpdate>,
    mut offset: i64,
) {
    loop {
        match telegram.poll_updates(offset).await {
            Ok(updates) => {
                for update in updates {
                    let next = update["update_id"].as_i64().unwrap_or(0) + 1;
                    let (ack, received) = tokio::sync::oneshot::channel();
                    if tx.send((update, ack)).await.is_err() {
                        return;
                    }
                    match received.await {
                        Ok(true) => offset = next,
                        Ok(false) => {
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            break;
                        }
                        Err(_) => return,
                    }
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Telegram polling failed");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
async fn run(mut bot: Bot) -> Result<()> {
    let telegram = bot.telegram.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let offset_path = bot.config.state_dir().join("telegram-offset.json");
    let offset = storage::read::<i64>(&offset_path)?.unwrap_or(0);
    let poller = tokio::spawn(forward_updates(telegram, tx, offset));
    let voices = bot.voices.clone();
    let refresher = tokio::spawn(async move {
        if voices.refresh_enabled() {
            loop {
                let retry_in = match voices.refresh().await {
                    Ok(()) => {
                        tracing::info!("Voiceline cache refreshed");
                        7 * 24 * 3600
                    }
                    Err(error) => {
                        let retry_in = voices::refresh_retry_seconds(&error);
                        tracing::warn!(%error, retry_in_seconds = retry_in, "Voiceline refresh failed; retaining cache");
                        retry_in
                    }
                };
                tokio::time::sleep(Duration::from_secs(retry_in)).await;
            }
        }
    });
    let mut tick = tokio::time::interval(Duration::from_secs(10));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut crawl_at = Instant::now() + Duration::from_secs(10);
    let mut patches_at = crawl_at;
    let mut heroes_at = Instant::now();
    let mut daily_at = Instant::now();
    let mut details_at = crawl_at;
    tracing::info!("Sili bot started");
    loop {
        tokio::select! {
            biased;
            update = rx.recv() => {
                let Some((update,ack)) = update else {break};
                let mut accepted=true;
                let mut stop=false;
                match bot.update(&update).await {
                    Ok(true) => stop=true,
                    Ok(false) => {},
                    Err(error) => {
                        tracing::warn!(%error, "Command failed");
                        accepted=update.get("poll").is_none() && update.get("poll_answer").is_none();
                        if update.get("message").is_some() {
                            // Only the authorized chat can execute commands.
                            if let Err(e) = bot.telegram.text("Die Anfrage ist fehlgeschlagen. Bitte später erneut versuchen; /voiceline nutzt Entity: Text [level].").await {tracing::warn!(error=%e,"Error reply failed");}
                        }
                    }
                }
                if accepted {
                    let next=update["update_id"].as_i64().context("Missing Telegram update ID")?+1;
                    storage::write(&offset_path,&next)?;
                }
                let _=ack.send(accepted);
                if stop {break;}
            },
            _ = tick.tick() => {
                report("five yes sound",poll_sound::deliver(&bot.config,&bot.telegram,Utc::now().with_timezone(&bot.config.timezone()?)).await);
                report("Thursday challenge",challenges::advance(&bot.config,&bot.telegram,Utc::now().with_timezone(&bot.config.timezone()?)).await);
                if Instant::now() >= heroes_at {
                    let result = bot.dota.update_heroes().await;
                    let success = result.is_ok(); report("heroes",result);
                    heroes_at = Instant::now()+Duration::from_secs(if success {7*24*3600} else {600});
                }
                if Instant::now() >= crawl_at { report("matches", bot.dota.crawl(&bot.telegram).await); crawl_at = Instant::now()+Duration::from_secs(600); }
                if Instant::now() >= details_at { report("match details", bot.dota.details(&bot.telegram).await); details_at = Instant::now()+Duration::from_secs(60); }
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
    if let Some(message) = replay::command(&std::env::args().skip(1).collect::<Vec<_>>())? {
        println!("{message}");
        return Ok(());
    }
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
    fn community_bot(config: Config, server: &Server) -> Bot {
        let client = Client::new();
        Bot {
            config: config.clone(),
            telegram: Telegram::mock(client.clone(), server.url.clone()),
            voices: Voices::new(config.clone(), client.clone()).unwrap(),
            dota: Dota {
                config,
                client,
                punlines: json!({}),
            },
            whitelist: BTreeSet::new(),
        }
    }
    fn community_config(dir: &std::path::Path) -> Config {
        Config(json!({
            "dynamic_resources":{"hero_data_path":dir.join("heroes.json")},
            "voicelines":{"entity_data_file":dir.join("entities.json"),"resource_file":dir.join("responses.json")},
            "static_resources":{"dodo_voiceline_path":dir.join("dodo.mp3")},
            "accounts":{"account_list":[{"identifier":"1","name":"One"}]},
            "community":{"challenges_enabled":true}
        }))
    }
    #[tokio::test]
    async fn thursday_posts_only_poll_before_event_and_no_duplicate_after_restart() {
        use chrono::TimeZone;
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("dodo.mp3"), b"audio").unwrap();
        let ok = json!({"ok":true,"result":{}});
        let server = Server::new(vec![
            (200, ok.clone()),
            (
                200,
                json!({"ok":true,"result":{"message_id":99,"poll":{"id":"dodo","is_anonymous":false,"allows_multiple_answers":false}}}),
            ),
            (200, ok),
        ]);
        let config = community_config(temp.path());
        let bot = community_bot(config.clone(), &server);
        let morning = chrono_tz::Europe::Berlin
            .with_ymd_and_hms(2026, 10, 22, 10, 0, 0)
            .unwrap();
        bot.daily_at(morning).await.unwrap();
        let restarted = community_bot(config, &server);
        restarted.daily_at(morning).await.unwrap();
        restarted.poll_at(morning).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].0.starts_with("POST /sendVoice"));
        assert!(requests[1].0.starts_with("POST /sendPoll"));
        assert!(requests[2].1.get("reply_parameters").is_some());
        assert!(!temp.path().join("challenges.json").exists());
    }
    #[tokio::test]
    async fn poller_does_not_acknowledge_queued_updates_before_processing() {
        let server = Server::new(vec![(
            200,
            json!({"ok":true,"result":[{"update_id":77,"poll_answer":{}},{"update_id":78,"poll":{}}]}),
        )]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let poller = tokio::spawn(forward_updates(telegram, tx, 77));
        let (first, ack) = rx.recv().await.unwrap();
        assert_eq!(first["update_id"], 77);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(rx.try_recv().is_err());
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        ack.send(true).unwrap();
        let (second, ack) = rx.recv().await.unwrap();
        assert_eq!(second["update_id"], 78);
        ack.send(false).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        poller.abort();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests[0].1["offset"], 77);
        assert!(
            requests[0].1["allowed_updates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "poll_answer")
        );
    }
    #[tokio::test]
    async fn late_start_does_not_create_a_new_thursday_poll() {
        use chrono::TimeZone;
        let temp = tempfile::tempdir().unwrap();
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let config = community_config(temp.path());
        let bot = community_bot(config.clone(), &server);
        let now = chrono_tz::Europe::Berlin
            .with_ymd_and_hms(2026, 10, 22, 20, 0, 0)
            .unwrap();
        bot.daily_at(now).await.unwrap();
        assert!(server.requests.lock().unwrap().is_empty());
        bot.poll_at(now).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].0.starts_with("POST /sendMessage"));
        assert!(
            challenges::existing(&config, now.date_naive(), -123)
                .unwrap()
                .is_none()
        );
    }
    #[tokio::test]
    async fn weekday_dodo_poll_and_real_update_handler_trigger_the_five_yes_clip() {
        use chrono::TimeZone;
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("dodo.mp3"), b"opening clip").unwrap();
        let server = Server::new(vec![
            (200, json!({"ok":true,"result":{}})),
            (
                200,
                json!({"ok":true,"result":{"message_id":99,"poll":{"id":"friday","is_anonymous":false,"allows_multiple_answers":false}}}),
            ),
            (200, json!({"ok":true,"result":{}})),
        ]);
        let mut config = community_config(temp.path());
        config.0["community"]["five_yes_sound_enabled"] = json!(true);
        config.0["static_resources"]["five_yes_voiceline_path"] = json!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/static/sache.mp3")
        );
        let mut bot = community_bot(config.clone(), &server);
        let friday = chrono_tz::Europe::Berlin
            .with_ymd_and_hms(2026, 10, 23, 19, 0, 0)
            .unwrap();
        bot.poll_at(friday).await.unwrap();
        for (sequence, yes) in [(1, 4), (2, 5), (3, 6)] {
            bot.update(&json!({"update_id":sequence,"poll":{"id":"friday","options":[{"voter_count":yes},{"voter_count":0}]}})).await.unwrap();
            poll_sound::deliver(&config, &bot.telegram, friday)
                .await
                .unwrap();
        }
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[2].0.starts_with("POST /sendVoice"));
    }
    #[tokio::test]
    async fn public_community_commands_keep_chat_authorization() {
        let temp = tempfile::tempdir().unwrap();
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}})); 3]);
        let mut bot = community_bot(community_config(temp.path()), &server);
        bot.update(&json!({"message":{"chat":{"id":999},"text":"/memeteam global"}}))
            .await
            .unwrap();
        for text in ["/memeteam global", "/memestrats", "/challenge"] {
            bot.update(&json!({"message":{"chat":{"id":-123},"text":text}}))
                .await
                .unwrap();
        }
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(
            requests[0].1["text"]
                .as_str()
                .unwrap()
                .contains("Position 5:")
        );
        assert!(temp.path().join("memeteam.json").exists());
    }
}
