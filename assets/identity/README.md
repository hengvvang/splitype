# Identity

splitype's visual identity: the logo source file and every asset derived
from it — app icons, the README banner, and the platform bundle icons.

## Files

| File | Role | Generated? |
| --- | --- | --- |
| `logo.svg` | Scalable logo used at runtime (About dialog) | from `scripts/tools/icon-gen/logo.svg` |
| `logo.png` | 1024×1024 rendered logo, used as the app icon; also shown at runtime (About dialog, key `identity/logo.png`) | from `scripts/tools/icon-gen/logo.svg` |
| `logo-16.png` … `logo-512.png` | Rendered logo at 16/32/48/64/128/256/512 px (app icons at each size) | from `scripts/tools/icon-gen/logo.svg` |
| `banner.png` | README hero image | from `scripts/tools/icon-gen/logo.svg` |

Platform formats are generated into `packaging/`, not kept here:

| Output | Path |
| --- | --- |
| Windows executable icon | `packaging/windows/splitype.ico` |
| macOS bundle icon | `packaging/macos/splitype.icns` |
| Linux hicolor icons | `packaging/linux/icons/hicolor/{size}x{size}/apps/com.hengvvang.splitype.png` |

## Regenerating

Edit the single source of truth `scripts/tools/icon-gen/logo.svg`, then regenerate everything from
[`scripts/tools/icon-gen`](../../scripts/tools/icon-gen):

```bash
cd scripts/tools/icon-gen
cargo run --release
```

The generator renders the logo onto white canvases and distributes all outputs:
it copies `logo.svg` and writes `logo*.png` & `banner.png` into this directory, and distributes
the platform formats into `packaging/`.

## Runtime key

`identity/logo.png` is exposed through the GPUI asset source
(`src/app/assets.rs`) for the About dialog; `identity/logo.svg` is mapped
the same way for future use. These keys mirror the file paths under
`assets/`.
