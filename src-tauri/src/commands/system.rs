use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize)]
pub struct SystemInfo {
    pub cpu: String,
    pub gpu: String,
    pub kernel: String,
    pub desktop: String,
}

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        cpu: detect_cpu(),
        gpu: detect_gpu(),
        kernel: detect_kernel(),
        desktop: detect_desktop(),
    }
}

fn detect_cpu() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1))
                .map(|value| clean_cpu_name(value.trim()))
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Strips marketing noise from raw /proc/cpuinfo model names: integrated-GPU
/// suffixes ("w/ Radeon Graphics") duplicate the Graphics card, and "(R)"/"(TM)"
/// / "CPU" just add clutter without adding information.
fn clean_cpu_name(raw: &str) -> String {
    let without_igpu = match raw.find(" w/ ") {
        Some(index) => &raw[..index],
        None => raw,
    };

    without_igpu
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace(" CPU", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn detect_gpu() -> String {
    let output = Command::new("lspci").output();

    let Ok(output) = output else {
        return "Unknown".to_string();
    };

    let text = String::from_utf8_lossy(&output.stdout);

    let controller = text
        .lines()
        .find(|line| line.contains("VGA compatible controller"))
        .or_else(|| text.lines().find(|line| line.contains("3D controller")));

    controller
        .and_then(|line| line.split(':').nth(2))
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn detect_kernel() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn detect_desktop() -> String {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "Unknown".to_string());

    let session_type = std::env::var("XDG_SESSION_TYPE").ok();

    match session_type {
        Some(session_type) if !session_type.is_empty() => {
            format!("{desktop} ({session_type})")
        }
        _ => desktop,
    }
}

const AUTOSTART_DESKTOP_ENTRY: &str = "\
[Desktop Entry]
Name=Soplos Welcome
Comment=Welcome application for Soplos Linux
Exec=soplos-welcome
Icon=org.soplos.welcome
Terminal=false
Type=Application
Categories=System;Settings;
StartupNotify=true
X-GNOME-Autostart-enabled=true
";

fn autostart_file_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("autostart")
        .join("org.soplos.welcome.desktop"))
}

#[tauri::command]
pub fn get_autostart_enabled() -> Result<bool, String> {
    Ok(autostart_file_path()?.exists())
}

#[tauri::command]
pub fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    let path = autostart_file_path()?;

    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, AUTOSTART_DESKTOP_ENTRY).map_err(|e| e.to_string())?;
    } else if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }

    Ok(())
}
