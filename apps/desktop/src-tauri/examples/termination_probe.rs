//! Synthetic Tauri/AppKit quit probe. No database, vault, document or app commands.
#[cfg(target_os = "macos")]
fn main() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tauri::menu::{Menu, PredefinedMenuItem, Submenu};
    let automatic = std::env::args().any(|arg| arg == "--automatic-exit");
    let requests = Arc::new(AtomicUsize::new(0));
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "com.openresumetoolkit.synthetic-termination-probe".into();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .menu(|app| {
            let menu = Submenu::with_items(app, "ORT Synthetic Quit Probe", true,
                &[&PredefinedMenuItem::quit(app, Some("Quit Synthetic Probe"))?])?;
            Menu::with_items(app, &[&menu])
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let count = Arc::clone(&requests);
            assert!(ort_macos_lifecycle::install(move || {
                let number = count.fetch_add(1, Ordering::SeqCst) + 1;
                eprintln!("native termination request {number}");
                let main = handle.clone();
                // Simulate a later renderer decision arriving from a worker
                // thread, then returning through Tauri's main-thread executor.
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    main.run_on_main_thread(move || {
                        eprintln!("Tauri main-thread reply {number}");
                        assert!(ort_macos_lifecycle::reply(number >= 2));
                    }).expect("main-thread scheduling");
                });
                true
            }));
            let window = tauri::WebviewWindowBuilder::new(app, "probe", tauri::WebviewUrl::External("about:blank".parse().expect("fixed blank URL")))
                .title("ORT Synthetic Quit Probe — no profile access")
                .initialization_script("document.addEventListener('DOMContentLoaded', () => { document.body.textContent = 'Synthetic native quit probe. No profile is open. Use Quit Synthetic Probe twice: first cancels, second exits.'; });")
                .build()?;
            window.set_focus()?;
            tauri::WebviewWindowBuilder::new(app, "overlay", tauri::WebviewUrl::External("about:blank".parse().unwrap()))
                .title("ORT Synthetic Overlay — no profile access").build()?;
            if automatic {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let main = handle.clone();
                    handle.run_on_main_thread(move || {
                        eprintln!("AUTOMATIC: approved app.exit with two windows");
                        main.exit(0);
                    }).unwrap();
                });
            }
            eprintln!("READY: use native Quit twice; first cancels and second approves.");
            Ok(())
        })
        .build(context).expect("synthetic app")
        .run(|_, event| {
            if matches!(event, tauri::RunEvent::Exit) { eprintln!("EXIT observed"); }
        });
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This synthetic probe requires macOS.");
}
