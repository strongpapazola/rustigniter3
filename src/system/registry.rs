//! Registry — daftar controller yang dikenal framework.
//!
//! Adaptasi idiomatik terpenting vs CodeIgniter: CI memuat file controller berdasarkan
//! nama dari URL dan meng-instansiasi class-nya lewat refleksi PHP. Rust tidak punya
//! refleksi runtime, jadi controller didaftarkan secara eksplisit di sini (nama -> instance).
//! Router menghasilkan *nama* controller; Registry menerjemahkannya menjadi instance konkret.

use crate::system::controller::Controller;
use std::collections::HashMap;

/// Peta nama-controller -> instance. Dibangun sekali saat boot, lalu hanya dibaca.
#[derive(Default)]
pub struct Registry {
    controllers: HashMap<String, Box<dyn Controller>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Daftarkan controller dengan nama (case-insensitive: disimpan lowercase).
    pub fn register(&mut self, name: &str, controller: Box<dyn Controller>) {
        self.controllers.insert(name.to_lowercase(), controller);
    }

    /// Ambil controller berdasarkan nama, atau `None` bila tak terdaftar.
    pub fn get(&self, name: &str) -> Option<&dyn Controller> {
        self.controllers.get(&name.to_lowercase()).map(|b| b.as_ref())
    }
}
