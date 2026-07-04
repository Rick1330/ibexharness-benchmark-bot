use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{bot_err, Result};
use crate::model::WorkflowRun;

const API_VERSION: &str = "2022-11-28";

pub struct GitHubClient {
    http: reqwest::Client,
    token: String,
}

pub struct PutFileRequest<'a> {
    pub path: &'a str,
    pub branch: &'a str,
    pub bytes: &'a [u8],
    pub message: &'a str,
    pub file_sha: Option<&'a str>,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
        }
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("https://api.github.com{path}");
        let response = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, "ibexharness-benchmark-bot")
            .send()
            .await
            .map_err(|err| bot_err(format!("GET {path} failed: {err}")))?;

        if !response.status().is_success() {
            return Err(bot_err(format!(
                "GET {path} failed: status {}",
                response.status()
            )));
        }

        response
            .json::<T>()
            .await
            .map_err(|err| bot_err(format!("GET {path} decode failed: {err}")))
    }

    pub async fn get_workflow_run(&self, owner: &str, repo: &str, run_id: i64) -> Result<WorkflowRun> {
        self.get_json(&format!("/repos/{owner}/{repo}/actions/runs/{run_id}"))
            .await
    }

    pub async fn ref_exists(&self, owner: &str, repo: &str, branch: &str) -> Result<bool> {
        let path = format!("/repos/{owner}/{repo}/git/ref/heads/{branch}");
        let url = format!("https://api.github.com{path}");
        let response = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, "ibexharness-benchmark-bot")
            .send()
            .await
            .map_err(|err| bot_err(format!("ref check failed: {err}")))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        if !response.status().is_success() {
            return Err(bot_err(format!(
                "ref check failed: {}",
                response.text().await.unwrap_or_default()
            )));
        }
        Ok(true)
    }

    pub async fn download_artifact_zip(&self, owner: &str, repo: &str, run_id: i64) -> Result<Vec<u8>> {
        let artifacts: Value = self
            .get_json(&format!("/repos/{owner}/{repo}/actions/runs/{run_id}/artifacts"))
            .await?;
        let items = artifacts
            .get("artifacts")
            .and_then(|value| value.as_array())
            .ok_or_else(|| bot_err("artifacts list missing".to_string()))?;
        let artifact = items
            .iter()
            .find(|item| item.get("name").and_then(|v| v.as_str()) == Some("benchmark-data"))
            .ok_or_else(|| bot_err("benchmark-data artifact not found".to_string()))?;
        let artifact_id = artifact
            .get("id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| bot_err("artifact id missing".to_string()))?;

        let url = format!("https://api.github.com/repos/{owner}/{repo}/actions/artifacts/{artifact_id}/zip");
        let response = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, "ibexharness-benchmark-bot")
            .send()
            .await
            .map_err(|err| bot_err(format!("artifact download failed: {err}")))?;

        if !response.status().is_success() {
            return Err(bot_err(format!(
                "artifact download failed: {}",
                response.text().await.unwrap_or_default()
            )));
        }

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|err| bot_err(format!("artifact read failed: {err}")))
    }

    pub async fn create_branch(&self, owner: &str, repo: &str, branch: &str, sha: &str) -> Result<()> {
        self.post_json(
            &format!("/repos/{owner}/{repo}/git/refs"),
            serde_json::json!({
                "ref": format!("refs/heads/{branch}"),
                "sha": sha,
            }),
        )
        .await?;
        Ok(())
    }

    pub async fn get_file_bytes(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        git_ref: &str,
    ) -> Result<Option<Vec<u8>>> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={git_ref}");
        let response = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, "ibexharness-benchmark-bot")
            .send()
            .await
            .map_err(|err| bot_err(format!("contents get failed: {err}")))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(bot_err(format!(
                "contents get failed: status {}",
                response.status()
            )));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|err| bot_err(format!("contents decode failed: {err}")))?;
        let encoded = value
            .get("content")
            .and_then(|item| item.as_str())
            .ok_or_else(|| bot_err("contents missing content".to_string()))?;
        let bytes = STANDARD
            .decode(encoded.replace('\n', ""))
            .map_err(|err| bot_err(format!("contents base64 decode failed: {err}")))?;
        Ok(Some(bytes))
    }

    pub async fn find_labeled_pr_with_sha(
        &self,
        owner: &str,
        repo: &str,
        label: &str,
        head_sha: &str,
    ) -> Result<Option<Value>> {
        let pulls: Vec<Value> = self
            .get_json(&format!(
                "/repos/{owner}/{repo}/issues?state=open&labels={label}"
            ))
            .await?;
        Ok(pulls.into_iter().find(|item| {
            item.get("body")
                .and_then(|body| body.as_str())
                .is_some_and(|body| body.contains(head_sha))
        }))
    }

    pub async fn main_sha(&self, owner: &str, repo: &str) -> Result<String> {
        let value: Value = self
            .get_json(&format!("/repos/{owner}/{repo}/git/ref/heads/main"))
            .await?;
        value
            .pointer("/object/sha")
            .and_then(|sha| sha.as_str())
            .map(str::to_owned)
            .ok_or_else(|| bot_err("main sha missing".to_string()))
    }

    pub async fn file_sha(&self, owner: &str, repo: &str, path: &str, branch: &str) -> Result<Option<String>> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={branch}");
        let response = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, "ibexharness-benchmark-bot")
            .send()
            .await
            .map_err(|err| bot_err(format!("contents get failed: {err}")))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(bot_err(format!(
                "contents get failed: {}",
                response.text().await.unwrap_or_default()
            )));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|err| bot_err(format!("contents decode failed: {err}")))?;
        Ok(value.get("sha").and_then(|sha| sha.as_str()).map(str::to_owned))
    }

    pub async fn put_file(&self, owner: &str, repo: &str, req: PutFileRequest<'_>) -> Result<()> {
        let mut body = serde_json::json!({
            "message": req.message,
            "content": STANDARD.encode(req.bytes),
            "branch": req.branch,
        });
        if let Some(sha) = req.file_sha {
            body["sha"] = Value::String(sha.to_string());
        }
        self.put_json(&format!("/repos/{owner}/{repo}/contents/{}", req.path), body)
            .await?;
        Ok(())
    }

    pub async fn open_pull_request(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<Value> {
        self.post_json(
            &format!("/repos/{owner}/{repo}/pulls"),
            serde_json::json!({
                "title": title,
                "head": branch,
                "base": "main",
                "body": body,
                "maintainer_can_modify": false,
            }),
        )
        .await
    }

    pub async fn add_labels(&self, owner: &str, repo: &str, issue_number: i64, labels: &[&str]) -> Result<()> {
        let _ = self
            .post_json(
                &format!("/repos/{owner}/{repo}/issues/{issue_number}/labels"),
                serde_json::json!({ "labels": labels }),
            )
            .await;
        Ok(())
    }

    pub async fn find_open_pr(&self, owner: &str, repo: &str, branch: &str) -> Result<Option<Value>> {
        let pulls: Vec<Value> = self
            .get_json(&format!("/repos/{owner}/{repo}/pulls?state=open&head={owner}:{branch}"))
            .await?;
        Ok(pulls.into_iter().next())
    }

    pub async fn post_issue_comment(&self, owner: &str, repo: &str, issue: i64, body: &str) -> Result<()> {
        self.post_json(
            &format!("/repos/{owner}/{repo}/issues/{issue}/comments"),
            serde_json::json!({ "body": body }),
        )
        .await?;
        Ok(())
    }

    async fn post_json(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("https://api.github.com{path}");
        let response = self
            .http
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, "ibexharness-benchmark-bot")
            .json(&body)
            .send()
            .await
            .map_err(|err| bot_err(format!("POST {path} failed: {err}")))?;

        if !response.status().is_success() {
            return Err(bot_err(format!(
                "POST {path} failed: {}",
                response.text().await.unwrap_or_default()
            )));
        }
        response
            .json::<Value>()
            .await
            .map_err(|err| bot_err(format!("POST {path} decode failed: {err}")))
    }

    async fn put_json(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("https://api.github.com{path}");
        let response = self
            .http
            .put(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header(USER_AGENT, "ibexharness-benchmark-bot")
            .json(&body)
            .send()
            .await
            .map_err(|err| bot_err(format!("PUT {path} failed: {err}")))?;

        if !response.status().is_success() {
            return Err(bot_err(format!(
                "PUT {path} failed: {}",
                response.text().await.unwrap_or_default()
            )));
        }
        response
            .json::<Value>()
            .await
            .map_err(|err| bot_err(format!("PUT {path} decode failed: {err}")))
    }
}

pub fn split_repo(full_name: &str) -> Result<(&str, &str)> {
    let (owner, repo) = full_name
        .split_once('/')
        .ok_or_else(|| bot_err(format!("invalid repo: {full_name}")))?;
    Ok((owner, repo))
}
