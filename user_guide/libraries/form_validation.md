# Form Validation

`Validator` memvalidasi input form/JSON, meniru `$this->form_validation` CodeIgniter:
tetapkan aturan per field, jalankan, lalu baca pesan kesalahan.

## Pemakaian Dasar

```rust
use crate::system::Validator;

let input  = ctx.input_map();   // gabungan POST + JSON
let errors = Validator::new(&input)
    .rule("text",  "Catatan", "required|min_length[3]|max_length[200]")
    .rule("email", "Email",   "required")
    .validate();

if errors.is_empty() {
    // lolos
} else {
    for pesan in errors.messages() {
        // "Catatan minimal 3 karakter.", ...
    }
}
```

## Aturan yang Didukung

| Aturan             | Arti                                  |
|--------------------|---------------------------------------|
| `required`         | tidak boleh kosong                    |
| `min_length[n]`    | minimal `n` karakter                  |
| `max_length[n]`    | maksimal `n` karakter                 |
| `exact_length[n]`  | tepat `n` karakter                    |
| `numeric`          | berupa angka (boleh desimal)          |
| `integer`          | berupa bilangan bulat                 |
| `matches[field]`   | sama dengan nilai field lain          |

Beberapa aturan dirangkai dengan `|`. Selain `required`, aturan dilewati bila nilai kosong
(perilaku yang sama seperti CodeIgniter). Hanya **satu pesan per field** yang dilaporkan
(berhenti di kegagalan pertama).

## Membaca Errors

```rust
errors.is_empty();          // bool
errors.len();               // jumlah field bermasalah
errors.messages();          // Vec<String> semua pesan
errors.for_field("text");   // Option<&str> pesan pertama untuk field
```

## Pola Form + PRG (HTML)

```rust
fn add(&self, ctx: &mut Ctx) -> Response {
    let errors = Validator::new(ctx.post_data())
        .rule("text", "Catatan", "required|min_length[3]|max_length[200]")
        .validate();
    let text = ctx.post("text").unwrap_or("").trim().to_string();

    if !errors.is_empty() {
        // tampilkan ulang form dengan error + input lama (status 422)
        return self.render_index(ctx, errors.messages(), &text);
    }
    // valid -> simpan lalu redirect (Post/Redirect/Get)
    Note::create(ctx.db(), &text).ok();
    Response::redirect(&ctx.base_url("notes"))
}
```

Di view, tampilkan pesan:

```html
{% if errors %}
<div class="errors"><ul>{% for e in errors %}<li>{{ e }}</li>{% endfor %}</ul></div>
{% endif %}
<input name="text" value="{{ old_text }}">
```

## Validasi untuk REST/JSON

Karena `ctx.input_map()` menggabungkan POST dan body JSON, validator yang sama bekerja
untuk endpoint JSON — balas `Response::json(422, json!({ "errors": errors.messages() }))`.
