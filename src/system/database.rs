//! Database — koneksi & eksekusi SQL di balik abstraksi multi-driver.
//!
//! Ide dari `CI_DB` (`$this->db`) + sistem *driver* CodeIgniter. [`Database`] tidak terikat
//! ke satu backend: ia memegang sebuah [`Driver`] (`Arc<dyn Driver>`), sehingga driver baru
//! bisa ditambahkan tanpa mengubah Model/Controller. Tersedia dua driver:
//!
//! - [`SqliteDriver`]   — `rusqlite` (fitur `bundled`), self-contained, placeholder `?`.
//! - [`PostgresDriver`] — `tokio-postgres` (pure-Rust, tanpa libpq), placeholder `$1`.
//!
//! Baris hasil query dikembalikan sebagai `serde_json::Value` (objek per baris) agar langsung
//! bisa diteruskan ke view/JSON — selaras `result_array()` CodeIgniter.
//!
//! Catatan async: server berjalan di runtime tokio dan `App::handle` sinkron. SqliteDriver
//! murni sinkron. PostgresDriver menjembatani klien async ke API sinkron lewat
//! `block_in_place` + `Handle::block_on` (butuh runtime multi-thread — default `#[tokio::main]`).

use crate::system::query::QueryBuilder;
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::Connection;
use serde_json::{Map, Value};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tokio_postgres::types::{ToSql, Type};

/// Dialek SQL — memengaruhi gaya placeholder & DDL (lihat `app::migrate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Sqlite,
    Postgres,
}

/// Kontrak sebuah backend database. Semua method sinkron; driver async menjembatani sendiri.
pub trait Driver: Send + Sync {
    fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, String>;
    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64, String>;
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Value>, String>;
    /// Placeholder parameter ke-`idx` (1-based): `"?"` (SQLite) atau `"$idx"` (Postgres).
    fn placeholder(&self, idx: usize) -> String;
    fn dialect(&self) -> Dialect;
}

/// Handle database yang bisa di-clone (berbagi driver/koneksi yang sama).
#[derive(Clone)]
pub struct Database {
    driver: Arc<dyn Driver>,
}

impl Database {
    /// Buka database SQLite dengan ukuran pool default. `":memory:"` untuk DB sementara.
    pub fn open(path: &str) -> Result<Self, String> {
        Self::open_sqlite(path, 4)
    }

    /// Buka database SQLite dengan ukuran pool koneksi tertentu.
    pub fn open_sqlite(path: &str, pool_size: usize) -> Result<Self, String> {
        Ok(Self {
            driver: Arc::new(SqliteDriver::open(path, pool_size)?),
        })
    }

    /// Bungkus klien Postgres yang sudah tersambung menjadi `Database`.
    pub fn from_postgres(client: tokio_postgres::Client) -> Self {
        Self {
            driver: Arc::new(PostgresDriver {
                client: Arc::new(client),
            }),
        }
    }

    /// Mulai membangun query untuk sebuah tabel (CI: `$this->db->from('tabel')`).
    pub fn table(&self, name: &str) -> QueryBuilder {
        QueryBuilder::new(self.clone(), name)
    }

    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, String> {
        self.driver.execute(sql, params)
    }

    pub fn insert(&self, sql: &str, params: &[Value]) -> Result<i64, String> {
        self.driver.insert(sql, params)
    }

    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Value>, String> {
        self.driver.query(sql, params)
    }

    /// Placeholder untuk parameter ke-`idx` sesuai dialek driver aktif.
    pub fn placeholder(&self, idx: usize) -> String {
        self.driver.placeholder(idx)
    }

    pub fn dialect(&self) -> Dialect {
        self.driver.dialect()
    }
}

// ====================================================================
// Driver SQLite
// ====================================================================

/// Pool koneksi SQLite sederhana: kumpulan koneksi siap-pakai dengan checkout yang
/// memblokir (Condvar) saat kosong. Untuk `:memory:` ukuran dipaksa 1 (tiap koneksi
/// `:memory:` adalah database terpisah, jadi harus satu koneksi bersama).
struct SqlitePool {
    conns: Mutex<Vec<Connection>>,
    available: Condvar,
}

impl SqlitePool {
    /// Ambil satu koneksi (blokir bila semua sedang dipakai).
    fn checkout(&self) -> Connection {
        let mut conns = self.conns.lock().expect("pool mutex");
        while conns.is_empty() {
            conns = self.available.wait(conns).expect("pool condvar");
        }
        conns.pop().unwrap()
    }

    /// Kembalikan koneksi ke pool.
    fn checkin(&self, conn: Connection) {
        self.conns.lock().expect("pool mutex").push(conn);
        self.available.notify_one();
    }
}

/// RAII: kembalikan koneksi ke pool saat keluar scope (termasuk saat `?`/panic).
struct PooledConn<'a> {
    pool: &'a SqlitePool,
    conn: Option<Connection>,
}

impl Drop for PooledConn<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.checkin(conn);
        }
    }
}

impl std::ops::Deref for PooledConn<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("koneksi ter-checkout")
    }
}

/// Driver SQLite berbasis `rusqlite` dengan pool koneksi.
pub struct SqliteDriver {
    pool: Arc<SqlitePool>,
}

impl SqliteDriver {
    fn open(path: &str, pool_size: usize) -> Result<Self, String> {
        if path != ":memory:" {
            if let Some(parent) = Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("gagal membuat folder db '{}': {e}", parent.display()))?;
                }
            }
        }
        // :memory: harus 1 koneksi (DB tiap koneksi terpisah).
        let size = if path == ":memory:" { 1 } else { pool_size.max(1) };
        let mut conns = Vec::with_capacity(size);
        for _ in 0..size {
            conns.push(new_connection(path)?);
        }
        Ok(Self {
            pool: Arc::new(SqlitePool {
                conns: Mutex::new(conns),
                available: Condvar::new(),
            }),
        })
    }

    fn acquire(&self) -> PooledConn<'_> {
        PooledConn {
            pool: &self.pool,
            conn: Some(self.pool.checkout()),
        }
    }
}

impl Driver for SqliteDriver {
    fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, String> {
        let conn = self.acquire();
        let p = to_sqlite_params(params);
        conn.execute(sql, rusqlite::params_from_iter(p.iter()))
            .map_err(|e| format!("execute gagal: {e}\nSQL: {sql}"))
    }

    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64, String> {
        let conn = self.acquire();
        let p = to_sqlite_params(params);
        conn.execute(sql, rusqlite::params_from_iter(p.iter()))
            .map_err(|e| format!("insert gagal: {e}\nSQL: {sql}"))?;
        Ok(conn.last_insert_rowid())
    }

    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Value>, String> {
        let conn = self.acquire();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| format!("prepare gagal: {e}\nSQL: {sql}"))?;
        let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let p = to_sqlite_params(params);
        let mut rows = stmt
            .query(rusqlite::params_from_iter(p.iter()))
            .map_err(|e| format!("query gagal: {e}\nSQL: {sql}"))?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| format!("baca baris gagal: {e}"))? {
            let mut obj = Map::new();
            for (i, name) in columns.iter().enumerate() {
                obj.insert(name.clone(), sqlite_to_json(row, i)?);
            }
            out.push(Value::Object(obj));
        }
        Ok(out)
    }

    fn placeholder(&self, _idx: usize) -> String {
        "?".to_string()
    }

    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
}

/// Buka satu koneksi SQLite; untuk DB berkas aktifkan WAL + busy_timeout agar
/// pembacaan bisa berbarengan dan penulisan menunggu alih-alih error SQLITE_BUSY.
fn new_connection(path: &str) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("gagal membuka db '{path}': {e}"))?;
    if path != ":memory:" {
        let _ = conn.busy_timeout(Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
    }
    Ok(conn)
}

fn to_sqlite_params(params: &[Value]) -> Vec<SqlValue> {
    params.iter().map(json_to_sqlite).collect()
}

fn json_to_sqlite(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else {
                SqlValue::Real(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

fn sqlite_to_json(row: &rusqlite::Row, i: usize) -> Result<Value, String> {
    let vr = row
        .get_ref(i)
        .map_err(|e| format!("ambil kolom {i} gagal: {e}"))?;
    Ok(match vr {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => Value::from(n),
        ValueRef::Real(f) => Value::from(f),
        ValueRef::Text(t) => Value::from(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Value::from(b.to_vec()),
    })
}

// ====================================================================
// Driver PostgreSQL
// ====================================================================

/// Driver PostgreSQL berbasis `tokio-postgres`. Klien async (`Send + Sync`) dibagi via `Arc`;
/// API sinkron dijembatani dengan `block_in_place` + `Handle::block_on`.
pub struct PostgresDriver {
    client: Arc<tokio_postgres::Client>,
}

impl Driver for PostgresDriver {
    fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, String> {
        let client = self.client.clone();
        let sql = sql.to_string();
        let owned = pg_params(params);
        run_blocking(async move {
            let refs = pg_refs(&owned);
            client.execute(&sql, &refs).await
        })
        .map(|n| n as usize)
        .map_err(|e| format!("execute gagal: {e}"))
    }

    fn insert(&self, sql: &str, params: &[Value]) -> Result<i64, String> {
        let client = self.client.clone();
        let sql = format!("{sql} RETURNING id");
        let owned = pg_params(params);
        let rows = run_blocking(async move {
            let refs = pg_refs(&owned);
            client.query(&sql, &refs).await
        })
        .map_err(|e| format!("insert gagal: {e}"))?;
        let id: i64 = rows
            .first()
            .and_then(|r| r.try_get::<_, i64>(0).ok())
            .unwrap_or(0);
        Ok(id)
    }

    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Value>, String> {
        let client = self.client.clone();
        let sql = sql.to_string();
        let owned = pg_params(params);
        let rows = run_blocking(async move {
            let refs = pg_refs(&owned);
            client.query(&sql, &refs).await
        })
        .map_err(|e| format!("query gagal: {e}"))?;
        Ok(rows.iter().map(pg_row_to_json).collect())
    }

    fn placeholder(&self, idx: usize) -> String {
        format!("${idx}")
    }

    fn dialect(&self) -> Dialect {
        Dialect::Postgres
    }
}

/// Jalankan future di konteks sinkron (dipanggil dari worker runtime tokio multi-thread).
fn run_blocking<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

/// Konversi parameter JSON -> nilai bindable Postgres (tiap elemen tipe konkret berbeda).
fn pg_params(params: &[Value]) -> Vec<Box<dyn ToSql + Sync>> {
    params
        .iter()
        .map(|v| -> Box<dyn ToSql + Sync> {
            match v {
                Value::Null => Box::new(Option::<i64>::None),
                Value::Bool(b) => Box::new(*b),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Box::new(i)
                    } else {
                        Box::new(n.as_f64().unwrap_or(0.0))
                    }
                }
                Value::String(s) => Box::new(s.clone()),
                other => Box::new(other.to_string()),
            }
        })
        .collect()
}

fn pg_refs(owned: &[Box<dyn ToSql + Sync>]) -> Vec<&(dyn ToSql + Sync)> {
    owned.iter().map(|b| b.as_ref()).collect()
}

fn pg_row_to_json(row: &tokio_postgres::Row) -> Value {
    let mut obj = Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        obj.insert(col.name().to_string(), pg_value_to_json(row, i, col.type_()));
    }
    Value::Object(obj)
}

/// Ambil satu kolom Postgres -> nilai JSON, berdasarkan tipe kolom.
fn pg_value_to_json(row: &tokio_postgres::Row, i: usize, ty: &Type) -> Value {
    if *ty == Type::BOOL {
        opt(row.try_get::<_, Option<bool>>(i))
    } else if *ty == Type::INT2 {
        opt(row.try_get::<_, Option<i16>>(i).map(|o| o.map(|v| v as i64)))
    } else if *ty == Type::INT4 {
        opt(row.try_get::<_, Option<i32>>(i).map(|o| o.map(|v| v as i64)))
    } else if *ty == Type::INT8 {
        opt(row.try_get::<_, Option<i64>>(i))
    } else if *ty == Type::FLOAT4 {
        opt(row.try_get::<_, Option<f32>>(i).map(|o| o.map(|v| v as f64)))
    } else if *ty == Type::FLOAT8 {
        opt(row.try_get::<_, Option<f64>>(i))
    } else {
        // TEXT/VARCHAR/NAME/BPCHAR dan lainnya -> coba sebagai string.
        opt(row.try_get::<_, Option<String>>(i))
    }
}

/// Bungkus hasil `try_get` (Result<Option<T>>) menjadi `Value`, default Null bila gagal.
fn opt<T: Into<Value>>(r: Result<Option<T>, tokio_postgres::Error>) -> Value {
    match r {
        Ok(Some(v)) => v.into(),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_dialek_dan_placeholder() {
        let db = Database::open(":memory:").unwrap();
        assert_eq!(db.dialect(), Dialect::Sqlite);
        assert_eq!(db.placeholder(1), "?");
        assert_eq!(db.placeholder(3), "?");
    }
}
