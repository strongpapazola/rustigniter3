# URL RustIgniter

Seperti CodeIgniter, RustIgniter memakai pendekatan **berbasis segmen**. Setiap segmen
URL umumnya mengikuti pola:

```
contoh.com/class/method/id
```

| Segmen   | Arti                              |
|----------|-----------------------------------|
| pertama  | nama **controller**               |
| kedua    | nama **method/aksi** (default `index`) |
| sisanya  | **argumen** yang diteruskan ke aksi |

Contoh:

```
/notes/show/5
        │     │  └── argumen ke-0  -> ctx.arg(0) == "5"
        │     └──── method          -> Notes::show
        └────────── controller      -> "notes"
```

## Segmen

Di dalam controller, segmen URI bisa diakses berbasis-1 (seperti `$this->uri->segment()`):

```rust
ctx.request.segment(1);   // "notes"
ctx.request.segment(2);   // "show"
```

Sedangkan **argumen** aksi (segmen setelah method) diakses berbasis-0:

```rust
ctx.arg(0);   // Option<&str> -> "5"
```

## Query String

```
/cari?q=rust&hal=2
```

```rust
ctx.query("q");     // Some("rust")
ctx.query("hal");   // Some("2")
```

## Controller Default

Jika URL kosong (`/`), RustIgniter memuat *default controller* yang diatur di
`config/routes.toml`:

```toml
default_controller = "welcome"
```

Lihat [URI Routing](routing.md) untuk remapping URL lebih lanjut.
