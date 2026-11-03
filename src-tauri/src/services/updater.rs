use serde::Deserialize;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{UpdateChannel, UpdateInfo};
use crate::infra::http::HttpClient;

/// GitHub repository used for update checks (owner / name).
pub const GITHUB_OWNER: &str = "lzdbh";
pub const GITHUB_REPO: &str = "BingWallpaper2";

const API_BASE: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
struct Release {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

pub async fn check(
    http: &HttpClient,
    current_version: &str,
    channel: UpdateChannel,
) -> AppResult<Option<UpdateInfo>> {
    let release = match channel {
        UpdateChannel::Stable => {
            let url = format!("{API_BASE}/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest");
            match http.get_json::<Release>(&url).await {
                Ok(release) => Some(release),
                // A repository without releases returns 404, which is not an error
                // from the user's point of view.
                Err(AppError::Network(message)) if message.contains("HTTP 404") => None,
                Err(error) => return Err(error),
            }
        }
        UpdateChannel::Beta => {
            let url = format!(
                "{API_BASE}/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases?per_page=10"
            );
            match http.get_json::<Vec<Release>>(&url).await {
                Ok(releases) => releases
                    .into_iter()
                    .find(|release| !release.draft && !release.html_url.is_empty()),
                Err(AppError::Network(message)) if message.contains("HTTP 404") => None,
                Err(error) => return Err(error),
            }
        }
    };

    let Some(release) = release else {
        return Ok(None);
    };
    if release.draft {
        return Ok(None);
    }
    // Stable channel must never surface a prerelease; Beta channel may.
    if release.prerelease && channel == UpdateChannel::Stable {
        return Ok(None);
    }

    let version = normalize_version(&release.tag_name);
    if version.is_empty() || !is_newer(current_version, &version) {
        return Ok(None);
    }

    Ok(Some(UpdateInfo {
        version,
        current_version: current_version.to_string(),
        notes: if release.body.trim().is_empty() {
            release.name
        } else {
            release.body
        },
        url: release.html_url,
        published_at: release.published_at,
        mandatory: false,
    }))
}

fn normalize_version(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn is_newer(current: &str, candidate: &str) -> bool {
    let current = parse_versions(current);
    let candidate = parse_versions(candidate);
    for index in 0..current.len().max(candidate.len()) {
        let left = *current.get(index).unwrap_or(&0);
        let right = *candidate.get(index).unwrap_or(&0);
        if right > left {
            return true;
        }
        if right < left {
            return false;
        }
    }
    false
}

fn parse_versions(value: &str) -> Vec<u64> {
    value
        .trim()
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .take(3)
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("2.1.0", "2.1.1"));
        assert!(is_newer("2.1.0", "2.2.0"));
        assert!(!is_newer("2.1.0", "2.1.0"));
        assert!(!is_newer("2.2.0", "2.1.9"));
        assert!(is_newer("2.1.0", "3.0"));
    }

    #[test]
    fn normalizes_tags() {
        assert_eq!(normalize_version("v2.1.0"), "2.1.0");
    }
}
