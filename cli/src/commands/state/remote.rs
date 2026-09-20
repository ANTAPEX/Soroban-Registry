//! The optional remote backing for `state get` and `state set`; both fall back
//! to the local store when the registry does not answer.

use crate::support::net::RequestBuilderExt;
use anyhow::{Context, Result};
use serde_json::json;

pub(crate) async fn try_remote_state_get(
    api_url: &str,
    contract_id: &str,
    key: &str,
) -> Result<Option<serde_json::Value>> {
    let mut url = reqwest::Url::parse(api_url).context("Invalid API URL")?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Invalid API URL"))?
        .extend(["api", "contracts", contract_id, "state", key]);

    let response = match crate::support::net::client()
        .get(url)
        .send_with_retry()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if response.status() == reqwest::StatusCode::NOT_IMPLEMENTED {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Ok(None);
    }

    let payload: serde_json::Value = response.json().await.unwrap_or_else(|_| json!({}));
    if let Some(value) = payload.get("value") {
        return Ok(Some(value.clone()));
    }
    Ok(Some(payload))
}

pub(crate) async fn try_remote_state_set(
    api_url: &str,
    contract_id: &str,
    key: &str,
    value: &serde_json::Value,
) -> Result<bool> {
    let mut url = reqwest::Url::parse(api_url).context("Invalid API URL")?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Invalid API URL"))?
        .extend(["api", "contracts", contract_id, "state", key]);

    let response = match crate::support::net::client()
        .put(url)
        .json(&json!({ "value": value }))
        .send_with_retry()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return Ok(false),
    };

    if response.status() == reqwest::StatusCode::NOT_IMPLEMENTED {
        return Ok(false);
    }

    Ok(response.status().is_success())
}
