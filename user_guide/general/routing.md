# URI Routing

Secara default, URL dipetakan satu-ke-satu ke `controller/method/argumen`. Namun kamu bisa
**meremap** URL lewat `config/routes.toml`.

## Reserved Routes

```toml
default_controller   = "welcome"   # controller saat URL kosong
not_found_override   = ""           # controller saat tidak ada match (CI: 404_override)
translate_uri_dashes = false        # ganti '-' jadi '_' pada nama controller/method
```

- **default_controller** — boleh `"welcome"` atau `"welcome/index"`.
- **not_found_override** — mis. `"errors/show_404"`; bila kosong → 404 bawaan.
- **translate_uri_dashes** — `my-controller/my-method` → `my_controller/my_method`.

## Custom Routes

Setiap aturan adalah pasangan `from` → `to`:

```toml
[[routes]]
from = "produk/(:num)"
to   = "katalog/lihat/$1"

[[routes]]
from = "blog/(:any)"
to   = "blog/post/$1"
```

### Placeholder

| Placeholder | Cocok dengan | Regex      |
|-------------|--------------|------------|
| `(:num)`    | angka        | `[0-9]+`   |
| `(:any)`    | satu segmen apa pun | `[^/]+` |

Grup yang ditangkap dirujuk dengan `$1`, `$2`, … pada target.

> Pola lain ditulis apa adanya sebagai regex, jadi kamu bisa memakai regex penuh bila perlu.

## Contoh: routing REST

REST resource memetakan beberapa URL ke satu controller resource (lihat
[REST Resource](../libraries/rest.md)):

```toml
[[routes]]
from = "api/notes"
to   = "notes_api/_resource"

[[routes]]
from = "api/notes/(:num)"
to   = "notes_api/_resource/$1"
```

## Urutan Resolusi

1. URL kosong → `default_controller`.
2. Cocokkan **custom routes** (berurutan, match pertama menang).
3. Mapping default `class/method/args`.
4. Tidak ada controller cocok → `not_found_override`, lalu 404.
