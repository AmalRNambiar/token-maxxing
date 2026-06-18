// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tailer;

fn main() {
    // GNOME/Wayland (Mutter) ignores always-on-top for native Wayland clients.
    // Force the GTK/WebKit window onto XWayland, where Mutter honors
    // _NET_WM_STATE_ABOVE, so the widget truly floats above every window.
    #[cfg(target_os = "linux")]
    std::env::set_var("GDK_BACKEND", "x11");

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            // Background thread: poll client logs and emit usage updates to the UI.
            std::thread::spawn(move || {
                tailer::run(handle);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running token-maxxing");
}
