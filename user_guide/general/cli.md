# CLI & Migrations

RustIgniter punya antarmuka baris-perintah (gaya `php index.php <cmd>` di CodeIgniter) untuk
menjalankan server dan mengelola migrasi database.

## Perintah

```bash
cargo run                  # = serve  (jalankan server)
cargo run -- serve         # jalankan server
cargo run -- migrate           # terapkan migrasi tertunda
cargo run -- migrate:rollback  # batalkan migrasi terakhir
cargo run -- migrate:status    # tampilkan status migrasi
cargo run -- db:seed           # isi data contoh
```

> Saat `serve`, migrasi tertunda **otomatis diterapkan** lalu seed dijalankan (bila
> `seed = true` di `config/database.toml`) — aplikasi langsung siap.

## Contoh `migrate:status`

```
VERSI    NAMA                   STATUS
1        create_notes           ✔ applied
2        create_users           · pending
```

## Menulis Migrasi

Migrasi didefinisikan di `src/app/migrations.rs` — tiap migrasi punya `version`, `name`,
dan fungsi `up`/`down` yang **dialek-aware** (SQLite vs PostgreSQL):

```rust
pub fn all() -> Vec<Migration> {
    vec![
        Migration { version: 1, name: "create_notes", up: up_notes, down: down_notes },
        Migration { version: 2, name: "create_users", up: up_users, down: down_users },
    ]
}

fn up_users(db: &Database) -> Result<(), String> {
    let ddl = match db.dialect() {
        Dialect::Sqlite   => "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, …)",
        Dialect::Postgres => "CREATE TABLE IF NOT EXISTS users (id BIGSERIAL PRIMARY KEY, …)",
    };
    db.execute(ddl, &[]).map(|_| ())
}

fn down_users(db: &Database) -> Result<(), String> {
    db.execute("DROP TABLE IF EXISTS users", &[]).map(|_| ())
}
```

Versi yang sudah diterapkan dicatat di tabel `schema_migrations`. `migrate` menjalankan yang
belum diterapkan (urut versi); `migrate:rollback` menjalankan `down` migrasi tertinggi lalu
menghapus catatannya.

## Seed

Data contoh ada di `app::seed` (`src/app/mod.rs`) — idempotent (hanya mengisi saat tabel
kosong). Jalankan manual dengan `db:seed`.
