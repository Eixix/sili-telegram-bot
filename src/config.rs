use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize)]
pub struct Account {
    pub identifier: String,
    pub name: String,
    pub birthday: Option<String>,
}

#[derive(Clone)]
pub struct Config(pub Value);
impl Config {
    pub fn load() -> Result<Self> {
        let mut defaults: Value = serde_json::from_str(include_str!("../config/default.json"))?;
        let path = std::env::var("CONFIG_FILE").unwrap_or_else(|_| "config.json".into());
        match fs::read_to_string(&path) {
            Ok(data) => merge(
                &mut defaults,
                serde_json::from_str(&data).context("Invalid config JSON")?,
            ),
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    && std::env::var_os("CONFIG_FILE").is_none() => {}
            Err(e) => return Err(e).context("Cannot read config file"),
        }
        for (upper, lower, key) in [
            ("BOT_TOKEN", "bot_token", "bot_token"),
            ("CHAT_ID", "chat_id", "chat_id"),
        ] {
            if let Some(value) = std::env::var(upper)
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| std::env::var(lower).ok().filter(|s| !s.is_empty()))
            {
                defaults["secrets"][key] = value.into();
            }
        }
        if let Ok(path) = std::env::var("BOT_TOKEN_FILE") {
            defaults["secrets"]["bot_token"] = fs::read_to_string(path)
                .context("Cannot read BOT_TOKEN_FILE")?
                .trim()
                .into();
        }
        let config = Self(defaults);
        config.chat_id()?;
        ensure!(
            !config.string("secrets", "bot_token").is_empty(),
            "Set BOT_TOKEN, BOT_TOKEN_FILE or secrets.bot_token"
        );
        let mut ids = std::collections::HashSet::new();
        for account in config.accounts()? {
            ensure!(
                account.identifier.parse::<u32>().is_ok(),
                "Account identifier must be a Steam32 ID"
            );
            ensure!(
                ids.insert(account.identifier),
                "Duplicate account identifier"
            );
            if let Some(birthday) = account.birthday {
                chrono::NaiveDate::parse_from_str(&birthday, "%d.%m.%Y")
                    .context("Invalid birthday; use d.m.YYYY")?;
            }
        }
        config.timezone()?;
        Ok(config)
    }
    pub fn string(&self, section: &str, key: &str) -> String {
        match &self.0[section][key] {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        }
    }
    pub fn path(&self, section: &str, key: &str) -> PathBuf {
        self.string(section, key).into()
    }
    pub fn accounts(&self) -> Result<Vec<Account>> {
        Ok(serde_json::from_value(
            self.0["accounts"]["account_list"].clone(),
        )?)
    }
    pub fn chat_id(&self) -> Result<i64> {
        self.string("secrets", "chat_id")
            .parse()
            .context("Set a numeric CHAT_ID or secrets.chat_id")
    }
    pub fn timezone(&self) -> Result<chrono_tz::Tz> {
        std::env::var("TZ")
            .unwrap_or_else(|_| "Europe/Berlin".into())
            .parse()
            .context("Invalid TZ")
    }
    pub fn state_dir(&self) -> PathBuf {
        self.path("dynamic_resources", "hero_data_path")
            .parent()
            .unwrap_or(Path::new("resources/dynamic"))
            .to_owned()
    }
}
fn merge(base: &mut Value, incoming: Value) {
    match (base, incoming) {
        (Value::Object(base), Value::Object(incoming)) => {
            for (key, value) in incoming {
                merge(base.entry(key).or_insert(Value::Null), value);
            }
        }
        (base, incoming) => *base = incoming,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn overlays_nested_values_and_replaces_arrays() {
        let mut base = json!({"secrets":{"bot_token":null,"chat_id":null},"accounts":[1,2]});
        merge(&mut base, json!({"secrets":{"chat_id":-123},"accounts":[]}));
        assert_eq!(
            base,
            json!({"secrets":{"bot_token":null,"chat_id":-123},"accounts":[]})
        );
    }
}
