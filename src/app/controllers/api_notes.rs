//! ApiNotes — REST API untuk resource catatan (JSON in/out).
//!
//! Dipetakan lewat custom route (lihat `config/routes.toml`) ke resource `notes_api`:
//!   GET    /api/notes        -> index   (200, daftar)
//!   POST   /api/notes        -> create  (201 / 422)
//!   GET    /api/notes/{id}   -> show    (200 / 404)
//!   PUT    /api/notes/{id}   -> update  (200 / 404 / 422)
//!   DELETE /api/notes/{id}   -> delete  (200 / 404)
//!
//! Menerima body JSON maupun form (lewat `ctx.input_map()`), selalu membalas JSON.

use crate::app::models::note::Note;
use crate::system::{Ctx, Response, RestController, Validator};
use serde_json::json;

pub struct ApiNotes;

impl RestController for ApiNotes {
    fn index(&self, ctx: &mut Ctx) -> Response {
        match Note::all(ctx.db()) {
            Ok(notes) => Response::json(200, json!({ "data": notes })),
            Err(e) => server_error(e),
        }
    }

    fn show(&self, ctx: &mut Ctx, id: &str) -> Response {
        let id = match parse_id(id) {
            Ok(id) => id,
            Err(resp) => return resp,
        };
        match Note::find(ctx.db(), id) {
            Ok(Some(note)) => Response::json(200, json!({ "data": note })),
            Ok(None) => not_found(id),
            Err(e) => server_error(e),
        }
    }

    fn create(&self, ctx: &mut Ctx) -> Response {
        let text = match validate_text(ctx) {
            Ok(text) => text,
            Err(resp) => return resp,
        };
        match Note::create(ctx.db(), &text) {
            Ok(id) => Response::json(201, json!({ "data": { "id": id, "text": text } })),
            Err(e) => server_error(e),
        }
    }

    fn update(&self, ctx: &mut Ctx, id: &str) -> Response {
        let id = match parse_id(id) {
            Ok(id) => id,
            Err(resp) => return resp,
        };
        let text = match validate_text(ctx) {
            Ok(text) => text,
            Err(resp) => return resp,
        };
        match Note::update(ctx.db(), id, &text) {
            Ok(0) => not_found(id),
            Ok(_) => Response::json(200, json!({ "data": { "id": id, "text": text } })),
            Err(e) => server_error(e),
        }
    }

    fn delete(&self, ctx: &mut Ctx, id: &str) -> Response {
        let id = match parse_id(id) {
            Ok(id) => id,
            Err(resp) => return resp,
        };
        match Note::delete(ctx.db(), id) {
            Ok(0) => not_found(id),
            Ok(_) => Response::json(200, json!({ "deleted": id })),
            Err(e) => server_error(e),
        }
    }
}

/// Validasi field `text` dari input (JSON/form). `Ok(text)` atau respons 422.
fn validate_text(ctx: &Ctx) -> Result<String, Response> {
    let input = ctx.input_map();
    let errors = Validator::new(&input)
        .rule("text", "text", "required|min_length[3]|max_length[200]")
        .validate();
    if !errors.is_empty() {
        return Err(Response::json(422, json!({ "errors": errors.messages() })));
    }
    Ok(input.get("text").map(|s| s.trim().to_string()).unwrap_or_default())
}

fn parse_id(id: &str) -> Result<i64, Response> {
    id.parse::<i64>()
        .map_err(|_| Response::json(400, json!({ "error": "id tidak valid" })))
}

fn not_found(id: i64) -> Response {
    Response::json(404, json!({ "error": format!("Catatan #{id} tidak ditemukan") }))
}

fn server_error(e: String) -> Response {
    Response::json(500, json!({ "error": e }))
}
