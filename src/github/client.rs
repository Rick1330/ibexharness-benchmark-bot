use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::StatusCode;
use serde_json::Value;

use crate::error::{bot_err, Result};
use crate::model::WorkflowRun;

use super::http::{github_raw_accept, HttpClient};

pub struct GitHubClient {
    http: HttpClient,
}

#[derive(Clone, Copy)]
pub struct RepoRef<'a> {
    pub owner: &'a str,
    pub repo: &'a str,
}

#[derive(Clone, Copy)]
pub struct RepoPathRef<'a> {
    pub repo: RepoRef<'a>,
    pub path: &'a str,
    pub git_ref: &'a str,
}

#[derive(Clone, Copy)]
pub struct CreateBranch<'a> {
    pub repo: RepoRef<'a>,
    pub branch: &'a str,
    pub sha: &'a str,
}

#[derive(Clone, Copy)]
pub struct OpenPullRequest<'a> {
    pub repo: RepoRef<'a>,
    pub branch: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}

#[derive(Clone, Copy)]
pub struct IssueRef<'a> {
    pub repo: RepoRef<'a>,
    pub number: i64,
}

#[derive(Clone, Copy)]
pub struct IssueCommentUpdate<'a> {
    pub repo: RepoRef<'a>,
    pub comment_id: i64,
    pub body: &'a str,
}

#[derive(Clone, Copy)]
pub struct CommitStatus<'a> {
    pub repo: RepoRef<'a>,
    pub sha: &'a str,
    pub state: &'a str,
    pub description: &'a str,
    pub context: &'a str,
}

#[derive(Clone, Copy)]
pub struct LabeledPrSearch<'a> {
    pub repo: RepoRef<'a>,
    pub label: &'a str,
    pub head_sha: &'a str,
}

pub struct PutFileRequest<'a> {
    pub path: &'a str,
    pub branch: &'a str,
    pub bytes: &'a [u8],
    pub message: &'a str,
    pub file_sha: Option<&'a str>,
}

/// One path/content pair for a single Git Data API commit.
pub struct CommitFile<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

pub struct CommitFilesRequest<'a> {
    pub branch: &'a str,
    pub message: &'a str,
    pub files: &'a [CommitFile<'a>],
}

const BOT_COMMIT_NAME: &str = "ibex-harness-benchmark[bot]";

/// DCO trailer + noreply email derived from APP_ID (GitHub App convention).
pub fn bot_commit_message(subject: &str) -> String {
    let app_id = std::env::var("APP_ID").unwrap_or_else(|_| "0".to_string());
    let email = format!("{app_id}+ibex-harness-benchmark[bot]@users.noreply.github.com");
    format!("{subject}\n\nSigned-off-by: {BOT_COMMIT_NAME} <{email}>")
}

fn bot_author_email() -> String {
    let app_id = std::env::var("APP_ID").unwrap_or_else(|_| "0".to_string());
    format!("{app_id}+ibex-harness-benchmark[bot]@users.noreply.github.com")
}

impl<'a> RepoRef<'a> {
    pub fn new(owner: &'a str, repo: &'a str) -> Self {
        Self { owner, repo }
    }

    fn base_path(&self) -> String {
        format!("/repos/{}/{}", self.owner, self.repo)
    }
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        Self {
            http: HttpClient::new(token),
        }
    }

    pub async fn get_workflow_run(&self, repo: RepoRef<'_>, run_id: i64) -> Result<WorkflowRun> {
        self.http
            .get_json(&format!("{}/actions/runs/{run_id}", repo.base_path()))
            .await
    }

    pub async fn ref_exists(&self, repo: RepoRef<'_>, branch: &str) -> Result<bool> {
        let path = format!("{}/git/ref/heads/{branch}", repo.base_path());
        let response = self
            .http
            .get_raw(&path, "application/vnd.github+json")
            .await?;
        match response.status() {
            StatusCode::NOT_FOUND => Ok(false),
            status if status.is_success() => Ok(true),
            _ => Err(bot_err(format!(
                "ref check failed: {}",
                response.text().await.unwrap_or_default()
            ))),
        }
    }

    pub async fn download_artifact_zip(&self, repo: RepoRef<'_>, run_id: i64) -> Result<Vec<u8>> {
        let artifacts: Value = self
            .http
            .get_json(&format!(
                "{}/actions/runs/{run_id}/artifacts",
                repo.base_path()
            ))
            .await?;
        let artifact_id = find_benchmark_artifact_id(&artifacts)?;
        let path = format!("{}/actions/artifacts/{artifact_id}/zip", repo.base_path());
        let response = self
            .http
            .get_with_accept(&path, "application/vnd.github+json")
            .await?;
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|err| bot_err(format!("artifact read failed: {err}")))
    }

    pub async fn create_branch(&self, req: CreateBranch<'_>) -> Result<()> {
        let path = format!("{}/git/refs", req.repo.base_path());
        self.http
            .post_json(
                &path,
                serde_json::json!({
                    "ref": format!("refs/heads/{}", req.branch),
                    "sha": req.sha,
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn get_file_bytes(&self, req: RepoPathRef<'_>) -> Result<Option<Vec<u8>>> {
        let path = contents_path(&req.repo, req.path, req.git_ref);
        let response = self
            .http
            .get_raw(&path, "application/vnd.github+json")
            .await?;
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
        decode_contents_json(&self.http, &value, &req).await
    }

    pub async fn find_labeled_pr_with_sha(
        &self,
        search: LabeledPrSearch<'_>,
    ) -> Result<Option<Value>> {
        let pulls: Vec<Value> = self
            .http
            .get_json(&format!(
                "{}/issues?state=open&labels={}",
                search.repo.base_path(),
                search.label
            ))
            .await?;
        Ok(pulls.into_iter().find(|item| {
            item.get("body")
                .and_then(|body| body.as_str())
                .is_some_and(|body| body.contains(search.head_sha))
        }))
    }

    pub async fn main_sha(&self, repo: RepoRef<'_>) -> Result<String> {
        let value: Value = self
            .http
            .get_json(&format!("{}/git/ref/heads/main", repo.base_path()))
            .await?;
        value
            .pointer("/object/sha")
            .and_then(|sha| sha.as_str())
            .map(str::to_owned)
            .ok_or_else(|| bot_err("main sha missing".to_string()))
    }

    pub async fn file_sha(&self, req: RepoPathRef<'_>) -> Result<Option<String>> {
        let path = contents_path(&req.repo, req.path, req.git_ref);
        let response = self
            .http
            .get_raw(&path, "application/vnd.github+json")
            .await?;
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
        Ok(value
            .get("sha")
            .and_then(|sha| sha.as_str())
            .map(str::to_owned))
    }

    pub async fn put_file(&self, repo: RepoRef<'_>, req: PutFileRequest<'_>) -> Result<()> {
        let mut body = serde_json::json!({
            "message": req.message,
            "content": STANDARD.encode(req.bytes),
            "branch": req.branch,
        });
        if let Some(sha) = req.file_sha {
            body["sha"] = Value::String(sha.to_string());
        }
        self.http
            .put_json(&format!("{}/contents/{}", repo.base_path(), req.path), body)
            .await?;
        Ok(())
    }

    /// Create one commit on `branch` that updates all `files` (Git Data API).
    pub async fn commit_files(
        &self,
        repo: RepoRef<'_>,
        req: CommitFilesRequest<'_>,
    ) -> Result<String> {
        if req.files.is_empty() {
            return Err(bot_err(
                "commit_files requires at least one file".to_string(),
            ));
        }
        let parent_sha = self.branch_sha(repo, req.branch).await?;
        let base_tree = self.commit_tree_sha(repo, &parent_sha).await?;

        let mut tree_items = Vec::with_capacity(req.files.len());
        for file in req.files {
            let blob_sha = self.create_blob(repo, file.bytes).await?;
            tree_items.push(serde_json::json!({
                "path": file.path,
                "mode": "100644",
                "type": "blob",
                "sha": blob_sha,
            }));
        }

        let tree: Value = self
            .http
            .post_json(
                &format!("{}/git/trees", repo.base_path()),
                serde_json::json!({
                    "base_tree": base_tree,
                    "tree": tree_items,
                }),
            )
            .await?;
        let tree_sha = tree
            .get("sha")
            .and_then(|sha| sha.as_str())
            .ok_or_else(|| bot_err("git tree sha missing".to_string()))?
            .to_owned();

        let author_email = bot_author_email();
        let commit: Value = self
            .http
            .post_json(
                &format!("{}/git/commits", repo.base_path()),
                serde_json::json!({
                    "message": req.message,
                    "tree": tree_sha,
                    "parents": [parent_sha],
                    "author": {
                        "name": BOT_COMMIT_NAME,
                        "email": author_email,
                    },
                    "committer": {
                        "name": BOT_COMMIT_NAME,
                        "email": author_email,
                    },
                }),
            )
            .await?;
        let commit_sha = commit
            .get("sha")
            .and_then(|sha| sha.as_str())
            .ok_or_else(|| bot_err("git commit sha missing".to_string()))?
            .to_owned();

        self.http
            .patch_json(
                &format!("{}/git/refs/heads/{}", repo.base_path(), req.branch),
                serde_json::json!({ "sha": commit_sha, "force": false }),
            )
            .await?;
        Ok(commit_sha)
    }

    async fn branch_sha(&self, repo: RepoRef<'_>, branch: &str) -> Result<String> {
        let value: Value = self
            .http
            .get_json(&format!("{}/git/ref/heads/{branch}", repo.base_path()))
            .await?;
        value
            .pointer("/object/sha")
            .and_then(|sha| sha.as_str())
            .map(str::to_owned)
            .ok_or_else(|| bot_err(format!("branch sha missing for {branch}")))
    }

    async fn commit_tree_sha(&self, repo: RepoRef<'_>, commit_sha: &str) -> Result<String> {
        let value: Value = self
            .http
            .get_json(&format!("{}/git/commits/{commit_sha}", repo.base_path()))
            .await?;
        value
            .pointer("/tree/sha")
            .and_then(|sha| sha.as_str())
            .map(str::to_owned)
            .ok_or_else(|| bot_err("commit tree sha missing".to_string()))
    }

    async fn create_blob(&self, repo: RepoRef<'_>, bytes: &[u8]) -> Result<String> {
        let value: Value = self
            .http
            .post_json(
                &format!("{}/git/blobs", repo.base_path()),
                serde_json::json!({
                    "content": STANDARD.encode(bytes),
                    "encoding": "base64",
                }),
            )
            .await?;
        value
            .get("sha")
            .and_then(|sha| sha.as_str())
            .map(str::to_owned)
            .ok_or_else(|| bot_err("blob sha missing".to_string()))
    }

    pub async fn open_pull_request(&self, req: OpenPullRequest<'_>) -> Result<Value> {
        self.http
            .post_json(
                &format!("{}/pulls", req.repo.base_path()),
                serde_json::json!({
                    "title": req.title,
                    "head": req.branch,
                    "base": "main",
                    "body": req.body,
                    "maintainer_can_modify": false,
                }),
            )
            .await
    }

    pub async fn add_labels(&self, issue: IssueRef<'_>, labels: &[&str]) -> Result<()> {
        let _ = self
            .http
            .post_json(
                &format!("{}/issues/{}/labels", issue.repo.base_path(), issue.number),
                serde_json::json!({ "labels": labels }),
            )
            .await;
        Ok(())
    }

    pub async fn find_open_pr(&self, repo: RepoRef<'_>, branch: &str) -> Result<Option<Value>> {
        let pulls: Vec<Value> = self
            .http
            .get_json(&format!(
                "{}/pulls?state=open&head={}:{}",
                repo.base_path(),
                repo.owner,
                branch
            ))
            .await?;
        Ok(pulls.into_iter().next())
    }

    pub async fn post_issue_comment(&self, issue: IssueRef<'_>, body: &str) -> Result<()> {
        self.http
            .post_json(
                &format!(
                    "{}/issues/{}/comments",
                    issue.repo.base_path(),
                    issue.number
                ),
                serde_json::json!({ "body": body }),
            )
            .await?;
        Ok(())
    }

    pub async fn list_issue_comments(&self, issue: IssueRef<'_>) -> Result<Vec<Value>> {
        self.http
            .get_json(&format!(
                "{}/issues/{}/comments",
                issue.repo.base_path(),
                issue.number
            ))
            .await
    }

    pub async fn update_issue_comment(&self, update: IssueCommentUpdate<'_>) -> Result<()> {
        self.http
            .patch_json(
                &format!(
                    "{}/issues/comments/{}",
                    update.repo.base_path(),
                    update.comment_id
                ),
                serde_json::json!({ "body": update.body }),
            )
            .await?;
        Ok(())
    }

    pub async fn create_commit_status(&self, status: CommitStatus<'_>) -> Result<()> {
        self.http
            .post_json(
                &format!("{}/statuses/{}", status.repo.base_path(), status.sha),
                serde_json::json!({
                    "state": status.state,
                    "description": status.description,
                    "context": status.context,
                }),
            )
            .await?;
        Ok(())
    }
}

async fn decode_contents_json(
    http: &HttpClient,
    value: &Value,
    req: &RepoPathRef<'_>,
) -> Result<Option<Vec<u8>>> {
    let encoding = value
        .get("encoding")
        .and_then(|item| item.as_str())
        .unwrap_or("base64");
    let content = value.get("content").and_then(|item| item.as_str());
    if encoding == "none" || content.is_none_or(str::is_empty) {
        return fetch_file_bytes_raw(http, req).await;
    }
    let encoded = content.ok_or_else(|| bot_err("contents missing content".to_string()))?;
    let bytes = STANDARD
        .decode(encoded.replace('\n', ""))
        .map_err(|err| bot_err(format!("contents base64 decode failed: {err}")))?;
    Ok(Some(bytes))
}

async fn fetch_file_bytes_raw(http: &HttpClient, req: &RepoPathRef<'_>) -> Result<Option<Vec<u8>>> {
    let path = contents_path(&req.repo, req.path, req.git_ref);
    let response = http.get_raw(&path, github_raw_accept()).await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(bot_err(format!(
            "contents raw get failed: status {}",
            response.status()
        )));
    }
    response
        .bytes()
        .await
        .map(|bytes| Some(bytes.to_vec()))
        .map_err(|err| bot_err(format!("contents raw read failed: {err}")))
}

fn contents_path(repo: &RepoRef<'_>, path: &str, git_ref: &str) -> String {
    format!("{}/contents/{path}?ref={git_ref}", repo.base_path())
}

fn find_benchmark_artifact_id(artifacts: &Value) -> Result<i64> {
    let items = artifacts
        .get("artifacts")
        .and_then(|value| value.as_array())
        .ok_or_else(|| bot_err("artifacts list missing".to_string()))?;
    let artifact = items
        .iter()
        .find(|item| item.get("name").and_then(|v| v.as_str()) == Some("benchmark-data"))
        .ok_or_else(|| bot_err("benchmark-data artifact not found".to_string()))?;
    artifact
        .get("id")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| bot_err("artifact id missing".to_string()))
}

pub fn split_repo(full_name: &str) -> Result<(&str, &str)> {
    let (owner, repo) = full_name
        .split_once('/')
        .ok_or_else(|| bot_err(format!("invalid repo: {full_name}")))?;
    Ok((owner, repo))
}

#[cfg(test)]
mod message_tests {
    use super::bot_commit_message;

    #[test]
    fn commit_message_includes_signed_off_by() {
        std::env::set_var("APP_ID", "424242");
        let message = bot_commit_message("chore(bench): benchmark data update (run #1)");
        assert!(message.contains("Signed-off-by: ibex-harness-benchmark[bot]"));
        assert!(message.contains("424242+ibex-harness-benchmark[bot]@users.noreply.github.com"));
        std::env::remove_var("APP_ID");
    }
}
