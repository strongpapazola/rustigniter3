//! Hooks aplikasi (analog `application/hooks/` + `config/hooks.php` di CodeIgniter).
//!
//! Tiga contoh middleware lintas-cutting:
//! - [`RequestLogger`]  — mencatat tiap request & status response ke stdout.
//! - [`PoweredBy`]      — menambah header `X-Powered-By` ke semua response.
//! - [`ApiKeyGuard`]    — menolak operasi tulis pada `/api/*` tanpa header API key.

use crate::system::{Ctx, Hook, HookResult, Response};
use serde_json::json;

/// Catat `→ METHOD /path` sebelum, dan `← status` sesudah.
pub struct RequestLogger;

impl Hook for RequestLogger {
    fn before(&self, ctx: &mut Ctx) -> HookResult {
        println!("→ {} {}", ctx.method(), ctx.request.path);
        HookResult::Continue
    }

    fn after(&self, _ctx: &mut Ctx, response: Response) -> Response {
        println!("← {}", response.status);
        response
    }
}

/// Tambahkan header identitas framework ke setiap response.
pub struct PoweredBy;

impl Hook for PoweredBy {
    fn after(&self, _ctx: &mut Ctx, response: Response) -> Response {
        response.with_header("X-Powered-By", "RustIgniter/3.0.0")
    }
}

/// Lindungi operasi tulis (POST/PUT/PATCH/DELETE) pada path diawali `api/`:
/// wajib menyertakan header `X-Api-Key: rahasia`, jika tidak -> 401 JSON.
pub struct ApiKeyGuard;

impl Hook for ApiKeyGuard {
    fn before(&self, ctx: &mut Ctx) -> HookResult {
        let path = ctx.request.path.trim_start_matches('/');
        let is_api = path.starts_with("api/");
        let is_write = matches!(ctx.method(), "POST" | "PUT" | "PATCH" | "DELETE");

        if is_api && is_write && ctx.header("x-api-key") != Some("rahasia") {
            return HookResult::Halt(Response::json(
                401,
                json!({ "status": 401, "error": "Butuh header X-Api-Key yang valid" }),
            ));
        }
        HookResult::Continue
    }
}
