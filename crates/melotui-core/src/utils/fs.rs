//! Writing files so that a crash or a full disk never leaves one half
//! written.

use std::io::Write;
use std::path::Path;

/// Writes `contents` to `path` all at once: into a temporary file beside it,
/// flushed to disk, then renamed over it. Whatever happens part way, `path`
/// holds either what it held before or all of `contents`.
pub fn write_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    if let Some(dir) = dir {
        std::fs::create_dir_all(dir)?;
    }
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("a path to write to has no file name"))?;
    let mut temporary = path.to_path_buf();
    temporary.set_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let written = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_whole_file_and_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("session.toml");
        write_atomically(&path, b"first").unwrap();
        write_atomically(&path, b"second, longer").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second, longer");
        let left: Vec<_> = std::fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(left.len(), 1, "a temporary file was left behind");
    }

    #[test]
    fn a_failed_write_keeps_what_was_there() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.toml");
        std::fs::write(&path, b"kept").unwrap();
        // A directory where the file should be renamed to makes the rename
        // fail, after the temporary file was written.
        let blocked = dir.path().join("blocked");
        std::fs::create_dir_all(blocked.join("x")).unwrap();
        assert!(write_atomically(&blocked, b"new").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"kept");
        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| !n.ends_with(".tmp")), "{names:?}");
    }
}
