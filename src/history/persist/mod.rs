//! Filesystem persistence adapter for [`GameCollection`].
//!
//! This is the I/O seam that sits *outside* the pure history kernel. The kernel
//! produces YAML via [`GameCollection::to_yaml`]; the functions here own the
//! filesystem — paths, directory creation, and writes. They are gated behind the
//! `persistence` feature so a bare `history` (serialization-only) build stays
//! free of filesystem I/O, and they are the single place in the crate that is
//! permitted to call `std::fs` (marked with an explicit `#[allow]`).
//!
//! Timestamps are *injected by the caller* rather than read from the clock here,
//! keeping this layer deterministic and testable; the delivery layer that owns a
//! clock supplies the value.

use crate::error::GfError;
use crate::history::GameCollection;

/// Writes `collection` to `path` as YAML, creating parent directories as needed.
///
/// Returns `path` on success.
///
/// # Errors
///
/// - [`GfError::IoError`] — directory creation or file write failed.
/// - [`GfError::ParseError`] — YAML serialization failed.
///
/// # Examples
///
/// ```no_run
/// use gfcore::history::{persist, GameCollection};
///
/// let col = GameCollection::new();
/// let path = persist::save_to(&col, "/tmp/test_collection.yaml").expect("save must succeed");
/// assert_eq!(path, "/tmp/test_collection.yaml");
/// ```
// The sanctioned adapter seam: filesystem access is deliberate and confined here.
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
pub fn save_to(collection: &GameCollection, path: &str) -> Result<String, GfError> {
    let yaml = collection.to_yaml()?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| GfError::IoError(e.to_string()))?;
        }
    }
    std::fs::write(path, &yaml).map_err(|e| GfError::IoError(e.to_string()))?;
    Ok(path.to_string())
}

/// Writes `collection` to `generated/<run_name>_<timestamp>.yaml`.
///
/// `timestamp` (Unix epoch seconds) is supplied by the caller rather than read
/// from the clock, so this function is deterministic. The `generated/` directory
/// is relative to the process's current working directory and is created if
/// absent. Returns the path written on success.
///
/// # Errors
///
/// - [`GfError::IoError`] — directory creation or file write failed.
/// - [`GfError::ParseError`] — YAML serialization failed.
///
/// # Examples
///
/// ```no_run
/// use gfcore::history::{persist, GameCollection};
///
/// let col = GameCollection::new();
/// let path = persist::save_run(&col, "my_session", 1_700_000_000).expect("save must succeed");
/// assert!(path.contains("my_session"));
/// ```
pub fn save_run(
    collection: &GameCollection,
    run_name: &str,
    timestamp: u64,
) -> Result<String, GfError> {
    let path = format!("generated/{run_name}_{timestamp}.yaml");
    save_to(collection, &path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::GameRecord;

    fn make_record() -> GameRecord {
        GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()])
    }

    #[test]
    fn test_save_to_round_trips_through_the_filesystem() {
        let mut col = GameCollection::new();
        col.push(make_record());
        let path = std::env::temp_dir()
            .join("gfcore_test_save_to.yaml")
            .to_string_lossy()
            .to_string();
        let result = save_to(&col, &path);
        assert!(result.is_ok(), "save_to failed: {result:?}");
        assert!(std::path::Path::new(&path).exists());
        let yaml = std::fs::read_to_string(&path).unwrap();
        let loaded = GameCollection::from_yaml(&yaml).unwrap();
        assert_eq!(col, loaded);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_run_embeds_run_name_and_timestamp_in_path() {
        let mut col = GameCollection::new();
        col.push(make_record());
        let path = save_run(&col, "unit_run", 1_700_000_000).expect("save_run must succeed");
        assert!(path.contains("unit_run"));
        assert!(path.contains("1700000000"));
        assert!(path.starts_with("generated/"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir("generated");
    }
}
