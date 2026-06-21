//! Response — builder respons HTTP.
//!
//! Ide dari `CI_Output`: controller membangun objek Response (status, header, body)
//! lalu framework mengubahnya menjadi respons hyper di lapisan transport.

use http_body_util::Full;
use hyper::body::Bytes;

/// Respons yang dibangun controller, independen dari hyper.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    /// Respons HTML 200.
    pub fn html(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
            body: body.into(),
        }
    }

    /// Respons teks biasa dengan status tertentu.
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())],
            body: body.into(),
        }
    }

    /// Respons JSON dengan status tertentu (CI: `$this->output->set_content_type('application/json')`).
    pub fn json(status: u16, value: serde_json::Value) -> Self {
        Self {
            status,
            headers: vec![(
                "Content-Type".into(),
                "application/json; charset=utf-8".into(),
            )],
            body: value.to_string(),
        }
    }

    /// Redirect 302 ke `location` (CI: `redirect()` dari URL helper).
    pub fn redirect(location: &str) -> Self {
        Self {
            status: 302,
            headers: vec![("Location".into(), location.to_string())],
            body: String::new(),
        }
    }

    /// Respons 404 standar (dipakai saat controller/method tidak ditemukan).
    pub fn not_found(body: impl Into<String>) -> Self {
        let mut r = Self::html(body);
        r.status = 404;
        r
    }

    /// Ganti/tambah header.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Konversi ke respons hyper untuk dikirim ke socket.
    pub fn into_hyper(self) -> hyper::Response<Full<Bytes>> {
        let mut builder = hyper::Response::builder().status(self.status);
        for (k, v) in &self.headers {
            builder = builder.header(k, v);
        }
        builder
            .body(Full::new(Bytes::from(self.body)))
            .expect("response valid")
    }
}
