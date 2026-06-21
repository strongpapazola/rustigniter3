# Cache & Logging

## Cache

Cache key-value in-memory dengan TTL (CI: `$this->cache`). Nilai berupa `serde_json::Value`.

```rust
ctx.cache().save("kunci", json!({ "x": 1 }), 60);   // simpan 60 detik (0 = selamanya)
let v = ctx.cache().get("kunci");                    // Option<Value>
ctx.cache().has("kunci");                            // bool
ctx.cache().delete("kunci");
ctx.cache().clear();
```

Pola umum (compute-bila-miss):

```rust
let data = match ctx.cache().get("mahal") {
    Some(v) => v,
    None => {
        let v = hitung_yang_mahal();
        ctx.cache().save("mahal", v.clone(), 30);
        v
    }
};
```

Entri kedaluwarsa dibersihkan otomatis saat diakses. Backend lain (file/Redis) bisa
ditambahkan di `system/cache.rs`.

## Logging

Logger berlevel ke berkas (CI: `log_message()`). Konfigurasi di `config/app.toml`:

```toml
[log]
level = "info"               # error | warn | info | debug (pesan <= level dicatat)
path = "storage/logs/app.log"
```

Dari controller/hook:

```rust
ctx.log().info("user masuk");
ctx.log().error("koneksi DB putus");
ctx.log().warn("…");
ctx.log().debug("…");        // hanya tercatat bila level = debug
```

Format baris (timestamp UTC):

```
[2026-06-21 18:40:28] INFO --> GET /uploads
[2026-06-21 18:40:28] INFO upload: contoh.txt (33 bytes)
[2026-06-21 18:40:28] ERROR koneksi DB putus
```

Hook `RequestLogger` (`src/app/hooks.rs`) mencatat setiap request & status ke log ini.
