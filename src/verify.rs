use regex::Regex;

use serde_json::Value;

use crate::error::{bot_err, Result};
use crate::github::GitHubClient;
use crate::model::{DispatchPayload, WorkflowRun};

const EXPECTED_WORKFLOW: &str = "Benchmarks";
const EXPECTED_BRANCH: &str = "main";
const EXPECTED_CONCLUSION: &str = "success";

pub fn parse_payload_json(raw: &str) -> Result<DispatchPayload> {
    let value: Value =
        serde_json::from_str(raw).map_err(|err| bot_err(format!("invalid dispatch payload: {err}")))?;
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

pub async fn verify_dispatch(
    client: &GitHubClient,
    repo_full: &str,
    payload: &DispatchPayload,
) -> Result<WorkflowRun> {
    let head_sha = require_sha(&payload.head_sha)?;
    let (owner, repo) = crate::github::split_repo(repo_full)?;
    let run = client
        .get_workflow_run(owner, repo, payload.run_id)
        .await?;

    if run.conclusion.as_deref() != Some(EXPECTED_CONCLUSION) {
        return Err(bot_err(format!(
            "run conclusion must be {EXPECTED_CONCLUSION}"
        )));
    }
    if run.head_branch.as_deref() != Some(EXPECTED_BRANCH) {
        return Err(bot_err(format!("run head_branch must be {EXPECTED_BRANCH}")));
    }
    if run.head_sha.as_deref().map(str::to_lowercase).as_deref() != Some(head_sha.as_str()) {
        return Err(bot_err("head_sha mismatch with Actions API".to_string()));
    }
    if run.run_number != Some(payload.run_number) {
        return Err(bot_err("run_number mismatch with Actions API".to_string()));
    }
    let name = run.name.as_deref().unwrap_or("");
    if name != EXPECTED_WORKFLOW && !name.to_lowercase().contains("benchmark") {
        return Err(bot_err(format!("workflow name must be {EXPECTED_WORKFLOW}")));
    }
    Ok(run)
}
