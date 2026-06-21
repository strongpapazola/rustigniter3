//! Request — pembungkus HTTP request masuk.
//!
//! Ide dari `CI_Input` + `CI_URI`: kita pecah path menjadi *segmen* (seperti
//! `$this->uri->segment()`) dan sediakan akses query string. Untuk milestone 1
//! cukup method, path, segmen, dan query (body/POST menyusul di milestone berikut).

use std::collections::HashMap;

/// Representasi request yang sudah dinormalisasi untuk dipakai Router & Controller.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// Metode HTTP, mis. "GET".
    pub method: String,
    /// Path mentah, mis. "/welcome/index".
    pub path: String,
    /// Segmen URI hasil pecah path, mis. ["welcome", "index"].
    pub segments: Vec<String>,
    /// Pasangan query string, mis. ?id=5 -> {"id": "5"}.
    pub query: HashMap<String, String>,
    /// Field body form (application/x-www-form-urlencoded), mis. POST.
    pub post: HashMap<String, String>,
    /// Body JSON terurai bila Content-Type application/json (REST API).
    pub json: Option<serde_json::Value>,
    /// Header request, key di-lowercase (mis. "x-api-key").
    pub headers: HashMap<String, String>,
}

impl Request {
    /// Bangun `Request` dari metode dan URI mentah (path + query). `post` kosong;
    /// gunakan [`Request::with_body`] untuk mengisi field form.
    pub fn new(method: &str, raw_uri: &str) -> Self {
        let (path, query_str) = match raw_uri.split_once('?') {
            Some((p, q)) => (p, q),
            None => (raw_uri, ""),
        };

        let segments = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(decode_segment)
            .collect();

        let query = parse_query(query_str);

        Self {
            method: method.to_string(),
            path: path.to_string(),
            segments,
            query,
            post: HashMap::new(),
            json: None,
            headers: HashMap::new(),
        }
    }

    /// Pasang header request (key akan di-lowercase).
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
        self
    }

    /// Ambil header berdasarkan nama (case-insensitive). CI: `$this->input->get_request_header()`.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(String::as_str)
    }

    /// Ambil nilai cookie berdasarkan nama (di-parse dari header `Cookie`).
    pub fn cookie(&self, name: &str) -> Option<String> {
        let raw = self.headers.get("cookie")?;
        parse_cookies(raw).remove(name)
    }

    /// Isi body sesuai Content-Type:
    /// - `application/x-www-form-urlencoded` -> `post`
    /// - `application/json` -> `json`
    pub fn with_body(mut self, content_type: &str, body: &[u8]) -> Self {
        let ct = content_type.to_ascii_lowercase();
        if ct.starts_with("application/x-www-form-urlencoded") {
            let raw = String::from_utf8_lossy(body);
            self.post = parse_query(&raw);
        } else if ct.starts_with("application/json") {
            self.json = serde_json::from_slice(body).ok();
        }
        self
    }

    /// Ambil field POST berdasarkan key (CI: `$this->input->post('field')`).
    pub fn post(&self, key: &str) -> Option<&str> {
        self.post.get(key).map(String::as_str)
    }

    /// Segmen URI berbasis-1 ala CodeIgniter: `segment(1)` = segmen pertama.
    pub fn segment(&self, n: usize) -> Option<&str> {
        if n == 0 {
            return None;
        }
        self.segments.get(n - 1).map(String::as_str)
    }

    /// Ambil nilai query berdasarkan key, mis. `query("id")`.
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(String::as_str)
    }
}

/// Parse header Cookie "a=1; b=2" menjadi map.
fn parse_cookies(raw: &str) -> HashMap<String, String> {
    raw.split(';')
        .filter_map(|pair| {
            let pair = pair.trim();
            pair.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// Parse "a=1&b=2" menjadi map. Decoding sangat minimal (cukup untuk dev).
fn parse_query(q: &str) -> HashMap<String, String> {
    q.split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((k, v)) => (decode_segment(k), decode_segment(v)),
            None => (decode_segment(pair), String::new()),
        })
        .collect()
}

/// Percent-decode minimal: ubah '+' jadi spasi dan "%XX" jadi byte-nya.
fn decode_segment(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 3;
                    }
                    _ => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pecah_segmen_dan_abaikan_slash_kosong() {
        let r = Request::new("GET", "/welcome/index/");
        assert_eq!(r.segments, vec!["welcome", "index"]);
        assert_eq!(r.segment(1), Some("welcome"));
        assert_eq!(r.segment(2), Some("index"));
        assert_eq!(r.segment(3), None);
    }

    #[test]
    fn root_menghasilkan_segmen_kosong() {
        let r = Request::new("GET", "/");
        assert!(r.segments.is_empty());
    }

    #[test]
    fn parse_query_string() {
        let r = Request::new("GET", "/cari?q=rust+igniter&hal=2");
        assert_eq!(r.query("q"), Some("rust igniter"));
        assert_eq!(r.query("hal"), Some("2"));
        assert_eq!(r.query("none"), None);
    }

    #[test]
    fn parse_body_urlencoded() {
        let r = Request::new("POST", "/notes/add").with_body(
            "application/x-www-form-urlencoded",
            b"text=Halo+dunia&extra=1",
        );
        assert_eq!(r.post("text"), Some("Halo dunia"));
        assert_eq!(r.post("extra"), Some("1"));
        assert_eq!(r.post("none"), None);
    }

    #[test]
    fn cookie_diparse_dari_header() {
        let mut headers = HashMap::new();
        headers.insert("Cookie".to_string(), "ri_session=abc123; theme=dark".to_string());
        let r = Request::new("GET", "/").with_headers(headers);
        assert_eq!(r.cookie("ri_session").as_deref(), Some("abc123"));
        assert_eq!(r.cookie("theme").as_deref(), Some("dark"));
        assert_eq!(r.cookie("none"), None);
    }

    #[test]
    fn body_json_diparse_bukan_post() {
        let r = Request::new("POST", "/api").with_body("application/json", b"{\"text\":\"halo\"}");
        assert!(r.post.is_empty());
        assert_eq!(r.json.as_ref().unwrap()["text"], serde_json::json!("halo"));
    }
}
