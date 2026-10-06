use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_LIMIT_BYTES: u64 = 100 * 1024 * 1024 * 1024;

/// Best-effort post-run cleanup for persistent developer and self-hosted-runner checkouts.
///
/// `code test` can leave a very large incremental build tree behind. A persistent runner
/// should keep that cache while it is useful, but once `target/` grows beyond 100 GiB it is
/// cheaper and safer to discard it than to let one repository consume the machine's disk.
/// Cleanup is deliberately best-effort: a test failure must stay the reported failure even
/// if measuring or deleting the cache also fails.
pub(crate) fn cleanup_target_if_oversized() {
    let result = (|| -> io::Result<()> {
        let root = repository_root()?;
        let target = root.join("target");
        if !target.exists() || !exceeds_limit(&target, TARGET_LIMIT_BYTES)? {
            return Ok(());
        }

        println!(
            "target/ exceeded 100 GiB; removing {}",
            target.display()
        );
        fs::remove_dir_all(&target)?;
        Ok(())
    })();

    if let Err(error) = result {
        eprintln!("Warning: could not inspect/clean target/: {error}");
    }
}

fn repository_root() -> io::Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git rev-parse --show-toplevel failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let root = String::from_utf8(output.stdout).map_err(io::Error::other)?;
    Ok(PathBuf::from(root.trim_end_matches(['\r', '\n'])))
}

fn exceeds_limit(path: &Path, limit: u64) -> io::Result<bool> {
    let mut bytes = 0u64;
    let mut pending = vec![path.to_path_buf()];

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.path().symlink_metadata()?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
                continue;
            }
            if file_type.is_file() {
                bytes = bytes.saturating_add(metadata.len());
                if bytes > limit {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn size_check_is_strictly_greater_than_limit() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("artifact");
        let mut handle = fs::File::create(&file).expect("create artifact");
        handle.write_all(b"1234").expect("write artifact");

        assert!(!exceeds_limit(temp.path(), 4).expect("measure exact limit"));
        assert!(exceeds_limit(temp.path(), 3).expect("measure over limit"));
    }

    #[cfg(unix)]
    #[test]
    fn size_check_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let mut handle = fs::File::create(outside.path().join("large")).expect("create large");
        handle.write_all(b"12345678").expect("write large");
        symlink(outside.path(), temp.path().join("external")).expect("create symlink");

        assert!(!exceeds_limit(temp.path(), 1).expect("symlink must be ignored"));
    }
}
