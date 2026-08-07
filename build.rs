fn main() {
    println!("cargo:rerun-if-changed=resources/windows/splitype.rc");
    println!("cargo:rerun-if-changed=resources/windows/splitype.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/windows/splitype.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("failed to compile splitype Windows resources");
    }
}
