//! Files persisted across restarts in `~/.cache/home-control-panel`.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub fn path(name: &str) -> Option<PathBuf> {
    std::env::home_dir().map(|home| home.join(".cache/home-control-panel").join(name))
}

/// Write the file only readable by the user, as caches hold private data.
///
/// # Errors
/// Returns an error if the directory or file can't be written.
pub fn write(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write to a temporary file first, so a crash never leaves a half written file
    let temporary = path.with_extension("tmp");
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?
        .write_all(contents)?;

    std::fs::rename(&temporary, path)
}
