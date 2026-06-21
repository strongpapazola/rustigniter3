//! Hooks aplikasi (analog `application/hooks/` + `config/hooks.php` di CodeIgniter).
//!
//! Empat contoh middleware lintas-cutting:
//! - [`RequestLogger`]  — mencatat tiap request & status response ke stdout.
//! - [`PoweredBy`]      — menambah header `X-Powered-By` ke semua response.
//! - [`CsrfGuard`]      — memvalidasi token CSRF untuk operasi tulis non-API.
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

/// Validasi token CSRF untuk operasi tulis (POST/PUT/PATCH/DELETE) pada path **non-API**.
/// Endpoint `api/*` dikecualikan karena memakai autentikasi API key yang stateless.
/// Token disisipkan ke form lewat field tersembunyi `csrf_token` (lihat view).
pub struct CsrfGuard;

impl Hook for CsrfGuard {
    fn before(&self, ctx: &mut Ctx) -> HookResult {
        let path = ctx.request.path.trim_start_matches('/');
        let is_write = matches!(ctx.method(), "POST" | "PUT" | "PATCH" | "DELETE");

        if is_write && !path.starts_with("api/") {
            let token = ctx.csrf_token();
            let submitted = ctx.post("csrf_token").unwrap_or("");
            if token.is_empty() || token != submitted {
                return HookResult::Halt(Response::text(403, "Token CSRF tidak valid."));
            }
        }
        HookResult::Continue
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
