use crate::{
    config::Account,
    dota::{Dota, Match},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Deserialize)]
struct Player {
    account: Account,
    game: Match,
}
#[derive(Deserialize)]
struct Fixture {
    players: Vec<Player>,
    #[serde(default)]
    heroes: Vec<Value>,
    #[serde(default)]
    match_messages: Value,
    #[serde(default)]
    punlines: Option<Value>,
}
pub fn render(path: &Path, seed: u64) -> Result<String> {
    let fixture: Fixture =
        serde_json::from_slice(&std::fs::read(path)?).context("Invalid replay fixture")?;
    ensure!(
        !fixture.players.is_empty(),
        "Replay needs at least one player"
    );
    let id = fixture.players[0].game.match_id;
    ensure!(
        fixture
            .players
            .iter()
            .all(|p| p.game.match_id == id && p.game.radiant_win.is_some()),
        "Replay players must belong to one completed match"
    );
    let punlines = fixture
        .punlines
        .unwrap_or(serde_json::from_str(include_str!(
            "../resources/static/punlines.json"
        ))?);
    let players: Vec<_> = fixture
        .players
        .iter()
        .map(|p| (&p.account, &p.game))
        .collect();
    Ok(Dota::replay_summary(
        &players,
        &fixture.heroes,
        &punlines,
        &fixture.match_messages,
        seed,
    ))
}
pub fn command(args: &[String]) -> Result<Option<String>> {
    if args.first().map(String::as_str) != Some("--replay-match") {
        return Ok(None);
    }
    ensure!(
        args.len() == 2 || (args.len() == 4 && args[2] == "--seed"),
        "Usage: --replay-match FILE [--seed NUMBER]"
    );
    let seed = if args.len() == 4 {
        args[3].parse().context("Invalid replay seed")?
    } else {
        0
    };
    Ok(Some(render(Path::new(&args[1]), seed)?))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_fixture_and_argument_validation() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/match_replay.json");
        let a = render(&path, 42).unwrap();
        assert_eq!(a, render(&path, 42).unwrap());
        assert!(a.contains("opendota.com/matches/42"));
        assert!(command(&["--replay-match".into()]).is_err());
        assert!(command(&["--other".into()]).unwrap().is_none());
    }
}
