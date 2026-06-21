//! Session — penyimpanan sesi + flashdata + token CSRF.
//!
//! Ide dari `CI_Session` (`$this->session`). Setiap pengunjung mendapat **id sesi** yang
//! disimpan di cookie; datanya disimpan di sisi server (in-memory store — hilang saat
//! restart; cukup untuk dev, bisa diganti backend file/DB nanti).
//!
//! Tiga jenis data:
//! - **userdata**  — bertahan antar-request (`set`/`get`).
//! - **flashdata** — hanya tersedia di request BERIKUTNYA, lalu otomatis hilang
//!   (CI: `set_flashdata`). Cocok untuk pesan sukses lewat redirect (PRG).
//! - **csrf**      — token anti-CSRF per sesi (lihat `hooks`/`Ctx`).

use rand::RngCore;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const CSRF_KEY: &str = "__csrf";

/// Representasi tersimpan di store antar-request.
#[derive(Clone, Default)]
struct Stored {
    data: Map<String, Value>,
    /// Flash yang menunggu untuk request berikutnya.
    flash: Map<String, Value>,
}

/// Store sesi in-memory yang bisa di-clone (berbagi `Arc`).
#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<Mutex<HashMap<String, Stored>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Muat sesi berdasarkan id dari cookie; buat baru bila tidak ada/kedaluwarsa.
    pub fn load(&self, sid: Option<String>) -> Session {
        let store = self.inner.lock().expect("session mutex poisoned");
        match sid.as_ref().and_then(|id| store.get(id).cloned()) {
            Some(stored) => Session {
                id: sid.unwrap(),
                data: stored.data,
                flash_now: stored.flash, // flash yang di-set request lalu, kini tersedia
                flash_next: Map::new(),
                is_new: false,
                destroyed: false,
            },
            None => Session::fresh(),
        }
    }

    /// Simpan sesi kembali ke store (atau hapus bila di-destroy). Flash yang baru di-set
    /// (`flash_next`) menjadi flash yang tersedia di request berikutnya.
    pub fn save(&self, session: &Session) {
        let mut store = self.inner.lock().expect("session mutex poisoned");
        if session.destroyed {
            store.remove(&session.id);
            return;
        }
        store.insert(
            session.id.clone(),
            Stored {
                data: session.data.clone(),
                flash: session.flash_next.clone(),
            },
        );
    }
}

/// Sesi untuk satu request (CI: `$this->session`).
pub struct Session {
    pub id: String,
    data: Map<String, Value>,
    flash_now: Map<String, Value>,
    flash_next: Map<String, Value>,
    is_new: bool,
    destroyed: bool,
}

impl Default for Session {
    fn default() -> Self {
        Session::fresh()
    }
}

impl Session {
    /// Sesi baru dengan id + token CSRF acak.
    fn fresh() -> Self {
        let mut data = Map::new();
        data.insert(CSRF_KEY.to_string(), Value::String(random_token()));
        Self {
            id: random_token(),
            data,
            flash_now: Map::new(),
            flash_next: Map::new(),
            is_new: true,
            destroyed: false,
        }
    }

    // ---- userdata ----
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        self.data.insert(key.to_string(), value.into());
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }
    pub fn get_str(&self, key: &str) -> Option<String> {
        self.data.get(key).map(scalar_str)
    }
    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
    }

    // ---- flashdata ----
    pub fn set_flash(&mut self, key: &str, value: impl Into<Value>) {
        self.flash_next.insert(key.to_string(), value.into());
    }
    pub fn flash(&self, key: &str) -> Option<&Value> {
        self.flash_now.get(key)
    }
    /// Salinan seluruh flash yang tersedia (untuk diteruskan ke view).
    pub fn flash_all(&self) -> Map<String, Value> {
        self.flash_now.clone()
    }

    // ---- csrf ----
    pub fn csrf_token(&self) -> String {
        self.data.get(CSRF_KEY).map(scalar_str).unwrap_or_default()
    }

    // ---- status / lifecycle ----
    pub fn is_new(&self) -> bool {
        self.is_new
    }
    pub fn destroyed(&self) -> bool {
        self.destroyed
    }
    /// Hapus seluruh sesi (logout). Token CSRF baru dibuat di sesi berikutnya.
    pub fn destroy(&mut self) {
        self.destroyed = true;
        self.data.clear();
        self.flash_next.clear();
    }
}

/// Token acak hex 32 karakter (16 byte). Dipakai untuk id sesi & token CSRF.
pub fn random_token() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn scalar_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flashdata_hidup_satu_request() {
        let store = SessionStore::new();

        // Request 1: set flash.
        let mut s1 = store.load(None);
        let sid = s1.id.clone();
        s1.set_flash("msg", "halo");
        assert_eq!(s1.flash("msg"), None); // belum tersedia di request yang sama
        store.save(&s1);

        // Request 2: flash tersedia.
        let mut s2 = store.load(Some(sid.clone()));
        assert_eq!(s2.flash("msg").map(|v| v.as_str().unwrap().to_string()), Some("halo".into()));
        store.save(&s2);

        // Request 3: flash sudah hilang.
        let s3 = store.load(Some(sid));
        assert_eq!(s3.flash("msg"), None);
    }

    #[test]
    fn userdata_bertahan_dan_destroy() {
        let store = SessionStore::new();
        let mut s = store.load(None);
        let sid = s.id.clone();
        s.set("user_id", 7);
        store.save(&s);

        let mut s2 = store.load(Some(sid.clone()));
        assert_eq!(s2.get("user_id").and_then(|v| v.as_i64()), Some(7));
        s2.destroy();
        store.save(&s2);

        // Setelah destroy, id lama tak ada -> sesi baru.
        let s3 = store.load(Some(sid));
        assert!(s3.is_new());
        assert!(s3.get("user_id").is_none());
    }

    #[test]
    fn csrf_token_ada_dan_stabil() {
        let store = SessionStore::new();
        let s = store.load(None);
        let t = s.csrf_token();
        assert_eq!(t.len(), 32);
        store.save(&s);
        let s2 = store.load(Some(s.id.clone()));
        assert_eq!(s2.csrf_token(), t); // token persisten antar-request
    }
}
