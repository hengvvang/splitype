# Scripts & Tooling

Developer utilities, quality assurance entry points, and specialized offline toolchains for **splitype**.

> [!NOTE]
> Workspace automation tasks (e.g. `cargo xtask check`, `cargo xtask test`, `cargo xtask dist`) are primarily implemented in [`xtask/`](../xtask/). The scripts here provide convenience wrappers and offline tooling.
> Platform installation and packaging scripts (such as macOS `.app` and `.pkg` builders) live in [`packaging/`](../packaging/).

---

## Directory Structure

```text
scripts/
├── dev/                     # Developer pre-flight & quality assurance shortcuts
│   ├── check.sh             # Linux/macOS quality check (wraps `cargo xtask check`)
│   └── check.ps1            # Windows PowerShell quality check (wraps `cargo xtask check`)
│
├── tools/                   # Standalone developer tools and asset generators
│   ├── icon-gen/            # Rust CLI: multi-format platform icon generator
│   └── analyze_logo.py      # Python utility: terminal ASCII renderer for PNG logos
│
└── README.md                # This directory index
```

---

## Quick Reference

### 1. Daily Development (`dev/`)

Run the standard quality gate (formatting check, compilation check, Clippy lints, and test suite):

- **Linux / macOS**:
  ```bash
  ./scripts/dev/check.sh
  ./scripts/dev/check.sh --fix      # Automatically format and fix safe clippy warnings
  ./scripts/dev/check.sh -p app     # Restrict checks to a specific package
  ```

- **Windows (PowerShell)**:
  ```powershell
  .\scripts\dev\check.ps1
  .\scripts\dev\check.ps1 --fix
  .\scripts\dev\check.ps1 -p app
  ```

*(Both scripts proxy directly to `cargo xtask check`.)*

---

### 2. Specialized Toolchains (`tools/`)

#### `icon-gen`
A standalone Rust tool that takes `scripts/tools/icon-gen/logo.svg` as its source of truth and renders/distributes all required icon sizes and formats for Windows, macOS, and Linux:

```bash
cd scripts/tools/icon-gen
cargo run --release
```

Generated outputs:
- `assets/identity/logo.svg` (synchronized SVG source for runtime use)
- `assets/identity/logo-*.png` & `banner.png`
- `packaging/windows/splitype.ico`
- `packaging/macos/splitype.icns`
- `packaging/linux/icons/hicolor/.../com.hengvvang.splitype.png`

#### `analyze_logo.py`
A zero-dependency Python script to inspect PNG logos directly within terminal environments via ASCII shading:

```bash
python scripts/tools/analyze_logo.py assets/identity/logo.png
```
