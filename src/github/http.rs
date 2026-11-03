use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::{Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{bot_err, Result};

const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
const USER_AGENT_VALUE: &str = "ibex-benchmark-bot";
/// A single GitHub 5xx must not lose a publish run: on 2026-11-03 one `504 Gateway Timeout`
/// on `contents get` failed the whole HNSW publish. Four attempts cover a short outage
/// without tying up a runner for minutes.
const MAX_ATTEMPTS: u32 = 4;
const BASE_DELAY_MS: u64 = 250;
const JITTER_MS: u64 = 125;

/// Only transient shapes are retried (AGENTS.md §8.3): a 4xx is an answer about the
/// request, and retrying it just delays the failure.
fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
}

fn backoff_delay(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(6);
    let base = BASE_DELAY_MS.saturating_mul(1u64 << exponent);
    Duration::from_millis(base + jitter_ms())
}

/// Jitter keeps parallel runs from retrying in lockstep; time-of-day is enough entropy here
/// and avoids pulling in an RNG dependency for one number.
fn jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::from(elapsed.subsec_nanos()) % (JITTER_MS + 1))
        .unwrap_or(0)
}

pub(crate) struct HttpClient {
    http: reqwest::Client,
    token: String,
}

impl HttpClient {
    pub(crate) fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
        }
    }

    pub(crate) async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .send(Method::GET, path, github_json_accept(), None)
            .await?;
        response
            .json::<T>()
            .await
            .map_err(|err| bot_err(format!("GET {path} decode failed: {err}")))
    }

    pub(crate) async fn post_json(&self, path: &str, body: Value) -> Result<Value> {
        self.json_response(Method::POST, path, body).await
    }

    /// POST JSON and return the raw response (caller inspects status).
    pub(crate) async fn post_json_raw(&self, path: &str, body: Value) -> Result<Response> {
        self.send_unchecked(Method::POST, path, github_json_accept(), Some(body))
            .await
    }

    pub(crate) async fn put_json(&self, path: &str, body: Value) -> Result<Value> {
        self.json_response(Method::PUT, path, body).await
    }

    pub(crate) async fn patch_json(&self, path: &str, body: Value) -> Result<Value> {
        self.json_response(Method::PATCH, path, body).await
    }

    pub(crate) async fn get_with_accept(&self, path: &str, accept: &str) -> Result<Response> {
        self.send(Method::GET, path, accept, None).await
    }

    pub(crate) async fn get_raw(&self, path: &str, accept: &str) -> Result<Response> {
        let url = format!("{API_BASE}{path}");
        self.authorized(self.http.request(Method::GET, url), accept)
            .send()
            .await
            .map_err(|err| bot_err(format!("GET {path} failed: {err}")))
    }

    async fn json_response(&self, method: Method, path: &str, body: Value) -> Result<Value> {
        let label = method.clone();
        let response = self
            .send(method, path, github_json_accept(), Some(body))
            .await?;
        response
            .json::<Value>()
            .await
            .map_err(|err| bot_err(format!("{label} {path} decode failed: {err}")))
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        accept: &str,
        body: Option<Value>,
    ) -> Result<Response> {
        let response = self
            .send_unchecked(method.clone(), path, accept, body)
            .await?;
        if !response.status().is_success() {
            return Err(bot_err(format!(
                "{method} {path} failed: {}",
                response.text().await.unwrap_or_default()
            )));
        }
        Ok(response)
    }

    async fn send_unchecked(
        &self,
        method: Method,
        path: &str,
        accept: &str,
        body: Option<Value>,
    ) -> Result<Response> {
        let label = method.clone();
        let url = format!("{API_BASE}{path}");
        let mut attempt = 1;
        loop {
            let mut request =
                self.authorized(self.http.request(method.clone(), url.clone()), accept);
            if let Some(payload) = body.as_ref() {
                request = request.json(payload);
            }
            match request.send().await {
                Ok(response)
                    if is_retryable_status(response.status()) && attempt < MAX_ATTEMPTS =>
                {
                    tokio::time::sleep(backoff_delay(attempt)).await;
                    attempt += 1;
                }
                Ok(response) => return Ok(response),
                Err(_) if attempt < MAX_ATTEMPTS => {
                    tokio::time::sleep(backoff_delay(attempt)).await;
                    attempt += 1;
                }
                Err(err) => return Err(bot_err(format!("{label} {path} failed: {err}"))),
            }
        }
    }

    fn authorized(&self, builder: RequestBuilder, accept: &str) -> RequestBuilder {
        builder
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, accept)
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, USER_AGENT_VALUE)
    }
}

fn github_json_accept() -> &'static str {
    "application/vnd.github+json"
}

pub(crate) fn github_raw_accept() -> &'static str {
    "application/vnd.github.raw+json"
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn retries_only_transient_statuses() {
        for status in [429, 500, 502, 503, 504] {
            assert!(is_retryable_status(StatusCode::from_u16(status).unwrap()));
        }
        for status in [200, 201, 400, 401, 403, 404, 409, 422] {
            assert!(!is_retryable_status(StatusCode::from_u16(status).unwrap()));
        }
    }

    #[test]
    fn backoff_grows_and_is_bounded() {
        let first = backoff_delay(1);
        let third = backoff_delay(3);
        assert!(third > first);
        // Exponential growth is capped, and jitter adds at most JITTER_MS.
        assert!(backoff_delay(9) <= Duration::from_millis(BASE_DELAY_MS * 64 + JITTER_MS));
    }
}
