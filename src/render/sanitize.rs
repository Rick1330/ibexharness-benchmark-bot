use regex::Regex;

pub fn sanitize_sha(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "unknown".to_string();
    };
    let trimmed = value.trim();
    let re = Regex::new(r"^[0-9a-f]{7,40}$").expect("sha regex");
    if re.is_match(trimmed) {
        trimmed.to_lowercase()
    } else {
        "invalid".to_string()
    }
}

pub fn sanitize_branch(value: &str) -> String {
    let trimmed = value.trim();
    let re = Regex::new(r"^[a-zA-Z0-9._/-]{1,200}$").expect("branch regex");
    if re.is_match(trimmed) {
        trimmed.to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn escape_cell(value: Option<&str>) -> String {
    match value {
        None => "—".to_string(),
        Some(text) => text.replace('|', "\\|").replace(['\n', '\r'], " "),
    }
}

pub fn format_number(value: Option<f64>) -> String {
    format_number_precise(value, 3)
}

pub fn format_number_precise(value: Option<f64>, digits: usize) -> String {
    match value {
        Some(number) if !number.is_nan() => format!("{number:.digits$}"),
        _ => "—".to_string(),
    }
}

pub fn format_delta(delta: Option<f64>) -> String {
    match delta {
        Some(number) if !number.is_nan() => {
            let sign = if number > 0.0 { "+" } else { "" };
            format!("{sign}{number:.1}%")
        }
        _ => "n/a".to_string(),
    }
}

pub fn status_emoji(status: &str) -> &'static str {
    match status {
        "pass" => "✅",
        "regression" => "⚠️",
        "fail" => "❌",
        _ => "❔",
    }
}
