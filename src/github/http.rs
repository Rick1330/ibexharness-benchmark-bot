use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::{Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{bot_err, Result};

const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
const USER_AGENT_VALUE: &str = "ibex-benchmark-bot";

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
        let label = method.clone();
        let url = format!("{API_BASE}{path}");
        let mut request = self.authorized(self.http.request(method, url), accept);
        if let Some(payload) = body {
            request = request.json(&payload);
        }
        let response = request
            .send()
            .await
            .map_err(|err| bot_err(format!("{label} {path} failed: {err}")))?;
        if !response.status().is_success() {
            return Err(bot_err(format!(
                "{label} {path} failed: {}",
                response.text().await.unwrap_or_default()
            )));
        }
        Ok(response)
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
