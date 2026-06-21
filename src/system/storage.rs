//! Storage — abstraksi penyimpanan berkas: lokal atau bucket S3-compatible.
//!
//! Pilih driver di `config/storage.toml`. Backend `s3` bekerja dengan AWS S3 maupun yang
//! kompatibel (MinIO, Cloudflare R2, DigitalOcean Spaces) — cukup isi kredensial di config.
//! Penandatanganan memakai **AWS Signature V4** (PutObject), permintaan HTTP via `reqwest`.
//!
//! `put()` sinkron (dipanggil dari controller); permintaan async dijembatani dengan
//! `block_in_place` + `Handle::block_on`, sama seperti driver Postgres.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Konfigurasi bucket S3-compatible.
pub struct S3Config {
    pub endpoint: String, // mis. "http://127.0.0.1:9100" atau "https://s3.amazonaws.com"
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub public_base: String, // URL publik objek; kosong = "{endpoint}/{bucket}"
    pub client: reqwest::Client,
}

/// Penyimpanan berkas aktif.
pub enum Storage {
    /// Tulis ke direktori lokal; kembalikan URL berbasis `url_base`.
    Local { dir: String, url_base: String },
    /// Unggah ke bucket S3-compatible.
    S3(S3Config),
}

impl Storage {
    pub fn local(dir: &str, url_base: &str) -> Self {
        let _ = std::fs::create_dir_all(dir);
        Storage::Local {
            dir: dir.to_string(),
            url_base: url_base.to_string(),
        }
    }

    pub fn driver(&self) -> &'static str {
        match self {
            Storage::Local { .. } => "local",
            Storage::S3(_) => "s3",
        }
    }

    /// Simpan `bytes` di bawah `key`. Kembalikan URL untuk mengakses berkas.
    pub fn put(&self, key: &str, bytes: &[u8], content_type: &str) -> Result<String, String> {
        match self {
            Storage::Local { dir, url_base } => {
                std::fs::create_dir_all(dir).map_err(|e| format!("buat dir gagal: {e}"))?;
                std::fs::write(format!("{dir}/{key}"), bytes)
                    .map_err(|e| format!("tulis berkas gagal: {e}"))?;
                Ok(join_url(url_base, key))
            }
            Storage::S3(cfg) => s3_put(cfg, key, bytes, content_type),
        }
    }
}

/// PutObject ke S3 dengan tanda tangan SigV4.
fn s3_put(cfg: &S3Config, key: &str, bytes: &[u8], content_type: &str) -> Result<String, String> {
    let scheme = if cfg.endpoint.starts_with("https") { "https" } else { "http" };
    let host = cfg
        .endpoint
        .split("://")
        .nth(1)
        .unwrap_or(&cfg.endpoint)
        .trim_end_matches('/')
        .to_string();
    let canonical_uri = format!("/{}/{}", cfg.bucket, key);
    let url = format!("{scheme}://{host}{canonical_uri}");

    let payload_hash = hex(sha256(bytes));
    let (amzdate, datestamp) = now_amz();
    let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";
    let canonical_headers = format!(
        "content-type:{content_type}\nhost:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amzdate}\n"
    );
    let canonical_request =
        format!("PUT\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let scope = format!("{datestamp}/{}/s3/aws4_request", cfg.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amzdate}\n{scope}\n{}",
        hex(sha256(canonical_request.as_bytes()))
    );
    let signing_key = signature_key(&cfg.secret_key, &datestamp, &cfg.region, "s3");
    let signature = hex(hmac(&signing_key, string_to_sign.as_bytes()));
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        cfg.access_key
    );

    let client = cfg.client.clone();
    let body = bytes.to_vec();
    let ct = content_type.to_string();
    let req_url = url.clone();

    let (status, text) = run_blocking(async move {
        let resp = client
            .put(&req_url)
            .header("content-type", ct)
            .header("x-amz-content-sha256", payload_hash)
            .header("x-amz-date", amzdate)
            .header("authorization", authorization)
            .body(body)
            .send()
            .await
            .map_err(|e| format!("kirim ke S3 gagal: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Ok::<_, String>((status, text))
    })?;

    if !status.is_success() {
        return Err(format!("S3 menolak ({status}): {text}"));
    }

    let public = if cfg.public_base.is_empty() {
        format!("{}/{}/{}", cfg.endpoint.trim_end_matches('/'), cfg.bucket, key)
    } else {
        join_url(&cfg.public_base, key)
    };
    Ok(public)
}

/// Jalankan future dari konteks sinkron (worker runtime tokio multi-thread).
fn run_blocking<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

fn join_url(base: &str, key: &str) -> String {
    format!("{}/{}", base.trim_end_matches('/'), key.trim_start_matches('/'))
}

fn sha256(data: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().to_vec()
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("kunci HMAC");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn signature_key(secret: &str, datestamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac(format!("AWS4{secret}").as_bytes(), datestamp.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, service.as_bytes());
    hmac(&k_service, b"aws4_request")
}

fn hex(bytes: Vec<u8>) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// (amzdate "YYYYMMDDTHHMMSSZ", datestamp "YYYYMMDD") UTC dari jam sistem.
fn now_amz() -> (String, String) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    (
        format!("{y:04}{m:02}{d:02}T{h:02}{mi:02}{s:02}Z"),
        format!("{y:04}{m:02}{d:02}"),
    )
}

/// Hari sejak epoch -> (tahun, bulan, hari). Algoritma Howard Hinnant.
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

#[cfg(test)]
mod tests {
    use super::*;

    // Vektor uji resmi AWS SigV4 untuk turunan signing key.
    // https://docs.aws.amazon.com/general/latest/gr/signature-v4-examples.html
    #[test]
    fn signing_key_sesuai_vektor_aws() {
        let key = signature_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        assert_eq!(
            hex(key),
            "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9"
        );
    }
}
