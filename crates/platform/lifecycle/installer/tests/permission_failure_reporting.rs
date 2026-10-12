#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only integration file: unwrap/expect is the intended failure signal, and the \
              assertions compare the error the installer actually produced rather than trusting the \
              call to fail in a particular way."
)]
//! How the installer reports a permission failure (#635, then fixed by #897).
//!
//! `InstallerError` declares a `Permission` variant that used to be **unreachable**:
//! `From<std::io::Error>` mapped every kind, `PermissionDenied` included, onto `FileSystem`, and
//! nothing anywhere constructed `Permission`.
//!
//! This file originally pinned that loss so it could not be fixed silently. #897 changed the
//! conversion to branch on `error.kind()`, so the three assertions that recorded the old mapping are
//! now **deliberately inverted** rather than deleted or loosened -- each still fails loudly if the
//! mapping regresses to "every io error is FileSystem":
//!
//! | test | before #897 | after #897 |
//! | --- | --- | --- |
//! | `a_permission_denied_io_error_is_reported_as_permission` | `FileSystem`, `Permission` unreachable | `Permission`, OS message preserved |
//! | `the_conversion_distinguishes_not_found_from_permission_denied` | denied and missing report the same kind | `Permission` vs `FileSystem` |
//! | `writing_into_an_unwritable_directory_reports_permission` | `FileSystem` end to end | `Permission` end to end |
//!
//! `the_permission_variant_exists_and_formats_distinctly_from_filesystem` is unchanged, and
//! `only_permission_denied_changes_the_mapping` was added so "every other kind keeps `FileSystem`" is
//! asserted rather than assumed.
//!
//! Nothing in this file installs anything, runs npm/git, or touches a real system directory: the
//! only writes go to a uniquely named subdirectory of `std::env::temp_dir()`.

use burncloud_installer::error::InstallerError;
use std::path::{Path, PathBuf};

/// A unique directory under the temporary directory, removed by the owning guard.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "bc_installer_perm_{}_{}_{}",
            tag,
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&dir).expect("the scratch directory must be creatable");
        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Restore the mode first: a read-only directory cannot have its contents removed.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&self.dir) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                // Explicitly drop the Result so a failed restore stays non-fatal without hiding a
                // `#[must_use]` value behind a non-binding `let` (the workspace lints deny that).
                drop(std::fs::set_permissions(&self.dir, perms));
            }
        }
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

/// Make a directory read-only, or report that this platform cannot.
#[cfg(unix)]
fn make_read_only(dir: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(dir) {
        Ok(meta) => {
            let mut perms = meta.permissions();
            perms.set_mode(0o500);
            std::fs::set_permissions(dir, perms).is_ok()
        }
        Err(_) => false,
    }
}

/// A `PermissionDenied` io error is reported as `Permission`, and the OS message survives.
///
/// The counterpart of the pin that used to assert `FileSystem`: the defect it recorded was fixed by
/// #897, so the assertion was inverted deliberately. It is still a strong assertion about what
/// happens -- a caller matching `Permission` now succeeds, and one matching `FileSystem` does not.
#[test]
fn a_permission_denied_io_error_is_reported_as_permission() {
    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied by test");
    let converted: InstallerError = io_error.into();

    match &converted {
        InstallerError::Permission(message) => assert!(
            message.contains("denied by test"),
            "the OS message must survive the conversion: {message}"
        ),
        other => panic!("expected Permission for a PermissionDenied io error, got {other:?}"),
    }

    assert!(
        !matches!(converted, InstallerError::FileSystem(_)),
        "a permission failure must not be reported as a generic file system error, or a caller \
         cannot tell 'retry with better rights' from 'the file is absent'"
    );
}

/// The conversion distinguishes the *kind*, which is the part a caller acts on.
#[test]
fn the_conversion_distinguishes_not_found_from_permission_denied() {
    let denied: InstallerError =
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied").into();
    let missing: InstallerError =
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing").into();

    let kind_of = |e: &InstallerError| match e {
        InstallerError::FileSystem(_) => "FileSystem",
        InstallerError::Permission(_) => "Permission",
        _ => "other",
    };

    assert_eq!(
        kind_of(&denied),
        "Permission",
        "a permission failure must keep its own kind"
    );
    assert_eq!(
        kind_of(&missing),
        "FileSystem",
        "a non-permission kind must keep the mapping it had before #897"
    );
    assert_ne!(
        kind_of(&denied),
        kind_of(&missing),
        "two very different failures must not produce the same error kind"
    );
}

/// `PermissionDenied` is the **only** kind whose mapping changed.
///
/// Asserted rather than assumed: the fix routes one kind and leaves the rest alone, so a future
/// change that widened it (or narrowed it) would be caught here.
#[test]
fn only_permission_denied_changes_the_mapping() {
    let kinds = [
        std::io::ErrorKind::NotFound,
        std::io::ErrorKind::AlreadyExists,
        std::io::ErrorKind::InvalidData,
        std::io::ErrorKind::UnexpectedEof,
        std::io::ErrorKind::TimedOut,
    ];

    for kind in kinds {
        let converted: InstallerError =
            std::io::Error::new(kind, format!("{kind:?} by test")).into();
        match &converted {
            InstallerError::FileSystem(message) => assert!(
                message.contains("by test"),
                "the OS message must survive for {kind:?}: {message}"
            ),
            other => panic!("expected FileSystem for {kind:?}, got {other:?}"),
        }
    }
}

/// The `Permission` variant still exists, formats distinctly, and is distinguishable.
///
/// Included so the "unreachable" claim above is about construction and not about the enum being
/// absent: if a future change starts producing `Permission`, the two tests together show the
/// transition precisely.
#[test]
fn the_permission_variant_exists_and_formats_distinctly_from_filesystem() {
    let permission = InstallerError::Permission("cannot write target".to_string());
    let filesystem = InstallerError::FileSystem("cannot write target".to_string());

    let permission_text = permission.to_string();
    let filesystem_text = filesystem.to_string();

    assert!(
        permission_text.starts_with("Permission error"),
        "the variant must keep a distinct prefix: {permission_text}"
    );
    // The sibling's prefix is "File system error" (two words), not "FileSystem error" -- the enum
    // variant name and its human text differ, which is worth pinning rather than assuming.
    assert!(
        filesystem_text.starts_with("File system error"),
        "the sibling variant must keep its own prefix: {filesystem_text}"
    );
    assert_ne!(
        permission_text, filesystem_text,
        "the two kinds must not be textually identical, or a structured caller gains nothing"
    );
}

/// End to end: an actual write into a real read-only directory.
///
/// A temp target made unwritable, then asserted on the error that reaches the caller -- which is now
/// `Permission`, the kind the API advertises. Where the platform cannot make a directory unwritable
/// (for instance a root user), the test **registers the gap by panicking with a reason** rather than
/// silently passing: a test that cannot set up its own premise must not report success.
#[cfg(unix)]
#[test]
fn writing_into_an_unwritable_directory_reports_permission() {
    let scratch = Scratch::new("readonly");
    let target = scratch.path().join("target");
    std::fs::create_dir_all(&target).expect("the target directory must be creatable");

    if !make_read_only(&target) {
        panic!(
            "could not make {} read-only, so this test cannot establish its premise",
            target.display()
        );
    }

    let result = std::fs::write(target.join("artifact.bin"), b"partial");
    let error = match result {
        Ok(()) => panic!(
            "writing into a read-only directory succeeded, so this environment cannot express the \
             premise (running as root?); the other tests still cover the mapping"
        ),
        Err(e) => e,
    };

    assert_eq!(
        error.kind(),
        std::io::ErrorKind::PermissionDenied,
        "the OS must report this as a permission problem for the test to mean anything: {error}"
    );

    // The installer would carry this through its `?` conversion.
    let converted: InstallerError = error.into();
    assert!(
        matches!(converted, InstallerError::Permission(_)),
        "an unwritable directory must reach the caller as Permission, not as a generic file system \
         error: {converted:?}"
    );
    assert!(
        !target.join("artifact.bin").exists(),
        "a failed write must not have produced the artifact"
    );
}
