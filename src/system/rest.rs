//! REST — resource controller yang sadar metode HTTP.
//!
//! Router RustIgniter (seperti CodeIgniter) memetakan URL ke controller/method tanpa
//! melihat verb HTTP. Untuk REST, satu URL bisa berarti aksi berbeda tergantung verb:
//!
//! | Verb + URL                | Aksi                |
//! |---------------------------|---------------------|
//! | GET    /api/notes         | `index`  (daftar)   |
//! | POST   /api/notes         | `create`            |
//! | GET    /api/notes/{id}    | `show`              |
//! | PUT/PATCH /api/notes/{id} | `update`            |
//! | DELETE /api/notes/{id}    | `delete`            |
//!
//! Pola: implementasikan [`RestController`] (hanya aksi yang didukung; sisanya default 405),
//! lalu bungkus dengan [`Resource`] dan daftarkan ke registry. Arahkan URL resource ke
//! controller itu lewat custom route (lihat `config/routes.toml`), mis.
//! `api/notes` dan `api/notes/(:num)` -> `notes_api/_resource`.

use crate::system::{Controller, Ctx, Response};
use serde_json::json;

/// Kontrak resource REST. Setiap method punya default "405 Method Not Allowed",
/// jadi resource cukup mengimplementasikan aksi yang ia dukung.
pub trait RestController: Send + Sync {
    fn index(&self, _ctx: &mut Ctx) -> Response {
        method_not_allowed()
    }
    fn show(&self, _ctx: &mut Ctx, _id: &str) -> Response {
        method_not_allowed()
    }
    fn create(&self, _ctx: &mut Ctx) -> Response {
        method_not_allowed()
    }
    fn update(&self, _ctx: &mut Ctx, _id: &str) -> Response {
        method_not_allowed()
    }
    fn delete(&self, _ctx: &mut Ctx, _id: &str) -> Response {
        method_not_allowed()
    }
}

/// Respons JSON 405 standar.
pub fn method_not_allowed() -> Response {
    Response::json(405, json!({ "status": 405, "error": "Metode tidak diizinkan" }))
}

/// Adapter yang menjadikan sebuah [`RestController`] bisa dipanggil sebagai [`Controller`]
/// biasa, dengan memetakan (verb HTTP, ada/tidaknya id) ke aksi resource.
pub struct Resource<T: RestController> {
    inner: T,
}

impl<T: RestController> Resource<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: RestController> Controller for Resource<T> {
    fn dispatch(&self, _action: &str, ctx: &mut Ctx) -> Option<Response> {
        // Ambil verb & id lebih dulu sebagai nilai milik sendiri agar pinjaman `ctx`
        // selesai sebelum aksi memakai `&mut ctx`.
        let method = ctx.method().to_ascii_uppercase();
        let id = ctx.arg(0).map(str::to_string);

        let response = match (id, method.as_str()) {
            (None, "GET") => self.inner.index(ctx),
            (None, "POST") => self.inner.create(ctx),
            (Some(id), "GET") => self.inner.show(ctx, &id),
            (Some(id), "PUT") | (Some(id), "PATCH") => self.inner.update(ctx, &id),
            (Some(id), "DELETE") => self.inner.delete(ctx, &id),
            _ => method_not_allowed(),
        };
        Some(response)
    }
}
