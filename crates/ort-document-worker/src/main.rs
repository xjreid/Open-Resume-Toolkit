//! Hostile-document worker placeholder. It must remain inert until sandbox proof passes.

fn main() {
    eprintln!("Document import is disabled: the platform sandbox gate has not passed.");
    std::process::exit(78);
}
