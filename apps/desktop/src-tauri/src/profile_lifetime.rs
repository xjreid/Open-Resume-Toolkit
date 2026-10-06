//! One retirement boundary for every profile-scoped native cache and webview.
use tauri::{Emitter, Manager};
pub(crate) fn retire(app: &tauri::AppHandle) {
    let _ = app.state::<crate::pdf_preview::PdfState>().clear();
    let _ = app.state::<crate::pdf_preview::PortablePdfState>().clear();
    app.state::<crate::application_exports::ApplicationExportState>()
        .clear();
    app.state::<crate::application_exports::DragFiles>().clear();
    app.state::<crate::browser_bridge::BrowserBridgeState>()
        .disconnect();
    if let Some(popup) = app.get_webview_window("application-popup") {
        let _ = popup.hide();
    }
    let _ = app.emit("ort:profile-replaced", ());
}
