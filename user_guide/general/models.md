# Models

Model meng-enkapsulasi akses data. Di RustIgniter, model adalah kumpulan fungsi yang
menerima `&Database` dan memakai [Query Builder](../database/query_builder.md).

## Contoh Model

```rust
// src/app/models/note.rs
use crate::system::Database;
use serde_json::{json, Value};

pub struct Note;

impl Note {
    pub fn all(db: &Database) -> Result<Vec<Value>, String> {
        db.table("notes").order_by("id", "DESC").get()
    }

    pub fn find(db: &Database, id: i64) -> Result<Option<Value>, String> {
        db.table("notes").where_("id", id).first()
    }

    pub fn create(db: &Database, text: &str) -> Result<i64, String> {
        db.table("notes").insert(json!({ "text": text }))
    }

    pub fn update(db: &Database, id: i64, text: &str) -> Result<usize, String> {
        db.table("notes").where_("id", id).update(json!({ "text": text }))
    }

    pub fn delete(db: &Database, id: i64) -> Result<usize, String> {
        db.table("notes").where_("id", id).delete()
    }
}
```

Daftarkan modul di `src/app/models/mod.rs`:

```rust
pub mod note;
```

## Memakai Model dari Controller

```rust
fn index(&self, ctx: &mut Ctx) -> Response {
    match Note::all(ctx.db()) {
        Ok(notes) => ctx.view("notes_index", json!({ "notes": notes })),
        Err(e) => Response::text(500, format!("DB error: {e}")),
    }
}
```

Baris hasil query berupa `serde_json::Value` (objek per baris), sehingga langsung bisa
diteruskan ke view — setara `result_array()` di CodeIgniter.

## Skema & Seed (Migrasi)

Skema didefinisikan di userland (`src/app/mod.rs`), dijalankan saat boot:

```rust
pub fn migrate(db: &Database, seed: bool) -> Result<(), String> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            created TEXT NOT NULL DEFAULT (datetime('now')))",
        &[],
    )?;
    if seed && db.table("notes").get()?.is_empty() {
        Note::create(db, "Catatan pertama")?;
    }
    Ok(())
}
```
