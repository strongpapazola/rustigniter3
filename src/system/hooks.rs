//! Hooks — pipeline middleware di sekitar controller.
//!
//! Ide dari sistem *Hooks* CodeIgniter (`pre_controller`, `post_controller`). Di RustIgniter,
//! sebuah [`Hook`] punya dua titik:
//!
//! - [`Hook::before`] dijalankan SEBELUM controller. Mengembalikan [`HookResult::Halt`]
//!   untuk men-short-circuit request (mis. menolak karena auth) tanpa memanggil controller.
//! - [`Hook::after`] dijalankan SETELAH response terbentuk (termasuk bila di-halt), dan
//!   boleh memodifikasi response (mis. menambah header).
//!
//! Hook dijalankan berurutan sesuai urutan registrasi. `after` selalu dijalankan untuk
//! semua hook sehingga lintas-cutting concern (logging, header) tetap konsisten.

use crate::system::{Ctx, Response};

/// Keputusan sebuah `before` hook.
pub enum HookResult {
    /// Lanjutkan ke hook berikutnya / controller.
    Continue,
    /// Hentikan dan balas response ini (controller tidak dipanggil).
    Halt(Response),
}

/// Kontrak sebuah hook (middleware). Kedua method punya default no-op, jadi sebuah hook
/// cukup mengimplementasikan sisi yang relevan.
pub trait Hook: Send + Sync {
    /// Sebelum controller. Default: lanjut.
    fn before(&self, _ctx: &mut Ctx) -> HookResult {
        HookResult::Continue
    }

    /// Setelah response terbentuk. Default: kembalikan apa adanya.
    fn after(&self, _ctx: &mut Ctx, response: Response) -> Response {
        response
    }
}
