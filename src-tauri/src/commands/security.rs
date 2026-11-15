// Security & system protection commands — ported from soplos-welcome 2.x's
// security_tab.py. Plain apt packages (Timeshift, Deja Dup, GUFW, BleachBit,
// Sweeper, Soplos Sys Cleaner, ClamTk, rkhunter) and Flatpak apps (Proton
// VPN, Surfshark, Mozilla VPN, VPN Unlimited, ClamUI) reuse the generic
// is_apt_package_installed / install_apt_packages / uninstall_apt_packages
// and is_flatpak_installed / install_flatpak / uninstall_flatpak commands
// from software.rs — only the tools with bespoke install methods (their own
// apt repo, a GitHub-hosted .deb/AppImage) or extra actions (UFW toggle,
// ClamAV definitions, rkhunter scan) get commands of their own here.

use std::process::Command;

use super::software::start_pkexec_job;
use crate::utils::VERIFY_DOWNLOAD_SH;

fn run_stdout(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[tauri::command]
pub fn detect_filesystem() -> String {
    let fs = run_stdout("findmnt", &["-n", "-o", "FSTYPE", "/"]);
    if fs.is_empty() {
        "unknown".to_string()
    } else {
        fs.to_lowercase()
    }
}

// === Firewall (UFW) ===

#[tauri::command]
pub fn is_ufw_active() -> bool {
    std::fs::read_to_string("/etc/ufw/ufw.conf")
        .map(|content| {
            content
                .lines()
                .find(|l| l.trim().starts_with("ENABLED="))
                .map(|l| l.to_lowercase().contains("yes"))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

#[tauri::command]
pub fn enable_ufw() -> u64 {
    start_pkexec_job("ufw --force enable && systemctl enable ufw && systemctl start ufw".to_string())
}

#[tauri::command]
pub fn disable_ufw() -> u64 {
    start_pkexec_job("ufw disable".to_string())
}

// === Launching installed apps ===
// Configure/open buttons for tools already on the system — a couple need
// root (Timeshift's GUI writes snapshots, BTRFS Assistant edits subvolumes).

#[tauri::command]
pub fn launch_app(program: String, args: Vec<String>) -> Result<(), String> {
    Command::new(&program).args(&args).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launch_privileged_app(program: String) -> Result<(), String> {
    Command::new("pkexec").arg(&program).spawn().map(|_| ()).map_err(|e| e.to_string())
}

// === Portmaster (application firewall, .deb from Safing's own update index) ===

#[tauri::command]
pub fn is_portmaster_installed() -> bool {
    super::drivers::is_package_installed("portmaster") || std::path::Path::new("/opt/safing/portmaster").exists()
}

#[tauri::command]
pub fn install_portmaster() -> u64 {
    let script = format!(
        r#"set -e
{VERIFY_DOWNLOAD_SH}
VERSION=$(curl -s https://updates.safing.io/stable.v3.json | python3 -c "import json,sys; print(json.load(sys.stdin)['Version'])")
if [ -z "$VERSION" ]; then echo "Error: could not get Portmaster version"; exit 1; fi
echo "Downloading Portmaster $VERSION..."
verify_and_download "https://updates.safing.io/latest/linux_amd64/packages/Portmaster_${{VERSION}}_amd64.deb" /tmp/portmaster-installer.deb
apt install -y /tmp/portmaster-installer.deb
rm -f /tmp/portmaster-installer.deb
echo "Installation complete."
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_portmaster() -> u64 {
    let script = r#"apt purge -y portmaster 2>/dev/null || true
rm -rf /opt/safing
rm -f /usr/share/applications/portmaster.desktop
echo "Uninstallation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

// === Cloudflare WARP (own apt repo) ===
// Cloudflare only publishes Debian codenames trixie/bookworm/bullseye/buster
// — Soplos's real base isn't published there, so the suite is hardcoded to
// trixie, same reasoning as the ROCm/XanMod codename fixes elsewhere.

#[tauri::command]
pub fn install_cloudflare_warp() -> u64 {
    let script = r#"set -e
mkdir -p /etc/apt/keyrings
curl -fsSL https://pkg.cloudflareclient.com/pubkey.gpg | gpg --yes --dearmor --output /etc/apt/keyrings/cloudflare-warp-archive-keyring.gpg
echo "deb [signed-by=/etc/apt/keyrings/cloudflare-warp-archive-keyring.gpg] https://pkg.cloudflareclient.com/ trixie main" > /etc/apt/sources.list.d/cloudflare-client.list
apt update
apt install -y cloudflare-warp
echo "Installation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_cloudflare_warp() -> u64 {
    let script = r#"apt purge -y cloudflare-warp 2>/dev/null || true
rm -f /etc/apt/sources.list.d/cloudflare-client.list /etc/apt/keyrings/cloudflare-warp-archive-keyring.gpg /usr/share/keyrings/cloudflare-warp-archive-keyring.gpg
echo "Uninstallation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

// === NordVPN (own apt repo, needs the desktop user in the nordvpn group) ===

#[tauri::command]
pub fn install_nordvpn() -> u64 {
    let user = run_stdout("whoami", &[]);
    let script = format!(
        r#"set -e
mkdir -p /etc/apt/keyrings
curl -fsSL https://repo.nordvpn.com/gpg/nordvpn_public.asc | gpg --yes --dearmor --output /etc/apt/keyrings/nordvpn-archive-keyring.gpg
echo "deb [signed-by=/etc/apt/keyrings/nordvpn-archive-keyring.gpg] https://repo.nordvpn.com/deb/nordvpn/debian stable main" > /etc/apt/sources.list.d/nordvpn-app.list
apt update
apt install -y nordvpn-gui
getent group nordvpn >/dev/null && usermod -aG nordvpn '{user}'
echo "Installation complete."
echo "Log out and log back in before opening NordVPN, or it will not be able to reach its service."
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_nordvpn() -> u64 {
    let user = run_stdout("whoami", &[]);
    let script = format!(
        r#"apt purge -y nordvpn-gui nordvpn 2>/dev/null || true
gpasswd -d '{user}' nordvpn 2>/dev/null || true
[ -z "$(getent group nordvpn | cut -d: -f4)" ] && groupdel nordvpn 2>/dev/null || true
rm -f /etc/apt/sources.list.d/nordvpn-app.list /etc/apt/keyrings/nordvpn-archive-keyring.gpg /etc/apt/trusted.gpg.d/nordvpn_public.asc
echo "Uninstallation complete."
"#
    );
    start_pkexec_job(script)
}

// === Stacer (AppImage from its GitHub releases, installed to the user's home) ===

fn stacer_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/AppImages/Stacer.AppImage")
}

#[tauri::command]
pub fn is_stacer_installed() -> bool {
    std::path::Path::new(&stacer_path()).exists()
}

#[tauri::command]
pub fn open_stacer() -> Result<(), String> {
    Command::new(stacer_path()).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_stacer() -> u64 {
    let home = std::env::var("HOME").unwrap_or_default();
    let script = format!(
        r#"set -e
{VERIFY_DOWNLOAD_SH}
mkdir -p "{home}/AppImages/.icons"
verify_and_download "https://github.com/oguzhaninan/Stacer/releases/download/v1.1.0/Stacer-1.1.0-x64.AppImage" "{home}/AppImages/Stacer.AppImage"
chmod +x "{home}/AppImages/Stacer.AppImage"
mkdir -p "{home}/.local/share/applications"
wget -q -O "{home}/AppImages/.icons/stacer.png" "https://raw.githubusercontent.com/oguzhaninan/Stacer/master/icons/hicolor/256x256/apps/stacer.png" || true
cat > "{home}/.local/share/applications/stacer.desktop" << EOF
[Desktop Entry]
Name=Stacer
Exec={home}/AppImages/Stacer.AppImage
Icon={home}/AppImages/.icons/stacer.png
Type=Application
Categories=System;
Comment=Modern system optimizer with resource monitoring
EOF
echo "Installation complete."
"#
    );
    // Lives entirely under the user's home, so this runs without pkexec —
    // matches _on_install_stacer in the 2.x tab.
    super::software::start_job("bash", vec!["-c".to_string(), script])
}

#[tauri::command]
pub fn uninstall_stacer() -> u64 {
    let home = std::env::var("HOME").unwrap_or_default();
    let script = format!(
        r#"rm -f "{home}/AppImages/Stacer.AppImage"
rm -f "{home}/AppImages/.icons/stacer.png"
rm -f "{home}/.local/share/applications/stacer.desktop"
echo "Uninstallation complete."
"#
    );
    super::software::start_job("bash", vec!["-c".to_string(), script])
}

// === Kudu (.deb from its latest GitHub release) ===

#[tauri::command]
pub fn is_kudu_installed() -> bool {
    super::drivers::is_package_installed("kudu") || std::path::Path::new("/opt/Kudu").is_dir()
}

#[tauri::command]
pub fn install_kudu() -> u64 {
    let script = format!(
        r#"set -e
{VERIFY_DOWNLOAD_SH}
DEB_URL=$(curl -s https://api.github.com/repos/AdventDevInc/kudu/releases/latest | grep browser_download_url | grep -- '-amd64.deb' | cut -d'"' -f4)
if [ -z "$DEB_URL" ]; then echo "Error: could not get Kudu download URL"; exit 1; fi
echo "Downloading Kudu from $DEB_URL..."
verify_and_download "$DEB_URL" /tmp/kudu-installer.deb
apt install -y /tmp/kudu-installer.deb
rm -f /tmp/kudu-installer.deb
echo "Installation complete."
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_kudu() -> u64 {
    let script = r#"apt purge -y kudu 2>/dev/null || true
rm -rf /opt/Kudu
rm -f /usr/share/applications/kudu.desktop
echo "Uninstallation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

// === ClamTk extras: virus definitions update ===

#[tauri::command]
pub fn update_clamav_definitions() -> u64 {
    let script = r#"echo "Updating virus definitions..."
systemctl stop clamav-freshclam 2>/dev/null || true
stdbuf -oL -eL freshclam 2>&1
systemctl start clamav-freshclam 2>/dev/null || true
echo "Definitions updated successfully."
"#
    .to_string();
    start_pkexec_job(script)
}

// === rkhunter: on-demand rootkit scan ===

#[tauri::command]
pub fn scan_rkhunter() -> u64 {
    let script = r#"echo "Scanning system for rootkits..."
stdbuf -oL -eL rkhunter --check --skip-keypress --nocolors 2>&1 | grep --line-buffered -v '^$'
echo "Scan complete."
"#
    .to_string();
    start_pkexec_job(script)
}
