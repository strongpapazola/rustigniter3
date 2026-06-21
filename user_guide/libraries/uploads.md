# File Upload & Storage

RustIgniter mem-parse body `multipart/form-data` (field teks → `post`, berkas →
`UploadedFile`) lalu menyimpannya lewat abstraksi **Storage** yang bisa diarahkan ke disk
lokal **atau bucket S3-compatible** — cukup ganti config, tanpa ubah kode.

## Form

`enctype="multipart/form-data"` + token CSRF:

```html
<form action="{{ base_url('uploads/save') }}" method="post" enctype="multipart/form-data">
    <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
    <input type="file" name="berkas" required>
    <button type="submit">Unggah</button>
</form>
```

## Mengakses & menyimpan

```rust
let file = ctx.file("berkas").unwrap();        // field, filename, content_type, bytes
let key = format!("{}-{}", &random_token()[..8], safe_filename(&file.filename));
let url = ctx.storage().put(&key, &file.bytes, &file.content_type)?;   // -> URL berkas
```

`ctx.file(field)` / `ctx.files()` mengakses berkas. `ctx.storage().put(key, bytes, type)`
mengembalikan URL untuk diakses (path static untuk lokal, URL objek untuk S3).

## Konfigurasi — `config/storage.toml`

### Lokal (default)

```toml
driver = "local"
[local]
dir = "public/uploads"      # disimpan di sini
url_base = "assets/uploads" # dilayani sebagai static (lihat /assets)
```

### Bucket S3-compatible (AWS S3 / MinIO / R2 / Spaces)

```toml
driver = "s3"
[s3]
endpoint = "http://127.0.0.1:9100"   # AWS: "https://s3.<region>.amazonaws.com"
region = "us-east-1"
bucket = "uploads"
access_key = "..."                    # isi kredensial bucket-mu di sini
secret_key = "..."
public_base = ""                      # URL publik objek; kosong = "{endpoint}/{bucket}"
```

Backend `s3` menandatangani permintaan **PutObject** dengan **AWS Signature V4** dan
mengirimnya via HTTP — tidak butuh SDK/libpq. Ganti `driver` ke `"s3"` dan isi kredensial;
kode controller/model tetap sama.

## Keamanan

- **Sanitasi nama berkas** sebelum dipakai sebagai key (`safe_filename` di
  `src/app/controllers/uploads.rs`).
- Field `csrf_token` dalam multipart tetap divalidasi [CsrfGuard](../general/security.md).

Daftar berkas dibaca dari tabel `uploads` (model `Upload`), jadi tampilannya sama untuk
backend lokal maupun S3. Demo: buka `/uploads`.
