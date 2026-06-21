//! Loader — registrasi helper untuk view (subset milestone 1).
//!
//! Ide dari sistem *helper* + *autoload* CodeIgniter. Di CI, `$this->load->helper('url')`
//! mengaktifkan fungsi seperti `base_url()` / `site_url()` yang bisa dipanggil dari view.
//! Di RustIgniter, daftar helper dari `config/autoload.toml` diterjemahkan menjadi fungsi
//! global di environment minijinja sehingga template bisa memanggil `{{ base_url('css/app.css') }}`.
//!
//! Ke depannya `Loader` akan tumbuh ke `model()`, `library()`, dan `config()` tambahan.
//! Untuk milestone 1, fokusnya helper view.

use crate::system::config::Config;
use minijinja::Environment;

/// Registrasikan semua helper yang diminta autoload ke environment view.
pub fn register_helpers(env: &mut Environment<'static>, config: &Config, helpers: &[String]) {
    for helper in helpers {
        match helper.as_str() {
            "url" => register_url_helper(env, config),
            other => eprintln!("[autoload] helper '{other}' belum dikenal, dilewati"),
        }
    }
}

/// Helper "url": menyediakan `base_url()` dan `site_url()` di dalam template,
/// meniru URL helper CodeIgniter.
fn register_url_helper(env: &mut Environment<'static>, config: &Config) {
    let base = config.base_url("");
    let site_base = config.site_url("");

    let base_for_fn = base.clone();
    env.add_function("base_url", move |uri: Option<String>| -> String {
        join(&base_for_fn, uri.as_deref().unwrap_or(""))
    });

    env.add_function("site_url", move |uri: Option<String>| -> String {
        join(&site_base, uri.as_deref().unwrap_or(""))
    });
}

/// Gabungkan base + path tanpa double slash (versi lokal untuk closure helper).
fn join(base: &str, uri: &str) -> String {
    let uri = uri.trim_start_matches('/');
    if uri.is_empty() {
        base.to_string()
    } else if base.ends_with('/') {
        format!("{base}{uri}")
    } else {
        format!("{base}/{uri}")
    }
}
