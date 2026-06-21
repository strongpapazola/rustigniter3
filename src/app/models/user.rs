//! User — model autentikasi.
//!
//! Password disimpan sebagai hash SHA-256 dengan salt acak per-user. Untuk produksi,
//! sebaiknya pakai algoritma khusus password (argon2/bcrypt); SHA-256+salt dipilih agar
//! demo tetap ringan tanpa dependensi berat, sekaligus tidak menyimpan password polos.

use crate::system::session::random_token;
use crate::system::Database;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub struct User;

impl User {
    /// Buat user baru dengan salt acak; kembalikan id.
    pub fn create(db: &Database, username: &str, password: &str) -> Result<i64, String> {
        let salt = random_token();
        let hash = hash_password(&salt, password);
        db.table("users").insert(json!({
            "username": username,
            "password_hash": hash,
            "salt": salt,
        }))
    }

    /// Verifikasi kredensial; kembalikan baris user bila cocok.
    pub fn verify(db: &Database, username: &str, password: &str) -> Result<Option<Value>, String> {
        let row = db.table("users").where_("username", username).first()?;
        match row {
            Some(user) => {
                let salt = user.get("salt").and_then(Value::as_str).unwrap_or("");
                let stored = user.get("password_hash").and_then(Value::as_str).unwrap_or("");
                if !stored.is_empty() && hash_password(salt, password) == stored {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

/// Hash `salt || password` dengan SHA-256, dikembalikan sebagai hex.
fn hash_password(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
