# Views

View adalah berkas template HTML. RustIgniter memakai **minijinja** (sintaks ala Jinja2,
pure-Rust) sebagai pengganti template PHP CodeIgniter.

## Membuat View

Simpan di `src/app/views/`, berekstensi `.html`:

```html
<!-- src/app/views/welcome_message.html -->
<!doctype html>
<title>Selamat Datang di {{ app_name }}</title>
<h1>Halo, {{ app_name }} 🦀</h1>
<p>Base URL: {{ base_url() }}</p>
```

## Memuat View

Dari controller (CI: `$this->load->view('welcome_message', $data)`):

```rust
ctx.view("welcome_message", json!({ "app_name": "RustIgniter" }))
```

`view()` mengembalikan `Response` HTML (status 200).

## Mengirim Data

Data dikirim sebagai objek JSON dan tersedia sebagai variabel di template:

```rust
ctx.view("notes_index", json!({
    "notes": notes,        // array of objek
    "errors": [],
    "old_text": "",
}))
```

```html
<ul>
{% for n in notes %}
  <li>#{{ n.id }} — {{ n.text }}</li>
{% else %}
  <li>Belum ada catatan.</li>
{% endfor %}
</ul>
```

### Mengumpulkan variabel bertahap

```rust
ctx.set("judul", "Catatan");
ctx.vars(json!({ "user": "Budi" }));
ctx.view("halaman", json!({}));   // 'judul' & 'user' ikut terbawa
```

## Auto-escaping (Keamanan XSS)

Output `{{ ... }}` **di-escape otomatis** oleh minijinja. Jadi input pengguna yang
mengandung `<script>` akan ditampilkan sebagai teks, bukan dieksekusi — proteksi XSS
bawaan. (Inilah kenapa URL pada `href` muncul sebagai `&#x2f;` di-source — tetap valid di
browser.)

## Helper View

Bila helper `url` di-autoload (lihat [Config & Auto-load](config.md)), template bisa
memanggil:

```html
<a href="{{ base_url('notes') }}">Catatan</a>
<a href="{{ site_url('notes/show/1') }}">Detail</a>
```
