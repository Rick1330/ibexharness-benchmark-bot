use regex::Regex;

use serde_json::Value;

use crate::config::{
    EXPECTED_HNSW_WORKFLOW_NAME, EXPECTED_HNSW_WORKFLOW_PATH, EXPECTED_WORKFLOW_NAME,
    EXPECTED_WORKFLOW_PATH,
};
use crate::error::{bot_err, Result};
use crate::github::{GitHubClient, RepoRef};
use crate::model::{DispatchPayload, WorkflowRun};

const EXPECTED_BRANCH: &str = "main";
const EXPECTED_CONCLUSION: &str = "success";

pub fn parse_payload_json(raw: &str) -> Result<DispatchPayload> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| bot_err(format!("invalid dispatch payload: {err}")))?;
    let run_id = parse_i64_field(&value, "run_id")?;
    let run_number = parse_i64_field(&value, "run_number")?;
    let head_sha = value
        .get("head_sha")
        .and_then(|item| item.as_str())
        .ok_or_else(|| bot_err("head_sha missing".to_string()))?
        .to_string();
    Ok(DispatchPayload {
        run_id,
        head_sha,
        run_number,
    })
}

fn parse_i64_field(value: &Value, field: &str) -> Result<i64> {
    let item = value
        .get(field)
        .ok_or_else(|| bot_err(format!("{field} missing")))?;
    if let Some(number) = item.as_i64() {
        return Ok(number);
    }
    if let Some(text) = item.as_str() {
        return text
            .parse::<i64>()
            .map_err(|_| bot_err(format!("{field} must be an integer")));
    }
    Err(bot_err(format!("{field} must be an integer")))
}

pub fn require_sha(value: &str) -> Result<String> {
    let cleaned = value.trim().to_lowercase();
    let re = Regex::new(r"^[0-9a-f]{7,40}$").expect("sha regex");
    if !re.is_match(&cleaned) {
        return Err(bot_err("head_sha must be hexadecimal".to_string()));
    }
    Ok(cleaned)
}

pub fn verify_workflow_run(run: &WorkflowRun) -> Result<()> {
    verify_workflow_run_named(run, EXPECTED_WORKFLOW_NAME, EXPECTED_WORKFLOW_PATH)
}

pub fn verify_hnsw_workflow_run(run: &WorkflowRun) -> Result<()> {
    verify_workflow_run_named(
        run,
        EXPECTED_HNSW_WORKFLOW_NAME,
        EXPECTED_HNSW_WORKFLOW_PATH,
    )
}

fn verify_workflow_run_named(run: &WorkflowRun, name: &str, path: &str) -> Result<()> {
    if run.conclusion.as_deref() != Some(EXPECTED_CONCLUSION) {
        return Err(bot_err(format!(
            "run conclusion must be {EXPECTED_CONCLUSION}"
        )));
    }
    if run.head_branch.as_deref() != Some(EXPECTED_BRANCH) {
        return Err(bot_err(format!(
            "run head_branch must be {EXPECTED_BRANCH}"
        )));
    }
    if run.name.as_deref() != Some(name) {
        return Err(bot_err(format!("workflow name must be {name}")));
    }
    if run.path.as_deref() != Some(path) {
        return Err(bot_err(format!("workflow path must be {path}")));
    }
    Ok(())
}

pub async fn verify_dispatch(
    client: &GitHubClient,
    repo_full: &str,
    payload: &DispatchPayload,
) -> Result<WorkflowRun> {
    verify_dispatch_named(
        client,
        repo_full,
        payload,
        EXPECTED_WORKFLOW_NAME,
        EXPECTED_WORKFLOW_PATH,
    )
    .await
}

pub async fn verify_hnsw_dispatch(
    client: &GitHubClient,
    repo_full: &str,
    payload: &DispatchPayload,
) -> Result<WorkflowRun> {
    verify_dispatch_named(
        client,
        repo_full,
        payload,
        EXPECTED_HNSW_WORKFLOW_NAME,
        EXPECTED_HNSW_WORKFLOW_PATH,
    )
    .await
}

async fn verify_dispatch_named(
    client: &GitHubClient,
    repo_full: &str,
    payload: &DispatchPayload,
    workflow_name: &str,
    workflow_path: &str,
) -> Result<WorkflowRun> {
    let head_sha = require_sha(&payload.head_sha)?;
    let (owner, repo) = crate::github::split_repo(repo_full)?;
    let repo_ref = RepoRef::new(owner, repo);
    let run = client.get_workflow_run(repo_ref, payload.run_id).await?;

    verify_workflow_run_named(&run, workflow_name, workflow_path)?;

    if run.head_sha.as_deref().map(str::to_lowercase).as_deref() != Some(head_sha.as_str()) {
        return Err(bot_err("head_sha mismatch with Actions API".to_string()));
    }
    if run.run_number != Some(payload.run_number) {
        return Err(bot_err("run_number mismatch with Actions API".to_string()));
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WorkflowRun;

    fn valid_run() -> WorkflowRun {
        WorkflowRun {
            conclusion: Some("success".to_string()),
            head_branch: Some("main".to_string()),
            head_sha: Some("abc1234".to_string()),
            run_number: Some(42),
            name: Some(EXPECTED_WORKFLOW_NAME.to_string()),
            path: Some(EXPECTED_WORKFLOW_PATH.to_string()),
            html_url: Some("https://github.com/o/r/actions/runs/1".to_string()),
        }
    }

    #[test]
    fn rejects_fuzzy_workflow_name() {
        let mut run = valid_run();
        run.name = Some("evil-benchmark".to_string());
        assert!(verify_workflow_run(&run).is_err());
    }

    #[test]
    fn rejects_wrong_workflow_path() {
        let mut run = valid_run();
        run.path = Some(".github/workflows/evil.yml".to_string());
        assert!(verify_workflow_run(&run).is_err());
    }

    #[test]
    fn rejects_non_success_conclusion() {
        let mut run = valid_run();
        run.conclusion = Some("failure".to_string());
        assert!(verify_workflow_run(&run).is_err());
    }

    #[test]
    fn parse_payload_requires_fields() {
        assert!(parse_payload_json(r#"{"run_id":1}"#).is_err());
    }

    #[test]
    fn require_sha_rejects_invalid() {
        assert!(require_sha("not-hex").is_err());
    }
}
