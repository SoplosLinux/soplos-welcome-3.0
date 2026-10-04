// Desktop environment / display protocol detection for the status bar,
// ported from 2.x's core/environment.py (theme detection dropped — this
// app tracks its own light/dark theme in the frontend now, same as
// soplos-repo-selector-3.0, soplos-webapp-manager-3.0 and
// soplos-appimage-manager-3.0).

use serde::Serialize;

#[derive(Serialize)]
pub struct EnvironmentInfo {
    #[serde(rename = "desktopEnvironment")]
    pub desktop_environment: String,
    #[serde(rename = "displayProtocol")]
    pub display_protocol: String,
}

#[tauri::command]
pub fn get_environment_info() -> EnvironmentInfo {
    let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    let desktop_environment = if current_desktop.contains("gnome") {
        "gnome"
    } else if current_desktop.contains("kde") || current_desktop.contains("plasma") {
        "kde"
    } else if current_desktop.contains("xfce") {
        "xfce"
    } else if std::env::var("GNOME_DESKTOP_SESSION_ID").is_ok() {
        "gnome"
    } else if std::env::var("KDE_SESSION_VERSION").is_ok() {
        "kde"
    } else {
        "unknown"
    }
    .to_string();

    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default().to_lowercase();
    let display_protocol = if session_type == "wayland" {
        "wayland"
    } else if session_type == "x11" || std::env::var("DISPLAY").is_ok() {
        "x11"
    } else {
        "unknown"
    }
    .to_string();

    EnvironmentInfo { desktop_environment, display_protocol }
}
