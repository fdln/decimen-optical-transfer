//! Desktop shell for Decimen Optical Transfer.
//!
//! The pages are the web app, unmodified: protocol, codec and UI all run in
//! the webview exactly as they do in a browser. This file only supplies the
//! three things a bare webview does not do on its own:
//!
//! - links that leave the app open in the system browser instead of
//!   navigating the one window away from it;
//! - `<a download>` — the Save button on a received file and the sender's
//!   animation export — lands in the Downloads folder and is revealed there;
//! - nothing else. The pages make no IPC calls, so the capability file grants
//!   none.

use std::{path::PathBuf, sync::Mutex};

use tauri::{
    webview::{DownloadEvent, NewWindowResponse},
    Manager, Url, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

/// True for the app's own pages (and the blob:/data: URLs they mint), false
/// for anything that belongs in a browser.
///
/// The bundled pages are served from `tauri://localhost` (macOS, Linux) or
/// `http://tauri.localhost` (Windows); `tauri dev` loads them from the Vite
/// server on `http://localhost`.
fn is_app_url(url: &Url) -> bool {
    match url.scheme() {
        "tauri" | "blob" | "data" | "about" => true,
        "http" | "https" => matches!(url.host_str(), Some("localhost" | "tauri.localhost")),
        _ => false,
    }
}

fn is_browser_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https" | "mailto")
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = app
                .config()
                .app
                .windows
                .first()
                .expect("tauri.conf.json declares the main window")
                .clone();

            let nav_handle = app.handle().clone();
            let popup_handle = app.handle().clone();
            // Where the download in flight is going. macOS does not report the
            // path back on completion, so it is remembered from the request.
            let pending: Mutex<Option<PathBuf>> = Mutex::new(None);

            WebviewWindowBuilder::from_config(app, &config)?
                .on_navigation(move |url| {
                    if is_app_url(url) {
                        return true;
                    }
                    if is_browser_url(url) {
                        let _ = nav_handle.opener().open_url(url.as_str(), None::<&str>);
                    }
                    false
                })
                // target="_blank" never reaches on_navigation.
                .on_new_window(move |url, _features| {
                    if is_browser_url(&url) && !is_app_url(&url) {
                        let _ = popup_handle.opener().open_url(url.as_str(), None::<&str>);
                    }
                    NewWindowResponse::Deny
                })
                .on_download(move |webview, event| {
                    match event {
                        // The destination arrives as Downloads/<name>, already
                        // de-duplicated against existing files.
                        DownloadEvent::Requested { destination, .. } => {
                            *pending.lock().unwrap() = Some(destination.clone());
                        }
                        DownloadEvent::Finished { path, success, .. } => {
                            let saved = path.or_else(|| pending.lock().unwrap().take());
                            if let (true, Some(saved)) = (success, saved) {
                                let _ = webview.app_handle().opener().reveal_item_in_dir(saved);
                            }
                        }
                        _ => {}
                    }
                    true
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Decimen Optical Transfer");
}
