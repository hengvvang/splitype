# Examples

Configuration templates for the two partial-config systems: visual themes
and UI language packs.

| File | System | What it shows |
| --- | --- | --- |
| `custom-theme.example.jsonc` | Theme | A theme family with base inheritance, color/dimension/typography patches, and a light/dark variant pair |
| `custom-language.example.jsonc` | Language pack | Overriding UI strings with a fallback to English |

Both formats are JSONC (JSON with comments) and **partial**: every field is
optional, missing values inherit from the variant's `base` chain (ultimately
the built-in `splitype` family) or the English language pack.

## Trying them out

1. Copy a template into your user config directory (see the app's
   settings/logs for the path) or keep it anywhere on disk.
2. Import it in the app: **Theme → Add Theme Config** or **Language → Add
   Language Config**, then select the file.

The root [`README.md`](../../README.md) describes both systems in detail.
