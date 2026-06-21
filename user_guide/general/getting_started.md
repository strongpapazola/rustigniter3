# Memulai

## Kebutuhan

- Rust 1.74+ dan Cargo
- C compiler (`cc`/`gcc`) untuk SQLite *bundled*
- (Opsional) server **PostgreSQL** bila memakai driver `postgres` — lihat
  [Config & Auto-load](config.md) dan [Query Builder](../database/query_builder.md)

## Menjalankan

Dari dalam folder proyek (`RustIgniter-3.0.0/`):

```bash
cargo run
```

Server akan menyala (default `http://127.0.0.1:8099`):

```
RustIgniter 3.0.0 berjalan di http://127.0.0.1:8099
```

Buka di browser:

- `/` — halaman selamat datang (controller `Welcome`)
- `/notes` — demo CRUD berbasis HTML (form + validasi)
- `/api/notes` — demo REST API (JSON)

> **Catatan:** semua path konfigurasi (`config/`, `src/app/views/`, `storage/`) relatif
> terhadap *current working directory*. Jalankan `cargo run` dari root proyek.

## Struktur Direktori

```
config/                konfigurasi (TOML)  ~ application/config
  app.toml             base_url, port           ~ config.php
  routes.toml          routing                  ~ routes.php
  autoload.toml        helper yang dipreload    ~ autoload.php
  database.toml        koneksi database         ~ database.php
src/
  main.rs              entry point              ~ index.php
  system/              inti framework           ~ system/core
  app/                 kode aplikasimu          ~ application/
    controllers/
    models/
    views/
    hooks.rs
storage/               berkas database SQLite
user_guide/            dokumentasi ini
```

## Mengubah Port / Base URL

Edit `config/app.toml`:

```toml
base_url = "http://127.0.0.1:8099/"

[server]
host = "127.0.0.1"
port = 8099
```

## Menjalankan Test

```bash
cargo test
```
