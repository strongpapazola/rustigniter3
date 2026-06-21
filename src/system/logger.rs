//! Logger — pencatatan berlevel ke berkas (CI: `log_message()` + `application/logs/`).
//!
//! Level: Error < Warn < Info < Debug. Pesan dicatat bila level-nya <= ambang (`threshold`).
//! Format baris: `[YYYY-MM-DD HH:MM:SS] LEVEL pesan`. Timestamp UTC dihitung tanpa dependensi
//! (algoritma civil-from-days). Penulisan diserialisasi dengan Mutex (append).

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tingkat keparahan log.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        }
    }

    /// Parse dari string config ("error"/"warn"/"info"/"debug"); default Info.
    pub fn parse(s: &str) -> Level {
        match s.to_ascii_lowercase().as_str() {
            "error" => Level::Error,
            "warn" | "warning" => Level::Warn,
            "debug" => Level::Debug,
            _ => Level::Info,
        }
    }
}

/// Logger berbasis berkas, bisa di-clone (berbagi `Arc`).
#[derive(Clone)]
pub struct Logger {
    path: Arc<String>,
    threshold: Level,
    lock: Arc<Mutex<()>>,
}

impl Logger {
    pub fn new(path: &str, threshold: Level) -> Self {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self {
            path: Arc::new(path.to_string()),
            threshold,
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Catat sebuah pesan pada `level` tertentu (di-skip bila di bawah ambang).
    pub fn log(&self, level: Level, msg: &str) {
        if level > self.threshold {
            return;
        }
        let line = format!("[{}] {} {msg}\n", now_timestamp(), level.label());
        let _guard = self.lock.lock();
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&*self.path) {
            let _ = f.write_all(line.as_bytes());
        }
    }

    pub fn error(&self, msg: &str) {
        self.log(Level::Error, msg);
    }
    pub fn warn(&self, msg: &str) {
        self.log(Level::Warn, msg);
    }
    pub fn info(&self, msg: &str) {
        self.log(Level::Info, msg);
    }
    pub fn debug(&self, msg: &str) {
        self.log(Level::Debug, msg);
    }
}

/// Timestamp UTC "YYYY-MM-DD HH:MM:SS" dari jam sistem, tanpa dependensi.
fn now_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// Konversi jumlah hari sejak epoch Unix -> (tahun, bulan, hari). Algoritma Howard Hinnant.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
