//! Build provenance for the released binary.
//!
//! The harness consumes this bot as a pin: a full commit SHA
//! (`BENCHMARK_BOT_SHA`), a release tag (`bot-<7-char-sha>`) and a `sha256`
//! file. A binary that cannot name the commit it was built from makes pin
//! drift invisible, so the release workflow compiles with [`BUILD_SHA_ENV`]
//! set from `git rev-parse HEAD` and then refuses to publish unless the
//! binary reports that same commit.

use std::sync::OnceLock;

/// Compile-time environment variable carrying the commit the binary was built from.
pub const BUILD_SHA_ENV: &str = "IBEX_BENCHMARK_BOT_BUILD_SHA";

/// Reported when the binary was built without [`BUILD_SHA_ENV`], as a plain
/// local `cargo build` is.
pub const UNKNOWN_BUILD_SHA: &str = "unknown";

/// The commit this binary was compiled from, or [`UNKNOWN_BUILD_SHA`].
pub fn embedded_build_sha() -> &'static str {
    select_build_sha(option_env!("IBEX_BENCHMARK_BOT_BUILD_SHA"))
}

/// A blank build SHA means "not recorded", never an empty label.
fn select_build_sha(raw: Option<&'static str>) -> &'static str {
    match raw {
        Some(sha) if !sha.trim().is_empty() => sha.trim(),
        _ => UNKNOWN_BUILD_SHA,
    }
}

/// `--version` text: crate version plus the embedded build commit.
pub fn version_string() -> &'static str {
    static VERSION: OnceLock<&'static str> = OnceLock::new();
    // clap takes `&'static str`. The process leaks this once and exits.
    VERSION.get_or_init(|| {
        Box::leak(
            format!("{}+{}", env!("CARGO_PKG_VERSION"), embedded_build_sha()).into_boxed_str(),
        )
    })
}

/// `build-info` output: one `key=value` line per field, so a shell consumer can
/// read it with `sed` and needs no JSON parser.
pub fn format_build_info(name: &str, version: &str, build_sha: &str) -> String {
    format!("name={name}\nversion={version}\nbuild_sha={build_sha}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0c8b404bd5a5f5b4b0d8e2e1b0d7e4e0f0a1b2c3";

    #[test]
    fn build_info_lists_every_field_on_its_own_line() {
        let out = format_build_info("ibex-benchmark-bot", "0.1.0", SHA);
        assert_eq!(
            out,
            format!("name=ibex-benchmark-bot\nversion=0.1.0\nbuild_sha={SHA}\n")
        );
    }

    #[test]
    fn build_info_reports_unknown_when_sha_absent() {
        let out = format_build_info("ibex-benchmark-bot", "0.1.0", UNKNOWN_BUILD_SHA);
        assert!(out.ends_with("\nbuild_sha=unknown\n"));
    }

    #[test]
    fn missing_or_blank_sha_falls_back_to_unknown() {
        assert_eq!(select_build_sha(None), UNKNOWN_BUILD_SHA);
        assert_eq!(select_build_sha(Some("")), UNKNOWN_BUILD_SHA);
        assert_eq!(select_build_sha(Some("  \n")), UNKNOWN_BUILD_SHA);
    }

    #[test]
    fn recorded_sha_is_kept_verbatim() {
        assert_eq!(select_build_sha(Some(SHA)), SHA);
        assert_eq!(select_build_sha(Some(" deadbeef\n")), "deadbeef");
    }

    #[test]
    fn embedded_sha_is_never_blank() {
        // The concrete value depends on the compile-time environment, so assert
        // the contract the release guard relies on instead.
        assert!(!embedded_build_sha().is_empty());
        assert!(!embedded_build_sha().contains('\n'));
    }

    #[test]
    fn version_string_carries_the_embedded_sha() {
        let version = version_string();
        assert!(version.starts_with(env!("CARGO_PKG_VERSION")));
        assert!(version.ends_with(embedded_build_sha()));
        assert_eq!(version_string(), version);
    }
}
