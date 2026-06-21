# URL Helper

URL helper menyediakan fungsi untuk membangun URL aplikasi, meniru *URL Helper*
CodeIgniter. Aktif bila `"url"` ada di `config/autoload.toml`:

```toml
helpers = ["url"]
```

## Di Controller

```rust
ctx.base_url("");              // "http://127.0.0.1:8099/"
ctx.base_url("css/app.css");   // "http://127.0.0.1:8099/css/app.css"
ctx.site_url("notes/show/1");  // base + index_page + uri
```

Keduanya menggabungkan path tanpa menggandakan `/`, dan menghormati `base_url` /
`index_page` dari `config/app.toml`.

## Di View

Setelah helper `url` di-autoload, fungsi tersedia langsung di template:

```html
<a href="{{ base_url('notes') }}">Catatan</a>
<link rel="stylesheet" href="{{ base_url('css/app.css') }}">
<form action="{{ site_url('notes/add') }}" method="post">…</form>
```

## Redirect

Untuk berpindah halaman (CI: `redirect()`):

```rust
Response::redirect(&ctx.base_url("notes"))   // 302 Location: …/notes
```

Sering dipakai pada pola **Post/Redirect/Get** setelah submit form berhasil.
