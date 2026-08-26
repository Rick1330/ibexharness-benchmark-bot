//! HNSW / memory benchmark artifact helpers (separate from proxy schema v1).

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use zip::read::ZipArchive;

use crate::error::{bot_err, Result};

const MAX_ZIP_BYTES: usize = 10 * 1024 * 1024;
const MAX_ENTRIES: usize = 32;
const MAX_UNCOMPRESSED: u64 = 20 * 1024 * 1024;
const READ_CHUNK: usize = 8192;
pub const HNSW_JSON_NAME: &str = "hnsw-benchmark-data.json";

pub struct ExtractedHnswArtifact {
    _temp_dir: tempfile::TempDir,
    pub json_path: PathBuf,
}

pub fn extract_hnsw_artifact_zip(bytes: &[u8]) -> Result<ExtractedHnswArtifact> {
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
        if file_name != HNSW_JSON_NAME {
            continue;
        }

        let dest = root.join(file_name);
        write_zip_entry(&mut entry, &dest, &mut total_uncompressed)?;
        json_path = Some(dest);
    }

    let json_path =
        json_path.ok_or_else(|| bot_err("hnsw-benchmark-data.json not in artifact".to_string()))?;
    Ok(ExtractedHnswArtifact {
        _temp_dir: dir,
        json_path,
    })
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

fn write_zip_entry<R: Read>(
    entry: &mut R,
    dest: &Path,
    total_uncompressed: &mut u64,
) -> Result<()> {
    let mut file = fs::File::create(dest)
        .map_err(|err| bot_err(format!("write {} failed: {err}", dest.display())))?;
    let mut buffer = [0u8; READ_CHUNK];
    loop {
        let read = entry
            .read(&mut buffer)
            .map_err(|err| bot_err(format!("zip extract failed: {err}")))?;
        if read == 0 {
            break;
        }
        *total_uncompressed = total_uncompressed.saturating_add(read as u64);
        if *total_uncompressed > MAX_UNCOMPRESSED {
            return Err(bot_err(format!(
                "artifact zip uncompressed size exceeds {MAX_UNCOMPRESSED} bytes"
            )));
        }
        file.write_all(&buffer[..read])
            .map_err(|err| bot_err(format!("zip extract failed: {err}")))?;
    }
    Ok(())
}
