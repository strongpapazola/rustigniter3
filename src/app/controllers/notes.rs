//! Notes — controller CRUD contoh di atas lapisan Database.
//!
//! Rute (mapping default class/method/arg):
//!   GET  /notes                -> index  (daftar semua catatan + form)
//!   GET  /notes/show/{id}      -> show    (satu catatan)
//!   POST /notes/add            -> add     (validasi; valid -> INSERT + redirect,
//!                                          invalid -> render ulang index + error)

use crate::app::models::note::Note;
use crate::system::{Controller, Ctx, Response, Validator};
use serde_json::json;

pub struct Notes;

impl Controller for Notes {
    fn dispatch(&self, action: &str, ctx: &mut Ctx) -> Option<Response> {
        match action {
            "index" => Some(self.index(ctx)),
            "show" => Some(self.show(ctx)),
            "add" => Some(self.add(ctx)),
            _ => None,
        }
    }
}

impl Notes {
    fn index(&self, ctx: &mut Ctx) -> Response {
        let q = ctx.query("q").unwrap_or("").trim().to_string();
        let page = ctx
            .query("page")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1)
            .max(1);
        self.render_index(ctx, &q, page, Vec::new(), "")
    }

    /// Render daftar catatan (dengan search + pagination) + form. `errors`/`old_text`
    /// dipakai saat menampilkan kembali setelah validasi gagal (status 422).
    fn render_index(
        &self,
        ctx: &mut Ctx,
        search: &str,
        page: i64,
        errors: Vec<String>,
        old_text: &str,
    ) -> Response {
        const PER_PAGE: i64 = 5;
        let (notes, total) =
            Note::paginate(ctx.db(), search, page, PER_PAGE).unwrap_or((Vec::new(), 0));
        let total_pages = ((total + PER_PAGE - 1) / PER_PAGE).max(1);
        let invalid = !errors.is_empty();

        let mut resp = ctx.view(
            "notes_index",
            json!({
                "notes": notes,
                "errors": errors,
                "old_text": old_text,
                "q": search,
                "page": page,
                "total": total,
                "total_pages": total_pages,
                "has_prev": page > 1,
                "has_next": page < total_pages,
                "prev": page - 1,
                "next": page + 1,
            }),
        );
        if invalid {
            resp.status = 422;
        }
        resp
    }

    fn show(&self, ctx: &mut Ctx) -> Response {
        let id: i64 = match ctx.arg(0).and_then(|s| s.parse().ok()) {
            Some(id) => id,
            None => return Response::text(400, "id catatan tidak valid"),
        };
        match Note::find(ctx.db(), id) {
            Ok(Some(note)) => ctx.view("note_show", json!({ "note": note })),
            Ok(None) => Response::not_found(format!("Catatan #{id} tidak ditemukan")),
            Err(e) => Response::text(500, format!("DB error: {e}")),
        }
    }

    fn add(&self, ctx: &mut Ctx) -> Response {
        // Validasi input POST (CI: set_rules + run).
        let errors = Validator::new(ctx.post_data())
            .rule("text", "Catatan", "required|min_length[3]|max_length[200]")
            .validate();

        let text = ctx.post("text").unwrap_or("").trim().to_string();

        if !errors.is_empty() {
            // Gagal -> tampilkan kembali form (halaman 1, tanpa search) + error + input lama.
            return self.render_index(ctx, "", 1, errors.messages(), &text);
        }

        // Lolos -> simpan lalu redirect (pola Post/Redirect/Get).
        match Note::create(ctx.db(), &text) {
            Ok(_) => Response::redirect(&ctx.base_url("notes")),
            Err(e) => Response::text(500, format!("DB error: {e}")),
        }
    }
}
