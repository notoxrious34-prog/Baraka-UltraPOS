//! Isolated data-directory override (developer/test facility, not a user feature).
//!
//! `BARAKA_DATA_DIR` lets audit and design tasks run the real application
//! against a scratch directory instead of the owner's live data directory.
//! It is read once at startup; there is no user interface and no end-user
//! documentation for it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Environment variable selecting an isolated data directory.
pub const DATA_DIR_ENV: &str = "BARAKA_DATA_DIR";

/// A data directory resolved at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDataDir {
    /// Directory every persistent file (database, backups, quarantine,
    /// printer captures) derives from.
    pub path: PathBuf,
    /// True when the override variable selected a non-default directory.
    pub is_override: bool,
}

/// Ways an override value can be rejected.
///
/// Startup must abort on any of these; the default (live) directory is never
/// used as a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDirError {
    /// The trimmed value is not an absolute path.
    NotAbsolute,
    /// The value contains a `..` component.
    ParentTraversal,
    /// The value is the default directory or lies inside it.
    InsideLiveDir,
    /// The path exists but is not a directory.
    NotADirectory,
    /// The directory could not be created (or isolation unverifiable).
    CreateFailed,
}

impl DataDirError {
    /// Stable machine-readable diagnostic code (developer diagnostic, never translated).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotAbsolute => "DATA_DIR_NOT_ABSOLUTE",
            Self::ParentTraversal => "DATA_DIR_PARENT_TRAVERSAL",
            Self::InsideLiveDir => "DATA_DIR_INSIDE_LIVE_DIR",
            Self::NotADirectory => "DATA_DIR_NOT_A_DIRECTORY",
            Self::CreateFailed => "DATA_DIR_CREATE_FAILED",
        }
    }
}

/// Strip a leading `\\?\` verbatim prefix (byte-exact; canonicalized Windows paths carry it).
fn strip_verbatim_prefix(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(b"\\\\?\\") {
        &bytes[4..]
    } else {
        bytes
    }
}

/// Normalized comparison form: `/` and `\` alike, empty segments dropped,
/// ASCII case-insensitive. Compared component by component, never by string prefix.
fn normalized_components(path: &Path) -> Vec<Vec<u8>> {
    strip_verbatim_prefix(path.as_os_str().as_encoded_bytes())
        .split(|byte| *byte == b'/' || *byte == b'\\')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .collect()
}

/// True when `candidate` equals `base` or lies inside it (component comparison).
fn is_equal_or_descendant(candidate: &[Vec<u8>], base: &[Vec<u8>]) -> bool {
    candidate.len() >= base.len() && candidate[..base.len()] == base[..]
}

/// Resolve the data directory. Pure: no filesystem access.
///
/// R1 unset/empty/whitespace-only returns the default unchanged; otherwise the
/// trimmed value must be absolute (R2), contain no `..` (R3), and must neither
/// equal nor descend from the default directory (R4); anything else is the
/// override as given (R5).
#[must_use = "an invalid override must abort startup, never fall back"]
pub fn resolve_data_dir(
    default_dir: &Path,
    override_value: Option<&OsStr>,
) -> Result<ResolvedDataDir, DataDirError> {
    let default = || ResolvedDataDir {
        path: default_dir.to_path_buf(),
        is_override: false,
    };
    let raw = match override_value {
        None => return Ok(default()),
        Some(value) => value,
    };
    let text = raw.to_str().ok_or(DataDirError::NotAbsolute)?;
    if text.bytes().all(|byte| byte.is_ascii_whitespace()) {
        return Ok(default());
    }
    let trimmed = text.trim_matches(|c: char| c.is_ascii_whitespace());
    let candidate = Path::new(trimmed);
    if !candidate.is_absolute() {
        return Err(DataDirError::NotAbsolute);
    }
    if normalized_components(candidate)
        .iter()
        .any(|segment| segment == b"..")
    {
        return Err(DataDirError::ParentTraversal);
    }
    if is_equal_or_descendant(
        &normalized_components(candidate),
        &normalized_components(default_dir),
    ) {
        return Err(DataDirError::InsideLiveDir);
    }
    Ok(ResolvedDataDir {
        path: PathBuf::from(trimmed),
        is_override: true,
    })
}

/// Create and validate the override directory. No-op when not overriding.
///
/// Creates the full path, requires it to be a directory, then re-checks
/// isolation against the canonicalized default directory (symlink/junction
/// backstop) whenever the default directory exists.
pub fn prepare_data_dir(
    resolved: &ResolvedDataDir,
    default_dir: &Path,
) -> Result<(), DataDirError> {
    if !resolved.is_override {
        return Ok(());
    }
    let path = &resolved.path;
    match std::fs::create_dir_all(path) {
        Ok(()) => {}
        Err(_) if path.exists() => return Err(DataDirError::NotADirectory),
        Err(_) => return Err(DataDirError::CreateFailed),
    }
    if !path.is_dir() {
        return Err(DataDirError::NotADirectory);
    }
    if default_dir.exists() {
        let canonical_default =
            std::fs::canonicalize(default_dir).map_err(|_| DataDirError::CreateFailed)?;
        let canonical_override =
            std::fs::canonicalize(path).map_err(|_| DataDirError::CreateFailed)?;
        if is_equal_or_descendant(
            &normalized_components(&canonical_override),
            &normalized_components(&canonical_default),
        ) {
            return Err(DataDirError::InsideLiveDir);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh unique scratch root. The caller removes it at the end of the test.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baraka-brk058-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn none_returns_default_byte_identical() {
        // Protects: default mode must behave byte-for-byte as before.
        let default = Path::new("C:\\Users\\Shop\\AppData\\Roaming\\com.baraka.ultrapos");
        let resolved = resolve_data_dir(default, None).expect("None must resolve");
        assert!(!resolved.is_override);
        assert_eq!(resolved.path.as_os_str(), default.as_os_str());
    }

    #[test]
    fn empty_and_whitespace_only_return_default() {
        // Protects: an unset-looking variable never activates override mode.
        let default = Path::new("C:\\live\\data");
        for value in ["", "   ", "\t \n"] {
            let resolved =
                resolve_data_dir(default, Some(OsStr::new(value))).expect("blank must resolve");
            assert!(!resolved.is_override);
            assert_eq!(resolved.path.as_os_str(), default.as_os_str());
        }
    }

    #[test]
    fn relative_paths_are_not_absolute() {
        // Protects: a relative override can never anchor to the live directory.
        assert_eq!(
            resolve_data_dir(Path::new("C:\\live"), Some(OsStr::new("data\\scratch")))
                .expect_err("relative must fail"),
            DataDirError::NotAbsolute
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_rooted_and_drive_relative_paths_are_not_absolute() {
        // Protects: `\x` (drive-relative) and `C:x` never resolve against CWD.
        for value in ["\\x", "C:x"] {
            assert_eq!(
                resolve_data_dir(Path::new("C:\\live"), Some(OsStr::new(value)))
                    .expect_err("must fail"),
                DataDirError::NotAbsolute,
                "value {value:?}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn parent_component_is_rejected() {
        // Protects: `..` can never escape the chosen directory.
        assert_eq!(
            resolve_data_dir(Path::new("C:\\live"), Some(OsStr::new("C:\\a\\..\\b")))
                .expect_err("must fail"),
            DataDirError::ParentTraversal
        );
    }

    #[test]
    fn override_equal_to_default_case_and_separator_insensitive() {
        // Protects: the live directory itself (any case/separators) is refused.
        let default = scratch("t5-live");
        let upper = default.to_string_lossy().to_uppercase() + "\\";
        assert_eq!(
            resolve_data_dir(&default, Some(OsStr::new(&upper))).expect_err("must fail"),
            DataDirError::InsideLiveDir
        );
        let _ = std::fs::remove_dir_all(&default);
    }

    #[test]
    fn child_of_default_is_inside_live_dir() {
        // Protects: no scratch tree may nest inside the live directory.
        let default = scratch("t6-live");
        let child = default.join("child");
        assert_eq!(
            resolve_data_dir(&default, Some(child.as_os_str())).expect_err("must fail"),
            DataDirError::InsideLiveDir
        );
        let _ = std::fs::remove_dir_all(&default);
    }

    #[test]
    fn sibling_prefix_name_is_not_inside() {
        // Protects: component comparison (never string prefix) decides nesting.
        let default = scratch("t7a");
        let mut sibling = default.as_os_str().to_owned();
        sibling.push("b");
        let resolved =
            resolve_data_dir(&default, Some(&sibling)).expect("sibling prefix must pass");
        assert!(resolved.is_override);
        let _ = std::fs::remove_dir_all(&default);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        // Protects: pasted values with spaces still select the intended directory.
        let target = scratch("t8-trimmed");
        let padded = format!("  {}  ", target.to_string_lossy());
        let resolved = resolve_data_dir(Path::new("C:\\live"), Some(OsStr::new(&padded)))
            .expect("must resolve");
        assert!(resolved.is_override);
        assert_eq!(resolved.path, target);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn prepare_creates_nested_missing_directory() {
        // Protects: a fresh scratch path is created recursively.
        let target = scratch("t9-nested").join("level-one").join("level-two");
        let resolved = ResolvedDataDir {
            path: target.clone(),
            is_override: true,
        };
        prepare_data_dir(&resolved, Path::new("C:\\definitely-not-live-brk058"))
            .expect("must create");
        assert!(target.is_dir());
        let _ = std::fs::remove_dir_all(scratch("t9-nested"));
    }

    #[test]
    fn prepare_on_existing_file_is_not_a_directory() {
        // Protects: a file masquerading as the data dir fails loudly.
        let root = scratch("t10-file");
        std::fs::create_dir_all(&root).expect("setup");
        let file = root.join("blocker");
        std::fs::write(&file, b"x").expect("setup");
        let resolved = ResolvedDataDir {
            path: file,
            is_override: true,
        };
        assert_eq!(
            prepare_data_dir(&resolved, Path::new("C:\\definitely-not-live-brk058"))
                .expect_err("must fail"),
            DataDirError::NotADirectory
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_canonical_check_catches_nesting_into_live_dir() {
        // Protects: symlinks/junctions cannot smuggle the override into the live dir.
        let live = scratch("t11-live");
        std::fs::create_dir_all(&live).expect("setup");
        let nested = live.join("nested");
        let resolved = ResolvedDataDir {
            path: nested.clone(),
            is_override: true,
        };
        assert_eq!(
            prepare_data_dir(&resolved, &live).expect_err("must fail"),
            DataDirError::InsideLiveDir
        );
        let _ = std::fs::remove_dir_all(&live);
    }

    #[test]
    fn error_codes_are_stable() {
        // Protects: the startup diagnostic contract (exact codes in the message).
        assert_eq!(DataDirError::NotAbsolute.code(), "DATA_DIR_NOT_ABSOLUTE");
        assert_eq!(
            DataDirError::ParentTraversal.code(),
            "DATA_DIR_PARENT_TRAVERSAL"
        );
        assert_eq!(
            DataDirError::InsideLiveDir.code(),
            "DATA_DIR_INSIDE_LIVE_DIR"
        );
        assert_eq!(
            DataDirError::NotADirectory.code(),
            "DATA_DIR_NOT_A_DIRECTORY"
        );
        assert_eq!(DataDirError::CreateFailed.code(), "DATA_DIR_CREATE_FAILED");
    }

    #[test]
    fn prepare_without_override_creates_nothing() {
        // Protects: default mode touches nothing new on the filesystem.
        let target = scratch("t13-untouched");
        let resolved = ResolvedDataDir {
            path: target.clone(),
            is_override: false,
        };
        prepare_data_dir(&resolved, Path::new("C:\\live")).expect("must pass");
        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(&target);
    }
}
