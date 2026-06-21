//! `app` — kode userland aplikasi (analog folder `application/` di CodeIgniter).
//!
//! Berisi controller, model, dan view milik aplikasi, plus registrasi controller
//! dan skema database (migrasi/seed). Schema sengaja tinggal di userland (seperti
//! `application/migrations` di CI), bukan di `system/`.

pub mod controllers;
pub mod hooks;
pub mod migrations;
pub mod models;

use crate::system::{Database, Hook, Migration, Registry, Resource};
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

/// Daftar migrasi aplikasi (dipakai CLI & bootstrap).
pub fn migrations() -> Vec<Migration> {
    migrations::all()
}

/// Isi data contoh (idempotent: hanya saat tabel kosong).
pub fn seed(db: &Database) -> Result<(), String> {
    if db.table("notes").get()?.is_empty() {
        for text in [
            "Catatan pertama di RustIgniter",
            "Query Builder bekerja",
            "MVC + Database siap dipakai",
        ] {
            Note::create(db, text)?;
        }
    }
    if db.table("users").get()?.is_empty() {
        User::create(db, "admin", "admin123")?;
    }
    Ok(())
}
