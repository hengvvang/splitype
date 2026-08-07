# Identity

splitype's visual identity: the logo source file and every asset derived
from it — app icons, the README banner, and the platform bundle icons.

## Files

| File | Role | Generated? |
| --- | --- | --- |
| `logo.svg` | Single source of truth for the logo | hand-edited source |
| `logo.png` | 1024×1024 rendered logo, used as the app icon; also shown at runtime (About dialog, key `identity/logo.png`) | from `logo.svg` |
| `logo-16.png` … `logo-512.png` | Rendered logo at 16/32/48/64/128/256/512 px (app icons at each size) | from `logo.svg` |
| `banner.png` | README hero image | from `logo.svg` |

Platform formats are generated into `resources/`, not kept here:

| Output | Path |
| --- | --- |
| Windows executable icon | `resources/windows/splitype.ico` |
| macOS bundle icon | `resources/macos/splitype.icns` |
| Linux hicolor icons | `resources/linux/icons/hicolor/{size}x{size}/apps/com.hengvvang.splitype.png` |

## Regenerating

Edit `logo.svg`, then regenerate everything from
[`scripts/icon-gen`](../../scripts/icon-gen):

```bash
cd scripts/icon-gen
cargo run --release
```

The generator renders the logo onto white canvases: square icons fit it to
88% of the canvas, the banner to 84% of its height, everything centered. It
writes `logo*.png` and `banner.png` into this directory and distributes
the platform formats into `resources/`.

## Runtime key

`identity/logo.png` is exposed through the GPUI asset source
(`src/app/assets.rs`) for the About dialog; `identity/logo.svg` is mapped
the same way for future use. These keys mirror the file paths under
`assets/`.
