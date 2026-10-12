#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only integration file: unwrap/expect is the intended failure signal, and the \
              assertions compare the error the installer actually produced rather than trusting the \
              call to fail in a particular way."
)]
//! How the installer reports a permission failure (#635, scope
//! `installer-interruption-cleanup-and-permission-failure`).
//!
//! `InstallerError` declares a `Permission` variant (`src/error.rs:16-17`), and the plan's premise
//! was that permission handling might be silently swallowed. It is not swallowed -- it is
//! **unreachable**: `From<std::io::Error>` maps every kind, `PermissionDenied` included, onto
//! `FileSystem` (`src/error.rs:72-76`), and nothing anywhere constructs `Permission`.
//!
//! That distinction is the whole point of this file. "The error is reported" and "the error is
//! reported as the structured kind the API advertises" are different claims, and only the first is
//! true today. These tests pin the current, observable behaviour and make the loss of structure
//! visible, rather than asserting the behaviour we wish existed.
//!
//! AC4 applies here: the fact that `Permission` is unreachable is **reported**, not re-labelled as
//! expected behaviour, and no production code is changed. The decision belongs to 01.
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
                let _ = std::fs::set_permissions(&self.dir, perms);
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

/// A `PermissionDenied` io error is reported, and it is reported as `FileSystem`.
///
/// This is the pinned current behaviour. The `Permission` variant exists in the public error enum,
/// so a caller may reasonably match on it; this test records that such a match can never succeed
/// today. The assertion is deliberately on `FileSystem` -- a strong assertion about what happens,
/// not a weak one like `is_err()`.
#[test]
fn a_permission_denied_io_error_is_reported_as_a_filesystem_error_not_permission() {
    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied by test");
    let converted: InstallerError = io_error.into();

    match &converted {
        InstallerError::FileSystem(message) => assert!(
            message.contains("denied by test"),
            "the OS message must survive the conversion: {message}"
        ),
        other => panic!(
            "expected FileSystem (the current mapping for every io error kind), got {other:?}"
        ),
    }

    assert!(
        !matches!(converted, InstallerError::Permission(_)),
        "the Permission variant is declared but unreachable; if this now matches, the mapping was \
         changed and this test should be updated deliberately rather than silently"
    );
}

/// The same conversion loses the *kind*, which is the part a caller would act on.
#[test]
fn the_conversion_does_not_distinguish_not_found_from_permission_denied() {
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
        kind_of(&missing),
        "two very different failures currently produce the same error kind, so a caller cannot \
         tell 'retry with better rights' from 'the file is absent' without parsing the message"
    );
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
    assert!(
        filesystem_text.starts_with("FileSystem error"),
        "the sibling variant must keep its own prefix: {filesystem_text}"
    );
    assert_ne!(
        permission_text, filesystem_text,
        "the two kinds must not be textually identical, or a structured caller gains nothing"
    );
}

/// End to end: an actual write into a real read-only directory.
///
/// Constructed the way the plan asks -- a temp target made unwritable -- and asserted on the error
/// that reaches the caller. Where the platform cannot make a directory unwritable (for instance a
/// root user), the test **registers the gap by panicking with a reason** rather than silently
/// passing: a test that cannot set up its own premise must not report success.
#[cfg(unix)]
#[test]
fn writing_into_an_unwritable_directory_reports_permission_denied_as_filesystem() {
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
        matches!(converted, InstallerError::FileSystem(_)),
        "the installer currently reports this as FileSystem, losing the permission kind: {converted:?}"
    );
    assert!(
        !target.join("artifact.bin").exists(),
        "a failed write must not have produced the artifact"
    );
}
