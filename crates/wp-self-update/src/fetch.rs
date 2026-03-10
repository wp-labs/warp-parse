use crate::error::{err, Result};
use crate::manifest::{
    parse_v2_release, updates_manifest_path, updates_manifest_url, ResolvedRelease,
};
use crate::platform::detect_target_triple_v2;
use crate::types::{SelfCheckRequest, UpdateChannel};
use reqwest::StatusCode;
use std::time::Duration;

const FETCH_CONNECT_TIMEOUT_SECS: u64 = 5;
const FETCH_REQUEST_TIMEOUT_SECS: u64 = 10;
const FETCH_RETRY_MAX_ATTEMPTS: usize = 3;

pub(crate) async fn load_release(
    req: &SelfCheckRequest,
    channel: UpdateChannel,
) -> Result<(ResolvedRelease, String)> {
    let target = detect_target_triple_v2()?;

    if let Some(root) = req.updates_root.as_deref() {
        let path = updates_manifest_path(root, channel);
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| err(format!("failed to read manifest {}: {}", path.display(), e)))?;
        let source = path.display().to_string();
        let release = parse_v2_release(&raw, &source, channel, target)?;
        return Ok((release, source));
    }

    let url = updates_manifest_url(&req.updates_base_url, channel);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(FETCH_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(FETCH_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| err(format!("failed to build HTTP client: {}", e)))?;

    let raw = fetch_manifest_text(&client, &url).await?;
    let release = parse_v2_release(&raw, &url, channel, target)?;
    Ok((release, url))
}

async fn fetch_manifest_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let mut last_error: Option<String> = None;

    for attempt in 1..=FETCH_RETRY_MAX_ATTEMPTS {
        match client.get(url).send().await {
            Ok(rsp) => {
                let status = rsp.status();
                if status.is_success() {
                    return rsp.text().await.map_err(|e| {
                        err(format!("failed to read manifest response {}: {}", url, e))
                    });
                }

                if status == StatusCode::NOT_FOUND {
                    return Err(err(format!("manifest not found: {}", url)));
                }

                if is_retryable_status(status) && attempt < FETCH_RETRY_MAX_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                    continue;
                }

                return Err(err(format!(
                    "manifest request failed {}: HTTP {}",
                    url, status
                )));
            }
            Err(e) => {
                last_error = Some(e.to_string());
                if attempt < FETCH_RETRY_MAX_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                    continue;
                }
            }
        }
    }

    Err(err(format!(
        "failed to fetch manifest {} after {} attempts: {}",
        url,
        FETCH_RETRY_MAX_ATTEMPTS,
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn is_retryable_status(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_status_rules_ok() {
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!is_retryable_status(StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }
}
