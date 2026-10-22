//! Optional private Steam32 -> Telegram member mapping; voting needs no mapping.
use crate::config::Config;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub fn configured(config: &Config) -> Result<BTreeMap<String, i64>> {
    let value = &config.0["community"]["player_members"];
    let links: BTreeMap<String, i64> = if value.is_null() {
        BTreeMap::new()
    } else {
        serde_json::from_value(value.clone()).context("Invalid community.player_members")?
    };
    let accounts = config.accounts()?;
    let mut users = BTreeSet::new();
    for (account, user) in &links {
        ensure!(
            accounts.iter().any(|a| a.identifier == *account),
            "Mapping references an unknown Dota account"
        );
        ensure!(*user > 0, "Telegram member IDs must be positive");
        ensure!(
            users.insert(*user),
            "A Telegram member can map to only one Dota account"
        );
    }
    Ok(links)
}
pub fn label(config: &Config, user: i64, fallback: &str) -> Result<String> {
    let links = configured(config)?;
    let accounts = config.accounts()?;
    Ok(accounts
        .into_iter()
        .find(|a| links.get(&a.identifier) == Some(&user))
        .map(|a| a.name)
        .unwrap_or_else(|| {
            if fallback.is_empty() {
                "Teilnehmer".into()
            } else {
                fallback.into()
            }
        }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn config(links: serde_json::Value) -> Config {
        Config(
            json!({"accounts":{"account_list":[{"identifier":"1","name":"One"},{"identifier":"2","name":"Two"}]},"community":{"player_members":links}}),
        )
    }
    #[test]
    fn validates_config_only_mapping_and_labels_unmapped_voters() {
        let c = config(json!({"1":11}));
        assert_eq!(label(&c, 11, "Telegram name").unwrap(), "One");
        assert_eq!(label(&c, 99, "Other member").unwrap(), "Other member");
        assert!(configured(&config(json!({"1":11,"2":11}))).is_err());
        assert!(configured(&config(json!({"3":33}))).is_err());
        assert!(configured(&config(json!({"1":-1}))).is_err());
        assert!(configured(&config(json!({"1":"invalid"}))).is_err());
        assert!(configured(&config(json!({}))).unwrap().is_empty());
    }
}
