use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;

use crate::error::{bot_err, Result};

#[derive(Serialize)]
struct Claims {
    iat: u64,
    exp: u64,
    iss: String,
}

pub fn installation_token(app_id: &str, private_key_pem: &str, installation_id: &str) -> Result<String> {
    let jwt = create_app_jwt(app_id, private_key_pem)?;
    let url = format!("https://api.github.com/app/installations/{installation_id}/access_tokens");
    let response = reqwest::blocking::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "ibexharness-benchmark-bot")
        .send()
        .map_err(|err| bot_err(format!("installation token request failed: {err}")))?;

    if !response.status().is_success() {
        return Err(bot_err(format!(
            "installation token request failed: {}",
            response.text().unwrap_or_default()
        )));
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|err| bot_err(format!("installation token decode failed: {err}")))?;
    body.get("token")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .ok_or_else(|| bot_err("installation token missing from response".to_string()))
}

fn create_app_jwt(app_id: &str, private_key_pem: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| bot_err(format!("clock error: {err}")))?
        .as_secs();
    let claims = Claims {
        iat: now.saturating_sub(60),
        exp: now + 600,
        iss: app_id.to_string(),
    };
    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|err| bot_err(format!("invalid app private key: {err}")))?;
    encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|err| bot_err(format!("jwt encode failed: {err}")))
}
