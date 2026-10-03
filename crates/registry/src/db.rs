use rusqlite::Connection;

use crate::{RegistryError, RegistryResult};

pub struct RegistryDb {
    pub conn: Connection,
}

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("migrations/001_init.sql")),
    (2, include_str!("migrations/002_scan_integrity.sql")),
    (3, include_str!("migrations/003_programs.sql")),
];

pub fn open_path(path: &std::path::Path) -> RegistryResult<RegistryDb> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    init(conn)
}

pub fn open_memory() -> RegistryResult<RegistryDb> {
    let conn = Connection::open_in_memory()?;
    init(conn)
}

fn init(conn: Connection) -> RegistryResult<RegistryDb> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;
    let mut db = RegistryDb { conn };
    db.migrate()?;
    Ok(db)
}

impl RegistryDb {
    pub fn migrate(&mut self) -> RegistryResult<()> {
        for (version, sql) in MIGRATIONS {
            let applied: bool = self.conn.query_row(
                "SELECT COUNT(*) > 0 FROM schema_migrations WHERE version = ?1",
                [version],
                |r| r.get(0),
            )?;
            if applied {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql).map_err(|e| {
                RegistryError::Migration(format!("migration {version} failed: {e}"))
            })?;
            tx.execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![version, chrono::Utc::now().to_rfc3339()],
            )?;
            tx.commit()?;
        }
        Ok(())
    }

    pub fn schema_version(&self) -> RegistryResult<i64> {
        let v = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_from_empty() {
        let db = open_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), 3);
    }

    #[test]
    fn migration_replay_is_stable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let db = open_path(&path).unwrap();
            assert_eq!(db.schema_version().unwrap(), 3);
        }
        let db2 = open_path(&path).unwrap();
        assert_eq!(db2.schema_version().unwrap(), 3);
    }
}
