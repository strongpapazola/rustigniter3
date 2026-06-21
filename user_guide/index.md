# RustIgniter User Guide

**RustIgniter** adalah framework web untuk **Rust** yang mengambil ide-ide
**CodeIgniter 3** — MVC, routing `class/method/id`, Loader, Config, Form Validation,
Hooks — namun ditulis idiomatik Rust di atas `tokio` + `hyper`.

> Dokumentasi ini disusun mengikuti gaya *CodeIgniter 3 User Guide*: ringkas per-topik,
> banyak contoh, langsung bisa dipraktikkan.

---

## Selamat Datang

RustIgniter adalah framework *application development* yang membantu kamu membangun
aplikasi web lebih cepat dengan menyediakan kerangka MVC, routing, akses database lewat
Query Builder, validasi form, REST resource, dan sistem hook — semuanya dengan API yang
terasa familier bagi pengguna CodeIgniter, tetapi aman secara tipe dan cepat.

### Kebutuhan Sistem

- **Rust** 1.74+ (diuji pada 1.96) dan **Cargo**
- Sebuah **C compiler** (`cc`/`gcc`) — diperlukan oleh `rusqlite` (SQLite *bundled*)
- (Opsional) **PostgreSQL** bila memakai driver `postgres` — klien pure-Rust, tanpa libpq

---

## Daftar Isi

### Memulai
- [Memulai (Instalasi & Menjalankan)](general/getting_started.md)

### Topik Umum
- [URL RustIgniter](general/urls.md)
- [URI Routing](general/routing.md)
- [Controllers](general/controllers.md)
- [Views](general/views.md)
- [Models](general/models.md)
- [Config & Auto-load](general/config.md)
- [Hooks — Memperluas Inti Framework](general/hooks.md)

### Referensi Library
- [Form Validation](libraries/form_validation.md)
- [REST Resource](libraries/rest.md)

### Referensi Database
- [Query Builder](database/query_builder.md)

### Referensi Helper
- [URL Helper](helpers/url_helper.md)

---

## Arsitektur singkat

```
Request HTTP
   │
   ▼
main.rs (serve)  ── baca body & header (async)
   │
   ▼
App::handle ──► Router ──► Hook.before ──► Controller/Resource ──► Hook.after ──► Response
                                              │
                                              ├── Ctx ($this): config, view, db, input
                                              ├── Loader/View (minijinja)
                                              └── Database + Query Builder + Model
```

| CodeIgniter 3 | RustIgniter |
|---|---|
| `index.php` + `system/core/CodeIgniter.php` | `src/main.rs` + `src/system/mod.rs` |
| `system/core/` | `src/system/` |
| `application/` | `src/app/` |
| `application/config/*.php` | `config/*.toml` |
| `$this` (controller) | `Ctx` |
| `$this->load->view()` | `ctx.view()` |
| `$this->db` + Query Builder (driver SQLite/Postgres) | `ctx.db().table(...)` |
| `$this->form_validation` | `Validator` |
| Hooks | `Hook` (before/after) |
