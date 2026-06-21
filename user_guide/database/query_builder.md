# Query Builder

Query Builder bergaya *Active Record* CodeIgniter: rangkai klausa lalu jalankan. Semua nilai
di-*bind* sebagai parameter sehingga aman dari SQL injection; nama tabel & kolom dikutip
identifier.

## Driver

Query Builder bekerja di atas abstraksi `Database` (`Arc<dyn Driver>`), jadi kode yang sama
jalan di banyak backend. Pilih driver di `config/database.toml`:

| Driver | Crate | Placeholder | Insert id |
|---|---|---|---|
| **SQLite** (default) | `rusqlite` (bundled) | `?` | `last_insert_rowid()` |
| **PostgreSQL** | `tokio-postgres` (pure-Rust, NoTls) | `$1`, `$2`, … | `RETURNING id` |

Perbedaan dialek (placeholder, auto-increment) ditangani di lapisan driver — **controller dan
model tidak perlu berubah** saat berpindah backend. Untuk PostgreSQL, kolom kunci sebaiknya
`bigserial`/`bigint` agar cocok dengan binding integer 64-bit.

## Koneksi

Dikonfigurasi di `config/database.toml`, dibuka saat boot, dan diakses dari controller via
`ctx.db()`:

```rust
let db = ctx.db();          // &Database
db.table("notes");          // mulai membangun query
```

## SELECT

```rust
// SELECT * FROM "notes"
db.table("notes").get()?;                          // Vec<Value>

// SELECT "id","text" FROM "notes" WHERE "id" = ? ORDER BY "id" DESC LIMIT 10
db.table("notes")
    .select("id, text")
    .where_("id", 5)
    .order_by("id", "DESC")
    .limit(10)
    .get()?;

// Baris pertama saja
db.table("notes").where_("id", 5).first()?;        // Option<Value>
```

> `where` adalah kata-kunci Rust, jadi method-nya **`where_`**. Beberapa `where_` digabung
> dengan `AND`.

## INSERT

```rust
// INSERT INTO "notes" ("text") VALUES (?)
let id: i64 = db.table("notes").insert(json!({ "text": "Halo" }))?;
```

Mengembalikan `rowid` baris baru.

## UPDATE

```rust
// UPDATE "notes" SET "text" = ? WHERE "id" = ?
let n: usize = db.table("notes")
    .where_("id", 5)
    .update(json!({ "text": "Diperbarui" }))?;     // jumlah baris terdampak
```

## DELETE

```rust
// DELETE FROM "notes" WHERE "id" = ?
let n: usize = db.table("notes").where_("id", 5).delete()?;
```

## Hasil sebagai JSON

Setiap baris dikembalikan sebagai `serde_json::Value` objek (key = nama kolom), siap
diteruskan ke view atau dibungkus dalam respons JSON:

```json
{ "id": 1, "text": "Catatan pertama", "created": "2026-06-21 09:49:10" }
```

## Query Mentah

Untuk kebutuhan di luar builder:

```rust
db.execute("CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY)", &[])?;
db.query("SELECT COUNT(*) AS n FROM notes", &[])?;   // Vec<Value>
```

## Catatan Konkurensi

Koneksi dibungkus `Arc<Mutex<Connection>>` agar aman dibagi antar task. Karena
`App::handle` berjalan sinkron, kunci tak pernah melewati `.await`. Untuk beban tinggi,
connection pool bisa ditambahkan di balik abstraksi `Database` tanpa mengubah controller.
