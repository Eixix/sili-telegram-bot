use crate::{config::Config, storage};
use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Deserialize, Serialize)]
pub struct Entity {
    pub name: String,
    pub title: String,
    pub url: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Response {
    pub text: String,
    pub urls: Vec<Option<String>>,
}
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Catalog {
    pub entities: BTreeMap<String, BTreeMap<String, Entity>>,
    pub responses: BTreeMap<String, Vec<Response>>,
}
#[derive(Debug)]
pub struct Args {
    pub entity: String,
    pub line: String,
    pub category: String,
    pub level: usize,
}
impl Args {
    pub fn parse(text: &str) -> Result<Self> {
        let help = "Use /voiceline Entity Name [hero|voice_pack|announcer|legacy|other]: Voice line [level]. Put the line in double quotes for regex.";
        let pattern = Regex::new(
            r"^\s*([^\[\]:]+?)\s*(?:\[([^\[\]]+)\])?\s*:\s*(.+?)\s*(?:\[([0-9]+)\])?\s*$",
        )?;
        let captures = pattern.captures(text).context(help)?;
        let entity = captures[1].trim().to_owned();
        let line = captures[3].trim().to_owned();
        ensure!(!entity.is_empty() && !line.is_empty(), "{help}");
        let level = captures
            .get(4)
            .map(|s| s.as_str().parse::<usize>())
            .transpose()?
            .unwrap_or(1);
        ensure!(level > 0, "Response levels start at 1");
        Ok(Self {
            entity,
            line,
            category: captures.get(2).map_or("hero", |s| s.as_str()).to_owned(),
            level: level - 1,
        })
    }
}
impl Catalog {
    pub fn link(&self, args: &Args) -> Result<String> {
        let category = match args.category.as_str() {
            "hero" => "Hero responses",
            "voice_pack" => "Voice Packs",
            "announcer" => "Announcer Packs",
            "legacy" => "Archived",
            "other" => "Other responses",
            _ => bail!("Unknown entity type: {}", args.category),
        };
        let entities = self
            .entities
            .get(category)
            .context("Entity type has no responses yet")?;
        let name = args.entity.to_lowercase();
        let entity = entities
            .values()
            .max_by(|a, b| {
                strsim::normalized_levenshtein(&a.name.to_lowercase(), &name).total_cmp(
                    &strsim::normalized_levenshtein(&b.name.to_lowercase(), &name),
                )
            })
            .filter(|e| strsim::normalized_levenshtein(&e.name.to_lowercase(), &name) >= 0.2)
            .context("Could not find that entity")?;
        let pattern =
            if args.line.starts_with('"') && args.line.ends_with('"') && args.line.len() > 1 {
                Some(Regex::new(&format!(
                    "(?i){}",
                    &args.line[1..args.line.len() - 1]
                ))?)
            } else {
                None
            };
        let responses = self
            .responses
            .get(&entity.title)
            .context("Responses are unavailable for this entity")?;
        let response = responses
            .iter()
            .find(|r| match &pattern {
                Some(re) => re.is_match(&r.text),
                None => fuzzy_contains(&r.text, &args.line),
            })
            .with_context(|| {
                format!(
                    "Could not find line for '{}'. Check {}",
                    entity.name, entity.url
                )
            })?;
        response
            .urls
            .get(args.level)
            .and_then(Option::as_ref)
            .cloned()
            .with_context(|| {
                let levels = response
                    .urls
                    .iter()
                    .enumerate()
                    .filter(|(_, u)| u.is_some())
                    .map(|(i, _)| (i + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Response level is unavailable. Available levels: {levels}")
            })
    }
    pub fn inline(&self, query: &str) -> Vec<Value> {
        let mut results = Vec::new();
        for entities in self.entities.values() {
            for entity in entities.values() {
                if let Some(responses) = self.responses.get(&entity.title) {
                    for response in responses {
                        let title = format!("{}: {}", entity.name, response.text);
                        if !query.is_empty() && !fuzzy_contains(&title, query) {
                            continue;
                        }
                        for (level, url) in response.urls.iter().enumerate() {
                            if let Some(url) = url {
                                // Sequential IDs are unique within this answer and within Telegram's 64-byte limit.
                                results.push(json!({"type":"voice","id":results.len().to_string(),"voice_url":url,"title":format!("{title} [{}]", level+1)}));
                                if results.len() == 50 {
                                    return results;
                                }
                            }
                        }
                    }
                }
            }
        }
        results
    }
}
// Levenshtein substring distance <= 1, with Unicode characters and literal input.
pub fn fuzzy_contains(text: &str, query: &str) -> bool {
    let query: Vec<_> = query.to_lowercase().chars().collect();
    if query.is_empty() {
        return true;
    }
    let mut previous: Vec<usize> = (0..=query.len()).collect();
    for c in text.to_lowercase().chars() {
        let mut next = vec![0; query.len() + 1];
        for (j, q) in query.iter().enumerate() {
            next[j + 1] = (previous[j] + usize::from(c != *q))
                .min(previous[j + 1] + 1)
                .min(next[j] + 1);
        }
        if next[query.len()] <= 1 {
            return true;
        }
        previous = next;
    }
    false
}
#[derive(Clone)]
pub struct Voices {
    config: Config,
    client: Client,
    pub catalog: Arc<RwLock<Option<Catalog>>>,
}
impl Voices {
    pub fn new(config: Config, client: Client) -> Result<Self> {
        let catalog =
            if let Some(catalog) = storage::read(&config.state_dir().join("voicelines.json"))? {
                Some(catalog)
            } else {
                let entities = storage::read(&config.path("voicelines", "entity_data_file"))?;
                let responses = storage::read(&config.path("voicelines", "resource_file"))?;
                entities
                    .zip(responses)
                    .map(|(entities, responses)| Catalog {
                        entities,
                        responses,
                    })
            };
        Ok(Self {
            config,
            client,
            catalog: Arc::new(RwLock::new(catalog)),
        })
    }
    pub fn refresh_enabled(&self) -> bool {
        !self
            .config
            .string("voicelines", "user_agent_email")
            .is_empty()
    }
    pub async fn link(&self, text: &str) -> Result<String> {
        ensure!(text.chars().count() <= 512, "Query is too long");
        let args = Args::parse(text)?;
        let catalog = self.catalog.read().await;
        catalog.as_ref().context("Voiceline resources are not ready. Configure voicelines.user_agent_email or migrate the existing cache.")?.link(&args)
    }
    async fn api(&self, params: &[(&str, &str)]) -> Result<Value> {
        let email = self.config.string("voicelines", "user_agent_email");
        let agent = format!(
            "{} ({}; {})",
            self.config.string("voicelines", "user_agent_title"),
            self.config.string("voicelines", "user_agent_url"),
            email
        );
        let delay = self.config.0["voicelines"]["secs_between_requests"]
            .as_u64()
            .unwrap_or(4)
            .max(1);
        tokio::time::sleep(Duration::from_secs(delay)).await;
        let response: Value = self
            .client
            .get(format!(
                "{}/api.php",
                self.config
                    .string("voicelines", "base_url")
                    .trim_end_matches('/')
            ))
            .header(reqwest::header::USER_AGENT, agent)
            .query(&[("format", "json")])
            .query(params)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        ensure!(
            response.get("error").is_none(),
            "Wiki API returned an error"
        );
        Ok(response)
    }
    async fn page(&self, title: &str) -> Result<String> {
        self.api(&[("action", "parse"), ("page", title), ("prop", "text")])
            .await?["parse"]["text"]["*"]
            .as_str()
            .map(str::to_owned)
            .context("Wiki parse response has no HTML")
    }
    pub async fn refresh(&self) -> Result<()> {
        ensure!(
            !self
                .config
                .string("voicelines", "user_agent_email")
                .is_empty(),
            "Set voicelines.user_agent_email for wiki refresh"
        );
        let page = self.page("Template:VoiceNav").await?;
        let entities = parse_entities(&page, &self.config.string("voicelines", "base_url"))?;
        let mut titles: BTreeSet<_> = entities
            .values()
            .flat_map(|e| e.values().map(|e| e.title.clone()))
            .collect();
        let mut continuation = String::new();
        loop {
            let mut params = vec![
                ("action", "query"),
                ("list", "categorymembers"),
                ("cmtitle", "Category:Responses"),
                ("cmlimit", "500"),
            ];
            if !continuation.is_empty() {
                params.push(("cmcontinue", &continuation));
            }
            let data = self.api(&params).await?;
            for entry in data["query"]["categorymembers"]
                .as_array()
                .context("Missing response category")?
            {
                if let Some(title) = entry["title"].as_str().filter(|_| entry["ns"] == 0) {
                    titles.insert(title.into());
                }
            }
            match data["continue"]["cmcontinue"].as_str() {
                Some(c) => continuation = c.into(),
                None => break,
            }
        }
        let mut responses = BTreeMap::new();
        for title in titles {
            let page = self.page(&title).await?;
            let parsed = parse_responses(&page);
            if !parsed.is_empty() {
                responses.insert(title, parsed);
            }
        }
        ensure!(
            !responses.is_empty(),
            "No voicelines extracted; keeping previous cache"
        );
        let catalog = Catalog {
            entities,
            responses,
        };
        storage::write(&self.config.state_dir().join("voicelines.json"), &catalog)?;
        *self.catalog.write().await = Some(catalog);
        Ok(())
    }
}
fn selector(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
pub fn parse_entities(
    html: &str,
    base: &str,
) -> Result<BTreeMap<String, BTreeMap<String, Entity>>> {
    let document = Html::parse_document(html);
    let mut categories = BTreeMap::new();
    for row in document.select(&selector(".nowraplinks tr")) {
        let Some(heading) = row.select(&selector("th")).next() else {
            continue;
        };
        let heading = heading.text().collect::<String>().trim().to_owned();
        let category = match heading.as_str() {
            "Heroes" => "Hero responses",
            "Announcer" | "Announcers" => "Announcer Packs",
            "Misc" => "Other responses",
            other => other,
        }
        .to_owned();
        let mut entities = BTreeMap::new();
        for link in row.select(&selector(".hlist a[title][href]")) {
            let name = link
                .text()
                .collect::<String>()
                .replace('\u{a0}', " ")
                .trim()
                .to_owned();
            if name.is_empty() {
                continue;
            }
            let title = link.value().attr("title").unwrap().to_owned();
            let href = link.value().attr("href").unwrap();
            let url = if href.starts_with("http") {
                href.to_owned()
            } else if let Some(relative) = href.strip_prefix("/dota2/") {
                format!("{}/{relative}", base.trim_end_matches('/'))
            } else {
                format!(
                    "{}/{}",
                    base.trim_end_matches('/'),
                    href.trim_start_matches('/')
                )
            };
            entities.insert(name.clone(), Entity { name, title, url });
        }
        if !entities.is_empty() {
            categories.insert(category, entities);
        }
    }
    ensure!(
        categories.contains_key("Hero responses"),
        "Could not parse wiki navigation; keeping previous cache"
    );
    Ok(categories)
}
pub fn parse_responses(html: &str) -> Vec<Response> {
    let document = Html::parse_document(html);
    let mut responses = Vec::new();
    for li in document.select(&selector(
        ".mw-parser-output > ul li, .tabs-content li, .wikitable li",
    )) {
        let mut urls = Vec::new();
        for button in li.select(&selector("a.ext-audiobutton")) {
            let url = if button.value().attr("data-state") == Some("play") {
                button
                    .parent()
                    .and_then(scraper::ElementRef::wrap)
                    .and_then(|parent| parent.select(&selector("source[src]")).next())
                    .and_then(|s| s.value().attr("src"))
                    .map(str::to_owned)
            } else {
                None
            };
            urls.push(url);
        }
        if !urls.is_empty() {
            let text = li.text().collect::<String>();
            let text = process_text(&text);
            responses.push(Response { text, urls });
        }
    }
    responses
}
fn process_text(text: &str) -> String {
    let prefix = Regex::new(r"^(\s*)?((Link(▶.)\s)+)?\s*(u\s)?(r\s)?(rem\s)?(\d+\s)?\s*").unwrap();
    let suffix = Regex::new(r"(\s+followup)?(\s+\(broken file\))?$").unwrap();
    let text = prefix.replace(text, "");
    suffix.replace(&text, "").nfkd().collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "downloads a real wiki page to verify required gzip and current markup"]
    async fn live_wiki_gzip_and_sniper_responses() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut value: Value = serde_json::from_str(include_str!("../config/default.json"))?;
        value["dynamic_resources"]["hero_data_path"] = json!(temp.path().join("heroes.json"));
        value["voicelines"]["entity_data_file"] = json!(temp.path().join("entities.json"));
        value["voicelines"]["resource_file"] = json!(temp.path().join("responses.json"));
        let client = Client::builder()
            .user_agent(crate::HTTP_USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()?;
        let voices = Voices::new(Config(value), client)?;
        let html = voices.page("Sniper/Responses").await?;
        let responses = parse_responses(&html);
        ensure!(
            !responses.is_empty(),
            "No responses parsed from live Sniper page"
        );
        ensure!(
            responses
                .iter()
                .any(|r| r.text.trim() == "Sniper!" && r.urls.iter().any(Option::is_some)),
            "Sniper's requested line is missing"
        );
        println!(
            "Live gzip wiki fetch and Sniper parsing: {} response rows",
            responses.len()
        );
        Ok(())
    }
    #[test]
    fn original_python_scraper_regressions() {
        let cases: Value =
            serde_json::from_str(include_str!("../tests/test_data/scraper_cases.json")).unwrap();
        for case in cases["text"].as_array().unwrap() {
            assert_eq!(
                process_text(case[0].as_str().unwrap()),
                case[1].as_str().unwrap()
            );
        }
        for case in cases["html"].as_array().unwrap() {
            let html = format!(
                "<div class=\"mw-parser-output\"><ul>{}</ul></div>",
                case[0].as_str().unwrap()
            );
            assert_eq!(
                !parse_responses(&html).is_empty(),
                case[1].as_bool().unwrap()
            );
        }
    }
    #[test]
    fn parses_arguments_and_rejects_level_zero() {
        let a = Args::parse("Legion Commander [hero]: Ten hut! [2]").unwrap();
        assert_eq!(
            (a.entity.as_str(), a.line.as_str(), a.level),
            ("Legion Commander", "Ten hut!", 1)
        );
        assert!(Args::parse("A: line [0]").is_err());
        assert!(Args::parse("no colon").is_err());
    }
    #[test]
    fn legacy_fixtures_search_and_missing_levels() {
        let catalog = Catalog {
            entities: serde_json::from_str(include_str!("../tests/test_data/entity_data.json"))
                .unwrap(),
            responses: serde_json::from_str(include_str!("../tests/test_data/responses.json"))
                .unwrap(),
        };
        assert!(
            catalog
                .link(&Args::parse("Legion Comander: Ten hut! [2]").unwrap())
                .unwrap()
                .ends_with("Vo_legion_commander_legcom_dem_spawn_02.mp3")
        );
        assert!(
            catalog
                .link(&Args::parse("Legion Commander: Ten hut! [99]").unwrap())
                .is_err()
        );
        assert!(
            catalog
                .link(&Args::parse("Legion Commander: \"^Ten hut!$\"").unwrap())
                .is_ok()
        );
        assert_eq!(catalog.inline("").len(), 50);
        assert!(!catalog.inline("Legion Commander: Ten hut!").is_empty());
    }
    #[test]
    fn literal_fuzzy_unicode() {
        assert!(fuzzy_contains("Döta, let's go!", "döta"));
        assert!(fuzzy_contains("Ten hut!", "Ten hot!"));
        assert!(!fuzzy_contains("Ten hut!", "[.*]"));
    }
    #[test]
    fn scraper_keeps_missing_audio_levels() {
        let parsed = parse_responses(
            r#"<div class="mw-parser-output"><ul><li><span><a class="ext-audiobutton" data-state="play"></a><source src="https://example.com/a.mp3"></span><span><a class="ext-audiobutton" data-state="missing"></a></span>Ten hut!</li></ul></div>"#,
        );
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].text, "Ten hut!");
        assert_eq!(
            parsed[0].urls,
            vec![Some("https://example.com/a.mp3".into()), None]
        );
        let entities = parse_entities(r#"<div class="nowraplinks"><table><tr><th>Hero responses</th><td class="hlist"><a title="Axe/Responses" href="/dota2/Axe/Responses">Axe</a></td></tr></table></div>"#, "https://liquipedia.net/dota2").unwrap();
        assert_eq!(
            entities["Hero responses"]["Axe"].url,
            "https://liquipedia.net/dota2/Axe/Responses"
        );
    }
}
