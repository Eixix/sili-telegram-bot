//! Thursday participation and one durable challenge, resolved only after voting closes.
use crate::{config::Config, members, storage, telegram::Telegram};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Datelike, NaiveDate, Timelike};
use chrono_tz::Tz;
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const CHALLENGES: &[&str] = &[
    "Spiele einen Helden, den du normalerweise nicht pickst.",
    "Sage vor dem Spiel deinen Lane-Plan an und besprecht ihn danach im Team.",
    "Organisiere einen Smoke-Gank mit mindestens drei Teammitgliedern.",
    "Stelle deinem Team einen ungewöhnlichen, aber begründeten Item-Build vor.",
    "Übernimm heute die Ansagen für Roshan und die nächsten Objectives.",
];
#[derive(Default, Deserialize, Serialize)]
struct State {
    #[serde(default)]
    counts: BTreeMap<String, u64>,
    #[serde(default)]
    last_player: Option<String>,
    #[serde(default)]
    events: BTreeMap<NaiveDate, Event>,
}
#[derive(Deserialize, Serialize)]
struct Event {
    poll_id: String,
    message_id: i64,
    chat: i64,
    #[serde(default)]
    votes: BTreeMap<String, Vote>,
    #[serde(default)]
    final_counts: Option<[u64; 2]>,
    #[serde(default)]
    closed_update: Option<i64>,
    #[serde(default)]
    quiet_until: i64,
    #[serde(default)]
    next_close_try: i64,
    #[serde(default)]
    next_delivery_try: i64,
    #[serde(default)]
    outcome: Option<String>,
    #[serde(default)]
    delivered: bool,
}
#[derive(Deserialize, Serialize)]
struct Vote {
    #[serde(default)]
    name: String,
    update_id: i64,
    option: Option<usize>,
}
fn load(config: &Config) -> Result<State> {
    if let Some(state) = storage::read(&config.state_dir().join("dodo-challenges-v1.json"))? {
        return Ok(state);
    }
    // Retain fair-rotation counters, never reuse a pre-participation assignment.
    let old: Option<Value> = storage::read(&config.state_dir().join("challenges.json"))?;
    let mut state = State::default();
    if let Some(old) = old {
        let links = members::configured(config)?;
        let old_counts: BTreeMap<String, u64> =
            serde_json::from_value(old["counts"].clone()).unwrap_or_default();
        for (account, count) in old_counts {
            if let Some(user) = links.get(&account) {
                state.counts.insert(user.to_string(), count);
            }
        }
        state.last_player = old["last_player"]
            .as_str()
            .and_then(|a| links.get(a))
            .map(i64::to_string);
    }
    Ok(state)
}
fn save(config: &Config, state: &State) -> Result<()> {
    storage::write(&config.state_dir().join("dodo-challenges-v1.json"), state)
}
pub fn enabled(config: &Config) -> bool {
    config.0["community"]["challenges_enabled"].as_bool() == Some(true)
}
pub fn thursday(day: NaiveDate) -> bool {
    day.weekday() == chrono::Weekday::Thu
}
pub fn started(now: DateTime<Tz>) -> bool {
    now.hour() >= 20
}
pub fn existing(config: &Config, day: NaiveDate, chat: i64) -> Result<Option<i64>> {
    Ok(load(config)?
        .events
        .get(&day)
        .filter(|e| e.chat == chat)
        .map(|e| e.message_id))
}
/// Current open poll metadata for features that can reuse the trusted Thursday registry.
pub fn open_poll(config: &Config, day: NaiveDate, chat: i64) -> Result<Option<(String, u64)>> {
    if !enabled(config) {
        return Ok(None);
    }
    Ok(load(config)?
        .events
        .get(&day)
        .filter(|e| e.chat == chat && e.final_counts.is_none() && e.outcome.is_none())
        .map(|e| {
            (
                e.poll_id.clone(),
                e.votes.values().filter(|v| v.option == Some(0)).count() as u64,
            )
        }))
}
pub fn register(config: &Config, day: NaiveDate, chat: i64, message: &Value) -> Result<()> {
    if !thursday(day) || !enabled(config) {
        return Ok(());
    }
    let poll = &message["poll"];
    let id = poll["id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("Missing sent poll ID")?;
    let message_id = message["message_id"]
        .as_i64()
        .context("Missing sent poll message ID")?;
    ensure!(
        poll["is_anonymous"] == false && poll["allows_multiple_answers"] == false,
        "Participation requires an identified single-answer poll"
    );
    let mut state = load(config)?;
    ensure!(
        !state.events.contains_key(&day),
        "A Thursday poll is already registered"
    );
    state.events.insert(
        day,
        Event {
            poll_id: id.into(),
            message_id,
            chat,
            votes: BTreeMap::new(),
            final_counts: None,
            closed_update: None,
            quiet_until: 0,
            next_close_try: 0,
            next_delivery_try: 0,
            outcome: None,
            delivered: false,
        },
    );
    save(config, &state)
}
fn counts(poll: &Value) -> Result<[u64; 2]> {
    let options = poll["options"]
        .as_array()
        .context("Missing final poll options")?;
    ensure!(options.len() == 2, "Unexpected final poll options");
    let counts = [
        options[0]["voter_count"]
            .as_u64()
            .context("Missing yes count")?,
        options[1]["voter_count"]
            .as_u64()
            .context("Missing no count")?,
    ];
    let total = counts[0]
        .checked_add(counts[1])
        .context("Poll voter count overflow")?;
    ensure!(
        poll["total_voter_count"].as_u64() == Some(total),
        "Incomplete final poll voter counts"
    );
    Ok(counts)
}
pub fn update(config: &Config, chat: i64, update: &Value, now: DateTime<Tz>) -> Result<()> {
    let answer = update.get("poll_answer");
    let poll = update.get("poll");
    let id = answer
        .and_then(|a| a["poll_id"].as_str())
        .or_else(|| poll.and_then(|p| p["id"].as_str()));
    let Some(id) = id else {
        return Ok(());
    };
    let mut state = load(config)?;
    let Some(event) = state
        .events
        .values_mut()
        .find(|e| e.poll_id == id && e.chat == chat)
    else {
        return Ok(());
    };
    if event.outcome.is_some() {
        return Ok(());
    }
    let sequence = update["update_id"]
        .as_i64()
        .context("Missing Telegram update ID")?;
    if let Some(answer) = answer {
        let voter = if let Some(id) = answer["user"]["id"].as_i64().filter(|id| *id > 0) {
            format!("user:{id}")
        } else if let Some(id) = answer["voter_chat"]["id"].as_i64() {
            format!("chat:{id}")
        } else {
            return Ok(());
        };
        if event
            .votes
            .get(&voter)
            .is_some_and(|v| v.update_id >= sequence)
        {
            return Ok(());
        }
        let options = answer["option_ids"]
            .as_array()
            .context("Missing vote options")?;
        ensure!(
            options.len() <= 1,
            "Unexpected multiple participation votes"
        );
        let option = options
            .first()
            .map(|v| {
                v.as_u64()
                    .filter(|n| *n < 2)
                    .map(|n| n as usize)
                    .context("Unknown participation option")
            })
            .transpose()?;
        event.votes.insert(
            voter,
            Vote {
                name: answer["user"]["first_name"]
                    .as_str()
                    .or_else(|| answer["user"]["username"].as_str())
                    .unwrap_or("Teilnehmer")
                    .to_owned(),
                update_id: sequence,
                option,
            },
        );
        event.quiet_until = now.timestamp() + 5;
    } else if let Some(poll) = poll {
        if poll["is_closed"] != true {
            return Ok(());
        }
        if event.closed_update.is_some_and(|n| n >= sequence) {
            return Ok(());
        }
        event.final_counts = Some(counts(poll)?);
        event.closed_update = Some(sequence);
        event.quiet_until = now.timestamp() + 5;
    }
    save(config, &state)
}
fn synchronized(event: &Event) -> bool {
    let mut observed = [0_u64; 2];
    for vote in event.votes.values() {
        if let Some(option) = vote.option {
            observed[option] += 1;
        }
    }
    event.closed_update.is_some() && event.final_counts == Some(observed)
}
pub fn status(config: &Config, now: DateTime<Tz>, chat: i64) -> Result<String> {
    if !enabled(config) {
        return Ok("Die DoDo-Challenge ist deaktiviert.".into());
    }
    if !thursday(now.date_naive()) {
        return Ok("Die DoDo-Challenge gibt es donnerstags um 20:00 Uhr.".into());
    }
    if !started(now) {
        return Ok("Die DoDo-Challenge wird heute um 20:00 Uhr unter den angemeldeten Teilnehmern ausgelost. Stimme vorher in der DoDo-Umfrage ab.".into());
    }
    let state = load(config)?;
    Ok(match state.events.get(&now.date_naive()).filter(|e|e.chat==chat) {
        Some(event)=>event.outcome.clone().unwrap_or_else(||"Die DoDo-Challenge wartet auf die endgültigen Stimmen der DoDo-Umfrage.".into()),
        None=>"Für heute wurde noch keine DoDo-Umfrage erfasst. Bereits bestehende Umfragen aus älteren Bot-Versionen können nicht nachträglich ausgewertet werden.".into(),
    })
}
pub async fn advance(config: &Config, telegram: &Telegram, now: DateTime<Tz>) -> Result<()> {
    if !enabled(config) || !thursday(now.date_naive()) || !started(now) {
        return Ok(());
    }
    let mut state = load(config)?;
    let day = now.date_naive();
    let Some(event) = state
        .events
        .get_mut(&day)
        .filter(|e| e.chat == telegram.chat)
    else {
        return Ok(());
    };
    if event.final_counts.is_none() && now.timestamp() >= event.next_close_try {
        event.next_close_try = now.timestamp() + 60;
        let message_id = event.message_id;
        let id = event.poll_id.clone();
        save(config, &state)?;
        let poll = telegram
            .call(
                "stopPoll",
                json!({"chat_id":telegram.chat,"message_id":message_id}),
            )
            .await?;
        ensure!(
            poll["id"].as_str() == Some(&id) && poll["is_closed"] == true,
            "Mismatched closed poll"
        );
        state.events.get_mut(&day).unwrap().final_counts = Some(counts(&poll)?);
        save(config, &state)?;
    }
    let event = &state.events[&day];
    if event.outcome.is_none() {
        // Wait for the closed-poll update AND all individual votes, then let queued
        // changes settle. Aggregate voter counts alone cannot identify participants.
        if !synchronized(event) || now.timestamp() < event.quiet_until {
            return Ok(());
        }
        let eligible: Vec<_> = event
            .votes
            .iter()
            .filter_map(|(id, vote)| {
                if vote.option != Some(0) {
                    return None;
                }
                let user = id.strip_prefix("user:")?.parse::<i64>().ok()?;
                Some((user, vote))
            })
            .collect();
        let message = if eligible.is_empty() {
            "Heute keine DoDo-Challenge: Es gibt keine namentlich bekannten Zusagen.".into()
        } else {
            let min = eligible
                .iter()
                .map(|(user, _)| *state.counts.get(&user.to_string()).unwrap_or(&0))
                .min()
                .unwrap();
            let candidates: Vec<_> = eligible
                .iter()
                .filter(|(user, _)| *state.counts.get(&user.to_string()).unwrap_or(&0) == min)
                .collect();
            let alternate: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|(user, _)| Some(&user.to_string()) != state.last_player.as_ref())
                .collect();
            let candidates = if alternate.is_empty() {
                &candidates
            } else {
                &alternate
            };
            let (member, vote) = candidates.choose(&mut rand::rng()).unwrap();
            let name = members::label(config, *member, &vote.name)?;
            let custom: Vec<_> = config.0["community"]["challenges"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .collect();
            let pool = if custom.is_empty() {
                CHALLENGES
            } else {
                &custom
            };
            let text = pool.choose(&mut rand::rng()).unwrap();
            *state.counts.entry(member.to_string()).or_default() += 1;
            state.last_player = Some(member.to_string());
            format!(
                "<b>Donnerstagschallenge · KW {}</b>\n<a href=\"tg://user?id={member}\">{}</a>: {}\nBesprecht die Challenge nach dem Spiel gemeinsam!",
                day.iso_week().week(),
                storage::html(&name),
                storage::html(text)
            )
        };
        state.events.get_mut(&day).unwrap().outcome = Some(message);
        save(config, &state)?;
    }
    let event = &state.events[&day];
    if !event.delivered && now.timestamp() >= event.next_delivery_try {
        let message = event.outcome.clone().unwrap();
        state.events.get_mut(&day).unwrap().next_delivery_try = now.timestamp() + 30;
        save(config, &state)?;
        telegram.text(&message).await?;
        state.events.get_mut(&day).unwrap().delivered = true;
        save(config, &state)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Server;
    use chrono::TimeZone;
    use reqwest::Client;
    fn clock(day: u32, hour: u32, second: u32) -> DateTime<Tz> {
        chrono_tz::Europe::Berlin
            .with_ymd_and_hms(2026, 10, day, hour, 0, second)
            .unwrap()
    }
    fn config(path: &std::path::Path) -> Config {
        Config(
            json!({"dynamic_resources":{"hero_data_path":path.join("heroes.json")},"community":{"challenges_enabled":true,"player_members":{"1":11,"2":22},"challenges":["A & B"]},"accounts":{"account_list":[{"identifier":"1","name":"One"},{"identifier":"2","name":"Two"},{"identifier":"3","name":"<Three>"}]}}),
        )
    }
    fn register_poll(config: &Config, day: u32) {
        register(config,clock(day,10,0).date_naive(),-123,&json!({"message_id":99,"poll":{"id":format!("poll-{day}"),"is_anonymous":false,"allows_multiple_answers":false}})).unwrap();
    }
    fn answer(
        config: &Config,
        day: u32,
        sequence: i64,
        user: i64,
        option: Option<u8>,
        now: DateTime<Tz>,
    ) {
        update(config,-123,&json!({"update_id":sequence,"poll_answer":{"poll_id":format!("poll-{day}"),"user":{"id":user,"first_name":if user==33 {"<Three>"} else {"Telegram name"}},"option_ids":option.into_iter().collect::<Vec<_>>()}}),now).unwrap();
    }
    fn closed(day: u32, yes: u64, no: u64) -> Value {
        json!({"id":format!("poll-{day}"),"is_closed":true,"total_voter_count":yes+no,"options":[{"voter_count":yes},{"voter_count":no}]})
    }
    fn close_update(
        config: &Config,
        day: u32,
        sequence: i64,
        yes: u64,
        no: u64,
        now: DateTime<Tz>,
    ) {
        update(
            config,
            -123,
            &json!({"update_id":sequence,"poll":closed(day,yes,no)}),
            now,
        )
        .unwrap();
    }
    #[tokio::test]
    async fn waits_for_twenty_and_final_votes_then_selects_only_yes_members() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        register_poll(&config, 22);
        let ok = json!({"ok":true,"result":{}});
        let server = Server::new(vec![
            (200, json!({"ok":true,"result":closed(22,2,1)})),
            (200, ok),
        ]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        answer(&config, 22, 1, 11, Some(0), clock(22, 19, 0));
        answer(&config, 22, 2, 22, Some(1), clock(22, 19, 0));
        let mut state = load(&config).unwrap();
        state.counts.insert("11".into(), 10);
        save(&config, &state).unwrap();
        advance(
            &config,
            &telegram,
            clock(22, 19, 59) + chrono::Duration::minutes(59),
        )
        .await
        .unwrap();
        assert!(server.requests.lock().unwrap().is_empty());
        assert!(
            status(
                &config,
                clock(22, 19, 59) + chrono::Duration::minutes(59),
                -123
            )
            .unwrap()
            .contains("20:00")
        );
        advance(&config, &telegram, clock(22, 20, 0)).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        close_update(&config, 22, 3, 2, 1, clock(22, 20, 0));
        advance(&config, &telegram, clock(22, 20, 6)).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        answer(&config, 22, 4, 33, Some(0), clock(22, 20, 7));
        advance(&config, &telegram, clock(22, 20, 13))
            .await
            .unwrap();
        advance(&config, &telegram, clock(22, 20, 14))
            .await
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 2);
        advance(&config, &telegram, clock(22, 20, 15))
            .await
            .unwrap();
        advance(&config, &telegram, clock(22, 20, 40))
            .await
            .unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        let text = requests[1].1["text"].as_str().unwrap();
        assert!(text.contains("tg://user?id=33"));
        assert!(text.contains("&lt;Three&gt;"));
        assert!(text.contains("A &amp; B"));
        assert!(!text.contains("Two"));
        let state = load(&config).unwrap();
        assert_eq!(state.counts["11"], 10);
        assert_eq!(state.counts["33"], 1);
        assert!(!state.counts.contains_key("22"));
    }
    #[tokio::test]
    async fn equal_count_swaps_retractions_and_replayed_updates_do_not_select_wrong_player() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        register_poll(&config, 22);
        let server = Server::new(vec![
            (200, json!({"ok":true,"result":closed(22,1,1)})),
            (200, json!({"ok":true,"result":{}})),
        ]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        answer(&config, 22, 1, 11, Some(0), clock(22, 19, 0));
        answer(&config, 22, 2, 22, Some(1), clock(22, 19, 0));
        advance(&config, &telegram, clock(22, 20, 0)).await.unwrap();
        // Final aggregate counts happen to match the old roster: still no draw.
        advance(&config, &telegram, clock(22, 20, 6)).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        answer(&config, 22, 3, 11, None, clock(22, 20, 7));
        answer(&config, 22, 4, 11, Some(1), clock(22, 20, 8));
        answer(&config, 22, 5, 22, Some(0), clock(22, 20, 9));
        answer(&config, 22, 1, 11, Some(0), clock(22, 20, 10));
        close_update(&config, 22, 6, 1, 1, clock(22, 20, 10));
        advance(&config, &telegram, clock(22, 20, 14))
            .await
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        advance(&config, &telegram, clock(22, 20, 16))
            .await
            .unwrap();
        let state = load(&config).unwrap();
        assert_eq!(state.last_player.as_deref(), Some("22"));
        assert!(state.events[&clock(22, 20, 0).date_naive()].delivered);
    }
    #[tokio::test]
    async fn failed_delivery_reuses_assignment_and_rotation_across_weeks() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        let server = Server::new(vec![
            (503, json!({"ok":false,"error_code":503})),
            (200, json!({"ok":true,"result":{}})),
            (200, json!({"ok":true,"result":{}})),
        ]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        register_poll(&config, 22);
        answer(&config, 22, 1, 11, Some(0), clock(22, 19, 0));
        answer(&config, 22, 2, 22, Some(0), clock(22, 19, 0));
        close_update(&config, 22, 3, 2, 0, clock(22, 19, 0));
        assert!(advance(&config, &telegram, clock(22, 20, 0)).await.is_err());
        let saved = load(&config).unwrap();
        let player = saved.last_player.clone().unwrap();
        let message = saved.events[&clock(22, 20, 0).date_naive()].outcome.clone();
        advance(&config, &telegram, clock(22, 20, 10))
            .await
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        advance(&config, &telegram, clock(22, 20, 31))
            .await
            .unwrap();
        assert_eq!(
            load(&config).unwrap().events[&clock(22, 20, 0).date_naive()].outcome,
            message
        );
        register_poll(&config, 29);
        answer(&config, 29, 10, 11, Some(0), clock(29, 19, 0));
        answer(&config, 29, 11, 22, Some(0), clock(29, 19, 0));
        close_update(&config, 29, 12, 2, 0, clock(29, 19, 0));
        advance(&config, &telegram, clock(29, 20, 0)).await.unwrap();
        let state = load(&config).unwrap();
        assert_ne!(state.last_player.as_ref(), Some(&player));
        assert!(state.counts.values().all(|v| *v == 1));
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests[0].1["text"], requests[1].1["text"]);
        assert_eq!(requests.len(), 3);
        // DST ended between these two Thursdays; cutoff is always local 20:00.
        assert_eq!(clock(22, 20, 0).with_timezone(&chrono::Utc).hour(), 18);
        assert_eq!(clock(29, 20, 0).with_timezone(&chrono::Utc).hour(), 19);
    }
    #[tokio::test]
    async fn no_yes_votes_never_select_a_player_and_old_assignments_are_not_republished() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        storage::write(&temp.path().join("challenges.json"),&json!({"counts":{"1":2},"last_player":"1","assignment":{"week":"2026-43","message":"old pre-participation challenge"}})).unwrap();
        assert!(
            !status(&config, clock(22, 20, 0), -123)
                .unwrap()
                .contains("old pre-participation")
        );
        register_poll(&config, 22);
        answer(&config, 22, 1, 11, Some(1), clock(22, 19, 0));
        close_update(&config, 22, 2, 0, 1, clock(22, 19, 0));
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        advance(&config, &telegram, clock(22, 20, 0)).await.unwrap();
        let state = load(&config).unwrap();
        assert_eq!(state.counts["11"], 2);
        assert!(
            state.events[&clock(22, 20, 0).date_naive()]
                .outcome
                .as_ref()
                .unwrap()
                .contains("keine namentlich bekannten")
        );
        let before = serde_json::to_value(&state).unwrap();
        update(&config,-123,&json!({"update_id":999,"poll_answer":{"poll_id":"foreign","user":{"id":11},"option_ids":[0]}}),clock(22,20,20)).unwrap();
        assert_eq!(
            before,
            serde_json::to_value(load(&config).unwrap()).unwrap()
        );
    }
    #[tokio::test]
    async fn empty_dota_config_does_not_exclude_a_yes_voter() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = config(temp.path());
        config.0["accounts"]["account_list"] = json!([]);
        config.0["community"]["player_members"] = json!({});
        register_poll(&config, 22);
        answer(&config, 22, 1, 33, Some(0), clock(22, 19, 0));
        close_update(&config, 22, 2, 1, 0, clock(22, 19, 0));
        let server = Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        advance(&config, &telegram, clock(22, 20, 0)).await.unwrap();
        let requests = server.requests.lock().unwrap();
        let text = requests[0].1["text"].as_str().unwrap();
        assert!(text.contains("tg://user?id=33"));
        assert!(text.contains("&lt;Three&gt;"));
    }
    #[tokio::test]
    async fn failed_close_retries_with_backoff_and_never_draws_early() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(temp.path());
        register_poll(&config, 22);
        answer(&config, 22, 1, 11, Some(0), clock(22, 19, 0));
        let server = Server::new(vec![
            (503, json!({"ok":false,"error_code":503})),
            (200, json!({"ok":true,"result":closed(22,1,0)})),
            (200, json!({"ok":true,"result":{}})),
        ]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        assert!(advance(&config, &telegram, clock(22, 20, 0)).await.is_err());
        advance(&config, &telegram, clock(22, 20, 30))
            .await
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        let later = clock(22, 20, 0) + chrono::Duration::seconds(61);
        advance(&config, &telegram, later).await.unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 2);
        close_update(&config, 22, 2, 1, 0, later);
        advance(&config, &telegram, later + chrono::Duration::seconds(6))
            .await
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 3);
    }
}
