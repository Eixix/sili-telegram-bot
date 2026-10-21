use anyhow::{Result, bail};
use reqwest::{Client, multipart};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

#[derive(Clone)]
pub struct Telegram {
    client: Client,
    root: String,
    pub chat: i64,
}

impl Telegram {
    pub fn new(client: Client, token: &str, chat: i64) -> Self {
        Self {
            client,
            root: format!("https://api.telegram.org/bot{token}"),
            chat,
        }
    }
    #[cfg(test)]
    pub fn mock(client: Client, root: String) -> Self {
        Self {
            client,
            root,
            chat: -123,
        }
    }
    pub async fn call(&self, method: &str, body: Value) -> Result<Value> {
        let response = self
            .client
            .post(format!("{}/{method}", self.root))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.without_url())?;
        Self::result(response).await
    }
    async fn result(response: reqwest::Response) -> Result<Value> {
        let status = response.status();
        let body: Value = response.json().await.map_err(|e| e.without_url())?;
        if !status.is_success() || body["ok"] != true {
            // Telegram descriptions can include arbitrary request data; avoid logging secrets.
            bail!(
                "Telegram request failed (HTTP {}, code {}, retry after {}s)",
                status.as_u16(),
                body["error_code"],
                body["parameters"]["retry_after"]
            );
        }
        Ok(body["result"].clone())
    }
    pub async fn text(&self, text: &str) -> Result<()> {
        self.call(
            "sendMessage",
            json!({"chat_id":self.chat,"text":text,"parse_mode":"HTML"}),
        )
        .await?;
        Ok(())
    }
    pub async fn upload(&self, method: &str, field: &str, path: &Path) -> Result<()> {
        self.upload_bytes(
            method,
            field,
            path.file_name().unwrap().to_string_lossy().into_owned(),
            std::fs::read(path)?,
        )
        .await
    }
    pub async fn upload_bytes(
        &self,
        method: &str,
        field: &str,
        filename: String,
        bytes: Vec<u8>,
    ) -> Result<()> {
        self.upload_bytes_caption(method, field, filename, bytes, None)
            .await
    }
    pub async fn upload_bytes_caption(
        &self,
        method: &str,
        field: &str,
        filename: String,
        bytes: Vec<u8>,
        caption: Option<&str>,
    ) -> Result<()> {
        let mut form = multipart::Form::new()
            .text("chat_id", self.chat.to_string())
            .part(
                field.to_owned(),
                multipart::Part::bytes(bytes).file_name(filename),
            );
        if let Some(caption) = caption {
            form = form
                .text("caption", caption.to_owned())
                .text("parse_mode", "HTML");
        }
        let response = self
            .client
            .post(format!("{}/{method}", self.root))
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.without_url())?;
        Self::result(response).await?;
        Ok(())
    }
    pub async fn poll_updates(&self, offset: i64) -> Result<Vec<Value>> {
        let response = self
            .client
            .post(format!("{}/getUpdates", self.root))
            .timeout(Duration::from_secs(40))
            .json(
                &json!({"offset":offset,"timeout":30,"allowed_updates":["message","inline_query"]}),
            )
            .send()
            .await
            .map_err(|e| e.without_url())?;
        Ok(serde_json::from_value(Self::result(response).await?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn voice_upload_includes_source_caption_and_html_mode() -> Result<()> {
        let server = crate::test_support::Server::new(vec![(200, json!({"ok":true,"result":{}}))]);
        let telegram = Telegram::mock(Client::new(), server.url.clone());
        telegram.upload_bytes_caption("sendVoice","voice","voice.mp3".into(),b"audio".to_vec(),Some("Source: <a href=\"https://liquipedia.net/dota2/Sniper/Responses\">Liquipedia</a> · CC BY-SA 3.0")).await?;
        let bodies = server.bodies.lock().unwrap();
        let body = String::from_utf8_lossy(&bodies[0]);
        assert!(body.contains("name=\"caption\""));
        assert!(body.contains("https://liquipedia.net/dota2/Sniper/Responses"));
        assert!(body.contains("CC BY-SA 3.0"));
        assert!(body.contains("name=\"parse_mode\"\r\n\r\nHTML"));
        assert!(body.contains("filename=\"voice.mp3\""));
        Ok(())
    }
}
