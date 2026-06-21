//! Auth — login/logout berbasis session + halaman terproteksi.
//!
//! Rute (lewat custom route di routes.toml):
//!   GET  /login      -> form login
//!   POST /login      -> proses login (set session, redirect /dashboard)
//!   GET  /logout     -> hapus session, redirect /login
//!   GET  /dashboard  -> halaman terproteksi (redirect /login bila belum masuk)

use crate::app::models::user::User;
use crate::system::{Ctx, Response, Validator};
use serde_json::json;

pub struct Auth;

crate::actions!(Auth { login, logout, dashboard });

impl Auth {
    fn login(&self, ctx: &mut Ctx) -> Response {
        if ctx.method() != "POST" {
            return self.login_form(ctx, Vec::new());
        }

        // Validasi field wajib.
        let input = ctx.input_map();
        let errors = Validator::new(&input)
            .rule("username", "Username", "required")
            .rule("password", "Password", "required")
            .validate();
        if !errors.is_empty() {
            return self.login_form(ctx, errors.messages());
        }

        let username = ctx.post("username").unwrap_or("").trim().to_string();
        let password = ctx.post("password").unwrap_or("").to_string();

        match User::verify(ctx.db(), &username, &password) {
            Ok(Some(user)) => {
                let id = user.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                ctx.session.set("user_id", id);
                ctx.session.set("username", username);
                ctx.session.set_flash("success", "Berhasil masuk.");
                Response::redirect(&ctx.base_url("dashboard"))
            }
            Ok(None) => self.login_form(ctx, vec!["Username atau password salah.".to_string()]),
            Err(e) => Response::text(500, format!("DB error: {e}")),
        }
    }

    fn login_form(&self, ctx: &mut Ctx, errors: Vec<String>) -> Response {
        let invalid = !errors.is_empty();
        let mut resp = ctx.view("login", json!({ "errors": errors }));
        if invalid {
            resp.status = 422;
        }
        resp
    }

    fn logout(&self, ctx: &mut Ctx) -> Response {
        ctx.session.destroy();
        Response::redirect(&ctx.base_url("login"))
    }

    fn dashboard(&self, ctx: &mut Ctx) -> Response {
        // Proteksi: belum login -> ke /login.
        if !ctx.session.has("user_id") {
            return Response::redirect(&ctx.base_url("login"));
        }
        let username = ctx.session.get_str("username").unwrap_or_default();
        ctx.view("dashboard", json!({ "username": username }))
    }
}
