//! Router — pemetaan URI ke controller/method.
//!
//! Ide dari `CI_Router`. Aturan dasar CodeIgniter:
//!   `example.com/class/method/arg1/arg2`
//! - URI kosong  -> `default_controller`
//! - cocokkan *custom routes* lebih dulu; `(:any)` -> `[^/]+`, `(:num)` -> `[0-9]+`,
//!   backreference `$1`, `$2`, ... pada target.
//! - jika tak ada custom route: segmen[0]=controller, segmen[1]=method (default "index"),
//!   sisanya = argumen.
//! - `translate_uri_dashes`: ganti '-' jadi '_' pada nama controller & method.
//!
//! Catatan adaptasi: Router HANYA menentukan *nama* controller/method. Apakah controller
//! benar-benar terdaftar / method dikenal, diputuskan saat dispatch (lihat `registry` &
//! `Controller`). Bila tidak, framework memakai `not_found_override` atau membalas 404.

use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Satu aturan custom route dari `routes.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteEntry {
    pub from: String,
    pub to: String,
}

/// Konfigurasi routing (reserved routes + custom routes), hasil baca `routes.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct RoutesConfig {
    #[serde(default = "default_controller_default")]
    pub default_controller: String,
    #[serde(default)]
    pub not_found_override: String,
    #[serde(default)]
    pub translate_uri_dashes: bool,
    #[serde(default, rename = "routes")]
    pub routes: Vec<RouteEntry>,
}

fn default_controller_default() -> String {
    "welcome".to_string()
}

impl RoutesConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let raw = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("gagal membaca {}: {e}", path.as_ref().display()))?;
        toml::from_str(&raw).map_err(|e| format!("routes TOML tidak valid: {e}"))
    }
}

/// Hasil resolusi: controller, method, dan argumen yang akan diteruskan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    pub controller: String,
    pub method: String,
    pub args: Vec<String>,
}

/// Custom route yang sudah dikompilasi menjadi regex.
struct CompiledRoute {
    re: Regex,
    to: String,
}

/// Router siap pakai (custom routes sudah dikompilasi sekali di awal).
pub struct Router {
    default_controller: String,
    not_found_override: String,
    translate_uri_dashes: bool,
    compiled: Vec<CompiledRoute>,
}

impl Router {
    /// Bangun Router dari konfigurasi. Mengembalikan error bila ada pola route
    /// yang tidak bisa dikompilasi menjadi regex.
    pub fn new(cfg: RoutesConfig) -> Result<Self, String> {
        let mut compiled = Vec::new();
        for entry in &cfg.routes {
            let pattern = ci_pattern_to_regex(&entry.from);
            let re = Regex::new(&pattern)
                .map_err(|e| format!("route '{}' tidak valid: {e}", entry.from))?;
            compiled.push(CompiledRoute {
                re,
                to: entry.to.clone(),
            });
        }
        Ok(Self {
            default_controller: cfg.default_controller,
            not_found_override: cfg.not_found_override,
            translate_uri_dashes: cfg.translate_uri_dashes,
            compiled,
        })
    }

    /// Target untuk kasus tidak-ditemukan (CI: `404_override`), bila diset.
    pub fn not_found_override(&self) -> Option<Dispatch> {
        if self.not_found_override.is_empty() {
            None
        } else {
            Some(self.parse_target(&self.not_found_override, Vec::new()))
        }
    }

    /// Petakan segmen URI menjadi `Dispatch`.
    pub fn resolve(&self, segments: &[String]) -> Dispatch {
        // 1) URI kosong -> default controller.
        if segments.is_empty() {
            return self.parse_target(&self.default_controller, Vec::new());
        }

        // 2) Custom routes (dicocokkan terhadap path utuh).
        let uri = segments.join("/");
        for route in &self.compiled {
            if let Some(caps) = route.re.captures(&uri) {
                let target = substitute_backrefs(&route.to, &caps);
                return self.parse_target(&target, Vec::new());
            }
        }

        // 3) Default mapping: class / method / args...
        let controller = self.translate(&segments[0]);
        let method = segments
            .get(1)
            .map(|m| self.translate(m))
            .unwrap_or_else(|| "index".to_string());
        let args = segments.iter().skip(2).cloned().collect();

        Dispatch {
            controller,
            method,
            args,
        }
    }

    /// Parse string "class" atau "class/method/arg..." menjadi `Dispatch`.
    fn parse_target(&self, target: &str, extra_args: Vec<String>) -> Dispatch {
        let parts: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
        let controller = parts
            .first()
            .map(|s| self.translate(s))
            .unwrap_or_else(|| self.default_controller.clone());
        let method = parts
            .get(1)
            .map(|s| self.translate(s))
            .unwrap_or_else(|| "index".to_string());
        let mut args: Vec<String> = parts.iter().skip(2).map(|s| s.to_string()).collect();
        args.extend(extra_args);
        Dispatch {
            controller,
            method,
            args,
        }
    }

    /// Terapkan translate_uri_dashes pada nama controller/method.
    fn translate(&self, s: &str) -> String {
        if self.translate_uri_dashes {
            s.replace('-', "_")
        } else {
            s.to_string()
        }
    }
}

/// Ubah pola gaya CI menjadi regex penuh ber-anchor.
/// `(:any)` -> `([^/]+)`, `(:num)` -> `([0-9]+)`. Pola lain dibiarkan apa adanya
/// (memungkinkan regex kustom langsung seperti di CodeIgniter).
fn ci_pattern_to_regex(pattern: &str) -> String {
    let replaced = pattern
        .replace("(:any)", "([^/]+)")
        .replace("(:num)", "([0-9]+)");
    format!("^{replaced}$")
}

/// Ganti `$1`, `$2`, ... pada target dengan grup tangkapan regex.
fn substitute_backrefs(to: &str, caps: &regex::Captures) -> String {
    let mut out = String::with_capacity(to.len());
    let bytes = to.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let idx: usize = to[i + 1..j].parse().unwrap_or(0);
            if let Some(m) = caps.get(idx) {
                out.push_str(m.as_str());
            }
            i = j;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn router_with(routes: Vec<(&str, &str)>, dashes: bool) -> Router {
        let cfg = RoutesConfig {
            default_controller: "welcome".into(),
            not_found_override: "errors/not_found".into(),
            translate_uri_dashes: dashes,
            routes: routes
                .into_iter()
                .map(|(f, t)| RouteEntry {
                    from: f.into(),
                    to: t.into(),
                })
                .collect(),
        };
        Router::new(cfg).unwrap()
    }

    fn segs(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn uri_kosong_pakai_default_controller() {
        let r = router_with(vec![], false);
        let d = r.resolve(&[]);
        assert_eq!(d.controller, "welcome");
        assert_eq!(d.method, "index");
        assert!(d.args.is_empty());
    }

    #[test]
    fn default_mapping_class_method_args() {
        let r = router_with(vec![], false);
        let d = r.resolve(&segs(&["blog", "lihat", "5", "comments"]));
        assert_eq!(d.controller, "blog");
        assert_eq!(d.method, "lihat");
        assert_eq!(d.args, vec!["5", "comments"]);
    }

    #[test]
    fn satu_segmen_method_default_index() {
        let r = router_with(vec![], false);
        let d = r.resolve(&segs(&["produk"]));
        assert_eq!(d.controller, "produk");
        assert_eq!(d.method, "index");
    }

    #[test]
    fn custom_route_num_dengan_backref() {
        let r = router_with(vec![("produk/(:num)", "katalog/lihat/$1")], false);
        let d = r.resolve(&segs(&["produk", "42"]));
        assert_eq!(d.controller, "katalog");
        assert_eq!(d.method, "lihat");
        assert_eq!(d.args, vec!["42"]);
    }

    #[test]
    fn custom_route_any() {
        let r = router_with(vec![("blog/(:any)", "blog/post/$1")], false);
        let d = r.resolve(&segs(&["blog", "halo-dunia"]));
        assert_eq!(d.controller, "blog");
        assert_eq!(d.method, "post");
        assert_eq!(d.args, vec!["halo-dunia"]);
    }

    #[test]
    fn num_tidak_cocok_huruf_jatuh_ke_default() {
        // "(:num)" tidak match "abc", jadi pakai mapping default.
        let r = router_with(vec![("produk/(:num)", "katalog/lihat/$1")], false);
        let d = r.resolve(&segs(&["produk", "abc"]));
        assert_eq!(d.controller, "produk");
        assert_eq!(d.method, "abc");
    }

    #[test]
    fn translate_uri_dashes_aktif() {
        let r = router_with(vec![], true);
        let d = r.resolve(&segs(&["my-controller", "my-method"]));
        assert_eq!(d.controller, "my_controller");
        assert_eq!(d.method, "my_method");
    }

    #[test]
    fn not_found_override_diparse() {
        let r = router_with(vec![], false);
        let d = r.not_found_override().unwrap();
        assert_eq!(d.controller, "errors");
        assert_eq!(d.method, "not_found");
    }
}
