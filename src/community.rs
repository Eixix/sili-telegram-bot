use crate::{config::Config, storage};
use anyhow::{Result, bail};
use rand::prelude::IndexedRandom;
use serde::Serialize;

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
