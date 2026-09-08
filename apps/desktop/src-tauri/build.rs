fn main() {
    println!("cargo:rerun-if-env-changed=ORT_PARSER_HELPER_SHA256");
    println!("cargo:rerun-if-env-changed=ORT_PARSER_HELPER_CDHASH");
    tauri_build::build();
}
