use regex::Regex;

/// HTML anchor used to find and update the bot's sticky PR comment.
pub const COMMENT_MARKER: &str = "<!-- IBEX_BOT_COMMENT -->";

pub fn sanitize_sha(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "unknown".to_string();
    };
    let trimmed = strip_control_chars(value.trim());
    let re = Regex::new(r"^[0-9a-f]{7,40}$").expect("sha regex");
    if re.is_match(&trimmed) {
        trimmed.to_lowercase()
    } else {
        "invalid".to_string()
    }
}

pub fn sanitize_branch(value: &str) -> String {
    let trimmed = strip_control_chars(value.trim());
    let re = Regex::new(r"^[a-zA-Z0-9._/-]{1,200}$").expect("branch regex");
    if re.is_match(&trimmed) {
        trimmed.to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn sanitize_gate_name(value: Option<&str>) -> String {
    let Some(raw) = value else {
        return "—".to_string();
    };
    let trimmed = strip_control_chars(raw.trim());
    if trimmed.is_empty() {
        return "—".to_string();
    }
    let allowed = Regex::new(r"^[a-zA-Z0-9 %()._/-]{1,120}$").expect("gate name regex");
    if allowed.is_match(&trimmed) {
        escape_markdown_inline(&trimmed)
    } else {
        "invalid".to_string()
    }
}

pub fn escape_cell(value: Option<&str>) -> String {
    match value {
        None => "—".to_string(),
        Some(text) => escape_markdown_inline(&strip_control_chars(text)),
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

/// Latency delta with trend emoji (lower p99 is better).
pub fn format_latency_delta(delta: Option<f64>) -> String {
    match delta {
        Some(number) if !number.is_nan() => {
            let sign = if number > 0.0 { "+" } else { "" };
            let arrow = if number < 0.0 {
                "📉"
            } else if number > 0.0 {
                "📈"
            } else {
                "➖"
            };
            format!("{arrow} **{sign}{number:.1}%**")
        }
        _ => "n/a".to_string(),
    }
}

/// Throughput delta derived from latency regression (higher throughput is better).
pub fn format_throughput_delta(latency_regression_pct: Option<f64>) -> String {
    match latency_regression_pct {
        Some(number) if !number.is_nan() => {
            let throughput_delta = -number;
            let sign = if throughput_delta > 0.0 { "+" } else { "" };
            let arrow = if throughput_delta > 0.0 {
                "📈"
            } else if throughput_delta < 0.0 {
                "📉"
            } else {
                "➖"
            };
            format!("{arrow} **{sign}{throughput_delta:.1}%**")
        }
        _ => "n/a".to_string(),
    }
}

pub fn format_throughput(req_per_s: Option<f64>) -> String {
    match req_per_s {
        Some(number) if !number.is_nan() => {
            if number >= 1000.0 {
                format!("{:.1}k req/s", number / 1000.0)
            } else {
                format!("{number:.1} req/s")
            }
        }
        _ => "—".to_string(),
    }
}

pub fn visual_bar_filled(filled: usize, width: usize, label: &str) -> String {
    let filled = filled.min(width);
    format!(
        "`{}{} ` {}",
        "█".repeat(filled),
        "░".repeat(width - filled),
        label
    )
}

pub fn latency_visual_bar(p99_ms: Option<f64>, sla_ms: f64, width: usize) -> String {
    let p99 = p99_ms.unwrap_or(sla_ms);
    let ratio = (p99 / sla_ms).clamp(0.0, 1.0);
    let filled = ((1.0 - ratio) * width as f64).round() as usize;
    let label = if ratio < 0.25 {
        "(Fast)"
    } else if ratio < 0.75 {
        "(OK)"
    } else {
        "(Slow)"
    };
    visual_bar_filled(filled, width, label)
}

pub fn throughput_visual_bar(req_per_s: Option<f64>, scale: f64, width: usize) -> String {
    let rps = req_per_s.unwrap_or(0.0);
    let filled = ((rps / scale) * width as f64).round() as usize;
    let label = if rps >= scale * 0.8 {
        "(Max)"
    } else if rps >= scale * 0.4 {
        "(Good)"
    } else {
        "(Low)"
    };
    visual_bar_filled(filled.min(width), width, label)
}

pub fn status_emoji(status: &str) -> &'static str {
    match status {
        "pass" => "✅",
        "regression" => "⚠️",
        "fail" => "❌",
        _ => "❔",
    }
}

fn strip_control_chars(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\t')
        .collect()
}

fn escape_markdown_inline(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '\n' | '\r' => out.push(' '),
            '[' | ']' | '<' | '>' | '`' => {
                out.push('`');
                out.push(ch);
                out.push('`');
            }
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_backslash_before_pipe() {
        let value = escape_cell(Some(r"\|not-a-separator"));
        assert!(value.contains("\\\\"));
    }

    #[test]
    fn strips_control_chars_from_cells() {
        let value = escape_cell(Some("hello\u{0007}world"));
        assert!(!value.contains('\u{0007}'));
    }

    #[test]
    fn rejects_phishing_gate_name() {
        let value = sanitize_gate_name(Some("[click](https://evil.example)"));
        assert_eq!(value, "invalid");
    }
}
