//! RustIgniter — titik masuk (analog `index.php`).
//!
//! Tugasnya ringan: bootstrap (muat config & routes, bangun view + registry controller),
//! lalu menyalakan server hyper. Seluruh logika request hidup di `system::App::handle`.

mod app;
mod system;

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::CONTENT_TYPE;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use system::{
    App, Config, Database, Migrator, Registry, Request, Router, RoutesConfig, SessionStore, View,
};

/// Titik masuk: dispatch berdasarkan argumen CLI (gaya `php index.php <cmd>` di CI).
///   (tanpa arg) / serve   → jalankan server
///   migrate               → terapkan migrasi tertunda
///   migrate:rollback      → batalkan migrasi terakhir
///   migrate:status        → tampilkan status migrasi
///   db:seed               → isi data contoh
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("serve");

    let result = match command {
        "serve" => serve_command().await,
        "migrate" => migrate_command().await,
        "migrate:rollback" => rollback_command().await,
        "migrate:status" => status_command().await,
        "db:seed" => seed_command().await,
        other => {
            eprintln!(
                "Perintah tidak dikenal: '{other}'.\n\
                 Tersedia: serve, migrate, migrate:rollback, migrate:status, db:seed"
            );
            std::process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Bangun App lalu jalankan server hyper.
async fn serve_command() -> Result<(), String> {
    let app = Arc::new(bootstrap().await?);

    let host: IpAddr = app
        .config
        .server_host()
        .parse()
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let addr = SocketAddr::new(host, app.config.server_port());

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("gagal bind ke {addr}: {e}"))?;
    println!(
        "RustIgniter 3.0.0 berjalan di http://{addr}  [env: {}]",
        app.config.environment()
    );

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };
        let io = TokioIo::new(stream);
        let app = app.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req| {
                let app = app.clone();
                async move { Ok::<_, Infallible>(serve(app, req).await) }
            });
            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("connection error: {e}");
            }
        });
    }
}

/// `migrate` — jalankan migrasi tertunda.
async fn migrate_command() -> Result<(), String> {
    let (db, _) = build_database("config/database.toml").await?;
    let done = Migrator::new(app::migrations()).migrate(&db)?;
    if done.is_empty() {
        println!("Tidak ada migrasi tertunda.");
    } else {
        for v in done {
            println!("✔ migrated  v{v}");
        }
    }
    Ok(())
}

/// `migrate:rollback` — batalkan migrasi terakhir.
async fn rollback_command() -> Result<(), String> {
    let (db, _) = build_database("config/database.toml").await?;
    match Migrator::new(app::migrations()).rollback(&db)? {
        Some(v) => println!("✔ rolled back  v{v}"),
        None => println!("Tidak ada migrasi untuk dibatalkan."),
    }
    Ok(())
}

/// `migrate:status` — tampilkan status tiap migrasi.
async fn status_command() -> Result<(), String> {
    let (db, _) = build_database("config/database.toml").await?;
    println!("{:<8} {:<22} STATUS", "VERSI", "NAMA");
    for (version, name, applied) in Migrator::new(app::migrations()).status(&db)? {
        let mark = if applied { "✔ applied" } else { "· pending" };
        println!("{version:<8} {name:<22} {mark}");
    }
    Ok(())
}

/// `db:seed` — isi data contoh.
async fn seed_command() -> Result<(), String> {
    let (db, _) = build_database("config/database.toml").await?;
    app::seed(&db)?;
    println!("✔ seed selesai.");
    Ok(())
}

/// Rakit seluruh komponen framework menjadi sebuah `App`.
async fn bootstrap() -> Result<App, String> {
    let config = Config::load("config/app.toml")?;
    let routes = RoutesConfig::load("config/routes.toml")?;
    let router = Router::new(routes)?;

    let autoload = load_autoload_helpers("config/autoload.toml");
    let view = View::new("src/app/views", &config, &autoload);

    let (database, do_seed) = build_database("config/database.toml").await?;
    // Terapkan migrasi tertunda otomatis saat boot, lalu seed bila diminta.
    let applied = Migrator::new(app::migrations()).migrate(&database)?;
    if !applied.is_empty() {
        println!("Migrasi diterapkan: {applied:?}");
    }
    if do_seed {
        app::seed(&database)?;
    }

    let mut registry = Registry::new();
    app::register(&mut registry);

    let hooks = app::register_hooks();
    // Backend sesi: "file" (persisten) atau "memory" (default).
    let sessions = match config.item("session.driver").as_deref() {
        Some("file") => {
            let dir = config
                .item("session.path")
                .unwrap_or_else(|| "storage/sessions".to_string());
            println!("Session: file ({dir})");
            SessionStore::file(&dir)
        }
        _ => SessionStore::new(),
    };

    Ok(App {
        config,
        router,
        view,
        registry,
        database,
        hooks,
        sessions,
    })
}

/// Baca `config/database.toml`, pilih driver, dan buka koneksi. Kembalikan (Database, seed).
async fn build_database(path: &str) -> Result<(Database, bool), String> {
    #[derive(serde::Deserialize, Default)]
    struct SqliteCfg {
        #[serde(default)]
        path: String,
    }
    #[derive(serde::Deserialize, Default)]
    struct PgCfg {
        host: String,
        port: u16,
        user: String,
        password: String,
        dbname: String,
    }
    #[derive(serde::Deserialize)]
    struct DbCfg {
        #[serde(default = "default_driver")]
        driver: String,
        #[serde(default = "default_true")]
        seed: bool,
        #[serde(default)]
        sqlite: SqliteCfg,
        #[serde(default)]
        postgres: PgCfg,
    }
    fn default_driver() -> String {
        "sqlite".to_string()
    }
    fn default_true() -> bool {
        true
    }

    let raw = std::fs::read_to_string(path).map_err(|e| format!("baca {path}: {e}"))?;
    let cfg: DbCfg = toml::from_str(&raw).map_err(|e| format!("database.toml tidak valid: {e}"))?;

    let database = match cfg.driver.as_str() {
        "postgres" | "pg" => {
            let p = &cfg.postgres;
            let dsn = format!(
                "host={} port={} user={} password={} dbname={}",
                p.host, p.port, p.user, p.password, p.dbname
            );
            let (client, connection) = tokio_postgres::connect(&dsn, tokio_postgres::NoTls)
                .await
                .map_err(|e| format!("koneksi postgres gagal: {e}"))?;
            // Jalankan task koneksi (driver protokol) di latar belakang.
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("postgres connection error: {e}");
                }
            });
            println!("Database: PostgreSQL {}:{}/{}", p.host, p.port, p.dbname);
            Database::from_postgres(client)
        }
        _ => {
            let path = if cfg.sqlite.path.is_empty() {
                "storage/rustigniter.db".to_string()
            } else {
                cfg.sqlite.path.clone()
            };
            println!("Database: SQLite {path}");
            Database::open(&path)?
        }
    };
    Ok((database, cfg.seed))
}

/// Baca daftar helper dari `config/autoload.toml` (CI: `$autoload['helper']`).
fn load_autoload_helpers(path: &str) -> Vec<String> {
    #[derive(serde::Deserialize, Default)]
    struct Autoload {
        #[serde(default)]
        helpers: Vec<String>,
    }
    match std::fs::read_to_string(path) {
        Ok(raw) => toml::from_str::<Autoload>(&raw)
            .map(|a| a.helpers)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Jembatan transport: ubah request hyper -> `Request`, proses, lalu kembalikan respons hyper.
/// Body dibaca (await) di sini supaya `App::handle` bisa tetap sinkron.
async fn serve(app: Arc<App>, req: HyperRequest<Incoming>) -> HyperResponse<Full<Bytes>> {
    let (parts, body) = req.into_parts();

    let method = parts.method.as_str().to_string();
    let path = parts.uri.path().to_string();

    // Static file serving: GET /assets/* dilayani dari folder public/ (lewati pipeline).
    if method == "GET" {
        if let Some(resp) = try_static(&path) {
            return resp;
        }
    }

    let raw_uri = match parts.uri.query() {
        Some(q) => format!("{path}?{q}"),
        None => path.to_string(),
    };
    let content_type = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Salin semua header (string-able) ke map untuk diakses controller/hook.
    let headers: std::collections::HashMap<String, String> = parts
        .headers
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.as_str().to_string(), val.to_string())))
        .collect();

    // Kumpulkan body (batasi sewajarnya untuk dev).
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => Bytes::new(),
    };

    let request = Request::new(&method, &raw_uri)
        .with_headers(headers)
        .with_body(&content_type, &body_bytes);
    app.handle(request).into_hyper()
}

/// Coba layani berkas statis untuk path `/assets/<rel>` dari folder `public/`.
/// Mengembalikan `None` bila bukan path assets / berkas tak ada / path mencurigakan.
fn try_static(path: &str) -> Option<HyperResponse<Full<Bytes>>> {
    let rel = path.strip_prefix("/assets/")?;
    if rel.is_empty() || rel.contains("..") {
        return None; // cegah path traversal
    }
    let file = format!("public/{rel}");
    let bytes = std::fs::read(&file).ok()?;
    Some(
        HyperResponse::builder()
            .status(200)
            .header(CONTENT_TYPE, content_type_for(&file))
            .body(Full::new(Bytes::from(bytes)))
            .expect("response statis valid"),
    )
}

/// Tebak Content-Type dari ekstensi berkas.
fn content_type_for(file: &str) -> &'static str {
    match file.rsplit('.').next().unwrap_or("") {
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
