//! Uploads — demo unggah berkas ke penyimpanan aktif (lokal atau bucket S3).
//!
//!   GET  /uploads        -> form unggah + daftar (dari DB)
//!   POST /uploads/save    -> simpan via Storage (local/s3), catat ke DB, redirect
//!
//! Karena daftar dibaca dari tabel `uploads`, tampilannya sama untuk semua backend.

use crate::app::models::upload::Upload;
use crate::system::session::random_token;
use crate::system::{Ctx, Response};
use serde_json::json;

pub struct Uploads;

crate::actions!(Uploads { index, save });

impl Uploads {
    fn index(&self, ctx: &mut Ctx) -> Response {
        let files = Upload::all(ctx.db()).unwrap_or_default();
        let driver = ctx.storage().driver();
        ctx.view("uploads", json!({ "files": files, "driver": driver }))
    }

    fn save(&self, ctx: &mut Ctx) -> Response {
        // Salin data berkas ke nilai milik sendiri agar pinjaman `ctx` selesai
        // sebelum kita memakai `ctx` secara mutable (session/flash).
        let (filename, bytes, content_type) = match ctx.file("berkas") {
            Some(f) if !f.bytes.is_empty() => {
                (safe_filename(&f.filename), f.bytes.clone(), f.content_type.clone())
            }
            _ => {
                ctx.session.set_flash("error", "Tidak ada berkas dipilih.");
                return Response::redirect(&ctx.base_url("uploads"));
            }
        };

        // Key unik agar tak saling menimpa.
        let key = format!("{}-{}", &random_token()[..8], filename);

        match ctx.storage().put(&key, &bytes, &content_type) {
            Ok(url) => {
                let _ = Upload::create(ctx.db(), &filename, &url);
                ctx.log().info(&format!(
                    "upload [{}]: {key} ({} bytes) -> {url}",
                    ctx.storage().driver(),
                    bytes.len()
                ));
                ctx.session
                    .set_flash("success", format!("Terunggah ({}): {url}", ctx.storage().driver()));
            }
            Err(e) => {
                ctx.log().error(&format!("upload gagal: {e}"));
                ctx.session.set_flash("error", format!("Gagal mengunggah: {e}"));
            }
        }
        Response::redirect(&ctx.base_url("uploads"))
    }
}

/// Ambil basename & buang karakter berbahaya (cegah path traversal).
fn safe_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("file");
    let cleaned: String = base
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    if cleaned.is_empty() {
        "file".to_string()
    } else {
        cleaned
    }
}
