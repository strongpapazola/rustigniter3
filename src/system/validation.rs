//! Form Validation — validasi input ala `CI_Form_validation`.
//!
//! Meniru rasa CodeIgniter:
//!
//! ```ignore
//! let errors = Validator::new(ctx.post_data())
//!     .rule("text", "Catatan", "required|min_length[3]|max_length[200]")
//!     .rule("email", "Email", "required")
//!     .validate();
//! if errors.is_empty() { /* lolos */ } else { /* errors.messages() */ }
//! ```
//!
//! Aturan yang didukung milestone 3: `required`, `min_length[n]`, `max_length[n]`,
//! `exact_length[n]`, `numeric`, `integer`, `matches[field]`. Pesan kesalahan
//! berbahasa Indonesia dan memakai *label* field.

use std::collections::HashMap;

/// Satu aturan validasi terkompilasi.
#[derive(Debug, Clone, PartialEq)]
enum Rule {
    Required,
    MinLength(usize),
    MaxLength(usize),
    ExactLength(usize),
    Numeric,
    Integer,
    Matches(String),
}

/// Kumpulan aturan untuk satu field beserta label-nya.
struct FieldRules {
    field: String,
    label: String,
    rules: Vec<Rule>,
}

/// Hasil validasi: daftar pasangan (field, pesan). Kosong = lolos.
#[derive(Debug, Clone, Default)]
pub struct Errors {
    items: Vec<(String, String)>,
}

impl Errors {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Semua pesan kesalahan (untuk ditampilkan sebagai daftar di view).
    pub fn messages(&self) -> Vec<String> {
        self.items.iter().map(|(_, m)| m.clone()).collect()
    }

    /// Pesan pertama untuk field tertentu (CI: `form_error('field')`).
    pub fn for_field(&self, field: &str) -> Option<&str> {
        self.items
            .iter()
            .find(|(f, _)| f == field)
            .map(|(_, m)| m.as_str())
    }
}

/// Pembangun + pelaksana validasi atas sekumpulan data input.
pub struct Validator<'a> {
    data: &'a HashMap<String, String>,
    fields: Vec<FieldRules>,
}

impl<'a> Validator<'a> {
    /// Buat validator atas data input (mis. `ctx.post_data()`).
    pub fn new(data: &'a HashMap<String, String>) -> Self {
        Self {
            data,
            fields: Vec::new(),
        }
    }

    /// Tetapkan aturan untuk sebuah field (CI: `set_rules`). `rules` dipisah `|`.
    pub fn rule(mut self, field: &str, label: &str, rules: &str) -> Self {
        self.fields.push(FieldRules {
            field: field.to_string(),
            label: label.to_string(),
            rules: parse_rules(rules),
        });
        self
    }

    /// Jalankan semua aturan, kumpulkan kesalahan.
    pub fn validate(&self) -> Errors {
        let mut errors = Errors::default();
        for fr in &self.fields {
            let value = self.data.get(&fr.field).map(String::as_str).unwrap_or("");
            for rule in &fr.rules {
                if let Some(msg) = check(rule, value, &fr.label, self.data) {
                    errors.items.push((fr.field.clone(), msg));
                    // Satu pesan per field sudah cukup — lanjut ke field berikutnya.
                    break;
                }
            }
        }
        errors
    }
}

/// Uraikan string aturan "required|min_length[3]" menjadi daftar `Rule`.
fn parse_rules(s: &str) -> Vec<Rule> {
    s.split('|')
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .filter_map(parse_one)
        .collect()
}

fn parse_one(token: &str) -> Option<Rule> {
    // Pisahkan "nama[param]".
    let (name, param) = match token.split_once('[') {
        Some((n, rest)) => (n, rest.strip_suffix(']').unwrap_or(rest)),
        None => (token, ""),
    };
    match name {
        "required" => Some(Rule::Required),
        "numeric" => Some(Rule::Numeric),
        "integer" => Some(Rule::Integer),
        "min_length" => param.parse().ok().map(Rule::MinLength),
        "max_length" => param.parse().ok().map(Rule::MaxLength),
        "exact_length" => param.parse().ok().map(Rule::ExactLength),
        "matches" => Some(Rule::Matches(param.to_string())),
        _ => None, // aturan tak dikenal diabaikan
    }
}

/// Periksa satu aturan; `Some(pesan)` bila gagal, `None` bila lolos.
/// Catatan: aturan selain `required` dilewati bila nilai kosong (seperti CI).
fn check(rule: &Rule, value: &str, label: &str, data: &HashMap<String, String>) -> Option<String> {
    let empty = value.trim().is_empty();
    match rule {
        Rule::Required => {
            if empty {
                Some(format!("{label} wajib diisi."))
            } else {
                None
            }
        }
        _ if empty => None,
        Rule::MinLength(n) => {
            if value.chars().count() < *n {
                Some(format!("{label} minimal {n} karakter."))
            } else {
                None
            }
        }
        Rule::MaxLength(n) => {
            if value.chars().count() > *n {
                Some(format!("{label} maksimal {n} karakter."))
            } else {
                None
            }
        }
        Rule::ExactLength(n) => {
            if value.chars().count() != *n {
                Some(format!("{label} harus tepat {n} karakter."))
            } else {
                None
            }
        }
        Rule::Numeric => {
            if value.parse::<f64>().is_err() {
                Some(format!("{label} harus berupa angka."))
            } else {
                None
            }
        }
        Rule::Integer => {
            if value.parse::<i64>().is_err() {
                Some(format!("{label} harus berupa bilangan bulat."))
            } else {
                None
            }
        }
        Rule::Matches(other) => {
            let other_val = data.get(other).map(String::as_str).unwrap_or("");
            if value != other_val {
                Some(format!("{label} tidak cocok."))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn required_gagal_saat_kosong() {
        let d = data(&[("text", "  ")]);
        let e = Validator::new(&d).rule("text", "Catatan", "required").validate();
        assert_eq!(e.len(), 1);
        assert_eq!(e.for_field("text"), Some("Catatan wajib diisi."));
    }

    #[test]
    fn min_dan_max_length() {
        let d = data(&[("text", "ab")]);
        let e = Validator::new(&d)
            .rule("text", "Catatan", "required|min_length[3]")
            .validate();
        assert_eq!(e.for_field("text"), Some("Catatan minimal 3 karakter."));

        let d2 = data(&[("text", "halo dunia")]);
        let e2 = Validator::new(&d2)
            .rule("text", "Catatan", "min_length[3]|max_length[200]")
            .validate();
        assert!(e2.is_empty());
    }

    #[test]
    fn numeric_dan_integer() {
        let d = data(&[("umur", "abc")]);
        let e = Validator::new(&d).rule("umur", "Umur", "integer").validate();
        assert_eq!(e.for_field("umur"), Some("Umur harus berupa bilangan bulat."));

        let d2 = data(&[("umur", "21")]);
        let e2 = Validator::new(&d2).rule("umur", "Umur", "integer|numeric").validate();
        assert!(e2.is_empty());
    }

    #[test]
    fn matches_field_lain() {
        let d = data(&[("pass", "rahasia"), ("konfirmasi", "beda")]);
        let e = Validator::new(&d)
            .rule("konfirmasi", "Konfirmasi", "matches[pass]")
            .validate();
        assert_eq!(e.for_field("konfirmasi"), Some("Konfirmasi tidak cocok."));
    }

    #[test]
    fn satu_pesan_per_field() {
        // "required" gagal -> "min_length" tak ikut dilaporkan (berhenti di gagal pertama).
        let d = data(&[("text", "")]);
        let e = Validator::new(&d)
            .rule("text", "Catatan", "required|min_length[3]")
            .validate();
        assert_eq!(e.len(), 1);
    }
}
