# Deployment

Hal-hal yang relevan saat menjalankan RustIgniter di luar pengembangan.

## Environment

```toml
# config/app.toml
environment = "production"   # "development" | "production"
```

Pengaruh `production`:
- Cookie sesi diberi flag **`Secure`**.
- Detail error **500 disembunyikan**.

Server menampilkan environment aktif saat start:

```
RustIgniter 3.0.0 berjalan di http://127.0.0.1:8099  [env: production]
```

## Session persisten

Secara default sesi disimpan **in-memory** (hilang saat restart). Untuk produksi (atau agar
login bertahan antar-restart), pakai backend **file**:

```toml
# config/app.toml
[session]
driver = "file"               # "memory" | "file"
path = "storage/sessions"
```

Tiap sesi disimpan sebagai berkas JSON di `storage/sessions/<id>.json`. Lihat
[Sessions](../libraries/sessions.md).

## Static files

Berkas di folder `public/` dilayani pada prefix `/assets/`:

```
public/app.css   →   GET /assets/app.css      (Content-Type: text/css)
public/img/x.png →   GET /assets/img/x.png    (Content-Type: image/png)
```

Dari view:

```html
<link rel="stylesheet" href="{{ base_url('assets/app.css') }}">
```

Path dengan `..` ditolak (proteksi path traversal). Content-Type ditebak dari ekstensi.

## Migrasi saat deploy

Jalankan migrasi sebelum/ saat start:

```bash
cargo run -- migrate
cargo run -- serve
```

`serve` juga menerapkan migrasi tertunda otomatis. Lihat [CLI & Migrations](cli.md).

## Catatan produksi lain (belum disertakan)

Untuk beban tinggi, pertimbangkan: connection pool database, TLS (lewat reverse proxy
seperti nginx/Caddy), dan session store DB/Redis.
