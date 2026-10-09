fn main() {
    match ort_codex_install::privileged_main(std::env::args_os().skip(1)) {
        Ok(()) => println!("ORT_CODEX_INSTALLED"),
        Err(code) => {
            eprintln!("{code}");
            std::process::exit(1);
        }
    }
}
