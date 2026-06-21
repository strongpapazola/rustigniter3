//! `system` — inti framework RustIgniter (analog `system/core` di CodeIgniter).
//!
//! Modul ini merangkai semua komponen menjadi satu [`App`]: ia menerima [`Request`],
//! meresolusi route, men-dispatch ke controller yang tepat, dan menangani kasus 404.
//! `main.rs` (analog `index.php`) hanya bertugas bootstrap + menyalakan server.

pub mod config;
pub mod controller;
pub mod database;
pub mod hooks;
pub mod loader;
pub mod migration;
pub mod query;
pub mod registry;
pub mod request;
pub mod response;
pub mod rest;
pub mod router;
pub mod session;
pub mod validation;
pub mod view;

pub use config::Config;
pub use controller::{Controller, Ctx};
pub use database::{Database, Dialect};
pub use hooks::{Hook, HookResult};
pub use migration::{Migration, Migrator};
pub use registry::Registry;
pub use request::Request;
pub use response::Response;
pub use rest::{Resource, RestController};
pub use router::{Dispatch, Router, RoutesConfig};
pub use session::SessionStore;
pub use validation::Validator;
pub use view::View;

/// Nama cookie sesi.
pub const SESSION_COOKIE: &str = "ri_session";

/// Aplikasi RustIgniter yang sudah dirakit dan siap melayani request.
pub struct App {
    pub config: Config,
    pub router: Router,
    pub view: View,
    pub registry: Registry,
    pub database: Database,
    pub hooks: Vec<Box<dyn Hook>>,
    pub sessions: SessionStore,
}

impl App {
    /// Proses satu request menjadi response.
    /// Alur: muat sesi -> hook `before` -> dispatch (atau 404) -> hook `after` -> simpan sesi.
    pub fn handle(&self, request: Request) -> Response {
        let dispatch = self.router.resolve(&request.segments);

        // Muat sesi dari cookie (atau buat baru).
        let session = self.sessions.load(request.cookie(SESSION_COOKIE));

        let mut ctx = Ctx::new(
            &request,
            dispatch.args.clone(),
            &self.config,
            &self.view,
            &self.database,
            session,
        );

        // 1) Hook `before` — boleh men-short-circuit (mis. auth/CSRF).
        let mut halted = None;
        for hook in &self.hooks {
            if let HookResult::Halt(resp) = hook.before(&mut ctx) {
                halted = Some(resp);
                break;
            }
        }

        // 2) Dispatch ke controller bila tidak di-halt.
        let mut response = match halted {
            Some(resp) => resp,
            None => self.dispatch_or_404(&dispatch, &mut ctx),
        };

        // 3) Hook `after` — selalu dijalankan (logging, header, dll).
        for hook in &self.hooks {
            response = hook.after(&mut ctx, response);
        }

        // 4) Hardening: di production, sembunyikan detail error 500.
        if self.config.is_production() && response.status >= 500 {
            response = Response::text(response.status, "Internal Server Error");
        }

        // 5) Simpan sesi + atur cookie (Secure di production).
        let secure = self.config.is_production();
        self.sessions.save(&ctx.session);
        if ctx.session.destroyed() {
            response = response.with_header("Set-Cookie", &expire_session_cookie(secure));
        } else if ctx.session.is_new() {
            response = response.with_header("Set-Cookie", &session_cookie(&ctx.session.id, secure));
        }
        response
    }

    /// Jalankan controller hasil resolusi; bila tak ada, coba `not_found_override`,
    /// lalu fallback 404 bawaan.
    fn dispatch_or_404(&self, dispatch: &Dispatch, ctx: &mut Ctx) -> Response {
        if let Some(controller) = self.registry.get(&dispatch.controller) {
            if let Some(response) = controller.dispatch(&dispatch.method, ctx) {
                return response;
            }
        }

        if let Some(override_dispatch) = self.router.not_found_override() {
            if let Some(controller) = self.registry.get(&override_dispatch.controller) {
                ctx.args = override_dispatch.args.clone();
                if let Some(response) = controller.dispatch(&override_dispatch.method, ctx) {
                    return response;
                }
            }
        }

        Response::not_found(default_404_page(dispatch))
    }
}

/// Nilai header Set-Cookie untuk sesi (HttpOnly + SameSite=Lax; `Secure` di production).
fn session_cookie(id: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{SESSION_COOKIE}={id}; Path=/; HttpOnly; SameSite=Lax{secure}")
}

/// Set-Cookie untuk menghapus cookie sesi (logout).
fn expire_session_cookie(secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{SESSION_COOKIE}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{secure}")
}

/// Halaman 404 bawaan saat tidak ada controller/override yang cocok.
fn default_404_page(dispatch: &Dispatch) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>404 Not Found</title>\
         <style>body{{font-family:system-ui,sans-serif;margin:3rem;color:#333}}\
         code{{background:#f4f4f4;padding:.15rem .35rem;border-radius:4px}}</style></head>\
         <body><h1>404 — Halaman Tidak Ditemukan</h1>\
         <p>RustIgniter tidak menemukan controller <code>{}</code> dengan method <code>{}</code>.</p>\
         </body></html>",
        dispatch.controller, dispatch.method
    )
}
