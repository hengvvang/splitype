//! Regenerates every splitype icon asset from `logo.svg`.
//!
//! Usage: `cargo run --release` from `scripts/icon-gen/`.
//!
//! The logo (459x524 viewBox, black strokes on white) is rendered onto a
//! white canvas. Square icons fit the logo to 88% of the canvas; the banner
//! fits it to 84% of its height. Everything is centered.

use std::fs;
use std::path::{Path, PathBuf};

fn root_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        if ancestor.join("Cargo.lock").exists() && ancestor.join("crates").is_dir() {
            return ancestor.to_path_buf();
        }
    }
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .unwrap_or(&manifest_dir)
        .to_path_buf()
}

/// Render `logo.svg` centered on a `w x h` white canvas.
fn render_logo(svg: &str, w: u32, h: u32, fit: f32) -> tiny_skia::Pixmap {
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default())
        .expect("parse logo.svg");
    let size = tree.size();
    let scale = fit * (w as f32 / size.width()).min(h as f32 / size.height());
    let content_w = size.width() * scale;
    let content_h = size.height() * scale;
    let dx = (w as f32 - content_w) / 2.0;
    let dy = (h as f32 - content_h) / 2.0;

    let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("allocate pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);
    let ts = tiny_skia::Transform::from_scale(scale, scale).post_translate(dx, dy);
    resvg::render(&tree, ts, &mut pixmap.as_mut());
    pixmap
}

fn write_ico(entries: &[(u32, Vec<u8>)], path: &Path) {
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for (size, rgba) in entries {
        let img = ico::IconImage::from_rgba_data(*size, *size, rgba.clone());
        dir.add_entry(ico::IconDirEntry::encode(&img).expect("encode ico entry"));
    }
    let mut buf = Vec::new();
    dir.write(&mut buf).expect("write ico");
    fs::write(path, buf).expect("save ico");
}

fn write_icns(sizes: &[(u32, Vec<u8>)], path: &Path) {
    let mut family = icns::IconFamily::new();
    for (size, rgba) in sizes {
        let mut img = icns::Image::new(icns::PixelFormat::RGBA, *size, *size);
        img.data_mut().copy_from_slice(rgba);
        family.add_icon(&img).expect("add icns entry");
    }
    let mut buf = Vec::new();
    family.write(&mut buf).expect("write icns");
    fs::write(path, buf).expect("save icns");
}

fn main() {
    let root = root_dir();
    let svg = fs::read_to_string(root.join("assets/identity/logo.svg")).expect("read logo.svg");
    let identity_dir = root.join("assets/identity");
    let windows_dir = root.join("packaging/windows");
    let macos_dir = root.join("packaging/macos");
    let linux_dir = root.join("packaging/linux/icons/hicolor");

    fs::create_dir_all(&windows_dir).expect("create packaging/windows");
    fs::create_dir_all(&macos_dir).expect("create packaging/macos");
    fs::create_dir_all(&linux_dir).expect("create packaging/linux/icons/hicolor");

    // --- Square PNGs -----------------------------------------------------
    let mut pngs: Vec<(u32, Vec<u8>)> = Vec::new();
    for size in [16u32, 32, 48, 64, 128, 256, 512, 1024] {
        let pixmap = render_logo(&svg, size, size, 1.0);
        let path = if size == 1024 {
            identity_dir.join("logo.png")
        } else {
            identity_dir.join(format!("logo-{size}.png"))
        };
        pixmap.save_png(&path).expect("save png");
        pngs.push((size, pixmap.data().to_vec()));
        println!("wrote {}", path.display());
    }

    // --- Linux hicolor icons ---------------------------------------------
    for size in [256u32, 512] {
        let pixmap = render_logo(&svg, size, size, 1.0);
        let path = linux_dir
            .join(format!("{size}x{size}"))
            .join("apps")
            .join("com.hengvvang.splitype.png");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir for linux icon");
        }
        pixmap.save_png(&path).expect("save png");
        println!("wrote {}", path.display());
    }

    // --- Banner (README hero image) ---------------------------------------
    let banner = render_logo(&svg, 1450, 357, 0.84);
    banner
        .save_png(&identity_dir.join("banner.png"))
        .expect("save png");
    println!("wrote {}", identity_dir.join("banner.png").display());

    // --- Windows .ico ------------------------------------------------------
    let ico_entries: Vec<(u32, Vec<u8>)> = pngs
        .iter()
        .filter(|(s, _)| matches!(s, 16 | 32 | 48 | 64 | 128 | 256))
        .cloned()
        .collect();
    write_ico(&ico_entries, &windows_dir.join("splitype.ico"));
    println!("wrote {}", windows_dir.join("splitype.ico").display());

    // --- macOS .icns -------------------------------------------------------
    let icns_sizes: Vec<(u32, Vec<u8>)> = pngs
        .iter()
        .filter(|(s, _)| matches!(s, 16 | 32 | 48 | 64 | 128 | 256 | 512 | 1024))
        .cloned()
        .collect();
    write_icns(&icns_sizes, &macos_dir.join("splitype.icns"));
    println!("wrote {}", macos_dir.join("splitype.icns").display());
}
