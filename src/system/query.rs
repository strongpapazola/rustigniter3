//! Query Builder — perakit query gaya Active Record CodeIgniter.
//!
//! Meniru rasa `$this->db->select()->join()->where()->like()->order_by()->get()`:
//!
//! ```ignore
//! let rows = ctx.db().table("notes")
//!     .like("text", "rust")           // WHERE text LIKE '%rust%'
//!     .where_op("id", ">", 3)         // AND id > ?
//!     .order_by("id", "DESC")
//!     .limit(5).offset(10)
//!     .get()?;
//!
//! let total = ctx.db().table("notes").like("text", "rust").count()?;
//! ```
//!
//! Semua nilai di-bind sebagai parameter sehingga aman dari SQL injection; nama tabel &
//! kolom dikutip identifier. Kondisi `ON` pada `join` dan operator pada `where_op` berasal
//! dari kode (bukan input pengguna), seperti di CodeIgniter.

use crate::system::database::Database;
use serde_json::Value;

/// Satu kondisi WHERE, dengan konektor (AND/OR) ke kondisi sebelumnya.
enum Cond {
    /// `col OP ?` — mis. `"age" > ?`, `"text" LIKE ?`.
    Binary {
        connector: &'static str,
        col: String,
        op: String,
        val: Value,
    },
    /// `col IN (?, ?, ...)`.
    In {
        connector: &'static str,
        col: String,
        vals: Vec<Value>,
    },
}

/// Perakit query yang menahan klausa lalu mengeksekusinya ke [`Database`].
pub struct QueryBuilder {
    db: Database,
    table: String,
    selects: Vec<String>,
    joins: Vec<String>,
    conds: Vec<Cond>,
    groups: Vec<String>,
    order: Option<(String, String)>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl QueryBuilder {
    pub fn new(db: Database, table: &str) -> Self {
        Self {
            db,
            table: table.to_string(),
            selects: Vec::new(),
            joins: Vec::new(),
            conds: Vec::new(),
            groups: Vec::new(),
            order: None,
            limit: None,
            offset: None,
        }
    }

    /// Kolom yang dipilih, mis. `.select("id, text")`. Default `*`.
    pub fn select(mut self, cols: &str) -> Self {
        self.selects = cols.split(',').map(|c| c.trim().to_string()).collect();
        self
    }

    /// Tambah JOIN. `kind` = "INNER" | "LEFT" | "RIGHT" (default INNER). `on` berupa SQL
    /// mentah, mis. `.join("users", "users.id = notes.user_id", "LEFT")`.
    pub fn join(mut self, table: &str, on: &str, kind: &str) -> Self {
        let kind = match kind.to_ascii_uppercase().as_str() {
            "LEFT" => "LEFT",
            "RIGHT" => "RIGHT",
            _ => "INNER",
        };
        self.joins
            .push(format!("{kind} JOIN {} ON {on}", quote_ident(table)));
        self
    }

    /// `AND col = val` (CI: `where('id', 5)`).
    pub fn where_(mut self, col: &str, val: impl Into<Value>) -> Self {
        self.push_binary("AND", col, "=", val.into());
        self
    }

    /// `OR col = val` (CI: `or_where`).
    pub fn or_where(mut self, col: &str, val: impl Into<Value>) -> Self {
        self.push_binary("OR", col, "=", val.into());
        self
    }

    /// `AND col OP val`, mis. `.where_op("age", ">", 18)`.
    pub fn where_op(mut self, col: &str, op: &str, val: impl Into<Value>) -> Self {
        self.push_binary("AND", col, op, val.into());
        self
    }

    /// `AND col IN (...)` (CI: `where_in`).
    pub fn where_in<V: Into<Value>>(mut self, col: &str, vals: Vec<V>) -> Self {
        self.conds.push(Cond::In {
            connector: "AND",
            col: col.to_string(),
            vals: vals.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// `AND col LIKE '%pattern%'` (CI: `like`).
    pub fn like(mut self, col: &str, pattern: &str) -> Self {
        self.push_binary("AND", col, "LIKE", Value::String(format!("%{pattern}%")));
        self
    }

    /// `OR col LIKE '%pattern%'` (CI: `or_like`).
    pub fn or_like(mut self, col: &str, pattern: &str) -> Self {
        self.push_binary("OR", col, "LIKE", Value::String(format!("%{pattern}%")));
        self
    }

    /// Kelompokkan hasil, mis. `.group_by("status")`.
    pub fn group_by(mut self, col: &str) -> Self {
        self.groups.push(col.to_string());
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

    /// Lewati `n` baris pertama (dipakai bersama `limit` untuk pagination).
    pub fn offset(mut self, n: i64) -> Self {
        self.offset = Some(n);
        self
    }

    fn push_binary(&mut self, connector: &'static str, col: &str, op: &str, val: Value) {
        self.conds.push(Cond::Binary {
            connector,
            col: col.to_string(),
            op: op.to_string(),
            val,
        });
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
        for j in &self.joins {
            sql.push(' ');
            sql.push_str(j);
        }
        let mut params = Vec::new();
        sql.push_str(&self.build_where(&mut params));
        if !self.groups.is_empty() {
            let cols = self
                .groups
                .iter()
                .map(|g| quote_ident(g))
                .collect::<Vec<_>>()
                .join(", ");
            sql.push_str(&format!(" GROUP BY {cols}"));
        }
        if let Some((col, dir)) = &self.order {
            let dir = if dir.eq_ignore_ascii_case("desc") { "DESC" } else { "ASC" };
            sql.push_str(&format!(" ORDER BY {} {dir}", quote_ident(col)));
        }
        if let Some(n) = self.limit {
            sql.push_str(&format!(" LIMIT {n}"));
            if let Some(o) = self.offset {
                sql.push_str(&format!(" OFFSET {o}"));
            }
        }
        (sql, params)
    }

    /// Rakit klausa WHERE (termasuk AND/OR, operator, IN) + kumpulkan parameter.
    /// Indeks placeholder 1-based diambil dari panjang `params` setelah tiap nilai
    /// dimasukkan, sehingga tetap benar saat ada parameter SET (UPDATE) yang mendahuluinya.
    fn build_where(&self, params: &mut Vec<Value>) -> String {
        if self.conds.is_empty() {
            return String::new();
        }
        let mut sql = String::from(" WHERE ");
        for (i, cond) in self.conds.iter().enumerate() {
            match cond {
                Cond::Binary { connector, col, op, val } => {
                    if i > 0 {
                        sql.push_str(&format!(" {connector} "));
                    }
                    params.push(val.clone());
                    sql.push_str(&format!("{} {op} {}", quote_ident(col), self.db.placeholder(params.len())));
                }
                Cond::In { connector, col, vals } => {
                    if i > 0 {
                        sql.push_str(&format!(" {connector} "));
                    }
                    let phs: Vec<String> = vals
                        .iter()
                        .map(|v| {
                            params.push(v.clone());
                            self.db.placeholder(params.len())
                        })
                        .collect();
                    sql.push_str(&format!("{} IN ({})", quote_ident(col), phs.join(", ")));
                }
            }
        }
        sql
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

    /// Hitung jumlah baris yang cocok (CI: `count_all_results()`). Mengabaikan
    /// order/limit/offset/group; menghormati join & where.
    pub fn count(&self) -> Result<i64, String> {
        let mut sql = format!("SELECT COUNT(*) AS n FROM {}", quote_ident(&self.table));
        for j in &self.joins {
            sql.push(' ');
            sql.push_str(j);
        }
        let mut params = Vec::new();
        sql.push_str(&self.build_where(&mut params));
        let rows = self.db.query(&sql, &params)?;
        Ok(rows
            .first()
            .and_then(|r| r.get("n"))
            .and_then(Value::as_i64)
            .unwrap_or(0))
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

    /// UPDATE kolom dari objek JSON, dibatasi klausa where. Kembalikan jumlah baris.
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
        let where_sql = self.build_where(&mut params);
        let sql = format!(
            "UPDATE {} SET {}{}",
            quote_ident(&self.table),
            sets.join(", "),
            where_sql
        );
        self.db.execute(&sql, &params)
    }

    /// DELETE dibatasi klausa where. Kembalikan jumlah baris.
    pub fn delete(self) -> Result<usize, String> {
        let mut params: Vec<Value> = Vec::new();
        let where_sql = self.build_where(&mut params);
        let sql = format!("DELETE FROM {}{}", quote_ident(&self.table), where_sql);
        self.db.execute(&sql, &params)
    }
}

/// Kutip identifier SQLite/Postgres dengan tanda kutip ganda (escape `"` internal).
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
    fn where_and_or_dan_like() {
        let db = Database::open(":memory:").unwrap();
        let (sql, params) = db
            .table("users")
            .where_("age", 20)
            .or_where("name", "Budi")
            .like("name", "Bu")
            .select_sql();
        assert_eq!(
            sql,
            "SELECT * FROM \"users\" WHERE \"age\" = ? OR \"name\" = ? AND \"name\" LIKE ?"
        );
        assert_eq!(params, vec![json!(20), json!("Budi"), json!("%Bu%")]);
    }

    #[test]
    fn where_in_dan_op() {
        let db = Database::open(":memory:").unwrap();
        let (sql, params) = db
            .table("users")
            .where_op("age", ">", 18)
            .where_in("id", vec![1, 2, 3])
            .select_sql();
        assert_eq!(
            sql,
            "SELECT * FROM \"users\" WHERE \"age\" > ? AND \"id\" IN (?, ?, ?)"
        );
        assert_eq!(params, vec![json!(18), json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn join_group_offset() {
        let db = Database::open(":memory:").unwrap();
        let (sql, _) = db
            .table("notes")
            .select("notes.id")
            .join("users", "users.id = notes.user_id", "left")
            .group_by("users.id")
            .limit(10)
            .offset(20)
            .select_sql();
        assert_eq!(
            sql,
            "SELECT \"notes.id\" FROM \"notes\" LEFT JOIN \"users\" ON users.id = notes.user_id GROUP BY \"users.id\" LIMIT 10 OFFSET 20"
        );
    }

    #[test]
    fn roundtrip_like_count_offset() {
        let db = mem_db();
        for (n, a) in [("Budi", 20), ("Ani", 25), ("Bambang", 30)] {
            db.table("users").insert(json!({"name": n, "age": a})).unwrap();
        }
        // LIKE '%Ba%' -> hanya "Bambang"
        let rows = db.table("users").like("name", "Ba").get().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], json!("Bambang"));

        // count dengan where_op
        let n = db.table("users").where_op("age", ">=", 25).count().unwrap();
        assert_eq!(n, 2);

        // limit + offset (urut umur naik): lewati 1, ambil 1 -> "Ani"
        let page = db
            .table("users")
            .order_by("age", "asc")
            .limit(1)
            .offset(1)
            .get()
            .unwrap();
        assert_eq!(page[0]["name"], json!("Ani"));

        // where_in
        let in_rows = db.table("users").where_in("name", vec!["Budi", "Ani"]).get().unwrap();
        assert_eq!(in_rows.len(), 2);
    }

    #[test]
    fn insert_update_delete_tetap_jalan() {
        let db = mem_db();
        let id = db.table("users").insert(json!({"name":"Budi","age":20})).unwrap();
        assert_eq!(id, 1);
        let n = db.table("users").where_("id", 1).update(json!({"age": 21})).unwrap();
        assert_eq!(n, 1);
        let row = db.table("users").where_("id", 1).first().unwrap().unwrap();
        assert_eq!(row["age"], json!(21));
        assert_eq!(db.table("users").where_("id", 1).delete().unwrap(), 1);
        assert_eq!(db.table("users").count().unwrap(), 0);
    }
}
