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

    /// Halaman catatan dengan pencarian opsional. Mengembalikan (baris, total cocok).
    /// Mendemokan `like` + `count` + `limit`/`offset` dari Query Builder.
    pub fn paginate(
        db: &Database,
        search: &str,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Value>, i64), String> {
        let page = page.max(1);
        // Builder dibuat ulang per pemakaian karena get()/count() mengonsumsi self.
        let make = || {
            let q = db.table("notes");
            if search.is_empty() {
                q
            } else {
                q.like("text", search)
            }
        };
        let total = make().count()?;
        let offset = (page - 1) * per_page;
        let rows = make()
            .order_by("id", "DESC")
            .limit(per_page)
            .offset(offset)
            .get()?;
        Ok((rows, total))
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
