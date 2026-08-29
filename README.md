<p align="right">
  <a href="README.md">English</a> |
  <a href="docs/README.zh-CN.md">中文</a>
</p>

<h1 align="left">
  Splitype
</h1>

<p align="center">
  <img src="assets/identity/banner.png" alt="Splitype" />
</p>

<h3 align="right">
  A fast, native Markdown editor, built with Rust and GPUI.
</h3>

<p align="right">
    <a href="./assets/showcase/showcase.md">Showcase</a> | 
    <a href="https://github.com/hengvvang/splitype/discussions/1">Roadmap</a> | 
    <a href="https://github.com/hengvvang/splitype/wiki">Wiki</a>
</p>



## Features

### Split editing
> Splitype's split layout lets you build your own dedicated workspace exactly the way you like it.
>
> |       SPLIT                     |
> | -------------------------------- |
> | ![Preview](./assets/showcase/split.png) |

### Multi-modal editing
> Splitype offers two workflows: split-preview editing and live WYSIWYG editing.
>
> | Source editing                   | WYSIWYG editing                   |
> | -------------------------------- | --------------------------------- |
> | ![Preview](./assets/showcase/workflow_source-preview.png) | ![Preview](./assets/showcase/workflow_wysiwyg.png) |



## Installation

Download the latest release for your platform from the [Releases](https://github.com/hengvvang/splitype/releases) page.

### macOS

| Format | Description |
| ------ | ----------- |
| `.app` | Standalone application bundle. Drag to `/Applications` to install. |
| `.pkg` | System installer. Installs the app and registers the `splitype` CLI command in your `PATH`. |
| `.dmg` | Disk image containing the `.app` bundle. Open, drag to `/Applications`, then eject. |

### Windows

| Format | Description |
| ------ | ----------- |
| `.zip` | Portable archive. Extract to any folder and run `splitype.exe` directly — no installation required. |
| `.msi` | Windows Installer package. Double-click to install with Start Menu shortcuts and optional `PATH` registration. |

### Linux

| Format | Description |
| ------ | ----------- |
| `.tar.gz` | Portable archive. Extract and run `./splitype` directly — no installation required. |
| `.deb` | Debian/Ubuntu package. Install via `sudo dpkg -i splitype_*.deb`. |
| `.AppImage` | Single-file portable executable. `chmod +x` and run — no extraction needed. |

### Build from source

Requires [Rust](https://rustup.rs/) (Edition 2024, MSRV 1.91).

```bash
git clone https://github.com/hengvvang/splitype.git
cd splitype
cargo build --release
```

The output binary is at `./target/release/app` (the product keeps the
name Splitype; the crate/binary is `app`).

### Development

- `cargo xtask check` — format, check, clippy (`-D warnings`).
- `cargo xtask test` — run the workspace test suite.
- `cargo xtask machete` — unused-dependency audit (CI-enforced).
- `cargo xtask deny` — dependency advisories / licenses / bans
  (CI-enforced).

Architecture and design records live in
[`docs/develop/architecture.md`](docs/develop/architecture.md) and
[`docs/decisions.md`](docs/decisions.md).



## Acknowledgements

Splitype is built on the shoulders of these outstanding open-source projects:

- [velotype](https://github.com/manyougz/velotype) — the original codebase that made this project possible.
- [zed](https://github.com/zed-industries/zed) — splitype's file explorer is largely ported from zed's design.
- [blender](https://github.com/blender/blender) — the split-panel layout system is inspired by blender's workspace model.

**Assets & Resources**

- Icons by [dreamstale](https://www.flaticon.com/authors/dreamstale), licensed from [Flaticon](https://www.flaticon.com/). Redistribution without a valid license is strictly prohibited.
- [Lexend](https://fonts.google.com/specimen/Lexend) typeface by [Thomas Jockin](https://github.com/ThomasJockin), licensed under the [SIL Open Font License 1.1](http://scripts.sil.org/OFL).

## License

Splitype is licensed under the [GNU General Public License v3.0 or later](LICENSE-GPL) ([GPL-3.0-or-later](https://www.gnu.org/licenses/gpl-3.0.html)).

Certain bundled assets are covered by their own licenses:
- Icons: licensed from [Flaticon](https://www.flaticon.com/) — see [Acknowledgements](#acknowledgements).
- Lexend font: [SIL Open Font License 1.1](http://scripts.sil.org/OFL).
