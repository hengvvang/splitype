fn main() {
    println!("cargo:rerun-if-changed=resources/windows/splitype.rc");
    println!("cargo:rerun-if-changed=resources/windows/splitype.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/windows/splitype.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("failed to compile splitype Windows resources");

        // The GPUI rendering tree is deeply nested (Shell → tiled layout →
        // Editor → pane layout → document blocks → inline fragments) and
        // each closure / generic monomorphization consumes significant stack
        // in unoptimised debug builds. The default Windows PE stack of 1 MB
        // overflows; reserve 8 MB — the same value Zed uses.
        println!("cargo:rustc-link-arg=/STACK:{}", 16 * 1024 * 1024);
    }
}
