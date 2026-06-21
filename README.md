# RustIgniter 3.0.0

Sebuah framework web untuk **Rust**, terinspirasi ide-ide **CodeIgniter 3** — MVC,
routing `class/method/id`, Loader, Config, Query Builder, Form Validation, REST resource,
dan Hooks — ditulis idiomatik di atas `tokio` + `hyper`.

```
Request → Router → Hook.before → Controller/Resource → Hook.after → Response
                                      │
                                      ├─ Ctx ($this): config · view · db · input
                                      ├─ View (minijinja, auto-escape)
                                      └─ Database + Query Builder + Model
```

## Mulai cepat

```bash
cargo run                   # jalankan server di http://127.0.0.1:8099
cargo run -- migrate:status # status migrasi
cargo run -- migrate        # terapkan migrasi
cargo test                  # menjalankan unit test
```

- `/` — halaman selamat datang
- `/notes` — demo CRUD HTML (form + validasi)
- `/api/notes` — demo REST API (JSON)

## Fitur

| Area | Status |
|---|---|
| MVC + front controller | ✅ |
| URI Routing (default, custom `(:any)`/`(:num)`, 404 override, dashes) | ✅ |
| Controllers + `Ctx` (pengganti `$this`) | ✅ |
| Views (minijinja, auto-escape XSS) | ✅ |
| Config & Auto-load (TOML) | ✅ |
| Database + Query Builder + Model (SQLite **& PostgreSQL**) | ✅ |
| Form Validation | ✅ |
| REST resource (routing sadar verb, JSON in/out) | ✅ |
| Hooks / Middleware (before/after, halt) | ✅ |
| Session + flashdata (cookie; store memory/file persisten) | ✅ |
| Auth (login/logout, route terproteksi, password **bcrypt**) | ✅ |
| Security: XSS auto-escape, CSRF token | ✅ |
| CLI + migrations berversi (`migrate`/`rollback`/`status`/`seed`) | ✅ |
| Hardening: env dev/prod, cookie `Secure`, static file serving | ✅ |
| Macro `actions!` (auto-dispatch controller) | ✅ |
| Cache (TTL) + Logging berlevel ke berkas | ✅ |
| File upload (multipart) + Storage (lokal / **bucket S3** SigV4) | ✅ |
| Connection pool SQLite (WAL) | ✅ |

## Dokumentasi

User Guide bergaya CodeIgniter 3 ada di **[`user_guide/index.md`](user_guide/index.md)**:

- [Memulai](user_guide/general/getting_started.md)
- [URL](user_guide/general/urls.md) · [Routing](user_guide/general/routing.md)
- [Controllers](user_guide/general/controllers.md) · [Views](user_guide/general/views.md) · [Models](user_guide/general/models.md)
- [Config & Auto-load](user_guide/general/config.md) · [Hooks](user_guide/general/hooks.md) · [CLI & Migrations](user_guide/general/cli.md)
- [Security](user_guide/general/security.md) · [Deployment](user_guide/general/deployment.md)
- [Form Validation](user_guide/libraries/form_validation.md) · [Sessions](user_guide/libraries/sessions.md) · [REST Resource](user_guide/libraries/rest.md)
- [File Upload](user_guide/libraries/uploads.md) · [Cache & Logging](user_guide/libraries/cache_logging.md)
- [Query Builder](user_guide/database/query_builder.md) · [URL Helper](user_guide/helpers/url_helper.md)

## Struktur

```
config/      TOML  (app, routes, autoload, database)     ~ application/config
src/
  main.rs    entry point                                 ~ index.php
  system/    inti framework                              ~ system/core
  app/       controllers, models, views, hooks           ~ application
storage/     berkas database SQLite
user_guide/  dokumentasi
```

## Kebutuhan

- Rust 1.74+ & Cargo
- C compiler (`cc`/`gcc`) untuk SQLite *bundled*

## Lisensi

[Apache License 2.0](LICENSE).
