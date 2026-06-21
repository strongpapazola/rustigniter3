# Hooks — Memperluas Inti Framework

Hooks memungkinkanmu menyisipkan logika **sebelum** dan **sesudah** controller tanpa
mengubah inti framework — setara sistem *Hooks* (`pre_controller`/`post_controller`) di
CodeIgniter. Ini juga pola **middleware** untuk auth, logging, header, dsb.

## Pipeline

```
Request → Hook.before (boleh Halt) → Controller → Hook.after (selalu jalan) → Response
```

- **before** dijalankan sebelum controller. Mengembalikan `HookResult::Halt(response)`
  untuk men-*short-circuit* (controller tidak dipanggil).
- **after** dijalankan setelah response terbentuk — **termasuk** bila request di-halt —
  dan boleh memodifikasi response.

Hook dijalankan berurutan sesuai pendaftaran.

## Trait

```rust
pub trait Hook: Send + Sync {
    fn before(&self, _ctx: &mut Ctx) -> HookResult { HookResult::Continue }
    fn after(&self, _ctx: &mut Ctx, response: Response) -> Response { response }
}
```

Kedua method punya default, jadi sebuah hook cukup mengisi sisi yang relevan.

## Contoh

```rust
// src/app/hooks.rs
use crate::system::{Ctx, Hook, HookResult, Response};
use serde_json::json;

// 1) Logging tiap request
pub struct RequestLogger;
impl Hook for RequestLogger {
    fn before(&self, ctx: &mut Ctx) -> HookResult {
        println!("→ {} {}", ctx.method(), ctx.request.path);
        HookResult::Continue
    }
    fn after(&self, _ctx: &mut Ctx, response: Response) -> Response {
        println!("← {}", response.status);
        response
    }
}

// 2) Tambah header ke semua response
pub struct PoweredBy;
impl Hook for PoweredBy {
    fn after(&self, _ctx: &mut Ctx, response: Response) -> Response {
        response.with_header("X-Powered-By", "RustIgniter/3.0.0")
    }
}

// 3) Guard auth — tolak tulis ke /api/* tanpa API key
pub struct ApiKeyGuard;
impl Hook for ApiKeyGuard {
    fn before(&self, ctx: &mut Ctx) -> HookResult {
        let is_api   = ctx.request.path.trim_start_matches('/').starts_with("api/");
        let is_write = matches!(ctx.method(), "POST" | "PUT" | "PATCH" | "DELETE");
        if is_api && is_write && ctx.header("x-api-key") != Some("rahasia") {
            return HookResult::Halt(Response::json(401,
                json!({ "error": "Butuh header X-Api-Key yang valid" })));
        }
        HookResult::Continue
    }
}
```

## Mendaftarkan Hook

```rust
// src/app/mod.rs
pub fn register_hooks() -> Vec<Box<dyn Hook>> {
    vec![
        Box::new(hooks::RequestLogger),
        Box::new(hooks::PoweredBy),
        Box::new(hooks::ApiKeyGuard),
    ]
}
```

Urutan dalam `vec!` menentukan urutan eksekusi.
