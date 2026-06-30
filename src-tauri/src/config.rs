use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Persisted, NON-SECRET app config: the recent vault paths and the last one opened.
/// Stored as plaintext JSON in `app_config_dir()/config.json`. Never put secrets here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentVaults {
    #[serde(default)]
    pub recent: Vec<String>,
    #[serde(default)]
    pub last: Option<String>,
}

impl RecentVaults {
    /// Pure: move `path` to the front of `recent` (dedup by exact string), set it as
    /// `last`, and cap the list to `max` entries (oldest dropped).
    /// `max` must be > 0 (callers use the plan constant of 10).
    pub fn touch(&mut self, path: &str, max: usize) {
        debug_assert!(max > 0, "touch: max must be > 0");
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(max);
        self.last = Some(path.to_string());
    }
}

/// A recent-vault entry annotated with whether the file currently exists on disk.
/// This is a view/presentation type sent from Rust to the frontend only (Serialize).
/// It is never read back from JSON, so `Deserialize` is intentionally omitted.
#[derive(Debug, Clone, Serialize)]
pub struct RecentVaultView {
    pub path: String,
    pub exists: bool,
}

/// Pure: map each path in `recent` to a `RecentVaultView` using the caller-supplied
/// `exists_fn` predicate. Keeps the original order (most-recent-first).
pub fn recent_views(recent: &[String], exists_fn: impl Fn(&str) -> bool) -> Vec<RecentVaultView> {
    recent
        .iter()
        .map(|p| RecentVaultView {
            path: p.clone(),
            exists: exists_fn(p),
        })
        .collect()
}

/// I/O: Read `config.json` from `dir`. Returns `RecentVaults::default()` on any error
/// (missing dir, missing file, or malformed JSON) so startup is always non-fatal.
pub fn load(dir: &Path) -> RecentVaults {
    let path = dir.join("config.json");
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => RecentVaults::default(),
    }
}

/// I/O: Serialize `cfg` to JSON and write it atomically to `dir/config.json`.
/// Creates `dir` if it does not exist.
pub fn save(dir: &Path, cfg: &RecentVaults) -> Result<()> {
    std::fs::create_dir_all(dir)
        .map_err(|e| AppError::Other(format!("config dir create failed: {e}")))?;
    let bytes = serde_json::to_vec_pretty(cfg)
        .map_err(|e| AppError::Other(format!("config serialize failed: {e}")))?;
    crate::storage::write_atomic(&dir.join("config.json"), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_moves_to_front_dedups_and_caps() {
        let mut r = RecentVaults::default();
        r.touch("/a", 3);
        r.touch("/b", 3);
        r.touch("/a", 3); // re-touch moves /a to front, no duplicate
        assert_eq!(r.recent, vec!["/a".to_string(), "/b".to_string()]);
        assert_eq!(r.last.as_deref(), Some("/a"));
        r.touch("/c", 3);
        r.touch("/d", 3); // cap = 3 → oldest ("/b") dropped
        assert_eq!(r.recent, vec!["/d".to_string(), "/c".to_string(), "/a".to_string()]);
    }

    #[test]
    fn recent_views_reports_existence() {
        let recent = vec!["/exists".to_string(), "/gone".to_string()];
        let views = recent_views(&recent, |p| p == "/exists");
        assert_eq!(views.len(), 2);
        assert!(views[0].exists && views[0].path == "/exists");
        assert!(!views[1].exists && views[1].path == "/gone");
    }

    #[test]
    fn load_missing_dir_returns_default() {
        let dir = std::env::temp_dir().join(format!("votp_cfg_{}", uuid::Uuid::new_v4()));
        // dir does not exist yet → default
        let cfg = load(&dir);
        assert!(cfg.recent.is_empty() && cfg.last.is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("votp_cfg_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut cfg = RecentVaults::default();
        cfg.touch("/vault/one.bin", 10);
        save(&dir, &cfg).unwrap();
        let back = load(&dir);
        assert_eq!(back.recent, vec!["/vault/one.bin".to_string()]);
        assert_eq!(back.last.as_deref(), Some("/vault/one.bin"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
