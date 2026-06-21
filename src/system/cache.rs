//! Cache — penyimpanan key-value sementara dengan TTL (CI: `$this->cache`).
//!
//! Backend in-memory yang bisa dibagikan antar request (`Arc<Mutex<…>>`). Nilai berupa
//! `serde_json::Value`. TTL 0 = tanpa kedaluwarsa. Entri kedaluwarsa dibersihkan saat diakses.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

struct Entry {
    value: Value,
    expires: Option<SystemTime>,
}

/// Cache in-memory yang bisa di-clone (berbagi `Arc`).
#[derive(Clone, Default)]
pub struct Cache {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Simpan `value` di `key` selama `ttl_secs` detik (0 = selamanya).
    pub fn save(&self, key: &str, value: Value, ttl_secs: u64) {
        let expires = if ttl_secs == 0 {
            None
        } else {
            SystemTime::now().checked_add(Duration::from_secs(ttl_secs))
        };
        self.inner
            .lock()
            .expect("cache mutex")
            .insert(key.to_string(), Entry { value, expires });
    }

    /// Ambil nilai bila ada & belum kedaluwarsa.
    pub fn get(&self, key: &str) -> Option<Value> {
        let mut map = self.inner.lock().expect("cache mutex");
        let expired = match map.get(key) {
            Some(e) => e.expires.is_some_and(|exp| SystemTime::now() > exp),
            None => return None,
        };
        if expired {
            map.remove(key);
            return None;
        }
        map.get(key).map(|e| e.value.clone())
    }

    /// Apakah key ada & valid.
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn delete(&self, key: &str) {
        self.inner.lock().expect("cache mutex").remove(key);
    }

    pub fn clear(&self) {
        self.inner.lock().expect("cache mutex").clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn save_get_delete() {
        let c = Cache::new();
        assert_eq!(c.get("k"), None);
        c.save("k", json!({"n": 1}), 0);
        assert_eq!(c.get("k"), Some(json!({"n": 1})));
        assert!(c.has("k"));
        c.delete("k");
        assert_eq!(c.get("k"), None);
    }

    #[test]
    fn ttl_kedaluwarsa() {
        let c = Cache::new();
        // TTL "di masa lalu" disimulasikan dengan menaruh expires lampau langsung.
        c.inner.lock().unwrap().insert(
            "x".into(),
            Entry { value: json!(1), expires: Some(SystemTime::UNIX_EPOCH) },
        );
        assert_eq!(c.get("x"), None); // sudah lewat -> dibersihkan
        assert!(!c.has("x"));
    }
}
