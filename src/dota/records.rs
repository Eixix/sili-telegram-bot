use super::Match;
use crate::{config::Account, storage};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Default, Deserialize, Serialize)]
pub(super) struct Records {
    kills: u32,
    assists: u32,
    kda: f64,
}
impl Records {
    fn improvements(&mut self, game: &Match) -> Vec<String> {
        let mut lines = Vec::new();
        if game.kills > self.kills {
            self.kills = game.kills;
            lines.push(format!("{} Kills", game.kills));
        }
        if game.assists > self.assists {
            self.assists = game.assists;
            lines.push(format!("{} Assists", game.assists));
        }
        if game.ratio() > self.kda {
            self.kda = game.ratio();
            lines.push(format!("{:.2} KDA", game.ratio()));
        }
        lines
    }
}
pub(super) fn update(
    account: &Account,
    games: &[Match],
    fresh: &HashSet<u64>,
    previous: Option<Records>,
) -> (Records, Vec<String>) {
    let announce = previous.is_some();
    let mut records = previous.unwrap_or_default();
    let mut ordered: Vec<_> = games.iter().filter(|g| g.radiant_win.is_some()).collect();
    ordered.sort_by_key(|g| (g.start_time, g.match_id));
    let mut messages = Vec::new();
    for game in ordered {
        let improvements = records.improvements(game);
        if announce && fresh.contains(&game.match_id) && !improvements.is_empty() {
            messages.push(format!(
                "<b>Neuer persönlicher Rekord!</b>\n{}: {}\nhttps://www.opendota.com/matches/{}",
                storage::html(&account.name),
                improvements.join(", "),
                game.match_id
            ));
        }
    }
    (records, messages)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn game(id: u64, kills: u32, assists: u32) -> Match {
        serde_json::from_value(json!({"match_id":id,"radiant_win":true,"player_slot":0,"hero_id":1,"kills":kills,"deaths":0,"assists":assists})).unwrap()
    }
    #[test]
    fn baseline_is_silent_ties_are_ignored_and_retries_do_not_repeat() {
        let account = Account {
            identifier: "1".into(),
            name: "A & B".into(),
            birthday: None,
        };
        let (records, messages) = update(&account, &[game(1, 5, 2)], &HashSet::from([1]), None);
        assert!(messages.is_empty());
        let (records, messages) = update(
            &account,
            &[game(2, 5, 2), game(1, 5, 2)],
            &HashSet::from([2]),
            Some(records),
        );
        assert!(messages.is_empty());
        let (records, messages) = update(
            &account,
            &[game(3, 6, 3)],
            &HashSet::from([3]),
            Some(records),
        );
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("A &amp; B"));
        assert!(messages[0].contains("9.00 KDA"));
        let (records, messages) =
            update(&account, &[game(3, 6, 3)], &HashSet::new(), Some(records));
        assert!(messages.is_empty());
        let (records, messages) =
            update(&account, &[game(4, 10, 5)], &HashSet::new(), Some(records));
        assert!(messages.is_empty());
        assert!(
            update(
                &account,
                &[game(5, 9, 4)],
                &HashSet::from([5]),
                Some(records)
            )
            .1
            .is_empty()
        );
    }
}
