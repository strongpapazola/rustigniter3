//! Migrasi database aplikasi (analog `application/migrations/` di CodeIgniter).
//!
//! Setiap migrasi punya versi naik (`up`) & turun (`down`), dialek-aware. Daftarnya
//! dipakai oleh CLI (`migrate`/`migrate:rollback`/`migrate:status`) dan saat `serve`.

use crate::system::migration::Migration;
use crate::system::{Database, Dialect};

/// Daftar seluruh migrasi (urut versi).
pub fn all() -> Vec<Migration> {
    vec![
        Migration { version: 1, name: "create_notes", up: up_notes, down: down_notes },
        Migration { version: 2, name: "create_users", up: up_users, down: down_users },
        Migration { version: 3, name: "create_uploads", up: up_uploads, down: down_uploads },
    ]
}

fn up_uploads(db: &Database) -> Result<(), String> {
    let ddl = match db.dialect() {
        Dialect::Sqlite => {
            "CREATE TABLE IF NOT EXISTS uploads (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                filename TEXT NOT NULL, \
                url TEXT NOT NULL, \
                created TEXT NOT NULL DEFAULT (datetime('now'))\
            )"
        }
        Dialect::Postgres => {
            "CREATE TABLE IF NOT EXISTS uploads (\
                id BIGSERIAL PRIMARY KEY, \
                filename TEXT NOT NULL, \
                url TEXT NOT NULL, \
                created TEXT NOT NULL DEFAULT to_char(now(), 'YYYY-MM-DD HH24:MI:SS')\
            )"
        }
    };
    db.execute(ddl, &[]).map(|_| ())
}

fn down_uploads(db: &Database) -> Result<(), String> {
    db.execute("DROP TABLE IF EXISTS uploads", &[]).map(|_| ())
}

fn up_notes(db: &Database) -> Result<(), String> {
    let ddl = match db.dialect() {
        Dialect::Sqlite => {
            "CREATE TABLE IF NOT EXISTS notes (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                text TEXT NOT NULL, \
                created TEXT NOT NULL DEFAULT (datetime('now'))\
            )"
        }
        Dialect::Postgres => {
            "CREATE TABLE IF NOT EXISTS notes (\
                id BIGSERIAL PRIMARY KEY, \
                text TEXT NOT NULL, \
                created TEXT NOT NULL DEFAULT to_char(now(), 'YYYY-MM-DD HH24:MI:SS')\
            )"
        }
    };
    db.execute(ddl, &[]).map(|_| ())
}

fn down_notes(db: &Database) -> Result<(), String> {
    db.execute("DROP TABLE IF EXISTS notes", &[]).map(|_| ())
}

fn up_users(db: &Database) -> Result<(), String> {
    // Tanpa kolom salt: bcrypt menyertakan salt di dalam hash-nya.
    let ddl = match db.dialect() {
        Dialect::Sqlite => {
            "CREATE TABLE IF NOT EXISTS users (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                username TEXT NOT NULL UNIQUE, \
                password_hash TEXT NOT NULL\
            )"
        }
        Dialect::Postgres => {
            "CREATE TABLE IF NOT EXISTS users (\
                id BIGSERIAL PRIMARY KEY, \
                username TEXT NOT NULL UNIQUE, \
                password_hash TEXT NOT NULL\
            )"
        }
    };
    db.execute(ddl, &[]).map(|_| ())
}

fn down_users(db: &Database) -> Result<(), String> {
    db.execute("DROP TABLE IF EXISTS users", &[]).map(|_| ())
}
