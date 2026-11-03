use std::sync::RwLock;
use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::domain::{AppError, AppResult};

const USER_AGENT: &str =
    "BingWallpaper/2.1.0 (+https://github.com/lzdbh/BingWallpaper2)";

/// Small wrapper around `reqwest::Client` that can be rebuilt when the proxy
/// setting changes.
pub struct HttpClient {
    client: RwLock<reqwest::Client>,
}

impl HttpClient {
    pub fn new(proxy: Option<&str>) -> AppResult<Self> {
        Ok(Self {
            client: RwLock::new(build_client(proxy)?),
        })
    }

    pub fn rebuild(&self, proxy: Option<&str>) -> AppResult<()> {
        let client = build_client(proxy)?;
        if let Ok(mut guard) = self.client.write() {
            *guard = client;
        }
        Ok(())
    }

    fn client(&self) -> reqwest::Client {
        self.client
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| reqwest::Client::new())
    }

    pub async fn get_bytes(&self, url: &str) -> AppResult<Vec<u8>> {
        let response = self.client().get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::Network(format!(
                "HTTP {} · {}",
                status.as_u16(),
                url
            )));
        }
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> AppResult<T> {
        let response = self.client().get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::Network(format!(
                "HTTP {} · {}",
                status.as_u16(),
                url
            )));
        }
        let text = response.text().await?;
        serde_json::from_str(&text)
            .map_err(|error| AppError::Bing(format!("响应不是合法 JSON：{error}")))
    }
}

fn build_client(proxy: Option<&str>) -> AppResult<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(45))
        .connect_timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(6));

    if let Some(proxy) = proxy {
        let proxy = proxy.trim();
        if !proxy.is_empty() {
            builder = builder.proxy(
                reqwest::Proxy::all(proxy)
                    .map_err(|error| AppError::InvalidInput(format!("代理地址无效：{error}")))?,
            );
        }
    }

    builder
        .build()
        .map_err(|error| AppError::Network(error.to_string()))
}
