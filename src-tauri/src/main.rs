// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tailer;

/// Kill any older running instance of this app so a fresh launch takes over.
/// Orphaned widgets (e.g. a frozen one the user had to leave running) otherwise
/// pile up; the newest launch should win. Matches by exe name, skips our own pid.
#[cfg(unix)]
fn kill_other_instances() {
    use std::process::Command;

    let me = std::process::id();
    let name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_owned()))
        .and_then(|n| n.into_string().ok())
        .unwrap_or_else(|| "token-maxxing".into());

    let Ok(out) = Command::new("pgrep").arg("-x").arg(&name).output() else {
        return;
    };
    for pid in String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .filter(|&p| p != me)
    {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }
}

fn main() {
    #[cfg(unix)]
    kill_other_instances();

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
