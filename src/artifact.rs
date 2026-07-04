use std::fs;
use std::io::{copy, Cursor, Read};
use std::path::{Component, Path, PathBuf};

use zip::read::ZipArchive;

use crate::error::{bot_err, Result};

const MAX_ZIP_BYTES: usize = 10 * 1024 * 1024;
const MAX_ENTRIES: usize = 32;
const MAX_UNCOMPRESSED: u64 = 20 * 1024 * 1024;
pub const JSON_NAME: &str = "benchmark-data.json";
pub const BADGE_NAME: &str = "badge.svg";

pub struct ExtractedArtifact {
    pub json_path: PathBuf,
    pub badge_path: PathBuf,
}

pub fn extract_artifact_zip(bytes: &[u8]) -> Result<ExtractedArtifact> {
    if bytes.len() > MAX_ZIP_BYTES {
        return Err(bot_err(format!(
            "artifact zip exceeds {MAX_ZIP_BYTES} bytes"
        )));
    }

    let dir = tempfile::tempdir().map_err(|err| bot_err(format!("tempdir failed: {err}")))?;
    let root = dir.path().to_path_buf();
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|err| bot_err(format!("zip open failed: {err}")))?;

    if archive.len() > MAX_ENTRIES {
        return Err(bot_err(format!(
            "artifact zip exceeds {MAX_ENTRIES} entries"
        )));
    }

    let mut json_path = None;
    let mut badge_path = None;
    let mut total_uncompressed = 0u64;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| bot_err(format!("zip entry read failed: {err}")))?;
        let Some(safe_name) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            return Err(bot_err("zip entry has unsafe path".to_string()));
        };
        reject_unsafe_zip_path(&safe_name)?;

        let file_name = safe_name
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if file_name != JSON_NAME && file_name != BADGE_NAME {
            continue;
        }

        total_uncompressed = total_uncompressed.saturating_add(entry.size());
        if total_uncompressed > MAX_UNCOMPRESSED {
            return Err(bot_err(format!(
                "artifact zip uncompressed size exceeds {MAX_UNCOMPRESSED} bytes"
            )));
        }

        let dest = root.join(file_name);
        write_zip_entry(&mut entry, &dest)?;
        match file_name {
            JSON_NAME => json_path = Some(dest),
            BADGE_NAME => badge_path = Some(dest),
            _ => {}
        }
    }

    let json_path =
        json_path.ok_or_else(|| bot_err("benchmark-data.json not in artifact".to_string()))?;
    let badge_path = badge_path.ok_or_else(|| bot_err("badge.svg not in artifact".to_string()))?;
    Ok(ExtractedArtifact {
        json_path,
        badge_path,
    })
}

pub fn validate_badge_svg(bytes: &[u8]) -> Result<()> {
    if bytes.len() > 64 * 1024 {
        return Err(bot_err("badge.svg exceeds 64 KiB".to_string()));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| bot_err("badge.svg must be utf-8".to_string()))?;
    let lower = text.to_ascii_lowercase();
    let forbidden = [
        "<script",
        "onload=",
        "onerror=",
        "onclick=",
        "javascript:",
        "<foreignobject",
        "xlink:href=\"http",
        "href=\"http",
    ];
    for needle in forbidden {
        if lower.contains(needle) {
            return Err(bot_err(format!(
                "badge.svg contains forbidden pattern: {needle}"
            )));
        }
    }
    if !lower.contains("<svg") {
        return Err(bot_err("badge.svg must contain svg root".to_string()));
    }
    Ok(())
}

fn reject_unsafe_zip_path(path: &Path) -> Result<()> {
    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(bot_err("zip entry path not allowed".to_string()));
            }
            _ => {}
        }
    }
    Ok(())
}

fn write_zip_entry<R: Read>(entry: &mut R, dest: &Path) -> Result<()> {
    let mut file = fs::File::create(dest)
        .map_err(|err| bot_err(format!("write {} failed: {err}", dest.display())))?;
    copy(entry, &mut file).map_err(|err| bot_err(format!("zip extract failed: {err}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_script_in_badge() {
        assert!(validate_badge_svg(br#"<svg><script>alert(1)</script></svg>"#).is_err());
    }

    #[test]
    fn accepts_minimal_badge() {
        assert!(validate_badge_svg(br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#).is_ok());
    }
}
