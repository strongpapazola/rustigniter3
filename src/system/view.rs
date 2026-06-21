//! View — mesin render template.
//!
//! Ide dari template view CodeIgniter (`$this->load->view('nama', $data)`), tapi
//! alih-alih template PHP kita pakai **minijinja** (sintaks ala Jinja2, pure-Rust).
//! Template dimuat dari direktori `views` (default `src/app/views`) dengan ekstensi
//! `.html`, jadi `view("welcome_message")` me-render `welcome_message.html`.

use crate::system::config::Config;
use crate::system::loader;
use minijinja::Environment;

/// Pembungkus environment minijinja beserta helper view yang sudah diregistrasi.
pub struct View {
    env: Environment<'static>,
}

impl View {
    /// Bangun engine: pasang loader direktori dan registrasikan helper autoload
    /// (mis. helper "url" menambahkan fungsi `base_url`/`site_url` ke template).
    pub fn new(views_dir: &str, config: &Config, autoload_helpers: &[String]) -> Self {
        let mut env = Environment::new();
        env.set_loader(minijinja::path_loader(views_dir));
        loader::register_helpers(&mut env, config, autoload_helpers);
        Self { env }
    }

    /// Render template `name` (tanpa ekstensi) dengan data JSON.
    pub fn render(&self, name: &str, data: &serde_json::Value) -> Result<String, String> {
        let file = format!("{name}.html");
        let tmpl = self
            .env
            .get_template(&file)
            .map_err(|e| format!("view '{name}' tidak ditemukan ({file}): {e}"))?;
        tmpl.render(data)
            .map_err(|e| format!("gagal render view '{name}': {e}"))
    }
}
