//! User — model autentikasi.
//!
//! Password di-hash dengan **bcrypt** (salt disertakan otomatis di dalam hash, cost adaptif).
//! Ini standar yang layak produksi untuk penyimpanan password.

use crate::system::Database;
use serde_json::{json, Value};

pub struct User;

impl User {
    /// Buat user baru dengan password ter-hash bcrypt; kembalikan id.
    pub fn create(db: &Database, username: &str, password: &str) -> Result<i64, String> {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("hash password gagal: {e}"))?;
        db.table("users").insert(json!({
            "username": username,
            "password_hash": hash,
        }))
    }

    /// Verifikasi kredensial; kembalikan baris user bila cocok.
    pub fn verify(db: &Database, username: &str, password: &str) -> Result<Option<Value>, String> {
        let row = db.table("users").where_("username", username).first()?;
        match row {
            Some(user) => {
                let stored = user.get("password_hash").and_then(Value::as_str).unwrap_or("");
                match bcrypt::verify(password, stored) {
                    Ok(true) => Ok(Some(user)),
                    _ => Ok(None), // password salah atau hash tak valid
                }
            }
            None => Ok(None),
        }
    }
}
