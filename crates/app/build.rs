fn main() {
    let rc_path = if std::path::Path::new("resources/windows/splitype.rc").exists() {
        "resources/windows/splitype.rc"
    } else {
        "../../resources/windows/splitype.rc"
    };

    println!("cargo:rerun-if-changed={rc_path}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if std::path::Path::new(rc_path).exists() {
            embed_resource::compile(rc_path, embed_resource::NONE)
                .manifest_optional()
                .expect("failed to compile splitype Windows resources");
        }

        // The GPUI rendering tree is deeply nested (Shell → tiled layout →
        // Editor → pane layout → document blocks → inline fragments) and
        // each closure / generic monomorphization consumes significant stack
        // in unoptimised debug builds. The default Windows PE stack of 1 MB
        // overflows; reserve 16 MB — the same value Zed uses.
        println!("cargo:rustc-link-arg=/STACK:{}", 16 * 1024 * 1024);
    }
}
