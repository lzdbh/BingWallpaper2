use serde::Deserialize;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::ImageSize;
use crate::infra::http::HttpClient;

const BING_HOSTS: [&str; 2] = ["https://www.bing.com", "https://cn.bing.com"];

#[derive(Debug, Clone, Deserialize)]
struct RawResponse {
    #[serde(default)]
    images: Vec<RawImage>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawImage {
    #[serde(default)]
    startdate: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    urlbase: String,
    #[serde(default)]
    copyright: String,
    #[serde(default)]
    copyrightlink: String,
    #[serde(default)]
    title: String,
}

#[derive(Debug, Clone)]
pub struct BingImage {
    pub date: String,
    pub url_base: String,
    pub copyright: String,
    pub copyright_link: String,
    pub title: String,
}

pub struct BingClient<'a> {
    http: &'a HttpClient,
    hosts: Vec<String>,
}

impl<'a> BingClient<'a> {
    pub fn new(http: &'a HttpClient) -> Self {
        Self {
            http,
            hosts: BING_HOSTS.iter().map(|host| (*host).to_string()).collect(),
        }
    }

    /// Test-only constructor that points the client at arbitrary hosts, e.g. a
    /// local wiremock server, so the Bing JSON handling can be exercised
    /// without touching the network.
    #[cfg(test)]
    pub fn with_hosts(http: &'a HttpClient, hosts: Vec<String>) -> Self {
        Self { http, hosts }
    }

    /// Fetch up to the latest 8 images for a market.
    pub async fn fetch(&self, market: &str) -> AppResult<Vec<BingImage>> {
        let mut last_error: Option<AppError> = None;

        for host in &self.hosts {
            let url = format!(
                "{host}/HPImageArchive.aspx?format=js&idx=0&n=8&mkt={market}"
            );
            match self.http.get_json::<RawResponse>(&url).await {
                Ok(response) => {
                    let images: Vec<BingImage> = response
                        .images
                        .into_iter()
                        .filter_map(normalize)
                        .collect();
                    if images.is_empty() {
                        last_error = Some(AppError::Bing("接口未返回任何图片".into()));
                        continue;
                    }
                    return Ok(images);
                }
                Err(error) => {
                    log::warn!("Bing 接口 {host} 请求失败：{error}");
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AppError::Network("Bing 接口不可用".into())))
    }

    pub async fn fetch_latest(&self, market: &str) -> AppResult<BingImage> {
        self.fetch(market)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Bing("接口未返回任何图片".into()))
    }
}

pub fn image_url(image: &BingImage, size: ImageSize) -> String {
    // `normalize` guarantees `url_base` is non-empty, so it can be used directly.
    let base = image.url_base.as_str();
    if base.starts_with("http") {
        format!("{base}{}", size.suffix())
    } else {
        format!("https://www.bing.com{base}{}", size.suffix())
    }
}

fn normalize(raw: RawImage) -> Option<BingImage> {
    let date = parse_date(&raw.startdate)?;
    let url_base = if raw.urlbase.is_empty() {
        strip_query(&raw.url)
    } else {
        raw.urlbase
    };
    if url_base.is_empty() {
        return None;
    }
    Some(BingImage {
        date,
        url_base,
        copyright: raw.copyright,
        copyright_link: raw.copyrightlink,
        title: if raw.title.trim().is_empty() {
            "Bing 每日壁纸".to_string()
        } else {
            raw.title
        },
    })
}

/// `20261102` -> `2026-11-02`
fn parse_date(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() != 8 || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!(
        "{}-{}-{}",
        &value[0..4],
        &value[4..6],
        &value[6..8]
    ))
}

/// Drop the extra parameters and the `_1920x1080.jpg` suffix so a size can be
/// appended again. The id itself may contain underscores, so only a trailing
/// `_WxH.jpg` / `_UHD.jpg` token is removed.
fn strip_query(url: &str) -> String {
    let head = url.split('&').next().unwrap_or(url);
    if let Some(dot) = head.rfind(".jpg") {
        let stem = &head[..dot];
        if let Some(index) = stem.rfind('_') {
            let token = &stem[index + 1..];
            if token == "UHD" || is_size_token(token) {
                return stem[..index].to_string();
            }
        }
        return stem.to_string();
    }
    head.to_string()
}

fn is_size_token(value: &str) -> bool {
    let mut parts = value.split('x');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(width), Some(height), None) => {
            !width.is_empty()
                && !height.is_empty()
                && width.chars().all(|c| c.is_ascii_digit())
                && height.chars().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

pub fn today_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn compact_date(date: &str) -> String {
    date.replace('-', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_image(date: &str) -> BingImage {
        BingImage {
            date: date.to_string(),
            url_base: "/th?id=OHR.Test_ZH-CN123".to_string(),
            copyright: "© Test".to_string(),
            copyright_link: "https://example.com".to_string(),
            title: "Test".to_string(),
        }
    }

    #[test]
    fn parses_dates() {
        assert_eq!(parse_date("20261102").as_deref(), Some("2026-11-02"));
        assert_eq!(parse_date("2026-11-02"), None);
    }

    #[test]
    fn rejects_malformed_dates() {
        assert_eq!(parse_date("2026110"), None); // too short
        assert_eq!(parse_date("202611020"), None); // too long
        assert_eq!(parse_date("2026ab02"), None); // non-digits
        assert_eq!(parse_date(""), None);
    }

    #[test]
    fn strips_size_suffix() {
        assert_eq!(
            strip_query("/th?id=OHR.Test_ZH-CN123_1920x1080.jpg&rf=x"),
            "/th?id=OHR.Test_ZH-CN123"
        );
        assert_eq!(
            strip_query("/th?id=OHR.Test_ZH-CN123_UHD.jpg"),
            "/th?id=OHR.Test_ZH-CN123"
        );
        // Underscores inside the id are preserved when there is no size token.
        assert_eq!(
            strip_query("/th?id=OHR.Foo_Bar.jpg"),
            "/th?id=OHR.Foo_Bar"
        );
    }

    #[test]
    fn compacts_dates() {
        assert_eq!(compact_date("2026-11-02"), "20261102");
        assert_eq!(compact_date("20261102"), "20261102");
    }

    #[test]
    fn builds_image_urls_for_each_size() {
        let image = sample_image("2026-11-02");
        assert_eq!(
            image_url(&image, ImageSize::Uhd),
            "https://www.bing.com/th?id=OHR.Test_ZH-CN123_UHD.jpg"
        );
        assert_eq!(
            image_url(&image, ImageSize::W1920x1200),
            "https://www.bing.com/th?id=OHR.Test_ZH-CN123_1920x1200.jpg"
        );
        assert_eq!(
            image_url(&image, ImageSize::W1920x1080),
            "https://www.bing.com/th?id=OHR.Test_ZH-CN123_1920x1080.jpg"
        );
        assert_eq!(
            image_url(&image, ImageSize::W1366x768),
            "https://www.bing.com/th?id=OHR.Test_ZH-CN123_1366x768.jpg"
        );
    }

    #[test]
    fn keeps_absolute_url_base() {
        let mut image = sample_image("2026-11-02");
        image.url_base = "https://cdn.example.com/pic".to_string();
        assert_eq!(
            image_url(&image, ImageSize::W1920x1080),
            "https://cdn.example.com/pic_1920x1080.jpg"
        );
    }

    fn bing_body() -> serde_json::Value {
        serde_json::json!({
            "images": [{
                "startdate": "20261102",
                "url": "/th?id=OHR.Test_ZH-CN123_1920x1080.jpg&rf=1",
                "urlbase": "/th?id=OHR.Test_ZH-CN123",
                "copyright": "© Test",
                "copyrightlink": "https://example.com",
                "title": "Test title"
            }]
        })
    }

    async fn mock_server(body: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/HPImageArchive.aspx"))
            .respond_with(body)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn fetch_parses_normal_response() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(bing_body())).await;
        let http = HttpClient::new(None).unwrap();
        let client = BingClient::with_hosts(&http, vec![server.uri()]);

        let images = client.fetch("zh-CN").await.unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].date, "2026-11-02");
        assert_eq!(images[0].url_base, "/th?id=OHR.Test_ZH-CN123");
        assert_eq!(images[0].title, "Test title");
        assert_eq!(images[0].copyright, "© Test");
    }

    #[tokio::test]
    async fn fetch_latest_returns_first_image() {
        let server = mock_server(ResponseTemplate::new(200).set_body_json(bing_body())).await;
        let http = HttpClient::new(None).unwrap();
        let client = BingClient::with_hosts(&http, vec![server.uri()]);

        let image = client.fetch_latest("zh-CN").await.unwrap();
        assert_eq!(image.date, "2026-11-02");
    }

    #[tokio::test]
    async fn fetch_errors_on_empty_images() {
        let server =
            mock_server(ResponseTemplate::new(200).set_body_json(serde_json::json!({"images": []})))
                .await;
        let http = HttpClient::new(None).unwrap();
        let client = BingClient::with_hosts(&http, vec![server.uri()]);

        let error = client.fetch("zh-CN").await.unwrap_err();
        assert!(matches!(error, AppError::Bing(_)));
    }

    #[tokio::test]
    async fn fetch_errors_on_http_500() {
        let server =
            mock_server(ResponseTemplate::new(500).set_body_string("server error")).await;
        let http = HttpClient::new(None).unwrap();
        let client = BingClient::with_hosts(&http, vec![server.uri()]);

        let error = client.fetch("zh-CN").await.unwrap_err();
        assert!(matches!(error, AppError::Network(_)));
    }

    #[tokio::test]
    async fn fetch_errors_on_malformed_json() {
        let server =
            mock_server(ResponseTemplate::new(200).set_body_string("<html>not json</html>")).await;
        let http = HttpClient::new(None).unwrap();
        let client = BingClient::with_hosts(&http, vec![server.uri()]);

        let error = client.fetch("zh-CN").await.unwrap_err();
        assert!(matches!(error, AppError::Bing(_)));
    }
}
