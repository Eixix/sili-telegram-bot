use super::{Catalog, Entity, Response, Voices, parse_entities, parse_responses};
use crate::storage;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
struct Revision {
    id: u64,
    touched: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Navigation {
    source: String,
    revision: Revision,
    entities: BTreeMap<String, BTreeMap<String, Entity>>,
}

#[derive(Deserialize, Serialize)]
struct Page {
    source: String,
    revision: Revision,
    responses: Vec<Response>,
}

impl Voices {
    async fn revisions(&self, titles: &[String]) -> Result<BTreeMap<String, Revision>> {
        let mut revisions = BTreeMap::new();
        for chunk in titles.chunks(50) {
            let titles = chunk.join("|");
            let data = self
                .api(&[
                    ("action", "query"),
                    ("prop", "info"),
                    ("titles", &titles),
                    ("redirects", "1"),
                ])
                .await?;
            let query = &data["query"];
            for page in query["pages"]
                .as_object()
                .context("Missing wiki page metadata")?
                .values()
            {
                if let (Some(title), Some(revision)) = (
                    page["title"].as_str(),
                    page["lastrevid"].as_u64().filter(|r| *r > 0),
                ) {
                    revisions.insert(
                        title.to_owned(),
                        Revision {
                            id: revision,
                            touched: page["touched"].as_str().map(str::to_owned),
                        },
                    );
                }
            }
            // API title normalization/redirects must keep the caller's cache key.
            let aliases: BTreeMap<_, _> = ["normalized", "converted", "redirects"]
                .into_iter()
                .flat_map(|key| query[key].as_array().into_iter().flatten())
                .filter_map(|entry| {
                    Some((
                        entry["from"].as_str()?.to_owned(),
                        entry["to"].as_str()?.to_owned(),
                    ))
                })
                .collect();
            for requested in chunk {
                let mut title = requested.as_str();
                for _ in 0..10 {
                    match aliases.get(title) {
                        Some(next) => title = next,
                        None => break,
                    }
                }
                if let Some(revision) = revisions.get(title).cloned() {
                    revisions.insert(requested.clone(), revision);
                }
            }
        }
        Ok(revisions)
    }
    async fn render(&self, title: &str) -> Result<(u64, String)> {
        let data = self
            .api(&[("action", "parse"), ("page", title), ("prop", "text|revid")])
            .await?;
        let revision = data["parse"]["revid"]
            .as_u64()
            .filter(|r| *r > 0)
            .context("Missing rendered wiki revision")?;
        let html = data["parse"]["text"]["*"]
            .as_str()
            .context("Wiki parse response has no HTML")?;
        Ok((revision, html.to_owned()))
    }
    pub async fn refresh(&self) -> Result<()> {
        tracing::info!("Voiceline refresh started");
        let cache = self.config.state_dir().join("wiki-cache-v1");
        let source = self.config.string("voicelines", "base_url");
        let nav_title = "Template:VoiceNav".to_owned();
        let revision = self
            .revisions(std::slice::from_ref(&nav_title))
            .await?
            .get(&nav_title)
            .context("Voice navigation page is missing")?
            .clone();
        let navigation: Option<Navigation> = storage::read(&cache.join("navigation.json"))?;
        let navigation = match navigation.filter(|n| n.revision == revision && n.source == source) {
            Some(cached) => cached,
            None => {
                let (id, html) = self.render(&nav_title).await?;
                let entities =
                    parse_entities(&html, &self.config.string("voicelines", "base_url"))?;
                let navigation = Navigation {
                    source: source.clone(),
                    revision: Revision {
                        id,
                        touched: if id == revision.id {
                            revision.touched
                        } else {
                            None
                        },
                    },
                    entities,
                };
                storage::write(&cache.join("navigation.json"), &navigation)?;
                navigation
            }
        };
        let mut titles: BTreeSet<_> = navigation
            .entities
            .values()
            .flat_map(|entities| entities.values().map(|e| e.title.clone()))
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
                Some(next) => continuation = next.into(),
                None => break,
            }
        }
        let titles: Vec<_> = titles.into_iter().collect();
        let revisions = self.revisions(&titles).await?;
        let total = titles.len();
        tracing::info!(pages = total, "Voiceline pages discovered");
        let mut responses = BTreeMap::new();
        let (mut reused, mut fetched) = (0, 0);
        for (index, title) in titles.into_iter().enumerate() {
            let Some(revision) = revisions.get(&title).cloned() else {
                tracing::warn!(%title, "Wiki page is missing; skipping it");
                continue;
            };
            let path = cache.join("pages").join(format!("{}.json", revision.id));
            let cached: Option<Page> = storage::read(&path)?;
            let page = match cached.filter(|p| p.revision == revision && p.source == source) {
                Some(cached) => {
                    reused += 1;
                    cached
                }
                None => {
                    let (id, html) = self.render(&title).await?;
                    let page = Page {
                        source: source.clone(),
                        revision: Revision {
                            id,
                            touched: if id == revision.id {
                                revision.touched
                            } else {
                                None
                            },
                        },
                        responses: parse_responses(&html),
                    };
                    // Persist each successful page even if the full crawl later
                    // fails. Use the rendered revision, which can change after
                    // the metadata request, rather than stamping stale metadata.
                    storage::write(&cache.join("pages").join(format!("{id}.json")), &page)?;
                    fetched += 1;
                    page
                }
            };
            if !page.responses.is_empty() {
                responses.insert(title, page.responses);
            }
            if index == 0 || (index + 1) % 10 == 0 || index + 1 == total {
                tracing::info!(
                    completed = index + 1,
                    total,
                    reused,
                    fetched,
                    "Voiceline refresh progress"
                );
            }
        }
        ensure!(
            !responses.is_empty(),
            "No voicelines extracted; keeping previous cache"
        );
        let catalog = Catalog {
            entities: navigation.entities,
            responses,
        };
        storage::write(&self.config.state_dir().join("voicelines.json"), &catalog)?;
        *self.catalog.write().await = Some(catalog);
        tracing::info!(reused, fetched, "Voiceline revision cache saved");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, test_support::Server};
    use reqwest::Client;
    use serde_json::{Value, json};
    use std::time::Duration;

    fn info(axe: u64, sniper: u64) -> Value {
        json!({"query":{"pages":{
            "2":{"title":"Axe/Responses","lastrevid":axe},
            "3":{"title":"Sniper/Responses","lastrevid":sniper}
        }}})
    }
    fn navigation_info() -> Value {
        json!({"query":{"pages":{"1":{"title":"Template:VoiceNav","lastrevid":10}}}})
    }
    fn category() -> Value {
        json!({"query":{"categorymembers":[{"title":"Axe/Responses","ns":0},{"title":"Sniper/Responses","ns":0}]}})
    }
    fn touched_info() -> Value {
        let mut data = info(14, 15);
        data["query"]["pages"]["2"]["touched"] = json!("2026-10-21T13:00:00Z");
        data
    }
    fn rendered(revision: u64, text: &str) -> Value {
        let html = format!(
            r#"<div class="mw-parser-output"><ul><li><span><a class="ext-audiobutton" data-state="play"></a><source src="https://example.com/voice.mp3"></span>{text}</li></ul></div>"#
        );
        json!({"parse":{"revid":revision,"text":{"*":html}}})
    }
    fn voices(config: &Config) -> Voices {
        let mut voices = Voices::new(config.clone(), Client::new()).unwrap();
        voices.request_delay_override = Some(Duration::ZERO);
        voices
    }
    #[tokio::test]
    async fn metadata_batches_use_50_titles_and_follow_normalized_redirects() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut pages = serde_json::Map::new();
        for n in 0..50 {
            pages.insert(
                n.to_string(),
                json!({"title":format!("Hero {n}"),"lastrevid":n+1}),
            );
        }
        let server = Server::new(vec![
            (
                200,
                json!({"query":{"pages":pages,"normalized":[{"from":"Hero_0","to":"Alias"}],"redirects":[{"from":"Alias","to":"Hero 0"}]}}),
            ),
            (
                200,
                json!({"query":{"pages":{"51":{"title":"Hero 50","lastrevid":51}}}}),
            ),
        ]);
        let config = Config(json!({
            "dynamic_resources":{"hero_data_path":temp.path().join("heroes.json")},
            "voicelines":{"base_url":server.url,"entity_data_file":temp.path().join("entities.json"),"resource_file":temp.path().join("responses.json")}
        }));
        let mut titles: Vec<_> = (0..51).map(|n| format!("Hero {n}")).collect();
        titles[0] = "Hero_0".into();
        let revisions = voices(&config).revisions(&titles).await?;
        assert_eq!(revisions["Hero_0"].id, 1);
        assert_eq!(revisions["Hero 50"].id, 51);
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests.iter() {
            let path = request.0.split_whitespace().nth(1).unwrap();
            let url = reqwest::Url::parse(&format!("http://example.com{path}"))?;
            let (_, titles) = url.query_pairs().find(|(key, _)| key == "titles").unwrap();
            assert!(titles.split('|').count() <= 50);
        }
        Ok(())
    }
    #[tokio::test]
    async fn reuses_revisions_across_restarts_and_resumes_a_failed_refresh() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let nav = r#"<div class="nowraplinks"><table><tr><th>Hero responses</th><td class="hlist"><a title="Sniper/Responses" href="/dota2/Sniper/Responses">Sniper</a></td></tr></table></div>"#;
        let server = Server::new(vec![
            (200, navigation_info()),
            (200, json!({"parse":{"revid":10,"text":{"*":nav}}})),
            (
                200,
                json!({"query":{"categorymembers":[{"title":"Sniper/Responses","ns":0}]},"continue":{"cmcontinue":"next-page"}}),
            ),
            (
                200,
                json!({"query":{"categorymembers":[{"title":"Axe/Responses","ns":0},{"title":"Category:Responses","ns":14}]}}),
            ),
            (200, info(11, 12)),
            (200, rendered(11, "Axe!")),
            (200, rendered(12, "Sniper!")),
            // Restart with identical revisions: metadata only, no rendering.
            (200, navigation_info()),
            (200, category()),
            (200, info(11, 12)),
            // Change only Axe: exactly one parse call.
            (200, navigation_info()),
            (200, category()),
            (200, info(13, 12)),
            (200, rendered(13, "New Axe!")),
            // Both changed; keep the old complete catalog when Sniper fails.
            (200, navigation_info()),
            (200, category()),
            (200, info(14, 15)),
            (200, rendered(14, "Newest Axe!")),
            (400, json!({})),
            // Restart/retry reuses Axe's successful page from the failed crawl.
            (200, navigation_info()),
            (200, category()),
            (200, info(14, 15)),
            (200, rendered(15, "New Sniper!")),
            // A template/cache invalidation can change HTML without a page edit.
            (200, navigation_info()),
            (200, category()),
            (200, touched_info()),
            (200, rendered(14, "Template changed!")),
            (200, navigation_info()),
            (200, category()),
            (200, touched_info()),
        ]);
        let mut value: Value = serde_json::from_str(include_str!("../../config/default.json"))?;
        value["dynamic_resources"]["hero_data_path"] = json!(temp.path().join("heroes.json"));
        value["voicelines"]["entity_data_file"] = json!(temp.path().join("entities.json"));
        value["voicelines"]["resource_file"] = json!(temp.path().join("responses.json"));
        value["voicelines"]["base_url"] = json!(server.url);
        value["voicelines"]["secs_between_requests"] = json!(1);
        value["voicelines"]["user_agent_email"] = json!("private@example.com");
        let config = Config(value);
        assert!(voices(&config).refresh_enabled());
        assert_eq!(voices(&config).request_delay(&[("action", "parse")]), 60);
        assert_eq!(voices(&config).request_delay(&[("action", "query")]), 2);
        let first = voices(&config);
        first.refresh().await?;
        let path = temp.path().join("voicelines.json");
        let first_bytes = std::fs::read(&path)?;
        voices(&config).refresh().await?;
        assert_eq!(std::fs::read(&path)?, first_bytes);
        voices(&config).refresh().await?;
        let changed_bytes = std::fs::read(&path)?;
        let failed = voices(&config);
        assert!(failed.refresh().await.is_err());
        assert_eq!(std::fs::read(&path)?, changed_bytes);
        assert_eq!(
            failed.catalog.read().await.as_ref().unwrap().responses["Axe/Responses"][0].text,
            "New Axe!"
        );
        let recovered = voices(&config);
        recovered.refresh().await?;
        assert_eq!(
            recovered.catalog.read().await.as_ref().unwrap().responses["Axe/Responses"][0].text,
            "Newest Axe!"
        );
        assert_eq!(
            recovered.catalog.read().await.as_ref().unwrap().responses["Sniper/Responses"][0].text,
            "New Sniper!"
        );
        voices(&config).refresh().await?;
        let invalidated = voices(&config);
        invalidated.refresh().await?;
        assert_eq!(
            invalidated.catalog.read().await.as_ref().unwrap().responses["Axe/Responses"][0].text,
            "Template changed!"
        );
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 30);
        assert!(requests.iter().all(|r| r.0.starts_with("GET /api.php?")));
        assert!(requests[3].0.contains("cmcontinue=next-page"));
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.0.contains("action=parse"))
                .count(),
            8
        );
        assert!(!requests[7..10].iter().any(|r| r.0.contains("action=parse")));
        for headers in server.headers.lock().unwrap().iter() {
            assert!(
                headers
                    .to_ascii_lowercase()
                    .contains("accept-encoding: gzip")
            );
            let agent = headers
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("user-agent:"))
                .unwrap();
            assert!(agent.contains("contact: https://github.com/Eixix/sili-telegram-bot/issues"));
            assert!(!agent.contains('@'));
        }
        Ok(())
    }
}
