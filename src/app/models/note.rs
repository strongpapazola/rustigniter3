//! Note — model (analog `application/models/Note_model.php`).
//!
//! Di CodeIgniter model meng-encapsulasi akses data lewat `$this->db`. Di RustIgniter
//! model adalah kumpulan fungsi yang menerima `&Database` dan memakai Query Builder.

use crate::system::Database;
use serde_json::{json, Value};

pub struct Note;

impl Note {
    /// Semua catatan, terbaru di atas.
    pub fn all(db: &Database) -> Result<Vec<Value>, String> {
        db.table("notes").order_by("id", "DESC").get()
    }

    /// Satu catatan berdasarkan id, atau `None`.
    pub fn find(db: &Database, id: i64) -> Result<Option<Value>, String> {
        db.table("notes").where_("id", id).first()
    }

    /// Buat catatan baru; kembalikan id-nya.
    pub fn create(db: &Database, text: &str) -> Result<i64, String> {
        db.table("notes").insert(json!({ "text": text }))
    }

    /// Perbarui teks catatan; kembalikan jumlah baris terdampak (0 = tidak ada).
    pub fn update(db: &Database, id: i64, text: &str) -> Result<usize, String> {
        db.table("notes").where_("id", id).update(json!({ "text": text }))
    }

    /// Hapus catatan; kembalikan jumlah baris terdampak (0 = tidak ada).
    pub fn delete(db: &Database, id: i64) -> Result<usize, String> {
        db.table("notes").where_("id", id).delete()
    }
}
