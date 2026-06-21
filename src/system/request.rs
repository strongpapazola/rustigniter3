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
    /// Berkas hasil unggah (multipart/form-data).
    pub files: Vec<UploadedFile>,
}

/// Sebuah berkas yang diunggah lewat form multipart.
#[derive(Debug, Clone)]
pub struct UploadedFile {
    pub field: String,
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
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
            files: Vec::new(),
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
        } else if ct.starts_with("multipart/form-data") {
            if let Some(boundary) = extract_boundary(content_type) {
                let (post, files) = parse_multipart(&boundary, body);
                self.post = post;
                self.files = files;
            }
        }
        self
    }

    /// Berkas unggah pertama untuk sebuah field.
    pub fn file(&self, field: &str) -> Option<&UploadedFile> {
        self.files.iter().find(|f| f.field == field)
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

/// Ambil nilai `boundary=...` dari header Content-Type multipart.
fn extract_boundary(content_type: &str) -> Option<String> {
    let i = content_type.find("boundary=")? + "boundary=".len();
    let b = content_type[i..].trim();
    // boundary bisa dikutip dan/atau diikuti parameter lain.
    let b = b.split(';').next().unwrap_or(b).trim().trim_matches('"');
    if b.is_empty() {
        None
    } else {
        Some(b.to_string())
    }
}

/// Parse body `multipart/form-data` menjadi (field teks, berkas). Binary-safe.
fn parse_multipart(boundary: &str, body: &[u8]) -> (HashMap<String, String>, Vec<UploadedFile>) {
    let mut post = HashMap::new();
    let mut files = Vec::new();
    let delim = format!("--{boundary}");
    let segments = split_on(body, delim.as_bytes());

    // segments[0] = preamble (diabaikan). Sisanya tiap part; yang diawali "--" = penutup.
    for seg in segments.iter().skip(1) {
        if seg.starts_with(b"--") {
            break; // "--boundary--" penutup
        }
        // Buang CRLF pembungkus part.
        let part = seg
            .strip_prefix(b"\r\n")
            .unwrap_or(seg)
            .strip_suffix(b"\r\n")
            .unwrap_or(seg);
        // Pisahkan header dari konten pada CRLFCRLF pertama.
        let Some(sep) = find_sub(part, b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&part[..sep]);
        let content = &part[sep + 4..];

        let (name, filename, ctype) = parse_part_headers(&headers);
        let Some(name) = name else { continue };

        match filename {
            Some(filename) if !filename.is_empty() => files.push(UploadedFile {
                field: name,
                filename,
                content_type: ctype.unwrap_or_else(|| "application/octet-stream".to_string()),
                bytes: content.to_vec(),
            }),
            _ => {
                post.insert(name, String::from_utf8_lossy(content).into_owned());
            }
        }
    }
    (post, files)
}

/// Parse header sebuah part: kembalikan (name, filename, content_type).
fn parse_part_headers(headers: &str) -> (Option<String>, Option<String>, Option<String>) {
    let (mut name, mut filename, mut ctype) = (None, None, None);
    for line in headers.split("\r\n") {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("content-disposition:") {
            name = extract_quoted(line, "name=");
            filename = extract_quoted(line, "filename=");
        } else if lower.starts_with("content-type:") {
            ctype = line.splitn(2, ':').nth(1).map(|s| s.trim().to_string());
        }
    }
    (name, filename, ctype)
}

/// Ambil nilai berkutip dari `key="..."` dalam sebuah string.
fn extract_quoted(s: &str, key: &str) -> Option<String> {
    let start = s.find(key)? + key.len();
    let rest = s.get(start..)?.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Pisahkan `body` pada tiap kemunculan `delim` (potongan tidak menyertakan delim).
fn split_on<'a>(body: &'a [u8], delim: &[u8]) -> Vec<&'a [u8]> {
    let mut segs = Vec::new();
    let mut start = 0;
    while let Some(p) = find_sub(&body[start..], delim) {
        segs.push(&body[start..start + p]);
        start += p + delim.len();
    }
    segs.push(&body[start..]);
    segs
}

/// Cari posisi subslice `needle` dalam `haystack`.
fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
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

    #[test]
    fn body_multipart_field_dan_file() {
        let b = "X";
        let body = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"judul\"\r\n\r\nHalo\r\n\
             --{b}\r\nContent-Disposition: form-data; name=\"berkas\"; filename=\"a.txt\"\r\n\
             Content-Type: text/plain\r\n\r\nisi file\r\n--{b}--\r\n"
        );
        let r = Request::new("POST", "/upload")
            .with_body("multipart/form-data; boundary=X", body.as_bytes());
        assert_eq!(r.post.get("judul").map(String::as_str), Some("Halo"));
        let f = r.file("berkas").unwrap();
        assert_eq!(f.filename, "a.txt");
        assert_eq!(f.content_type, "text/plain");
        assert_eq!(f.bytes, b"isi file");
    }
}
