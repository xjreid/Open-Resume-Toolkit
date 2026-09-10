#[cfg(not(target_os = "macos"))]
use tauri::menu::MenuItem;
use tauri::{
    AppHandle,
    menu::{Menu, PredefinedMenuItem, Submenu},
};

pub(crate) const QUIT_ID: &str = "ort-request-quit";

pub(crate) fn editor_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    // On macOS, the native Quit command invokes `applicationShouldTerminate:`.
    // The AppKit lifecycle bridge in `lib.rs` guards that request and returns
    // the renderer's final decision through `replyToApplicationShouldTerminate:`.
    #[cfg(target_os = "macos")]
    let quit = PredefinedMenuItem::quit(app, Some("Quit Open Resume Toolkit"))?;
    #[cfg(not(target_os = "macos"))]
    let quit = MenuItem::with_id(
        app,
        QUIT_ID,
        "Quit Open Resume Toolkit",
        true,
        Some("CmdOrCtrl+Q"),
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window = Submenu::with_id_and_items(
        app,
        "__tauri_window_menu__",
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let help = Submenu::with_id_and_items(app, "__tauri_help_menu__", "Help", true, &[])?;
    #[cfg(target_os = "macos")]
    let application = Submenu::with_items(
        app,
        "Open Resume Toolkit",
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    #[cfg(not(target_os = "macos"))]
    let application = Submenu::with_items(app, "File", true, &[&quit])?;
    Menu::with_items(app, &[&application, &edit, &window, &help])
}
