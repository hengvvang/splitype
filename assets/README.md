# Assets

Everything splitype ships that is not source code, grouped by purpose.

| Directory | Purpose | Entry point |
| --- | --- | --- |
| [`icons/`](icons/README.md) | Runtime UI icons — monochrome SVGs embedded at build time and colored by the active theme | `src/app/assets.rs` |
| [`identity/`](identity/README.md) | Visual identity — the logo source file plus every derived app icon and banner | `scripts/tools/icon-gen/` |
| [`examples/`](examples/README.md) | Configuration templates users can import as custom themes and language packs | — |
| [`showcase/`](showcase/showcase.md) | Screenshots used by the README and docs | — |

## Conventions

- **Directory names are semantic**, not flat: icons are grouped by the UI
  surface they render in, identity files are named after their role
  (`logo.svg`, `logo-16.png`, `banner.png`).
- **Asset paths in code mirror disk paths.** A runtime asset key like
  `icons/explorer/folder.svg` resolves to the file with the same relative
  path under `assets/`, so renaming a file means updating `src/app/assets.rs`
  and every call site — grep for the old key.
- **Build-time platform resources live in `packaging/`**, not here
  (`packaging/windows/splitype.ico`, `packaging/macos/splitype.icns`,
  `packaging/linux/icons/hicolor/…`). `scripts/tools/icon-gen` writes them from the
  identity sources.

## Regenerating derived files

The app icons, banner, `.ico`, `.icns`, and hicolor PNGs are all generated
from `identity/logo.svg`:

```bash
cd scripts/tools/icon-gen
cargo run --release
```
