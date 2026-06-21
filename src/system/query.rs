//! Query Builder — perakit query gaya Active Record CodeIgniter.
//!
//! Meniru rasa `$this->db->select()->from()->where()->order_by()->get()`:
//!
//! ```ignore
//! let notes = ctx.db().table("notes")
//!     .where_("id", 5)
//!     .order_by("created", "DESC")
//!     .limit(10)
//!     .get()?;                 // -> Vec<serde_json::Value>
//!
//! let id = ctx.db().table("notes").insert(json!({"text": "Halo"}))?;
//! ```
//!
//! Semua nilai di-bind sebagai parameter (`?`) sehingga aman dari SQL injection;
//! nama tabel & kolom dikutip identifier (`"..."`).

use crate::system::database::Database;
use serde_json::Value;

/// Perakit query yang menahan klausa lalu mengeksekusinya ke [`Database`].
pub struct QueryBuilder {
    db: Database,
    table: String,
    selects: Vec<String>,
    wheres: Vec<(String, Value)>,
    order: Option<(String, String)>,
    limit: Option<i64>,
}

impl QueryBuilder {
    pub fn new(db: Database, table: &str) -> Self {
        Self {
            db,
            table: table.to_string(),
            selects: Vec::new(),
            wheres: Vec::new(),
            order: None,
            limit: None,
        }
    }

    /// Kolom yang dipilih, mis. `.select("id, text")`. Default `*`.
    pub fn select(mut self, cols: &str) -> Self {
        self.selects = cols.split(',').map(|c| c.trim().to_string()).collect();
        self
    }

    /// Tambah kondisi `kolom = nilai` (digabung dengan AND). `where` adalah keyword Rust,
    /// jadi method-nya `where_` (CI: `$this->db->where('id', 5)`).
    pub fn where_(mut self, col: &str, val: impl Into<Value>) -> Self {
        self.wheres.push((col.to_string(), val.into()));
        self
    }

    /// Urutan hasil, mis. `.order_by("created", "DESC")`.
    pub fn order_by(mut self, col: &str, dir: &str) -> Self {
        self.order = Some((col.to_string(), dir.to_string()));
        self
    }

    /// Batas jumlah baris.
    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Bangun SQL SELECT + parameter (dipisah agar bisa diuji tanpa DB).
    fn select_sql(&self) -> (String, Vec<Value>) {
        let cols = if self.selects.is_empty() {
            "*".to_string()
        } else {
            self.selects
                .iter()
                .map(|c| if c == "*" { c.clone() } else { quote_ident(c) })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut sql = format!("SELECT {cols} FROM {}", quote_ident(&self.table));
        let mut params = Vec::new();
        sql.push_str(&self.where_clause(&mut params));
        if let Some((col, dir)) = &self.order {
            let dir = if dir.eq_ignore_ascii_case("desc") { "DESC" } else { "ASC" };
            sql.push_str(&format!(" ORDER BY {} {dir}", quote_ident(col)));
        }
        if let Some(n) = self.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        (sql, params)
    }

    /// Rakit klausa WHERE dan kumpulkan parameternya. Placeholder mengikuti dialek driver;
    /// indeks 1-based diambil dari panjang `params` setelah tiap nilai dimasukkan, sehingga
    /// tetap benar saat ada parameter lain (mis. SET pada UPDATE) yang mendahuluinya.
    fn where_clause(&self, params: &mut Vec<Value>) -> String {
        if self.wheres.is_empty() {
            return String::new();
        }
        let conds: Vec<String> = self
            .wheres
            .iter()
            .map(|(col, val)| {
                params.push(val.clone());
                format!("{} = {}", quote_ident(col), self.db.placeholder(params.len()))
            })
            .collect();
        format!(" WHERE {}", conds.join(" AND "))
    }

    /// Jalankan SELECT, kembalikan semua baris.
    pub fn get(self) -> Result<Vec<Value>, String> {
        let (sql, params) = self.select_sql();
        self.db.query(&sql, &params)
    }

    /// Ambil baris pertama saja (CI: `->row_array()`), atau `None`.
    pub fn first(mut self) -> Result<Option<Value>, String> {
        self.limit = Some(1);
        Ok(self.get()?.into_iter().next())
    }

    /// INSERT dari objek JSON; kembalikan rowid baru.
    pub fn insert(self, data: Value) -> Result<i64, String> {
        let obj = data
            .as_object()
            .ok_or_else(|| "insert() butuh objek JSON".to_string())?;
        if obj.is_empty() {
            return Err("insert() tanpa kolom".to_string());
        }
        let cols: Vec<String> = obj.keys().map(|k| quote_ident(k)).collect();
        let placeholders: Vec<String> = (1..=obj.len()).map(|i| self.db.placeholder(i)).collect();
        let params: Vec<Value> = obj.values().cloned().collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            quote_ident(&self.table),
            cols.join(", "),
            placeholders.join(", ")
        );
        self.db.insert(&sql, &params)
    }

    /// UPDATE kolom dari objek JSON, dibatasi klausa `where_`. Kembalikan jumlah baris.
    pub fn update(self, data: Value) -> Result<usize, String> {
        let obj = data
            .as_object()
            .ok_or_else(|| "update() butuh objek JSON".to_string())?;
        if obj.is_empty() {
            return Err("update() tanpa kolom".to_string());
        }
        let mut params: Vec<Value> = Vec::new();
        let sets: Vec<String> = obj
            .iter()
            .map(|(k, v)| {
                params.push(v.clone());
                format!("{} = {}", quote_ident(k), self.db.placeholder(params.len()))
            })
            .collect();
        let where_sql = self.where_clause(&mut params);
        let sql = format!(
            "UPDATE {} SET {}{}",
            quote_ident(&self.table),
            sets.join(", "),
            where_sql
        );
        self.db.execute(&sql, &params)
    }

    /// DELETE dibatasi klausa `where_`. Kembalikan jumlah baris.
    pub fn delete(self) -> Result<usize, String> {
        let mut params: Vec<Value> = Vec::new();
        let where_sql = self.where_clause(&mut params);
        let sql = format!("DELETE FROM {}{}", quote_ident(&self.table), where_sql);
        self.db.execute(&sql, &params)
    }
}

/// Kutip identifier SQLite dengan tanda kutip ganda (escape `"` internal).
fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mem_db() -> Database {
        let db = Database::open(":memory:").unwrap();
        db.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, age INTEGER)",
            &[],
        )
        .unwrap();
        db
    }

    #[test]
    fn select_sql_lengkap() {
        let db = Database::open(":memory:").unwrap();
        let (sql, params) = db
            .table("users")
            .select("id, name")
            .where_("age", 20)
            .order_by("name", "desc")
            .limit(5)
            .select_sql();
        assert_eq!(
            sql,
            "SELECT \"id\", \"name\" FROM \"users\" WHERE \"age\" = ? ORDER BY \"name\" DESC LIMIT 5"
        );
        assert_eq!(params, vec![json!(20)]);
    }

    #[test]
    fn insert_lalu_get_roundtrip() {
        let db = mem_db();
        let id1 = db.table("users").insert(json!({"name":"Budi","age":20})).unwrap();
        let id2 = db.table("users").insert(json!({"name":"Ani","age":25})).unwrap();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        let all = db.table("users").order_by("id", "asc").get().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0]["name"], json!("Budi"));
        assert_eq!(all[0]["age"], json!(20));
        assert_eq!(all[1]["name"], json!("Ani"));
    }

    #[test]
    fn where_memfilter() {
        let db = mem_db();
        db.table("users").insert(json!({"name":"Budi","age":20})).unwrap();
        db.table("users").insert(json!({"name":"Ani","age":25})).unwrap();

        let rows = db.table("users").where_("name", "Ani").get().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["age"], json!(25));

        let first = db.table("users").where_("id", 1).first().unwrap();
        assert_eq!(first.unwrap()["name"], json!("Budi"));
    }

    #[test]
    fn update_dan_delete() {
        let db = mem_db();
        db.table("users").insert(json!({"name":"Budi","age":20})).unwrap();

        let n = db.table("users").where_("id", 1).update(json!({"age": 21})).unwrap();
        assert_eq!(n, 1);
        let row = db.table("users").where_("id", 1).first().unwrap().unwrap();
        assert_eq!(row["age"], json!(21));

        let d = db.table("users").where_("id", 1).delete().unwrap();
        assert_eq!(d, 1);
        assert_eq!(db.table("users").get().unwrap().len(), 0);
    }
}
