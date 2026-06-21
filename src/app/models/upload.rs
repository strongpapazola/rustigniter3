//! Upload — catatan berkas yang diunggah (nama + URL), agar daftar tak bergantung backend.

use crate::system::Database;
use serde_json::{json, Value};

pub struct Upload;

impl Upload {
    pub fn create(db: &Database, filename: &str, url: &str) -> Result<i64, String> {
        db.table("uploads").insert(json!({ "filename": filename, "url": url }))
    }

    pub fn all(db: &Database) -> Result<Vec<Value>, String> {
        db.table("uploads").order_by("id", "DESC").get()
    }
}
