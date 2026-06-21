# Security

Ringkasan fitur keamanan RustIgniter dan cara memakainya.

## XSS — Auto-escaping View

Output `{{ ... }}` di template **di-escape otomatis** oleh minijinja, jadi input pengguna
yang mengandung HTML/JS ditampilkan sebagai teks, bukan dieksekusi. Lihat [Views](views.md).

## CSRF — Proteksi Cross-Site Request Forgery

Setiap sesi punya **token CSRF** acak. Operasi tulis (POST/PUT/PATCH/DELETE) pada path
**non-API** wajib menyertakan token yang cocok, jika tidak → **403**. Pengecekan dilakukan
oleh hook [`CsrfGuard`](hooks.md) (`src/app/hooks.rs`).

### Menyisipkan token di form

Token tersedia di setiap view sebagai `csrf_token` (auto-inject). Tambahkan field tersembunyi:

```html
<form action="{{ base_url('notes/add') }}" method="post">
    <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
    …
</form>
```

### Pengecualian API

Endpoint `api/*` **dikecualikan** dari CSRF karena memakai autentikasi stateless
([API key](rest.md) lewat header), bukan cookie sesi. Ganti/atur kebijakan ini di `CsrfGuard`.

## Autentikasi (Login)

Pola login berbasis [session](../libraries/sessions.md). Contoh lengkap ada di
`src/app/controllers/auth.rs` + model `src/app/models/user.rs`.

### Alur

```
GET  /login      → tampilkan form (dengan token CSRF)
POST /login      → verifikasi kredensial → simpan user_id ke session → redirect /dashboard
GET  /dashboard  → cek session.has("user_id"); bila tidak → redirect /login
GET  /logout     → session.destroy() → redirect /login
```

### Verifikasi kredensial

```rust
match User::verify(ctx.db(), &username, &password) {
    Ok(Some(user)) => {
        let id = user.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        ctx.session.set("user_id", id);
        ctx.session.set("username", username);
        ctx.session.set_flash("success", "Berhasil masuk.");
        Response::redirect(&ctx.base_url("dashboard"))
    }
    Ok(None) => /* kredensial salah */,
    Err(e)   => /* error DB */,
}
```

### Melindungi halaman

```rust
fn dashboard(&self, ctx: &mut Ctx) -> Response {
    if !ctx.session.has("user_id") {
        return Response::redirect(&ctx.base_url("login"));
    }
    // … halaman terproteksi
}
```

> Untuk melindungi banyak rute sekaligus, tulis sebuah **hook** `before` yang memeriksa
> `ctx.session.has("user_id")` pada prefix path tertentu dan `Halt` dengan redirect.

### Penyimpanan password

Password di-hash dengan **bcrypt** (`User`/`models/user.rs`) — salt disertakan otomatis di
dalam hash, cost adaptif. Layak produksi.

```rust
let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;   // saat membuat user
bcrypt::verify(password, &stored_hash)?;                    // saat login
```

User demo hasil seed: `admin` / `admin123`.

## Production / Environment

Atur `environment` di `config/app.toml`:

```toml
environment = "development"   # atau "production"
```

Pada `production`:
- Cookie sesi diberi flag **`Secure`** (hanya dikirim lewat HTTPS).
- Respons error **500 disembunyikan** (body diganti "Internal Server Error", detail tidak bocor).

Lihat juga [Deployment](deployment.md) untuk static files & session persisten.
