//! Config — pemuat konfigurasi aplikasi.
//!
//! Ide dari `CI_Config`: muat berkas konfigurasi lalu ambil nilai lewat
//! `item("key")`, plus helper `base_url()` / `site_url()`. Di CodeIgniter sumbernya
//! array PHP (`config.php`); di RustIgniter sumbernya TOML (`config/app.toml`),
//! dibaca dengan `toml` + `serde`. Lookup mendukung path bertitik, mis.
//! `item("custom.app_name")` atau `item("server.port")`.

use std::fs;
use std::path::Path;

/// Konfigurasi aplikasi yang sudah dimuat dari `config/app.toml`.
#[derive(Debug, Clone)]
pub struct Config {
    table: toml::Table,
}

impl Config {
    /// Muat konfigurasi dari berkas TOML.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let raw = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("gagal membaca {}: {e}", path.as_ref().display()))?;
        let table: toml::Table =
            toml::from_str(&raw).map_err(|e| format!("config TOML tidak valid: {e}"))?;
        Ok(Self { table })
    }

    /// Ambil item config sebagai string. Mendukung path bertitik untuk tabel
    /// bersarang, mis. `item("custom.app_name")`. Mengembalikan `None` bila tak ada.
    pub fn item(&self, key: &str) -> Option<String> {
        let mut current: &toml::Value = self.table.get(key.split('.').next()?)?;
        // Telusuri sisa segmen path bertitik.
        for seg in key.split('.').skip(1) {
            current = current.as_table()?.get(seg)?;
        }
        value_to_string(current)
    }

    /// Base URL aplikasi (CI: `$this->config->base_url()`), dengan `uri` opsional
    /// ditempel di belakang. Selalu diakhiri tanpa duplikasi slash.
    pub fn base_url(&self, uri: &str) -> String {
        let base = self.item("base_url").unwrap_or_default();
        join_url(&base, uri)
    }

    /// Sama seperti `base_url` namun menyisipkan `index_page` bila diset
    /// (CI: `$this->config->site_url()`).
    pub fn site_url(&self, uri: &str) -> String {
        let base = self.item("base_url").unwrap_or_default();
        let index = self.item("index_page").unwrap_or_default();
        let prefix = if index.is_empty() {
            base
        } else {
            join_url(&base, &index)
        };
        join_url(&prefix, uri)
    }

    /// Lingkungan aplikasi: "development" (default) atau "production".
    /// Memengaruhi cookie `Secure` & verbositas error.
    pub fn environment(&self) -> String {
        self.item("environment")
            .unwrap_or_else(|| "development".to_string())
    }

    /// True bila environment = "production".
    pub fn is_production(&self) -> bool {
        self.environment().eq_ignore_ascii_case("production")
    }

    /// Host bind server (default "127.0.0.1").
    pub fn server_host(&self) -> String {
        self.item("server.host").unwrap_or_else(|| "127.0.0.1".to_string())
    }

    /// Port bind server (default 8080).
    pub fn server_port(&self) -> u16 {
        self.item("server.port")
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080)
    }
}

/// Gabungkan base + path tanpa menggandakan '/'.
fn join_url(base: &str, uri: &str) -> String {
    let uri = uri.trim_start_matches('/');
    if uri.is_empty() {
        return base.to_string();
    }
    if base.ends_with('/') {
        format!("{base}{uri}")
    } else {
        format!("{base}/{uri}")
    }
}

/// Ubah nilai TOML skalar menjadi string; tabel/array -> None.
fn value_to_string(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        let toml = r#"
base_url = "http://localhost:8080/"
index_page = ""
[server]
host = "0.0.0.0"
port = 9000
[custom]
app_name = "RustIgniter"
"#;
        Config {
            table: toml::from_str(toml).unwrap(),
        }
    }

    #[test]
    fn item_top_level_dan_bersarang() {
        let c = sample();
        assert_eq!(c.item("base_url").as_deref(), Some("http://localhost:8080/"));
        assert_eq!(c.item("custom.app_name").as_deref(), Some("RustIgniter"));
        assert_eq!(c.item("server.port").as_deref(), Some("9000"));
        assert_eq!(c.item("tidakada"), None);
    }

    #[test]
    fn base_url_gabung_tanpa_double_slash() {
        let c = sample();
        assert_eq!(c.base_url(""), "http://localhost:8080/");
        assert_eq!(c.base_url("css/app.css"), "http://localhost:8080/css/app.css");
        assert_eq!(c.base_url("/css/app.css"), "http://localhost:8080/css/app.css");
    }

    #[test]
    fn server_host_port() {
        let c = sample();
        assert_eq!(c.server_host(), "0.0.0.0");
        assert_eq!(c.server_port(), 9000);
    }
}
