//! Local idempotency state for the batch-anchor daemon.
//!
//! Two facts persisted:
//! - `seen_cids` — every CID we've ever buffered, so a restart doesn't double-anchor
//! - `meta(last_flush_timestamp)` — the unix time of the most recent successful registry write
//!
//! SQLite is overkill for this volume, but it gets us durability, atomicity, and zero
//! dependency on a running database. The on-disk file is fine to back up or rsync.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub struct State {
    conn: Connection,
}

impl State {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        let conn =
            Connection::open(path).with_context(|| format!("open sqlite at {}", path.display()))?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS seen_cids (
                cid TEXT PRIMARY KEY,
                first_seen INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn has_seen(&self, cid: &str) -> bool {
        self.conn
            .query_row(
                "SELECT 1 FROM seen_cids WHERE cid = ?1",
                params![cid],
                |_| Ok(()),
            )
            .optional()
            .unwrap_or(None)
            .is_some()
    }

    pub fn mark_seen(&mut self, cid: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR IGNORE INTO seen_cids(cid, first_seen) VALUES (?1, ?2)",
            params![cid, now],
        )?;
        Ok(())
    }

    pub fn record_flush(&mut self, ts: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES ('last_flush_timestamp', ?1)",
            params![ts.to_string()],
        )?;
        Ok(())
    }

    pub fn last_flush_timestamp(&self) -> Option<i64> {
        self.conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'last_flush_timestamp'",
                [],
                |row| {
                    let v: String = row.get(0)?;
                    Ok(v)
                },
            )
            .optional()
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
    }

    pub fn seen_count(&self) -> u64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM seen_cids", [], |row| {
                row.get::<_, i64>(0)
            })
            .map(|n| n as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns (state, tempfile) — the caller must hold the tempfile to keep the underlying file alive.
    fn fresh() -> (State, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let state = State::open(tmp.path()).unwrap();
        (state, tmp)
    }

    #[test]
    fn seen_lifecycle() {
        let (mut s, _tmp) = fresh();
        assert!(!s.has_seen("zABC"));
        s.mark_seen("zABC").unwrap();
        assert!(s.has_seen("zABC"));
        // Re-mark is idempotent.
        s.mark_seen("zABC").unwrap();
        assert_eq!(s.seen_count(), 1);
    }

    #[test]
    fn flush_timestamp_persists() {
        let (mut s, _tmp) = fresh();
        assert_eq!(s.last_flush_timestamp(), None);
        s.record_flush(1_716_500_000).unwrap();
        assert_eq!(s.last_flush_timestamp(), Some(1_716_500_000));
        s.record_flush(1_716_600_000).unwrap();
        assert_eq!(s.last_flush_timestamp(), Some(1_716_600_000));
    }

    #[test]
    fn state_survives_reopen() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_owned();
        {
            let mut s = State::open(&path).unwrap();
            s.mark_seen("zKEEP").unwrap();
            s.record_flush(42).unwrap();
        }
        let reopened = State::open(&path).unwrap();
        assert!(reopened.has_seen("zKEEP"));
        assert_eq!(reopened.last_flush_timestamp(), Some(42));
    }
}
