# Controllers

Controller adalah inti aplikasimu: ia menentukan bagaimana sebuah request ditangani.

## Controller Minimal

```rust
// src/app/controllers/welcome.rs
use crate::system::{Ctx, Response};
use serde_json::json;

pub struct Welcome;

// Bangkitkan dispatch: aksi "index" -> Welcome::index. Aksi lain -> 404.
crate::actions!(Welcome { index });

impl Welcome {
    fn index(&self, ctx: &mut Ctx) -> Response {
        ctx.view("welcome_message", json!({ "app_name": "RustIgniter" }))
    }
}
```

### Macro `actions!`

CodeIgniter memetakan nama method dari URL ke method PHP lewat **refleksi runtime**. Rust
tidak punya refleksi, jadi pemetaan `action → fungsi` dibuat eksplisit. Macro
`actions!(Ctrl { a, b, c })` membangkitkan `impl Controller` yang mencocokkan nama aksi ke
method bernama sama (signature `fn(&self, &mut Ctx) -> Response`); aksi tak dikenal → 404.

Tanpa macro, kamu bisa menulis `impl Controller` manual dengan `match action { ... }` — macro
hanya menghapus boilerplate itu.

## Mendaftarkan Controller

Tambahkan satu baris di `src/app/mod.rs`:

```rust
pub fn register(registry: &mut Registry) {
    registry.register("welcome", Box::new(Welcome));
    registry.register("notes", Box::new(Notes));
}
```

Nama registry (`"welcome"`) adalah segmen controller pada URL, *case-insensitive*.

## `Ctx` — pengganti `$this`

Objek `Ctx` memberi akses ke semua yang dibutuhkan aksi:

| Method | Guna | Padanan CI |
|---|---|---|
| `ctx.view(name, data)` | render view → Response | `$this->load->view()` |
| `ctx.db()` | akses database | `$this->db` |
| `ctx.config_item(key)` | item config | `$this->config->item()` |
| `ctx.base_url(uri)` / `ctx.site_url(uri)` | URL helper | `base_url()` |
| `ctx.arg(i)` | argumen URL (0-based) | `$this->uri->segment()` |
| `ctx.query(key)` | query string | `$this->input->get()` |
| `ctx.post(key)` / `ctx.post_data()` | field POST | `$this->input->post()` |
| `ctx.input(key)` / `ctx.input_map()` | input gabungan (POST + JSON) | — |
| `ctx.json()` | body JSON | — |
| `ctx.method()` | metode HTTP | `$this->input->method()` |
| `ctx.header(name)` | header request | `get_request_header()` |
| `ctx.set(k, v)` / `ctx.vars(obj)` | variabel view | `$this->load->vars()` |

## Mengembalikan Response

```rust
Response::html("<h1>Hai</h1>")          // 200 text/html
Response::text(404, "Tidak ada")        // status + text/plain
Response::json(201, json!({"ok": true}))// JSON
Response::redirect("/notes")            // 302
Response::not_found("hilang")           // 404 HTML
```
