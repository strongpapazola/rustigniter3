//! `app` — kode userland aplikasi (analog folder `application/` di CodeIgniter).
//!
//! Berisi controller, model, dan view milik aplikasi, plus registrasi controller
//! dan skema database (migrasi/seed). Schema sengaja tinggal di userland (seperti
//! `application/migrations` di CI), bukan di `system/`.

pub mod controllers;
pub mod hooks;
pub mod models;

use crate::system::{Database, Dialect, Hook, Registry, Resource};
use controllers::api_notes::ApiNotes;
use controllers::auth::Auth;
use controllers::notes::Notes;
use controllers::welcome::Welcome;
use models::note::Note;
use models::user::User;

/// Daftarkan semua controller aplikasi ke registry.
/// Tambahkan controller baru dengan satu baris `registry.register("nama", Box::new(...))`.
pub fn register(registry: &mut Registry) {
    registry.register("welcome", Box::new(Welcome));
    registry.register("notes", Box::new(Notes));
    registry.register("auth", Box::new(Auth));
    // Resource REST: URL-nya diarahkan lewat custom route di config/routes.toml.
    registry.register("notes_api", Box::new(Resource::new(ApiNotes)));
}

/// Daftar hook (middleware) yang aktif, dijalankan sesuai urutan ini.
pub fn register_hooks() -> Vec<Box<dyn Hook>> {
    vec![
        Box::new(hooks::RequestLogger),
        Box::new(hooks::PoweredBy),
        Box::new(hooks::CsrfGuard),
        Box::new(hooks::ApiKeyGuard),
    ]
}

/// Buat skema (idempotent) dan, bila `seed`, isi data contoh saat tabel kosong.
/// DDL menyesuaikan dialek driver aktif.
pub fn migrate(db: &Database, seed: bool) -> Result<(), String> {
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
    db.execute(ddl, &[])?;

    // Tabel users untuk autentikasi.
    let users_ddl = match db.dialect() {
        Dialect::Sqlite => {
            "CREATE TABLE IF NOT EXISTS users (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, \
                username TEXT NOT NULL UNIQUE, \
                password_hash TEXT NOT NULL, \
                salt TEXT NOT NULL\
            )"
        }
        Dialect::Postgres => {
            "CREATE TABLE IF NOT EXISTS users (\
                id BIGSERIAL PRIMARY KEY, \
                username TEXT NOT NULL UNIQUE, \
                password_hash TEXT NOT NULL, \
                salt TEXT NOT NULL\
            )"
        }
    };
    db.execute(users_ddl, &[])?;

    if seed && db.table("notes").get()?.is_empty() {
        for text in [
            "Catatan pertama di RustIgniter",
            "Query Builder bekerja",
            "MVC + Database siap dipakai",
        ] {
            Note::create(db, text)?;
        }
    }

    // Seed user demo: admin / admin123.
    if seed && db.table("users").get()?.is_empty() {
        User::create(db, "admin", "admin123")?;
    }
    Ok(())
}
