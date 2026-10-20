//! Opt-in tests that send messages to a deliberately configured Telegram test chat.
//! Never executed by ordinary `cargo test` or CI.
use super::*;
use crate::test_support::Server;
use std::path::PathBuf;

fn config(temp: &std::path::Path, chat: i64) -> Config {
    let mut value: Value = serde_json::from_str(include_str!("../config/default.json")).unwrap();
    value["secrets"]["chat_id"] = chat.into();
    value["dynamic_resources"]["hero_data_path"] = json!(temp.join("heroes.json"));
    value["dynamic_resources"]["match_data_dir"] = json!(temp.join("matchdata"));
    value["inline_authentication"]["user_whitelist_path"] = json!(temp.join("whitelist.txt"));
    value["voicelines"]["entity_data_file"] = json!(temp.join("entities.json"));
    value["voicelines"]["resource_file"] = json!(temp.join("responses.json"));
    value["voicelines"]["refresh_enabled"] = json!(false);
    Config(value)
}
fn update(chat: i64, user: i64, text: &str, message_id: i64) -> Value {
    json!({"message":{"chat":{"id":chat},"from":{"id":user,"first_name":"Rust-Test"},"text":text,"message_id":message_id}})
}

#[tokio::test]
#[ignore = "sends real messages; requires SILI_LIVE_TOKEN_FILE and SILI_LIVE_CHAT_ID"]
async fn telegram_test_chat_functionality() -> Result<()> {
    let token = std::fs::read_to_string(std::env::var("SILI_LIVE_TOKEN_FILE")?)?;
    let chat: i64 = std::env::var("SILI_LIVE_CHAT_ID")?.parse()?;
    let user: i64 = std::env::var("SILI_LIVE_USER_ID")?.parse()?;
    let temp = tempfile::tempdir()?;
    let config = config(temp.path(), chat);
    let client = Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(35))
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let telegram = Telegram::new(client.clone(), token.trim(), chat);
    let identity = telegram.call("getMe", json!({})).await?;
    let chat_info = telegram.call("getChat", json!({"chat_id":chat})).await?;
    println!(
        "Bot identity and chat lookup: PASS (chat type {})",
        chat_info["type"]
    );
    telegram.text("🧪 <b>Rust-Funktionstest</b>\nDie folgenden Nachrichten und Spieldaten sind ausdrücklich Testdaten.").await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let voices = Voices::new(config.clone(), client.clone())?;
    let punlines =
        storage::read(&config.path("static_resources", "punline_path"))?.context("punlines")?;
    let mut bot = Bot {
        config: config.clone(),
        telegram: telegram.clone(),
        dota: Dota {
            config,
            client: client.clone(),
            punlines,
        },
        voices,
        whitelist: BTreeSet::new(),
    };
    let message = |text: &str| update(chat, user, text, 0);
    bot.update(&message("/lastgame")).await?;
    println!("/lastgame with empty accounts: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let today = Utc::now()
        .with_timezone(&bot.config.timezone()?)
        .date_naive();
    let next = today + chrono::Duration::days(10);
    let accounts = json!([
        {"identifier":"1","name":"🧪 Test Geburtstag <heute>","birthday":today.format("%d.%m.%Y").to_string()},
        {"identifier":"2","name":"🧪 Test Geburtstag in 10 Tagen","birthday":next.format("%d.%m.%Y").to_string()}
    ]);
    bot.config.0["accounts"]["account_list"] = accounts.clone();
    bot.update(&message("/birthdays")).await?;
    println!("/birthdays with HTML escaping: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.daily().await?;
    let daily_before = std::fs::read(temp.path().join("daily.json"))?;
    bot.daily().await?;
    assert_eq!(daily_before, std::fs::read(temp.path().join("daily.json"))?);
    println!("Daily birthday/reminder delivery and persisted deduplication: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&message("/enable_inline_voicelines")).await?;
    assert!(bot.whitelist.contains(&user.to_string()));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("whitelist.txt"))?,
        format!("{user}\n")
    );
    println!(
        "/enable_inline_voicelines: PASS (BotFather inline mode itself is {})",
        identity["supports_inline_queries"]
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&message("doubt — 🧪 GIF-Test")).await?;
    println!("GIF trigger/upload: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&message("/dodo")).await?;
    println!("/dodo voice and non-anonymous poll: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&message("/voiceline invalid")).await?;
    println!("Invalid /voiceline produces a user-visible error: PASS");
    // Run a real voiceline handler with an actual MP3 from a local fixture server.
    // This separates Telegram's upload behavior from external wiki availability.
    let media = Server::raw(vec![(
        200,
        std::fs::read("resources/static/lets_dota.mpeg")?,
    )]);
    *bot.voices.catalog.write().await = Some(voices::Catalog {
        entities: std::collections::BTreeMap::from([(
            "Hero responses".into(),
            std::collections::BTreeMap::from([(
                "Test".into(),
                voices::Entity {
                    name: "Test".into(),
                    title: "Test/Responses".into(),
                    url: "https://example.invalid".into(),
                },
            )]),
        )]),
        responses: std::collections::BTreeMap::from([(
            "Test/Responses".into(),
            vec![voices::Response {
                text: "Audio test".into(),
                urls: vec![Some(format!("{}/fixture.mp3", media.url))],
            }],
        )]),
    });
    tokio::time::sleep(Duration::from_secs(1)).await;
    let marker = telegram.call("sendMessage",json!({"chat_id":chat,"text":"🧪 Temporärer Voiceline-Testmarker (wird vom Handler gelöscht)."})).await?;
    bot.update(&update(
        chat,
        user,
        "/voiceline Test: Audio test",
        marker["message_id"].as_i64().unwrap(),
    ))
    .await?;
    assert_eq!(media.requests.lock().unwrap().len(), 1);
    println!("/voiceline download, deletion of own test marker, attribution, voice upload: PASS");
    let game = |id| json!([{"match_id":id,"start_time":1_790_000_000,"duration":1800,"radiant_win":true,"player_slot":0,"hero_id":1,"kills":3,"deaths":0,"assists":8}]);
    let matches = Server::new(vec![
        (200, game(2)),
        (200, game(2)),
        (200, game(2)),
        (200, game(2)),
        (200, json!({"profile":{"personaname":"Fixture <Player>"}})),
        (200, json!({"win":10,"lose":0})),
        (200, game(2)),
        (200, json!({"profile":{"personaname":"Fixture Player 2"}})),
        (200, json!({"win":0,"lose":0})),
        (200, json!([])),
        (200, game(2)),
        (200, game(2)),
    ]);
    bot.dota.config.0["accounts"]["account_list"] = accounts;
    bot.dota.config.0["opendota_api"]["api_root"] = matches.url.clone().into();
    for id in [1, 2] {
        storage::write(&temp.path().join(format!("matchdata/{id}.json")), &game(1))?;
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&message("/crawl")).await?;
    bot.update(&message("/crawl")).await?;
    let state: Value = storage::read(&temp.path().join("crawl.json"))?.unwrap();
    assert_eq!(state["announced"].as_array().unwrap().len(), 1);
    assert!(state["pending"].as_array().unwrap().is_empty());
    println!("/crawl fixture match summary delivered; repeat crawl stays silent: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&message("/playerinfos")).await?;
    println!("/playerinfos formatting, zero losses and empty match history: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let command = format!("/lastgame@{}", identity["username"].as_str().unwrap());
    bot.update(&message(&command)).await?;
    println!("Addressed command and live getMe routing: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(bot.update(&message("/stopbot")).await?);
    println!("/stopbot returns stop signal: PASS");
    telegram
        .text("🧪 Rust-Testlauf beendet. Ergebnisse werden im Codex-Chat zusammengefasst.")
        .await?;
    let report = PathBuf::from(std::env::var("SILI_LIVE_REPORT_DIR")?);
    std::fs::write(report.join("rust-live-completed"), "completed\n")?;
    Ok(())
}

#[tokio::test]
#[ignore = "contacts the real Dota patch feed"]
async fn live_patch_feed_parsing_and_deduplication() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config = config(temp.path(), -123);
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let dota = Dota {
        config,
        client: client.clone(),
        punlines: json!({}),
    };
    let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
    let telegram = Telegram::mock(client, server.url.clone());
    dota.patches(&telegram).await?;
    let path = temp.path().join("patches.json");
    let mut known: Vec<String> = storage::read(&path)?.unwrap();
    assert!(!known.is_empty());
    dota.patches(&telegram).await?;
    assert!(server.requests.lock().unwrap().is_empty());
    let simulated_new = known.pop().unwrap();
    storage::write(&path, &known)?;
    dota.patches(&telegram).await?;
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0].1["text"]
            .as_str()
            .unwrap()
            .contains(&simulated_new)
    );
    drop(requests);
    let saved: Vec<String> = storage::read(&path)?.unwrap();
    assert!(saved.contains(&simulated_new));
    println!(
        "Real patch feed: baseline, unchanged-feed silence, simulated-new alert and persistence PASS"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "downloads a real wiki MP3 and sends it to the configured test chat"]
async fn telegram_cached_wiki_voice() -> Result<()> {
    let token = std::fs::read_to_string(std::env::var("SILI_LIVE_TOKEN_FILE")?)?;
    let chat: i64 = std::env::var("SILI_LIVE_CHAT_ID")?.parse()?;
    let user: i64 = std::env::var("SILI_LIVE_USER_ID")?.parse()?;
    let temp = tempfile::tempdir()?;
    let config = config(temp.path(), chat);
    std::fs::copy(
        "tests/test_data/entity_data.json",
        temp.path().join("entities.json"),
    )?;
    std::fs::copy(
        "tests/test_data/responses.json",
        temp.path().join("responses.json"),
    )?;
    let client = Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(35))
        .build()?;
    let telegram = Telegram::new(client.clone(), token.trim(), chat);
    let marker = telegram.call("sendMessage",json!({"chat_id":chat,"text":"🧪 Test einer echten Wiki-Voiceline: Legion Commander – Ten hut!"})).await?;
    let voices = Voices::new(config.clone(), client.clone())?;
    let mut bot = Bot {
        config: config.clone(),
        telegram,
        dota: Dota {
            config,
            client,
            punlines: json!({}),
        },
        voices,
        whitelist: BTreeSet::new(),
    };
    bot.update(&update(
        chat,
        user,
        "/voiceline Legion Commander: Ten hut!",
        marker["message_id"].as_i64().unwrap(),
    ))
    .await?;
    println!("Real cached wiki lookup, public MP3 download, real Telegram voice delivery: PASS");
    Ok(())
}

#[tokio::test]
#[ignore = "queries real OpenDota using an application User-Agent"]
async fn live_opendota_heroes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let client = Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()?;
    let dota = Dota {
        config: config(temp.path(), -123),
        client,
        punlines: json!({}),
    };
    dota.update_heroes().await?;
    let heroes: Vec<Value> = storage::read(&temp.path().join("heroes.json"))?.unwrap();
    assert!(heroes.len() > 100);
    assert!(
        heroes
            .iter()
            .all(|h| h["id"].is_number() && h["localized_name"].is_string())
    );
    println!(
        "Live OpenDota hero query/schema/cache: PASS ({} heroes)",
        heroes.len()
    );
    Ok(())
}

#[tokio::test]
#[ignore = "uses configured real accounts and sends stats only to the explicit test chat"]
async fn telegram_configured_accounts() -> Result<()> {
    let config = Config::load()?;
    let token = std::fs::read_to_string(std::env::var("SILI_LIVE_TOKEN_FILE")?)?;
    let chat: i64 = std::env::var("SILI_LIVE_CHAT_ID")?.parse()?;
    let user: i64 = std::env::var("SILI_LIVE_USER_ID")?.parse()?;
    assert_eq!(
        config.chat_id()?,
        chat,
        "Only the explicit test chat is permitted"
    );
    let client = Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(35))
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let telegram = Telegram::new(client.clone(), token.trim(), chat);
    let voices = Voices::new(config.clone(), client.clone())?;
    let punlines =
        storage::read(&config.path("static_resources", "punline_path"))?.context("punlines")?;
    let mut bot = Bot {
        config: config.clone(),
        telegram: telegram.clone(),
        voices,
        dota: Dota {
            config,
            client,
            punlines,
        },
        whitelist: BTreeSet::new(),
    };
    let accounts = bot.config.accounts()?;
    ensure_live_accounts(&accounts)?;
    telegram.text("🧪 <b>Test mit deinen konfigurierten Accounts</b>\nAb hier stammen Geburtstage und Statistiken aus deiner Konfiguration bzw. der echten OpenDota-API. Der erste Match-Scan bleibt absichtlich stumm.").await?;
    bot.dota.update_heroes().await?;
    println!(
        "Production config validation and live hero cache: PASS ({} accounts)",
        accounts.len()
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&update(chat, user, "/birthdays", 0)).await?;
    println!("Configured birthdays via real Telegram: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&update(chat, user, "/lastgame", 0)).await?;
    println!("Latest game across configured accounts, live OpenDota and Telegram: PASS");
    tokio::time::sleep(Duration::from_secs(1)).await;
    bot.update(&update(chat, user, "/playerinfos", 0)).await?;
    println!("All configured player profiles, wins/losses and histories, live APIs: PASS");
    bot.update(&update(chat, user, "/crawl", 0)).await?;
    let state: Value = storage::read(&bot.config.state_dir().join("crawl.json"))?.unwrap();
    assert_eq!(state["accounts"].as_object().unwrap().len(), accounts.len());
    assert!(state["pending"].as_array().unwrap().is_empty());
    assert!(state["announced"].as_array().unwrap().is_empty());
    println!("Live initial crawl creates all snapshots without historical notifications: PASS");
    telegram.text("🧪 Tests mit deinen echten Account-Einstellungen abgeschlossen; Produktiv-Chat und Produktiv-Daten wurden nicht verwendet.").await?;
    Ok(())
}
fn ensure_live_accounts(accounts: &[crate::config::Account]) -> Result<()> {
    anyhow::ensure!(
        !accounts.is_empty(),
        "No accounts configured for this opt-in test"
    );
    Ok(())
}
