# Config & Auto-load

Konfigurasi RustIgniter berupa berkas **TOML** di folder `config/` (padanan
`application/config/*.php` di CodeIgniter).

## `config/app.toml`

```toml
base_url   = "http://127.0.0.1:8099/"
index_page = ""

[server]
host = "127.0.0.1"
port = 8099

[custom]
app_name = "RustIgniter"
```

### Membaca config

Dari controller (CI: `$this->config->item('base_url')`):

```rust
ctx.config_item("base_url");        // Some("http://127.0.0.1:8099/")
ctx.config_item("custom.app_name"); // Some("RustIgniter")  — path bertitik
ctx.config_item("server.port");     // Some("8099")
```

Helper URL:

```rust
ctx.base_url("css/app.css");   // "http://127.0.0.1:8099/css/app.css"
ctx.site_url("notes");         // base + index_page + uri
```

## `config/autoload.toml`

Daftar komponen yang dipreload saat boot:

```toml
helpers   = ["url"]   # mengaktifkan base_url()/site_url() di template
libraries = []
models    = []
```

Saat ini helper yang dikenal: **`url`** (menambah fungsi `base_url`/`site_url` ke view).

## `config/routes.toml`

Lihat [URI Routing](routing.md).

## `config/database.toml`

Pilih driver lewat `driver` (`"sqlite"` atau `"postgres"`):

```toml
driver = "sqlite"          # "sqlite" | "postgres"
seed   = true

[sqlite]
path = "storage/rustigniter.db"   # ":memory:" untuk DB sementara

[postgres]
host = "127.0.0.1"
port = 5433
user = "rustigniter"
password = "rahasia"
dbname = "rustigniter"
```

Mengganti `driver = "postgres"` membuat aplikasi tersambung ke PostgreSQL tanpa mengubah
satu baris pun kode controller/model. Lihat [Query Builder](../database/query_builder.md).
