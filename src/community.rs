use crate::{config::Config, storage};
use anyhow::{Result, bail};
use chrono::{Datelike, NaiveDate};
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const CHALLENGES: &[&str] = &[
    "Spiele einen Helden, den du normalerweise nicht pickst.",
    "Sage vor dem Spiel deinen Lane-Plan an und besprecht ihn danach im Team.",
    "Organisiere einen Smoke-Gank mit mindestens drei Teammitgliedern.",
    "Stelle deinem Team einen ungewöhnlichen, aber begründeten Item-Build vor.",
    "Übernimm heute die Ansagen für Roshan und die nächsten Objectives.",
];
#[derive(Default, Deserialize, Serialize)]
struct Challenges {
    counts: BTreeMap<String, u64>,
    last_player: Option<String>,
    assignment: Option<Assignment>,
}
#[derive(Deserialize, Serialize)]
struct Assignment {
    week: String,
    message: String,
}

/// One stable assignment per ISO week, shared by the command and Thursday job.
pub fn challenge(config: &Config, today: NaiveDate) -> Result<Option<String>> {
    if config.0["community"]["challenges_enabled"].as_bool() != Some(true) {
        return Ok(None);
    }
    let accounts = config.accounts()?;
    if accounts.is_empty() {
        return Ok(None);
    }
    let path = config.state_dir().join("challenges.json");
    let mut state: Challenges = storage::read(&path)?.unwrap_or_default();
    let week = format!("{}-{:02}", today.iso_week().year(), today.iso_week().week());
    if let Some(assignment) = &state.assignment
        && assignment.week == week
    {
        return Ok(Some(assignment.message.clone()));
    }
    let min = accounts
        .iter()
        .map(|a| *state.counts.get(&a.identifier).unwrap_or(&0))
        .min()
        .unwrap_or(0);
    let candidates: Vec<_> = accounts
        .iter()
        .filter(|a| *state.counts.get(&a.identifier).unwrap_or(&0) == min)
        .collect();
    let without_last: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|a| Some(&a.identifier) != state.last_player.as_ref())
        .collect();
    let candidates = if without_last.is_empty() {
        &candidates
    } else {
        &without_last
    };
    let account = candidates
        .choose(&mut rand::rng())
        .expect("eligible player");
    let custom: Vec<_> = config.0["community"]["challenges"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .collect();
    let pool = if custom.is_empty() {
        CHALLENGES
    } else {
        &custom
    };
    let text = pool.choose(&mut rand::rng()).expect("challenge pool");
    let message = format!(
        "<b>Donnerstagschallenge · KW {}</b>\n{}: {}\nBesprecht die Challenge nach dem Spiel gemeinsam!",
        today.iso_week().week(),
        storage::html(&account.name),
        storage::html(text)
    );
    *state.counts.entry(account.identifier.clone()).or_default() += 1;
    state.last_player = Some(account.identifier.clone());
    state.assignment = Some(Assignment {
        week,
        message: message.clone(),
    });
    // Persist selection first, so retries/restarts never reroll the assignment.
    storage::write(&path, &state)?;
    Ok(Some(message))
}

#[derive(Clone, Copy, Serialize)]
struct Hero {
    id: u32,
    name: &'static str,
}
const fn hero(id: u32, name: &'static str) -> Hero {
    Hero { id, name }
}
struct Strategy {
    key: &'static str,
    title: &'static str,
    plan: &'static str,
    roles: [&'static [Hero]; 5],
}
const STRATEGIES: &[Strategy] = &[
    Strategy {
        key: "global",
        title: "Überall gleichzeitig",
        plan: "Globale Präsenz: gemeinsam Ziele auswählen und eure Fernwirkung kombinieren.",
        roles: [
            &[hero(67, "Spectre")],
            &[hero(22, "Zeus")],
            &[hero(53, "Nature's Prophet"), hero(135, "Dawnbreaker")],
            &[hero(71, "Spirit Breaker")],
            &[hero(68, "Ancient Apparition"), hero(75, "Silencer")],
        ],
    },
    Strategy {
        key: "invisible",
        title: "Wo sind denn alle?",
        plan: "Jeder Held kann unsichtbar werden. Zusammen angreifen; gegnerische Detection zuerst ausschalten.",
        roles: [
            &[hero(32, "Riki"), hero(56, "Clinkz")],
            &[hero(46, "Templar Assassin"), hero(74, "Invoker")],
            &[hero(16, "Sand King")],
            &[hero(62, "Bounty Hunter")],
            &[hero(83, "Treant Protector")],
        ],
    },
    Strategy {
        key: "push",
        title: "Immobilienabriss",
        plan: "Fünf Helden mit Push-Werkzeugen: nach gewonnenen Kämpfen sofort Türme nehmen.",
        roles: [
            &[hero(48, "Luna"), hero(109, "Terrorblade")],
            &[hero(49, "Dragon Knight"), hero(43, "Death Prophet")],
            &[hero(38, "Beastmaster"), hero(77, "Lycan")],
            &[hero(27, "Shadow Shaman")],
            &[hero(66, "Chen"), hero(58, "Enchantress")],
        ],
    },
];
pub fn strategies() -> String {
    format!(
        "<b>Meme-Strategien</b>\n{}\n/memeteam [Strategie] erzeugt einen neuen Draft für Position 1–5.",
        STRATEGIES
            .iter()
            .map(|s| format!("{}: {}", s.key, s.title))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
fn draft(strategy: &Strategy, rng: &mut impl rand::Rng) -> Result<Vec<Hero>> {
    // Backtrack rather than allowing duplicate heroes when pools overlap.
    fn fill(roles: &[&[Hero]], picked: &mut Vec<Hero>, rng: &mut impl rand::Rng) -> bool {
        use rand::seq::SliceRandom;
        let Some((pool, rest)) = roles.split_first() else {
            return true;
        };
        let mut options = pool.to_vec();
        options.shuffle(rng);
        for hero in options {
            if picked.iter().any(|p| p.id == hero.id) {
                continue;
            }
            picked.push(hero);
            if fill(rest, picked, rng) {
                return true;
            }
            picked.pop();
        }
        false
    }
    let mut heroes = Vec::new();
    if !fill(&strategy.roles, &mut heroes, rng) {
        bail!("Strategy has no valid five-hero draft");
    }
    Ok(heroes)
}
pub fn memeteam(config: &Config, name: &str) -> Result<String> {
    let mut rng = rand::rng();
    let name = name.trim().to_lowercase();
    let strategy = if name.is_empty() {
        STRATEGIES.choose(&mut rng).expect("strategies")
    } else {
        let Some(strategy) = STRATEGIES.iter().find(|s| s.key == name) else {
            return Ok(strategies());
        };
        strategy
    };
    let heroes = draft(strategy, &mut rng)?;
    let message = format!(
        "<b>Meme-Team: {}</b>\n{}\n\n{}\n\nNeuer Draft: /memeteam {}",
        strategy.title,
        strategy.plan,
        heroes
            .iter()
            .enumerate()
            .map(|(i, h)| format!("Position {}: {}", i + 1, storage::html(h.name)))
            .collect::<Vec<_>>()
            .join("\n"),
        strategy.key
    );
    storage::write(
        &config.state_dir().join("memeteam.json"),
        &serde_json::json!({"strategy":strategy.key,"heroes":heroes,"message":message}),
    )?;
    Ok(message)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn weekly_assignment_survives_restart_and_rotates_fairly() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config(
            json!({"dynamic_resources":{"hero_data_path":temp.path().join("heroes.json")},"community":{"challenges_enabled":true,"challenges":["A & B"]},"accounts":{"account_list":[{"identifier":"1","name":"One"},{"identifier":"2","name":"Two"},{"identifier":"3","name":"Three"}]}}),
        );
        let day = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let first = challenge(&config, day).unwrap().unwrap();
        assert!(first.contains("A &amp; B"));
        assert_eq!(
            Some(first.clone()),
            challenge(&config, day + chrono::Duration::days(1)).unwrap()
        );
        let second = challenge(&config, day + chrono::Duration::weeks(1))
            .unwrap()
            .unwrap();
        assert_ne!(first, second);
        challenge(&config, day + chrono::Duration::weeks(2)).unwrap();
        let state: Challenges = storage::read(&temp.path().join("challenges.json"))
            .unwrap()
            .unwrap();
        assert_eq!(state.counts.len(), 3);
        assert!(state.counts.values().all(|n| *n == 1));
        let empty =
            Config(json!({"community":{"challenges_enabled":true},"accounts":{"account_list":[]}}));
        assert!(challenge(&empty, day).unwrap().is_none());
    }
    #[test]
    fn all_strategies_produce_distinct_heroes_with_role_constraints() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        for strategy in STRATEGIES {
            for _ in 0..100 {
                let heroes = draft(strategy, &mut rng).unwrap();
                assert_eq!(heroes.len(), 5);
                assert_eq!(
                    heroes
                        .iter()
                        .map(|h| h.id)
                        .collect::<std::collections::HashSet<_>>()
                        .len(),
                    5
                );
                for (i, hero) in heroes.iter().enumerate() {
                    assert!(strategy.roles[i].iter().any(|h| h.id == hero.id));
                }
            }
        }
        const SINGLE: &[Hero] = &[hero(1, "A")];
        let impossible = Strategy {
            key: "bad",
            title: "bad",
            plan: "bad",
            roles: [SINGLE; 5],
        };
        assert!(draft(&impossible, &mut rng).is_err());
    }
}
