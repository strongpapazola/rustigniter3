//! Controller — kontrak controller + konteks request `Ctx`.
//!
//! Ide dari `CI_Controller` dan objek `$this`. Di CodeIgniter, sebuah controller adalah
//! class dan setiap *method* publik adalah action; `$this` memberi akses ke loader, config,
//! input, dll. Di RustIgniter:
//!
//! - [`Controller`] adalah trait. Karena Rust tidak punya refleksi runtime, pemetaan
//!   "nama method dari URL" -> "fungsi" dilakukan eksplisit di [`Controller::dispatch`]
//!   (biasanya lewat `match action { ... }`).
//! - [`Ctx`] adalah pengganti `$this`: ia membawa request, argumen, config, dan view engine,
//!   serta menyediakan API gaya loader CI (`view`, `vars`, `config_item`, `base_url`).

use crate::system::cache::Cache;
use crate::system::config::Config;
use crate::system::database::Database;
use crate::system::logger::Logger;
use crate::system::request::{Request, UploadedFile};
use crate::system::response::Response;
use crate::system::session::Session;
use crate::system::storage::Storage;
use crate::system::view::View;
use serde_json::{Map, Value};

/// Kontrak sebuah controller. Satu instance didaftarkan ke registry dengan sebuah nama
/// (mis. "welcome") dan menangani action berdasarkan namanya.
///
/// Implementor mengembalikan `Some(Response)` bila action dikenal, atau `None` bila tidak —
/// `None` membuat framework menghasilkan 404 (atau memakai `not_found_override`).
pub trait Controller: Send + Sync {
    fn dispatch(&self, action: &str, ctx: &mut Ctx) -> Option<Response>;
}

/// Konteks request — analog `$this` di CodeIgniter.
pub struct Ctx<'a> {
    /// Request yang sedang diproses.
    pub request: &'a Request,
    /// Argumen sisa dari URL (segmen setelah controller/method).
    pub args: Vec<String>,
    config: &'a Config,
    view: &'a View,
    db: &'a Database,
    cache: &'a Cache,
    logger: &'a Logger,
    storage: &'a Storage,
    /// Sesi pengunjung (userdata + flashdata + token CSRF).
    pub session: Session,
    /// Variabel view yang terkumpul (CI: `$this->load->vars()`).
    vars: Map<String, Value>,
}

impl<'a> Ctx<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request: &'a Request,
        args: Vec<String>,
        config: &'a Config,
        view: &'a View,
        db: &'a Database,
        session: Session,
        cache: &'a Cache,
        logger: &'a Logger,
        storage: &'a Storage,
    ) -> Self {
        Self {
            request,
            args,
            config,
            view,
            db,
            cache,
            logger,
            storage,
            session,
            vars: Map::new(),
        }
    }

    /// Akses penyimpanan berkas (lokal atau bucket S3).
    pub fn storage(&self) -> &Storage {
        self.storage
    }

    /// Akses cache (CI: `$this->cache`).
    pub fn cache(&self) -> &Cache {
        self.cache
    }

    /// Akses logger (CI: `log_message()`).
    pub fn log(&self) -> &Logger {
        self.logger
    }

    /// Nilai cookie request berdasarkan nama.
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.request.cookie(name)
    }

    /// Token CSRF sesi saat ini (untuk disisipkan ke form).
    pub fn csrf_token(&self) -> String {
        self.session.csrf_token()
    }

    /// Akses database (CI: `$this->db`). Mulai query dengan `ctx.db().table("...")`.
    pub fn db(&self) -> &Database {
        self.db
    }

    /// Akses objek config (untuk `base_url()`/`site_url()`/`item()` lanjutan).
    pub fn config(&self) -> &Config {
        self.config
    }

    /// Ambil item config sebagai string (CI: `$this->config->item()`).
    pub fn config_item(&self, key: &str) -> Option<String> {
        self.config.item(key)
    }

    /// Base URL aplikasi + uri opsional.
    pub fn base_url(&self, uri: &str) -> String {
        self.config.base_url(uri)
    }

    /// Site URL aplikasi + uri opsional.
    pub fn site_url(&self, uri: &str) -> String {
        self.config.site_url(uri)
    }

    /// Argumen ke-`i` (berbasis-0) dari URL, mis. `/blog/lihat/5` -> `arg(0) == "5"`.
    pub fn arg(&self, i: usize) -> Option<&str> {
        self.args.get(i).map(String::as_str)
    }

    /// Nilai query string (CI: `$this->input->get()`).
    pub fn query(&self, key: &str) -> Option<&str> {
        self.request.query(key)
    }

    /// Nilai field POST (CI: `$this->input->post('field')`).
    pub fn post(&self, key: &str) -> Option<&str> {
        self.request.post(key)
    }

    /// Seluruh data POST — untuk diberikan ke `Validator::new(...)`.
    pub fn post_data(&self) -> &std::collections::HashMap<String, String> {
        &self.request.post
    }

    /// Berkas unggah pertama untuk sebuah field (CI: `$this->upload`).
    pub fn file(&self, field: &str) -> Option<&UploadedFile> {
        self.request.file(field)
    }

    /// Seluruh berkas unggah.
    pub fn files(&self) -> &[UploadedFile] {
        &self.request.files
    }

    /// Metode HTTP request ("GET", "POST", "PUT", "DELETE", ...).
    pub fn method(&self) -> &str {
        &self.request.method
    }

    /// Header request berdasarkan nama (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.request.header(name)
    }

    /// Body JSON terurai (REST API), bila ada.
    pub fn json(&self) -> Option<&Value> {
        self.request.json.as_ref()
    }

    /// Ambil satu input dari body — cek field POST dulu, lalu field objek JSON.
    /// Memungkinkan endpoint menerima form maupun JSON.
    pub fn input(&self, key: &str) -> Option<String> {
        if let Some(v) = self.request.post(key) {
            return Some(v.to_string());
        }
        if let Some(Value::Object(obj)) = &self.request.json {
            return obj.get(key).map(json_scalar_to_string);
        }
        None
    }

    /// Gabungan input (POST + objek JSON) sebagai map string — untuk `Validator`.
    pub fn input_map(&self) -> std::collections::HashMap<String, String> {
        let mut map = self.request.post.clone();
        if let Some(Value::Object(obj)) = &self.request.json {
            for (k, v) in obj {
                map.entry(k.clone()).or_insert_with(|| json_scalar_to_string(v));
            }
        }
        map
    }

    /// Set satu variabel view (CI: `$data['key'] = ...`).
    pub fn set(&mut self, key: &str, value: impl Into<Value>) -> &mut Self {
        self.vars.insert(key.to_string(), value.into());
        self
    }

    /// Gabungkan objek JSON ke variabel view (CI: `$this->load->vars($array)`).
    pub fn vars(&mut self, data: Value) -> &mut Self {
        if let Value::Object(map) = data {
            for (k, v) in map {
                self.vars.insert(k, v);
            }
        }
        self
    }

    /// Render view dan kembalikan sebagai Response HTML
    /// (CI: `$this->load->view('nama', $data)`). `data` digabung di atas variabel terkumpul.
    pub fn view(&self, name: &str, data: Value) -> Response {
        let mut merged = self.vars.clone();
        // Auto-inject token CSRF & flashdata agar tersedia di semua template
        // (bisa ditimpa oleh data eksplisit di bawah).
        merged.insert("csrf_token".to_string(), Value::String(self.session.csrf_token()));
        merged.insert("flash".to_string(), Value::Object(self.session.flash_all()));
        if let Value::Object(map) = data {
            for (k, v) in map {
                merged.insert(k, v);
            }
        }
        let value = Value::Object(merged);
        match self.view.render(name, &value) {
            Ok(html) => Response::html(html),
            Err(e) => Response::text(500, format!("View error: {e}")),
        }
    }
}

/// Ubah nilai JSON skalar menjadi string (string tanpa tanda kutip, lainnya apa adanya).
fn json_scalar_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
