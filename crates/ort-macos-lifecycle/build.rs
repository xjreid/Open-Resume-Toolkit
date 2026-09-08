fn main() {
    println!("cargo:rerun-if-changed=native/termination.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("native/termination.m")
            .flag("-fobjc-arc")
            .warnings_into_errors(true)
            .compile("ort_termination");
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}
