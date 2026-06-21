# Sessions

Session menyimpan data per-pengunjung antar-request, meniru `CI_Session` (`$this->session`).
Id sesi disimpan di cookie `ri_session`; datanya disimpan di sisi server (store in-memory —
hilang saat restart; cukup untuk dev, bisa diganti backend file/DB nanti).

## Tiga Jenis Data

| Jenis | Umur | API |
|---|---|---|
| **userdata** | bertahan antar-request | `set` / `get` / `remove` |
| **flashdata** | hanya request berikutnya, lalu hilang | `set_flash` / `flash` |
| **csrf** | token per sesi | `csrf_token()` (lihat [Security](../general/security.md)) |

## Userdata

```rust
ctx.session.set("user_id", 7);
ctx.session.set("username", "admin");

ctx.session.get("user_id");           // Option<&Value>
ctx.session.get_str("username");      // Option<String>
ctx.session.has("user_id");           // bool
ctx.session.remove("user_id");
```

## Flashdata (untuk pesan sekali tampil)

Flashdata cocok untuk pola **Post/Redirect/Get**: set pesan, redirect, tampilkan sekali.

```rust
// Setelah aksi berhasil:
ctx.session.set_flash("success", "Data tersimpan.");
Response::redirect(&ctx.base_url("dashboard"))
```

Flashdata otomatis tersedia di view sebagai objek `flash`:

```html
{% if flash.success %}
<div class="success">{{ flash.success }}</div>
{% endif %}
```

Pesan hanya muncul pada request berikutnya; reload halaman → pesan sudah hilang.

## Logout / Menghapus Sesi

```rust
ctx.session.destroy();                  // hapus semua data + cookie kedaluwarsa
Response::redirect(&ctx.base_url("login"))
```

## Cookie

Cookie sesi diset otomatis (`HttpOnly; Path=/; SameSite=Lax`). Untuk membaca cookie lain:

```rust
ctx.cookie("nama");   // Option<String>
```

## Backend Penyimpanan

Pilih di `config/app.toml`:

```toml
[session]
driver = "memory"   # "memory" (hilang saat restart) | "file" (persisten)
path = "storage/sessions"
```

- **memory** — cepat, cocok untuk dev; hilang saat restart.
- **file** — tiap sesi disimpan sebagai `storage/sessions/<id>.json`; **bertahan antar-restart**.

Backend lain (DB/Redis) bisa ditambahkan di `system/session.rs` (`SessionStore`/`Backend`).
