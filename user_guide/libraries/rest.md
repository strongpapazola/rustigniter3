# REST Resource

RestController + `Resource` memungkinkanmu membangun REST API dengan routing **sadar metode
HTTP** — satu URL bisa berarti aksi berbeda tergantung verb.

## Peta Verb → Aksi

| Verb + URL                | Aksi resource |
|---------------------------|---------------|
| `GET /api/notes`          | `index`       |
| `POST /api/notes`         | `create`      |
| `GET /api/notes/{id}`     | `show`        |
| `PUT`/`PATCH /api/notes/{id}` | `update`  |
| `DELETE /api/notes/{id}`  | `delete`      |

## Mengimplementasikan Resource

Implementasikan hanya aksi yang didukung; sisanya otomatis **405 Method Not Allowed**:

```rust
// src/app/controllers/api_notes.rs
use crate::system::{Ctx, Response, RestController, Validator};
use serde_json::json;

pub struct ApiNotes;

impl RestController for ApiNotes {
    fn index(&self, ctx: &mut Ctx) -> Response {
        match Note::all(ctx.db()) {
            Ok(notes) => Response::json(200, json!({ "data": notes })),
            Err(e)    => Response::json(500, json!({ "error": e })),
        }
    }

    fn show(&self, ctx: &mut Ctx, id: &str) -> Response {
        let id: i64 = id.parse().unwrap_or(0);
        match Note::find(ctx.db(), id) {
            Ok(Some(n)) => Response::json(200, json!({ "data": n })),
            Ok(None)    => Response::json(404, json!({ "error": "tidak ditemukan" })),
            Err(e)      => Response::json(500, json!({ "error": e })),
        }
    }

    fn create(&self, ctx: &mut Ctx) -> Response {
        let input  = ctx.input_map();
        let errors = Validator::new(&input)
            .rule("text", "text", "required|min_length[3]")
            .validate();
        if !errors.is_empty() {
            return Response::json(422, json!({ "errors": errors.messages() }));
        }
        let text = input.get("text").cloned().unwrap_or_default();
        match Note::create(ctx.db(), text.trim()) {
            Ok(id) => Response::json(201, json!({ "data": { "id": id, "text": text } })),
            Err(e) => Response::json(500, json!({ "error": e })),
        }
    }

    // update(ctx, id), delete(ctx, id) -> serupa
}
```

## Mendaftarkan + Routing

Bungkus dengan `Resource` dan daftarkan seperti controller biasa:

```rust
// src/app/mod.rs
registry.register("notes_api", Box::new(Resource::new(ApiNotes)));
```

Arahkan URL resource ke controller itu lewat custom route (`config/routes.toml`):

```toml
[[routes]]
from = "api/notes"
to   = "notes_api/_resource"

[[routes]]
from = "api/notes/(:num)"
to   = "notes_api/_resource/$1"
```

> Nama aksi `_resource` hanya placeholder — `Resource` menentukan aksi nyata dari **verb
> HTTP** dan ada/tidaknya id.

## Body JSON

Endpoint menerima `Content-Type: application/json`. Akses lewat:

```rust
ctx.json();         // Option<&serde_json::Value> — body mentah
ctx.input("text");  // Option<String> — field dari POST atau JSON
ctx.input_map();    // HashMap untuk Validator
```

## Status Code yang Umum

`200` OK · `201` Created · `400` Bad Request · `404` Not Found · `422` Unprocessable
Entity (validasi) · `405` Method Not Allowed (default RestController).
