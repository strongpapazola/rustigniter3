//! Welcome — controller default (analog `application/controllers/Welcome.php`).
//!
//! Dipetakan dari URL:
//!   http://127.0.0.1:8080/            (lewat default_controller)
//!   http://127.0.0.1:8080/welcome
//!   http://127.0.0.1:8080/welcome/index
//!
//! Method publik tanpa prefix khusus menjadi action. Karena Rust tak punya refleksi,
//! pemetaan action -> method ditulis eksplisit di `dispatch`.

use crate::system::{Ctx, Response};
use serde_json::json;

pub struct Welcome;

// Dispatch otomatis: aksi "index" -> Welcome::index. Aksi lain -> 404.
crate::actions!(Welcome { index });

impl Welcome {
    /// Halaman index controller ini (CI: `$this->load->view('welcome_message')`).
    fn index(&self, ctx: &mut Ctx) -> Response {
        let app_name = ctx
            .config_item("custom.app_name")
            .unwrap_or_else(|| "RustIgniter".to_string());

        ctx.view(
            "welcome_message",
            json!({
                "app_name": app_name,
                "version": "3.0.0",
            }),
        )
    }
}
