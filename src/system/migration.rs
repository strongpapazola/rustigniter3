//! Migration — sistem migrasi database berversi (CI: `CI_Migration`).
//!
//! Setiap [`Migration`] punya `version`, `name`, dan fungsi `up`/`down`. [`Migrator`]
//! menjalankan migrasi yang belum diterapkan (urut versi), mencatatnya di tabel
//! `schema_migrations`, dan bisa me-rollback yang terakhir. Migrasi didefinisikan di
//! userland (`app::migrations()`), dialek-aware lewat `db.dialect()`.

use crate::system::Database;
use serde_json::json;

/// Satu migrasi: naik (terapkan) & turun (batalkan).
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub up: fn(&Database) -> Result<(), String>,
    pub down: fn(&Database) -> Result<(), String>,
}

/// Penjalan migrasi atas daftar migrasi terurut versi.
pub struct Migrator {
    migrations: Vec<Migration>,
}

impl Migrator {
    pub fn new(mut migrations: Vec<Migration>) -> Self {
        migrations.sort_by_key(|m| m.version);
        Self { migrations }
    }

    /// Pastikan tabel pencatat migrasi ada.
    fn ensure_table(&self, db: &Database) -> Result<(), String> {
        db.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (\
                version BIGINT PRIMARY KEY, \
                name TEXT NOT NULL\
            )",
            &[],
        )
        .map(|_| ())
    }

    /// Versi yang sudah diterapkan (urut naik).
    fn applied(&self, db: &Database) -> Result<Vec<i64>, String> {
        self.ensure_table(db)?;
        let rows = db.table("schema_migrations").order_by("version", "ASC").get()?;
        Ok(rows
            .iter()
            .filter_map(|r| r.get("version").and_then(|v| v.as_i64()))
            .collect())
    }

    /// Jalankan semua migrasi tertunda (urut versi). Kembalikan versi yang diterapkan.
    pub fn migrate(&self, db: &Database) -> Result<Vec<i64>, String> {
        let applied = self.applied(db)?;
        let mut done = Vec::new();
        for m in &self.migrations {
            if !applied.contains(&m.version) {
                (m.up)(db)?;
                self.record(db, m.version, m.name)?;
                done.push(m.version);
            }
        }
        Ok(done)
    }

    /// Batalkan migrasi terakhir yang diterapkan. Kembalikan versinya (atau None).
    pub fn rollback(&self, db: &Database) -> Result<Option<i64>, String> {
        let applied = self.applied(db)?;
        let Some(version) = applied.iter().max().copied() else {
            return Ok(None);
        };
        if let Some(m) = self.migrations.iter().find(|m| m.version == version) {
            (m.down)(db)?;
        }
        db.table("schema_migrations").where_("version", version).delete()?;
        Ok(Some(version))
    }

    /// Status tiap migrasi: (versi, nama, sudah_diterapkan).
    pub fn status(&self, db: &Database) -> Result<Vec<(i64, String, bool)>, String> {
        let applied = self.applied(db)?;
        Ok(self
            .migrations
            .iter()
            .map(|m| (m.version, m.name.to_string(), applied.contains(&m.version)))
            .collect())
    }

    /// Catat migrasi via raw execute (schema_migrations tak punya kolom `id`, jadi
    /// jangan pakai QueryBuilder::insert yang menambahkan `RETURNING id` di Postgres).
    fn record(&self, db: &Database, version: i64, name: &str) -> Result<(), String> {
        let sql = format!(
            "INSERT INTO schema_migrations (version, name) VALUES ({}, {})",
            db.placeholder(1),
            db.placeholder(2)
        );
        db.execute(&sql, &[json!(version), json!(name)]).map(|_| ())
    }
}
