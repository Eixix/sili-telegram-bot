use super::Match;
use crate::{config::Account, storage};
use rand::{Rng, prelude::IndexedRandom};
use serde_json::Value;
use std::collections::HashSet;

fn lines(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .collect()
}
fn pool(value: &Value, ratio: f64) -> Vec<&str> {
    if value.is_array() {
        return lines(value);
    }
    let mut buckets: Vec<_> = value
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| {
            key.parse::<f64>()
                .ok()
                .filter(|n| !n.is_nan())
                .map(|n| (n, value))
        })
        .collect();
    buckets.sort_by(|a, b| a.0.total_cmp(&b.0));
    let selected = buckets
        .iter()
        .find(|(upper, _)| ratio < *upper)
        .map(|(_, value)| lines(value))
        .unwrap_or_default();
    if selected.is_empty() {
        lines(&value["default"])
    } else {
        selected
    }
}
pub(super) fn summary(
    players: &[(&Account, &Match)],
    heroes: &[Value],
    punlines: &Value,
    overrides: &Value,
    rng: &mut impl Rng,
) -> String {
    let Some((_, first)) = players.first() else {
        return "Keine Spieler gefunden.".into();
    };
    let outcome = lines(&punlines["match_outcome"][if first.won() { "win" } else { "lose" }])
        .choose(rng)
        .copied()
        .unwrap_or("Dota!");
    let mut parts = vec![format!(
        "<b>{}: {}</b>",
        if first.won() { "W" } else { "L" },
        storage::html(outcome)
    )];
    let mut used = HashSet::new();
    for (account, game) in players {
        let hero_id = game.hero_id.to_string();
        let hero = heroes
            .iter()
            .find(|h| h["id"].as_u64() == Some(game.hero_id as u64))
            .and_then(|h| h["localized_name"].as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Unbekannter Held Nr. {}", game.hero_id));
        let player = &overrides["player_punchlines"][&account.identifier];
        let candidates = [
            &player[&hero_id],
            &player["default"],
            &overrides["hero_punchlines"][&hero_id],
            &punlines["hero_performance_verbs"][&hero_id],
            &punlines["performance_verbs"],
        ];
        let options = candidates
            .iter()
            .map(|value| pool(value, game.ratio()))
            .find(|pool| !pool.is_empty())
            .unwrap_or_default();
        let unused: Vec<_> = options
            .iter()
            .copied()
            .filter(|s| !used.contains(*s))
            .collect();
        let verb = unused
            .choose(rng)
            .or_else(|| options.choose(rng))
            .copied()
            .unwrap_or("gespielt");
        used.insert(verb.to_owned());
        parts.push(format!(
            "{} hat mit {} {} mit {} Kills, {} Toden und {} Assists",
            storage::html(&account.name),
            storage::html(&hero),
            storage::html(verb),
            game.kills,
            game.deaths,
            game.assists
        ));
    }
    parts.push(format!("<a href=\"https://www.opendota.com/matches/{}\">OpenDota</a> · <a href=\"https://www.dotabuff.com/matches/{}\">Dotabuff</a>",first.match_id,first.match_id));
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    use serde_json::json;
    #[test]
    fn precedence_fallback_escaping_and_match_links() {
        let account = Account {
            identifier: "123".into(),
            name: "<Player>".into(),
            birthday: None,
        };
        let mut game:Match=serde_json::from_value(json!({"match_id":42,"radiant_win":true,"player_slot":0,"hero_id":32,"kills":1,"deaths":1,"assists":0})).unwrap();
        let heroes = vec![json!({"id":32,"localized_name":"Riki & Friends"})];
        let puns = json!({"performance_verbs":{"inf":["generic"]},"hero_performance_verbs":{"32":["built-in"]}});
        let mut overrides = json!({"hero_punchlines":{"32":["hero"]},"player_punchlines":{"123":{"32":["personal <roast>"],"default":["player"]}}});
        let render = |game: &Match, overrides: &Value| {
            summary(
                &[(&account, game)],
                &heroes,
                &puns,
                overrides,
                &mut StdRng::seed_from_u64(1),
            )
        };
        let text = render(&game, &overrides);
        assert!(text.contains("personal &lt;roast&gt;"));
        assert!(text.contains("&lt;Player&gt;"));
        assert!(text.contains("Riki &amp; Friends"));
        assert_eq!(text.matches("opendota.com/matches/42").count(), 1);
        assert_eq!(text.matches("dotabuff.com/matches/42").count(), 1);
        overrides["player_punchlines"]["123"]["32"] = json!([]);
        assert!(render(&game, &overrides).contains(" player mit"));
        overrides["player_punchlines"]["123"]["default"] = json!([]);
        assert!(render(&game, &overrides).contains(" hero mit"));
        overrides["hero_punchlines"]["32"] = json!([]);
        assert!(render(&game, &overrides).contains("built-in"));
        game.hero_id = 999;
        game.radiant_win = Some(false);
        let text = render(&game, &overrides);
        assert!(text.contains("generic"));
        assert!(text.starts_with("<b>L:"));
        let renamed = Account {
            name: "Renamed".into(),
            ..account.clone()
        };
        assert!(
            summary(
                &[(
                    &renamed,
                    &Match {
                        hero_id: 32,
                        ..game
                    }
                )],
                &heroes,
                &puns,
                &json!({"player_punchlines":{"123":{"32":["stable ID"]}}}),
                &mut StdRng::seed_from_u64(1)
            )
            .contains("stable ID")
        );
    }
    #[test]
    fn specialized_performance_buckets_do_not_use_low_performance_text_for_high_ratios() {
        let value = json!({"2":["low"],"inf":["high"]});
        assert_eq!(pool(&value, 1.), vec!["low"]);
        assert_eq!(pool(&value, 2.), vec!["high"]);
        assert!(pool(&json!({"2":["low"]}), 9.).is_empty());
        assert_eq!(
            pool(&json!({"inf":[],"default":["fallback"]}), 9.),
            vec!["fallback"]
        );
    }
}
