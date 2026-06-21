# Deployment

Hal-hal yang relevan saat menjalankan RustIgniter di luar pengembangan.

## Docker / Compose

Cara termudah menjalankan RustIgniter untuk publik:

```bash
docker compose up --build
```

Service `rustigniter` membuka port `8099:8099` dan memakai volume:

- `rustigniter-storage` → `/app/storage` (SQLite DB, logs, sessions)
- `rustigniter-uploads` → `/app/public/uploads` (upload lokal)

Compose juga menyediakan service opsional:

```bash
docker compose --profile postgres up -d postgres  # Postgres di localhost:5433
docker compose --profile s3 up -d minio           # MinIO di localhost:9100, console 9101
```

Setelah mengaktifkan profile opsional, ubah `config/database.toml` atau `config/storage.toml`
sesuai kredensial yang ada di `docker-compose.yml`.

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
