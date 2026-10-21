// Recommended tab — one Tauri command per "custom" catalog entry in
// Recommended.svelte, replacing the old run_privileged_script/run_user_script
// generic shell executor (security plan point 1). Each script is fixed here
// in Rust; the frontend only ever names a command, never sends script text.

use super::software::{failed_job, start_job, start_pkexec_job};
use crate::utils::{VERIFY_DOWNLOAD_SH, WEBAPP_INSTALL_SH};

// === Browsers ===

#[tauri::command]
pub fn install_chrome() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb" /tmp/google-chrome.deb
apt install -y /tmp/google-chrome.deb
rm -f /tmp/google-chrome.deb
echo "Google Chrome installed.""#
    ))
}

#[tauri::command]
pub fn install_brave() -> u64 {
    start_pkexec_job(
        r#"apt install -y curl
curl -fsSLo /usr/share/keyrings/brave-browser-archive-keyring.gpg https://brave-browser-apt-release.s3.brave.com/brave-browser-archive-keyring.gpg
curl -fsSLo /etc/apt/sources.list.d/brave-browser-release.sources https://brave-browser-apt-release.s3.brave.com/brave-browser.sources
apt update
apt install -y brave-browser
echo "Brave installed.""#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_brave() -> u64 {
    start_pkexec_job(
        r#"apt purge -y brave-browser
rm -f /etc/apt/sources.list.d/brave-browser-release.sources /usr/share/keyrings/brave-browser-archive-keyring.gpg
echo "Brave removed.""#
            .to_string(),
    )
}

#[tauri::command]
pub fn install_zen_browser() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
ZEN_URL=$(curl -s https://api.github.com/repos/sh4r10/zen-browser-debian/releases/latest | grep browser_download_url | grep -v zip | grep -v tar | cut -d\" -f4)
verify_and_download "$ZEN_URL" /tmp/zen-browser.deb
apt install -y /tmp/zen-browser.deb
rm -f /tmp/zen-browser.deb
echo "Zen Browser installed.""#
    ))
}

#[tauri::command]
pub fn install_helium() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
HELIUM_URL=$(curl -s https://api.github.com/repos/imputnet/helium-linux/releases/latest | grep browser_download_url | grep amd64.deb | cut -d\" -f4)
verify_and_download "$HELIUM_URL" /tmp/helium.deb
apt install -y /tmp/helium.deb
rm -f /tmp/helium.deb
echo "Helium installed.""#
    ))
}

#[tauri::command]
pub fn install_midori() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://github.com/goastian/midori-desktop/releases/download/v11.6/midori_11.6-1_amd64.deb" /tmp/midori.deb
apt install -y /tmp/midori.deb
rm -f /tmp/midori.deb
echo "Midori installed.""#
    ))
}

#[tauri::command]
pub fn install_brave_origin() -> u64 {
    start_pkexec_job("curl -fsS https://dl.brave.com/install.sh | FLAVOR=origin sh && echo 'Brave Origin installed.'".to_string())
}

// === Office ===

#[tauri::command]
pub fn install_genoffice() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://genoffice.ai/download/linux-deb" /tmp/genoffice.deb
apt install -y /tmp/genoffice.deb
rm -f /tmp/genoffice.deb
echo "GenOffice installed.""#
    ))
}

#[tauri::command]
pub fn install_jopdf() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://cdn.jopdf.com/download/jopdf/jopdf-linux-amd64_setup.deb" /tmp/jopdf.deb
apt install -y /tmp/jopdf.deb
rm -f /tmp/jopdf.deb
echo "JoPDF installed.""#
    ))
}

// === Graphics: Affinity Suite (AppImage, user-scoped) ===

// X-AppImage-Version/-UpdateUrl let AppImage Manager's own update check
// (api mode, since this is a GitHub project URL — see
// soplos-appimage-manager-3.0's commands/updates.rs) work out of the box,
// without the user having to fill them in by hand under Properties.
const AFFINITY_DESKTOP_ENTRY: &str = "[Desktop Entry]\nName=Affinity Suite\nExec=%HOME%/AppImages/Affinity-3.0.2-x86_64.AppImage\nIcon=%HOME%/AppImages/.icons/affinity.png\nType=Application\nCategories=Graphics;\nComment=Professional photo editing, design and publishing suite\nX-AppImage-Version=3.0.2\nX-AppImage-UpdateUrl=https://github.com/ryzendew/Linux-Affinity-Installer\nX-AppImage-Integrate=true\n";

#[tauri::command]
pub fn install_affinity(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/affinity.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
mkdir -p "$HOME/AppImages/.icons"
mkdir -p "$HOME/.local/share/applications"
echo "Downloading Affinity Suite..."
verify_and_download "https://github.com/ryzendew/Linux-Affinity-Installer/releases/download/3.2.0/Affinity-3.0.2-x86_64.AppImage" "$HOME/AppImages/Affinity-3.0.2-x86_64.AppImage"
chmod +x "$HOME/AppImages/Affinity-3.0.2-x86_64.AppImage"
cp "{icon}" "$HOME/AppImages/.icons/affinity.png"
printf '%s' '{AFFINITY_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g" > "$HOME/.local/share/applications/affinity.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Affinity Suite installed.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_affinity() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"echo "Removing Affinity Suite..."
rm -f "$HOME/AppImages/Affinity-3.0.2-x86_64.AppImage"
rm -f "$HOME/AppImages/.icons/affinity.png"
rm -f "$HOME/.local/share/applications/affinity.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Affinity Suite removed.""#
                .to_string(),
        ],
    )
}

// === Communications: WhatsApp (Soplos WebApp, no third-party client) ===
//
// Ported from 2.x's recent switch away from the unofficial
// io.github.mimbrero.WhatsAppDesktop Flatpak (an Electron wrapper around
// WhatsApp Web made by an unrelated third party, which understandably makes
// people uneasy about a messaging app) to pinning web.whatsapp.com as its
// own app window via WEBAPP_INSTALL_SH — same mechanism CoolerControl's
// local web UI already uses in this file, just pointed at a real URL
// instead of localhost.
const WHATSAPP_WEBAPP_ID: &str = "whatsapp";

#[tauri::command]
pub fn install_whatsapp(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/whatsapp.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{WEBAPP_INSTALL_SH}
mkdir -p "$HOME/.local/share/icons"
cp "{icon}" "$HOME/.local/share/icons/soplos-webapp-{WHATSAPP_WEBAPP_ID}.png"
echo "Creating WhatsApp WebApp entry..."
install_soplos_webapp "{WHATSAPP_WEBAPP_ID}" "https://web.whatsapp.com/" "WhatsApp" "$HOME/.local/share/icons/soplos-webapp-{WHATSAPP_WEBAPP_ID}.png" "Network"
echo "WhatsApp installed.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_whatsapp() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"echo "Removing WhatsApp..."
rm -f "$HOME/.local/share/applications/soplos-webapp-{WHATSAPP_WEBAPP_ID}.desktop"
rm -f "$HOME/.local/share/icons/soplos-webapp-{WHATSAPP_WEBAPP_ID}.png"
rm -rf "$HOME/.local/share/soplos-webapps/{WHATSAPP_WEBAPP_ID}"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "WhatsApp removed.""#
            ),
        ],
    )
}

// === Developer ===

#[tauri::command]
pub fn install_vscode() -> u64 {
    start_pkexec_job(
        r#"apt install -y wget gpg apt-transport-https
wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor --yes > /usr/share/keyrings/packages.microsoft.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/packages.microsoft.gpg] https://packages.microsoft.com/repos/code stable main" | tee /etc/apt/sources.list.d/vscode.list > /dev/null
apt update
apt install -y code
echo "Visual Studio Code installed.""#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_vscode() -> u64 {
    // Only the vscode.list entry, not the packages.microsoft.gpg keyring:
    // Microsoft's own install instructions for other products (Azure CLI,
    // PowerShell, .NET) reuse that same generic keyring filename, so
    // removing it here could break an unrelated repo the user separately set
    // up following Microsoft's docs.
    start_pkexec_job(
        r#"apt purge -y code
rm -f /etc/apt/sources.list.d/vscode.list
echo "Visual Studio Code removed.""#
            .to_string(),
    )
}

// Google no longer ships Antigravity as a .deb — it's a plain tarball now,
// extracted to /opt like VS Code/Cursor's own upstream convention, with a
// hand-written .desktop entry since the tarball doesn't include one.
// ANTIGRAVITY_OLD_REPO_CLEANUP removes the previous apt-repo-based install
// (package, repo file and keyring), plus the manual per-user install at
// ~/.local/opt/antigravity-ide (the layout Google's own site recommended
// before this repo existed) so upgrading from either leaves nothing behind
// — same cleanup runs on both install and uninstall. Runs as root via
// pkexec, so $HOME resolves to root's, not the real user's: PKEXEC_UID
// (set by pkexec itself) is used to look up the real home instead.
const ANTIGRAVITY_OLD_REPO_CLEANUP: &str = r#"dpkg -s antigravity >/dev/null 2>&1 && apt purge -y antigravity
rm -f /etc/apt/sources.list.d/antigravity.list /etc/apt/keyrings/antigravity-repo-key.gpg
if [ -n "$PKEXEC_UID" ]; then
    REAL_HOME=$(getent passwd "$PKEXEC_UID" | cut -d: -f6)
else
    REAL_HOME="$HOME"
fi
rm -rf "$REAL_HOME/.local/opt/antigravity-ide"
rm -f "$REAL_HOME/.local/share/applications/antigravity-ide.desktop"
update-desktop-database "$REAL_HOME/.local/share/applications" 2>/dev/null || true"#;

const ANTIGRAVITY_DESKTOP_ENTRY: &str = "[Desktop Entry]\nName=Antigravity\nExec=/opt/antigravity/antigravity-ide\nIcon=/usr/share/pixmaps/antigravity.png\nType=Application\nCategories=Development;\nComment=Advanced Agentic AI Coding Assistant\n";

// The version this tarball URL points to — kept as its own constant so
// get_antigravity_version() can compare an installed copy against it without
// re-parsing the download URL. Bump both together when Google ships a newer
// release.
const ANTIGRAVITY_LATEST_VERSION: &str = "2.5.5";
const ANTIGRAVITY_DOWNLOAD_URL: &str = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/2.5.5-4923483625488384/linux-x64/Antigravity%20IDE.tar.gz";

#[tauri::command]
pub fn install_antigravity() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
{ANTIGRAVITY_OLD_REPO_CLEANUP}
echo "Downloading Antigravity..."
verify_and_download "{ANTIGRAVITY_DOWNLOAD_URL}" /tmp/antigravity.tar.gz
rm -rf /opt/antigravity
mkdir -p /opt/antigravity
tar --transform 's,^Antigravity IDE,antigravity,' -xzf /tmp/antigravity.tar.gz -C /opt
rm -f /tmp/antigravity.tar.gz
cp /opt/antigravity/resources/app/resources/linux/code.png /usr/share/pixmaps/antigravity.png
printf '%s' '{ANTIGRAVITY_DESKTOP_ENTRY}' > /usr/share/applications/antigravity.desktop
update-desktop-database /usr/share/applications 2>/dev/null || true
echo "Antigravity installed.""#
    ))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AntigravityStatus {
    installed_version: Option<String>,
    latest_version: String,
    update_available: bool,
}

// Reads the installed version straight from the Electron app's own
// package.json (the same file Antigravity's own About dialog reads its
// "Confirmar"/commit version from) instead of trusting checkPath alone,
// since checkPath only proves *something* is at that path — not which
// version, which is exactly what left the Sep 23 stale-install confusion
// undetected.
#[tauri::command]
pub fn get_antigravity_status() -> AntigravityStatus {
    let installed_version = std::fs::read_to_string("/opt/antigravity/resources/app/package.json")
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
        .and_then(|json| json.get("version").and_then(|v| v.as_str()).map(str::to_string));

    let update_available = match &installed_version {
        Some(v) => v != ANTIGRAVITY_LATEST_VERSION,
        None => false,
    };

    AntigravityStatus {
        installed_version,
        latest_version: ANTIGRAVITY_LATEST_VERSION.to_string(),
        update_available,
    }
}

#[tauri::command]
pub fn uninstall_antigravity() -> u64 {
    start_pkexec_job(format!(
        r#"echo "Removing Antigravity..."
rm -rf /opt/antigravity
rm -f /usr/share/pixmaps/antigravity.png /usr/share/applications/antigravity.desktop
{ANTIGRAVITY_OLD_REPO_CLEANUP}
update-desktop-database /usr/share/applications 2>/dev/null || true
echo "Antigravity removed.""#
    ))
}

#[tauri::command]
pub fn install_sublime_text() -> u64 {
    start_pkexec_job(
        r#"apt install -y wget gpg apt-transport-https
mkdir -p /etc/apt/keyrings
wget -qO /etc/apt/keyrings/sublimehq-pub.asc https://download.sublimetext.com/sublimehq-pub.gpg
printf 'Types: deb\nURIs: https://download.sublimetext.com/\nSuites: apt/stable/\nSigned-By: /etc/apt/keyrings/sublimehq-pub.asc\n' > /etc/apt/sources.list.d/sublime-text.sources
apt update
apt install -y sublime-text
echo "Sublime Text installed.""#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_sublime_text() -> u64 {
    start_pkexec_job(
        r#"apt purge -y sublime-text
rm -f /etc/apt/sources.list.d/sublime-text.sources /etc/apt/keyrings/sublimehq-pub.asc
echo "Sublime Text removed.""#
            .to_string(),
    )
}

#[tauri::command]
pub fn install_cursor() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://api2.cursor.sh/updates/download/golden/linux-x64-deb/cursor/0.43.3" /tmp/cursor.deb
apt install -y /tmp/cursor.deb
rm -f /tmp/cursor.deb
echo "Cursor installed.""#
    ))
}

// Waydroid needs a Wayland compositor (wlr-layer-shell) to render at all.
// Tyron is still X11-only for now, so this is checked here too, not just
// greyed out in the frontend — the UI guard is the primary one (same
// approach as grub-btrfs' is_btrfs_root() in soplos-grub-editor: read real
// system state, don't just warn after the fact), this is the backstop in
// case the command is ever reached another way.
#[tauri::command]
pub fn install_waydroid() -> u64 {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default().to_lowercase();
    if session_type == "x11" {
        return failed_job("Waydroid requires a Wayland session — this system is currently running X11.".to_string());
    }

    start_pkexec_job(
        r#"curl -fsSL https://repo.waydro.id/waydroid.gpg -o /usr/share/keyrings/waydroid.gpg
echo 'deb [signed-by=/usr/share/keyrings/waydroid.gpg] https://repo.waydro.id/ trixie main' > /etc/apt/sources.list.d/waydroid.list
apt update
apt install -y waydroid
echo "Waydroid installed.""#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_waydroid() -> u64 {
    start_pkexec_job(
        r#"apt purge -y waydroid 2>/dev/null || true
rm -f /etc/apt/sources.list.d/waydroid.list /usr/share/keyrings/waydroid.gpg
echo "Waydroid removed.""#
            .to_string(),
    )
}

// === Gaming ===

#[tauri::command]
pub fn install_geforcenow() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"flatpak remote-add --user --if-not-exists GeForceNOW https://international.download.nvidia.com/GFNLinux/flatpak/geforcenow.flatpakrepo
flatpak install --user -y GeForceNOW com.nvidia.geforcenow
echo "GeForce NOW installed.""#
                .to_string(),
        ],
    )
}

#[tauri::command]
pub fn uninstall_geforcenow() -> u64 {
    start_job("bash", vec!["-c".to_string(), "flatpak uninstall --user -y com.nvidia.geforcenow && echo 'GeForce NOW removed.'".to_string()])
}

// Patchy is only published as a standalone .flatpak bundle on GitHub, not on
// Flathub. Installed for the user (like GeForce NOW above); its runtime is
// pulled from Flathub, so a user remote is added only when no Flathub remote
// exists in ANY scope — adding a second one when a system remote is already
// there is what makes flatpak hang asking which one to use.
#[tauri::command]
pub fn install_patchy() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
if ! flatpak remotes --columns=name | grep -qx flathub; then
  flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
fi
echo "Downloading Patchy..."
PATCHY_URL=$(curl -s https://api.github.com/repos/SethRobinson/Patchy/releases/latest | grep browser_download_url | grep PatchyLinux.flatpak | cut -d\" -f4)
verify_and_download "$PATCHY_URL" /tmp/PatchyLinux.flatpak
flatpak install --user -y --noninteractive --bundle /tmp/PatchyLinux.flatpak
rm -f /tmp/PatchyLinux.flatpak
echo "Patchy installed.""#
            ),
        ],
    )
}

#[tauri::command]
pub fn uninstall_patchy() -> u64 {
    start_job("bash", vec!["-c".to_string(), "flatpak uninstall --user -y com.rtsoft.patchy && echo 'Patchy removed.'".to_string()])
}

// PhotoCraft ships a Debian package directly on its GitHub releases (no
// Flathub, no PPA), so the same "ask the GitHub API for the newest release,
// grab its .deb" pattern as Concat/Glassy Music above applies here.
#[tauri::command]
pub fn install_photocraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
PHOTOCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/photocraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$PHOTOCRAFT_URL" /tmp/photocraft.deb
apt install -y /tmp/photocraft.deb
rm -f /tmp/photocraft.deb
echo "PhotoCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_photocraft() -> u64 {
    start_pkexec_job("apt remove -y photocraft && echo 'PhotoCraft removed.'".to_string())
}

// Concat and Glassy Music are only published as GitHub Release .deb files
// (no Flathub, no PPA), so the same "ask the GitHub API for the newest
// release, grab its .deb" pattern as Helium/Zen Browser above applies here.
#[tauri::command]
pub fn install_concat() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
CONCAT_URL=$(curl -s https://api.github.com/repos/jub0t/Concat/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$CONCAT_URL" /tmp/concat.deb
apt install -y /tmp/concat.deb
rm -f /tmp/concat.deb
echo "Concat installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_concat() -> u64 {
    start_pkexec_job("apt remove -y concat && echo 'Concat removed.'".to_string())
}

#[tauri::command]
pub fn install_glassy_music() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
GLASSY_URL=$(curl -s https://api.github.com/repos/NanKillBro/glassy-music-nankill/releases/latest | grep browser_download_url | grep amd64.deb | cut -d\" -f4)
verify_and_download "$GLASSY_URL" /tmp/glassy-music.deb
apt install -y /tmp/glassy-music.deb
rm -f /tmp/glassy-music.deb
echo "Glassy Music installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_glassy_music() -> u64 {
    start_pkexec_job("apt remove -y glassy-music-nankill-mod && echo 'Glassy Music removed.'".to_string())
}

// Orchard's own "stable" GitHub releases have shipped with an empty asset
// list since v4.5.0 (their desktop build pipeline currently only publishes
// working Linux packages under the 5.0.0 beta track), and /releases/latest
// on this repo can resolve to an unrelated mobile-app release with no Linux
// assets at all — so unlike Helium/Zen/Concat/Glassy Music above, there is
// no reliable "always get the newest" URL to query here. This is pinned to
// the current beta and needs a manual bump if Orchard ever publishes a
// working stable release again. App id confirmed from the bundle's own ref
// header (app/dev.sfg.orchard/x86_64), since Orchard doesn't publish it
// anywhere else.
#[tauri::command]
pub fn install_orchard() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
if ! flatpak remotes --columns=name | grep -qx flathub; then
  flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
fi
echo "Downloading Orchard..."
verify_and_download "https://github.com/SFG5453/Orchard/releases/download/v5.0.0-beta.9/orchard-packages-5.0.0-linux-x64.flatpak" /tmp/orchard.flatpak
flatpak install --user -y --noninteractive --bundle /tmp/orchard.flatpak
rm -f /tmp/orchard.flatpak
echo "Orchard installed.""#
            ),
        ],
    )
}

#[tauri::command]
pub fn uninstall_orchard() -> u64 {
    start_job("bash", vec!["-c".to_string(), "flatpak uninstall --user -y dev.sfg.orchard && echo 'Orchard removed.'".to_string()])
}

// Sonora publishes its own single-app Flatpak repo instead of using
// Flathub — flatpak can install straight from a .flatpakref URL, which
// adds that repo (named "sonora") and installs the app in one step.
#[tauri::command]
pub fn install_sonora() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            "flatpak install --user -y --noninteractive https://sonorahq.github.io/sonora/sonora.flatpakref && echo 'Sonora installed.'".to_string(),
        ],
    )
}

#[tauri::command]
pub fn uninstall_sonora() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            "flatpak uninstall --user -y io.github.nolight132.sonora; flatpak remote-delete --user sonora 2>/dev/null || true; echo 'Sonora removed.'".to_string(),
        ],
    )
}

// %VERSION% is filled in at install time (see ESDE_VERSION below) — the
// download itself is resolved from GitLab's own "latest release" permalink
// rather than a pinned package_files ID, after finding the hard way that a
// pinned ID silently goes stale: the one this used to hardcode pointed at
// 3.4.0 while upstream was already on 3.5.0.
const ESDE_DESKTOP_ENTRY: &str = "[Desktop Entry]\nName=ES-DE\nExec=%HOME%/AppImages/ES-DE_x64.AppImage\nIcon=%HOME%/AppImages/.icons/es-de.png\nType=Application\nCategories=Game;\nComment=Frontend for emulators with a modern interface\nX-AppImage-Version=%VERSION%\nX-AppImage-UpdateUrl=https://gitlab.com/es-de/emulationstation-de\nX-AppImage-Integrate=true\n";

#[tauri::command]
pub fn install_esde(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/es-de.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
mkdir -p "$HOME/AppImages/.icons"
mkdir -p "$HOME/.local/share/applications"
echo "Downloading ES-DE..."
ESDE_RELEASE=$(curl -sL "https://gitlab.com/api/v4/projects/es-de%2Femulationstation-de/releases/permalink/latest")
ESDE_URL=$(echo "$ESDE_RELEASE" | grep -oE '"name":"ES-DE_x64\.AppImage","url":"[^"]*"' | head -1 | cut -d'"' -f8)
ESDE_VERSION=$(echo "$ESDE_RELEASE" | grep -oE '"tag_name":"[^"]*"' | head -1 | cut -d'"' -f4 | sed 's/^v//')
verify_and_download "$ESDE_URL" "$HOME/AppImages/ES-DE_x64.AppImage"
chmod +x "$HOME/AppImages/ES-DE_x64.AppImage"
cp "{icon}" "$HOME/AppImages/.icons/es-de.png"
printf '%s' '{ESDE_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g; s|%VERSION%|$ESDE_VERSION|g" > "$HOME/.local/share/applications/es-de.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "ES-DE installed.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_esde() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"echo "Removing ES-DE..."
rm -f "$HOME/AppImages/ES-DE_x64.AppImage"
rm -f "$HOME/AppImages/.icons/es-de.png"
rm -f "$HOME/.local/share/applications/es-de.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "ES-DE removed.""#
                .to_string(),
        ],
    )
}

// Xodus Nexus — an independent, unofficial Xbox / Microsoft Store game
// launcher (Game Pass library) built on Xodus. The release publishes the
// AppImage next to a "<name>.AppImage.sha512sum" file, which is checked here
// because verify_and_download() only knows the ".sha256" convention. The file
// is installed without its version in the name so an update replaces it.
// The upstream desktop entry runs "AppRun --no-sandbox", so the flag is kept.
const XODUS_DESKTOP_ENTRY: &str = "[Desktop Entry]\nName=Xodus Nexus\nExec=%HOME%/AppImages/Xodus-Nexus.AppImage --no-sandbox %U\nIcon=%HOME%/AppImages/.icons/xodus-nexus.png\nType=Application\nCategories=Game;\nComment=Xbox and Microsoft Store game launcher\nStartupWMClass=Xodus Nexus\nTerminal=false\nX-AppImage-Version=%VERSION%\nX-AppImage-UpdateUrl=https://github.com/domsvoboda/xodus-nexus\nX-AppImage-Integrate=true\n";

#[tauri::command]
pub fn install_xodus_nexus(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/xodus-nexus.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"set -e
mkdir -p "$HOME/AppImages/.icons"
mkdir -p "$HOME/.local/share/applications"
echo "Downloading Xodus Nexus..."
XODUS_RELEASE=$(curl -sL https://api.github.com/repos/domsvoboda/xodus-nexus/releases/latest)
XODUS_URL=$(echo "$XODUS_RELEASE" | grep browser_download_url | grep 'x86_64.AppImage"' | head -1 | cut -d'"' -f4)
XODUS_VERSION=$(echo "$XODUS_RELEASE" | grep '"tag_name"' | head -1 | cut -d'"' -f4 | sed 's/^v//')
if [ -z "$XODUS_URL" ]; then
    echo "Could not find the Xodus Nexus AppImage in the latest release."
    exit 1
fi
XODUS_DEST="$HOME/AppImages/Xodus-Nexus.AppImage"
wget -q -O "$XODUS_DEST.part" "$XODUS_URL"
wget -q -O "$XODUS_DEST.sha512sum" "$XODUS_URL.sha512sum" 2>/dev/null || true
if [ -s "$XODUS_DEST.sha512sum" ] && [ "$(wc -c < "$XODUS_DEST.sha512sum")" -le 1024 ] && grep -qE "^[0-9a-fA-F]{{128}}" "$XODUS_DEST.sha512sum"; then
    if ! (cd "$HOME/AppImages" && echo "$(awk '{{print $1}}' "$XODUS_DEST.sha512sum")  $(basename "$XODUS_DEST.part")" | sha512sum -c - >/dev/null 2>&1); then
        echo "Checksum verification failed for Xodus Nexus — aborting."
        rm -f "$XODUS_DEST.part" "$XODUS_DEST.sha512sum"
        exit 1
    fi
fi
rm -f "$XODUS_DEST.sha512sum"
mv "$XODUS_DEST.part" "$XODUS_DEST"
chmod +x "$XODUS_DEST"
cp "{icon}" "$HOME/AppImages/.icons/xodus-nexus.png"
printf '%s' '{XODUS_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g; s|%VERSION%|$XODUS_VERSION|g" > "$HOME/.local/share/applications/xodus-nexus.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Xodus Nexus installed.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_xodus_nexus() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"echo "Removing Xodus Nexus..."
rm -f "$HOME/AppImages/Xodus-Nexus.AppImage"
rm -f "$HOME/AppImages/.icons/xodus-nexus.png"
rm -f "$HOME/.local/share/applications/xodus-nexus.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Xodus Nexus removed.""#
                .to_string(),
        ],
    )
}

// Epic Games Launcher — Epic's own Linux AppImage. The URL always serves the
// current build (it redirects to a versioned file on Epic's CDN), and Epic
// publishes no checksum, so it is downloaded over HTTPS without one. The
// desktop entry carries the com.epicgames.launcher URL scheme the launcher
// registers itself for, so the browser login can hand control back to it.
const EPIC_DESKTOP_ENTRY: &str = "[Desktop Entry]\nName=Epic Games Launcher\nExec=%HOME%/AppImages/EpicGamesLauncher-x86_64.AppImage %u\nIcon=%HOME%/AppImages/.icons/epic-games-launcher.png\nType=Application\nCategories=Game;\nComment=Epic Games Launcher\nStartupWMClass=EpicGamesLauncher\nMimeType=x-scheme-handler/com.epicgames.launcher;\nTerminal=false\n";

#[tauri::command]
pub fn install_epic_games_launcher(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/epic-games-launcher.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
mkdir -p "$HOME/AppImages/.icons"
mkdir -p "$HOME/.local/share/applications"
echo "Downloading Epic Games Launcher..."
EPIC_DEST="$HOME/AppImages/EpicGamesLauncher-x86_64.AppImage"
verify_and_download "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/installer/download/EpicGamesLauncher-x86_64.AppImage" "$EPIC_DEST"
chmod +x "$EPIC_DEST"
cp "{icon}" "$HOME/AppImages/.icons/epic-games-launcher.png"
printf '%s' '{EPIC_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g" > "$HOME/.local/share/applications/epic-games-launcher.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
xdg-mime default epic-games-launcher.desktop x-scheme-handler/com.epicgames.launcher 2>/dev/null || true
echo "Epic Games Launcher installed.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_epic_games_launcher() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"echo "Removing Epic Games Launcher..."
rm -f "$HOME/AppImages/EpicGamesLauncher-x86_64.AppImage"
rm -f "$HOME/AppImages/.icons/epic-games-launcher.png"
rm -f "$HOME/.local/share/applications/epic-games-launcher.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Epic Games Launcher removed.""#
                .to_string(),
        ],
    )
}

#[tauri::command]
pub fn link_ppsspp_lutris_runner() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"LUTRIS_RUNNER_DIR="$HOME/.var/app/net.lutris.Lutris/data/lutris/runners/ppsspp"
LUTRIS_BINARY="$LUTRIS_RUNNER_DIR/PPSSPPSDL"
if flatpak info net.lutris.Lutris &>/dev/null; then
  PPSSPP_BIN=$(find "$HOME/.local/share/flatpak/app/org.ppsspp.PPSSPP" -name "PPSSPPSDL" -type f 2>/dev/null | head -1)
  if [ -n "$PPSSPP_BIN" ]; then
    mkdir -p "$LUTRIS_RUNNER_DIR"
    if [ -f "$LUTRIS_BINARY" ] && [ ! -L "$LUTRIS_BINARY" ]; then
      mv "$LUTRIS_BINARY" "$LUTRIS_BINARY.bak"
    fi
    [ -L "$LUTRIS_BINARY" ] || ln -s "$PPSSPP_BIN" "$LUTRIS_BINARY"
  fi
fi"#
                .to_string(),
        ],
    )
}

// === Hardware ===

#[tauri::command]
pub fn install_cpupower_gui() -> u64 {
    start_pkexec_job("apt install -y cpupower-gui linux-cpupower && echo 'CPU Power installed.'".to_string())
}

#[tauri::command]
pub fn uninstall_cpupower_gui() -> u64 {
    start_pkexec_job("apt remove -y cpupower-gui linux-cpupower && echo 'CPU Power removed.'".to_string())
}

#[tauri::command]
pub fn install_amdgpu_top() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
AMDGPU_TOP_URL=$(curl -s https://api.github.com/repos/Umio-Yasuno/amdgpu_top/releases/latest | grep browser_download_url | grep amd64.deb | grep -v without_gui | cut -d\" -f4)
verify_and_download "$AMDGPU_TOP_URL" /tmp/amdgpu-top.deb
apt install -y /tmp/amdgpu-top.deb
rm -f /tmp/amdgpu-top.deb
echo "amdgpu_top installed.""#
    ))
}

// CoolerControl is two things, ported verbatim from 2.x's
// services/coolercontrol-install.sh: the AppImage daemon itself (needs root
// to control fans, hence Exec=pkexec) and a companion "WebApp" .desktop
// entry that opens its local web UI (http://localhost:11987/) in whichever
// browser is available, in app/kiosk mode with its own taskbar identity.
// X-AppImage-Version/-UpdateUrl (GitLab project URL, api mode) let
// AppImage Manager's own update check work without manual setup.
const COOLERCONTROL_DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=CoolerControl\nExec=pkexec %HOME%/AppImages/CoolerControlD-x86_64.AppImage\nTryExec=%HOME%/AppImages/CoolerControlD-x86_64.AppImage\nIcon=%HOME%/AppImages/.icons/coolercontrol.png\nCategories=System;\nComment=Fan and cooling control daemon\nX-AppImage-Version=4.1.0\nX-AppImage-UpdateUrl=https://gitlab.com/coolercontrol/coolercontrol\nX-AppImage-RunAsRoot=true\nX-AppImage-Integrate=true\n";

const COOLERCONTROL_WEBAPP_ID: &str = "coolercontrol-ui-111987";

#[tauri::command]
pub fn install_coolercontrol(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/coolercontrol.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
WEBAPP_ID="{COOLERCONTROL_WEBAPP_ID}"
WEBAPP_PROFILE="$HOME/.local/share/soplos-webapps/$WEBAPP_ID"
mkdir -p "$HOME/AppImages/.icons"
mkdir -p "$HOME/.local/share/applications"
mkdir -p "$WEBAPP_PROFILE"

echo "Downloading CoolerControl..."
verify_and_download "https://gitlab.com/coolercontrol/coolercontrol/-/releases/4.1.0/downloads/packages/CoolerControlD-x86_64.AppImage" "$HOME/AppImages/CoolerControlD-x86_64.AppImage"
chmod +x "$HOME/AppImages/CoolerControlD-x86_64.AppImage"
cp "{icon}" "$HOME/AppImages/.icons/coolercontrol.png"

echo "Creating AppImage entry..."
printf '%s' '{COOLERCONTROL_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g" > "$HOME/.local/share/applications/soplos-appimage-coolercontrold.desktop"

echo "Detecting a browser for the CoolerControl web UI..."
BROWSER_EXEC_LINE=""
STARTUP_CLASS="soplos-webapp-$WEBAPP_ID"
for b in chromium chromium-browser brave-browser google-chrome vivaldi; do
    if command -v "$b" >/dev/null 2>&1; then
        BROWSER_EXEC_LINE="env CHROME_DESKTOP=soplos-webapp-$WEBAPP_ID.desktop $b --ozone-platform-hint=auto --wayland-app-id=soplos-webapp-$WEBAPP_ID --class=soplos-webapp-$WEBAPP_ID --name=soplos-webapp-$WEBAPP_ID --wm-class=soplos-webapp-$WEBAPP_ID --user-data-dir=$WEBAPP_PROFILE --app=http://localhost:11987/"
        STARTUP_CLASS="chrome-localhost__-Default"
        break
    fi
done
if [ -z "$BROWSER_EXEC_LINE" ] && flatpak info org.chromium.Chromium >/dev/null 2>&1; then
    BROWSER_EXEC_LINE="env CHROME_DESKTOP=soplos-webapp-$WEBAPP_ID.desktop flatpak run org.chromium.Chromium --ozone-platform-hint=auto --wayland-app-id=soplos-webapp-$WEBAPP_ID --class=soplos-webapp-$WEBAPP_ID --name=soplos-webapp-$WEBAPP_ID --wm-class=soplos-webapp-$WEBAPP_ID --user-data-dir=$WEBAPP_PROFILE --app=http://localhost:11987/"
    STARTUP_CLASS="chrome-localhost__-Default"
elif [ -z "$BROWSER_EXEC_LINE" ] && command -v firefox >/dev/null 2>&1; then
    BROWSER_EXEC_LINE="env MOZ_APP_REMOTINGNAME=soplos-webapp-$WEBAPP_ID firefox -no-remote -new-instance -profile $WEBAPP_PROFILE -class soplos-webapp-$WEBAPP_ID -name soplos-webapp-$WEBAPP_ID http://localhost:11987/"
elif [ -z "$BROWSER_EXEC_LINE" ] && flatpak info org.mozilla.firefox >/dev/null 2>&1; then
    BROWSER_EXEC_LINE="env MOZ_APP_REMOTINGNAME=soplos-webapp-$WEBAPP_ID flatpak run org.mozilla.firefox -no-remote -new-instance -profile $WEBAPP_PROFILE -class soplos-webapp-$WEBAPP_ID -name soplos-webapp-$WEBAPP_ID http://localhost:11987/"
elif [ -z "$BROWSER_EXEC_LINE" ]; then
    BROWSER_EXEC_LINE="xdg-open http://localhost:11987/"
fi

echo "Creating WebApp entry..."
cat > "$HOME/.local/share/applications/soplos-webapp-$WEBAPP_ID.desktop" << DESKTOP
[Desktop Entry]
Version=1.0
Name=CoolerControl UI
Comment=Soplos WebApp for CoolerControl UI
Exec=$BROWSER_EXEC_LINE
Terminal=false
X-MultipleArgs=false
Type=Application
Icon=$HOME/AppImages/.icons/coolercontrol.png
Categories=System;
StartupWMClass=$STARTUP_CLASS
StartupNotify=true
X-Soplos-Navbar=false
X-Soplos-ExtraParams=
X-Soplos-Incognito=false
DESKTOP

update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "CoolerControl installed. AppImage and WebApp entries created.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_coolercontrol() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"echo "Removing CoolerControl..."
WEBAPP_ID="{COOLERCONTROL_WEBAPP_ID}"
rm -f "$HOME/AppImages/CoolerControlD-x86_64.AppImage"
rm -f "$HOME/AppImages/.icons/coolercontrol.png"
rm -f "$HOME/.local/share/applications/soplos-appimage-coolercontrold.desktop"
rm -f "$HOME/.local/share/applications/soplos-webapp-$WEBAPP_ID.desktop"
rm -rf "$HOME/.local/share/soplos-webapps/$WEBAPP_ID"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "CoolerControl removed.""#
            ),
        ],
    )
}

// === Files ===

// No X-AppImage-Version here on purpose: this OBS build is a rolling
// "latest" with no version number anywhere, upstream or in the filename —
// setting one would just be made up. X-AppImage-UpdateUrl points at the
// same direct .appimage URL used to install it, which is exactly what
// makes AppImage Manager pick "direct" mode (HEAD + Last-Modified
// comparison) instead of the version-based "api" mode.
const DOUBLECMD_DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=Double Commander\nExec=%HOME%/AppImages/doublecmd-gtk-latest-x86_64.AppImage\nIcon=%HOME%/AppImages/.icons/doublecmd.png\nCategories=FileManager;\nComment=Twin-panel file manager with advanced features\nX-AppImage-UpdateUrl=https://download.opensuse.org/repositories/home:/Alexx2000/AppImage/doublecmd-gtk-latest-x86_64.AppImage\nX-AppImage-Integrate=true\n";

#[tauri::command]
pub fn install_doublecmd(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/doublecmd.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
mkdir -p "$HOME/AppImages/.icons"
mkdir -p "$HOME/.local/share/applications"
echo "Downloading Double Commander..."
verify_and_download "https://download.opensuse.org/repositories/home:/Alexx2000/AppImage/doublecmd-gtk-latest-x86_64.AppImage" "$HOME/AppImages/doublecmd-gtk-latest-x86_64.AppImage"
chmod +x "$HOME/AppImages/doublecmd-gtk-latest-x86_64.AppImage"
cp "{icon}" "$HOME/AppImages/.icons/doublecmd.png"
printf '%s' '{DOUBLECMD_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g" > "$HOME/.local/share/applications/doublecmd.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Double Commander installed.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_doublecmd() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"echo "Removing Double Commander..."
rm -f "$HOME/AppImages/doublecmd-gtk-latest-x86_64.AppImage"
rm -f "$HOME/AppImages/.icons/doublecmd.png"
rm -f "$HOME/.local/share/applications/doublecmd.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Double Commander removed.""#
                .to_string(),
        ],
    )
}

// === Virtualization ===
// VirtualBox is NOT in Soplos's own apt sources (confirmed: apt-cache show
// virtualbox returns nothing here, unlike virt-manager/gnome-boxes which
// are). Oracle's own repo is used instead — verified reachable, and its
// package is version-suffixed (virtualbox-7.2), not plain "virtualbox".

#[tauri::command]
pub fn install_virtualbox() -> u64 {
    start_pkexec_job(
        r#"set -e
wget -qO- https://www.virtualbox.org/download/oracle_vbox_2016.asc | gpg --yes --output /usr/share/keyrings/oracle-virtualbox-2016.gpg --dearmor
CODENAME=$(apt-cache policy | grep o=Debian | grep -o "n=[a-z]*" | cut -d= -f2 | sort | uniq -c | sort -rn | head -1 | tr -dc "a-z") || true
case "$CODENAME" in
    bookworm|trixie|bullseye) ;;
    *) CODENAME="trixie" ;;
esac
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/oracle-virtualbox-2016.gpg] https://download.virtualbox.org/virtualbox/debian $CODENAME contrib" > /etc/apt/sources.list.d/virtualbox.list
# On Debian testing (forky), libvpx9 has already been dropped from testing's
# own repos in favor of a newer soname, but the package Oracle builds
# against Debian 13 ("trixie") still needs it — and trixie-security still
# carries it. Confirmed by hand on this exact machine: adding this one repo
# is what made a plain `apt install virtualbox-7.2` succeed with zero
# further workarounds, after everything else (the Qt6/curl/ssl libs) turned
# out to already resolve fine on their own.
if ! grep -rq "trixie-security" /etc/apt/sources.list.d/*.sources /etc/apt/sources.list.d/*.list 2>/dev/null; then
    printf 'Types: deb\nURIs: https://security.debian.org/debian-security/\nSuites: trixie-security\nComponents: main\nSigned-By: /usr/share/keyrings/debian-archive-keyring.gpg\n' > /etc/apt/sources.list.d/virtualbox-trixie-security.sources
fi
apt update
apt install -y virtualbox-7.2
echo "VirtualBox installed.""#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_virtualbox() -> u64 {
    start_pkexec_job(
        r#"apt purge -y 'virtualbox-*' 2>/dev/null || true
rm -f /etc/apt/sources.list.d/virtualbox.list /etc/apt/sources.list.d/virtualbox-trixie-security.sources /usr/share/keyrings/oracle-virtualbox-2016.gpg
echo "VirtualBox removed.""#
            .to_string(),
    )
}

// The Extension Pack's version must match the installed VirtualBox version
// exactly (VBoxManage refuses a mismatch), so it's detected at runtime
// instead of pinned here. The license hash below is the real SHA-256 of
// ExtPack-license.txt bundled inside the 7.2.18 pack (PUEL v12, 22 Jul
// 2024) — computed directly from that file, not copied from memory — and
// is what VBoxManage's --accept-license expects.
#[tauri::command]
pub fn install_vbox_extpack() -> u64 {
    start_pkexec_job(format!(
        r#"set -e
{VERIFY_DOWNLOAD_SH}
VBOX_VERSION=$(VBoxManage --version 2>/dev/null | grep -oE '^[0-9]+\.[0-9]+\.[0-9]+' || true)
if [ -z "$VBOX_VERSION" ]; then
    echo "Could not detect an installed VirtualBox version. Install VirtualBox first."
    exit 1
fi
echo "Downloading the Extension Pack for VirtualBox $VBOX_VERSION..."
EXTPACK_FILE="/tmp/Oracle_VirtualBox_Extension_Pack-$VBOX_VERSION.vbox-extpack"
verify_and_download "https://download.virtualbox.org/virtualbox/$VBOX_VERSION/Oracle_VirtualBox_Extension_Pack-$VBOX_VERSION.vbox-extpack" "$EXTPACK_FILE"
VBoxManage extpack install --replace --accept-license=eb31505e56e9b4d0fbca139104da41ac6f6b98f8e78968bdf01b1f3da3c4f9ae "$EXTPACK_FILE"
rm -f "$EXTPACK_FILE"
echo "VirtualBox Extension Pack installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_vbox_extpack() -> u64 {
    start_pkexec_job("VBoxManage extpack uninstall \"Oracle VirtualBox Extension Pack\" && echo 'VirtualBox Extension Pack removed.'".to_string())
}

// === ArtCraft suite (Storytold) — clean-room Rust rewrites of Adobe apps,
// same pattern as PhotoCraft above: resolve the newest GitHub release and
// install its Linux .deb. ===

#[tauri::command]
pub fn install_vectorcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
VECTORCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/vectorcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$VECTORCRAFT_URL" /tmp/vectorcraft.deb
apt install -y /tmp/vectorcraft.deb
rm -f /tmp/vectorcraft.deb
echo "VectorCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_vectorcraft() -> u64 {
    start_pkexec_job("apt remove -y vectorcraft && echo 'VectorCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_filmcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
FILMCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/filmcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$FILMCRAFT_URL" /tmp/filmcraft.deb
apt install -y /tmp/filmcraft.deb
rm -f /tmp/filmcraft.deb
echo "FilmCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_filmcraft() -> u64 {
    start_pkexec_job("apt remove -y filmcraft && echo 'FilmCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_effectcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
EFFECTCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/effectcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$EFFECTCRAFT_URL" /tmp/effectcraft.deb
apt install -y /tmp/effectcraft.deb
rm -f /tmp/effectcraft.deb
echo "EffectCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_effectcraft() -> u64 {
    start_pkexec_job("apt remove -y effectcraft && echo 'EffectCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_soundcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
SOUNDCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/soundcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$SOUNDCRAFT_URL" /tmp/soundcraft.deb
apt install -y /tmp/soundcraft.deb
rm -f /tmp/soundcraft.deb
echo "SoundCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_soundcraft() -> u64 {
    start_pkexec_job("apt remove -y soundcraft && echo 'SoundCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_lightcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
LIGHTCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/lightcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$LIGHTCRAFT_URL" /tmp/lightcraft.deb
apt install -y /tmp/lightcraft.deb
rm -f /tmp/lightcraft.deb
echo "LightCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_lightcraft() -> u64 {
    start_pkexec_job("apt remove -y lightcraft && echo 'LightCraft removed.'".to_string())
}

// The storytold/pdfcraft repo (an Acrobat clone) actually ships its builds
// under the product name "PrintCraft" — confirmed from the real release
// asset names (printcraft-*-linux-x86_64.deb) and the installed package's
// own control file (Package: printcraft), not "pdfcraft". The GitHub repo
// path below is still /pdfcraft because that's genuinely where the
// releases live; only the package/product name differs.
#[tauri::command]
pub fn install_printcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
PRINTCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/pdfcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$PRINTCRAFT_URL" /tmp/printcraft.deb
apt install -y /tmp/printcraft.deb
rm -f /tmp/printcraft.deb
echo "PrintCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_printcraft() -> u64 {
    start_pkexec_job("apt remove -y printcraft && echo 'PrintCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_wordcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
WORDCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/wordcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$WORDCRAFT_URL" /tmp/wordcraft.deb
apt install -y /tmp/wordcraft.deb
rm -f /tmp/wordcraft.deb
echo "WordCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_wordcraft() -> u64 {
    start_pkexec_job("apt remove -y wordcraft && echo 'WordCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_gridcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
GRIDCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/gridcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$GRIDCRAFT_URL" /tmp/gridcraft.deb
apt install -y /tmp/gridcraft.deb
rm -f /tmp/gridcraft.deb
echo "GridCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_gridcraft() -> u64 {
    start_pkexec_job("apt remove -y gridcraft && echo 'GridCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_deckcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
DECKCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/deckcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$DECKCRAFT_URL" /tmp/deckcraft.deb
apt install -y /tmp/deckcraft.deb
rm -f /tmp/deckcraft.deb
echo "DeckCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_deckcraft() -> u64 {
    start_pkexec_job("apt remove -y deckcraft && echo 'DeckCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_designcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
DESIGNCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/designcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$DESIGNCRAFT_URL" /tmp/designcraft.deb
apt install -y /tmp/designcraft.deb
rm -f /tmp/designcraft.deb
echo "DesignCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_designcraft() -> u64 {
    start_pkexec_job("apt remove -y designcraft && echo 'DesignCraft removed.'".to_string())
}

#[tauri::command]
pub fn install_figcraft() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
FIGCRAFT_URL=$(curl -s https://api.github.com/repos/storytold/figcraft/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$FIGCRAFT_URL" /tmp/figcraft.deb
apt install -y /tmp/figcraft.deb
rm -f /tmp/figcraft.deb
echo "FigCraft installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_figcraft() -> u64 {
    start_pkexec_job("apt remove -y figcraft && echo 'FigCraft removed.'".to_string())
}

// ArtCraft Launcher — Storytold's own installer/updater for the whole Craft
// suite above. Release assets are named "artcraft-launcher-<ver>-linux-
// x86_64.deb", so the same substring filter used for the other Craft apps
// still matches.
#[tauri::command]
pub fn install_artcraft_launcher() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
ARTCRAFT_LAUNCHER_URL=$(curl -s https://api.github.com/repos/storytold/craft-launcher/releases/latest | grep browser_download_url | grep linux-x86_64.deb | cut -d\" -f4)
verify_and_download "$ARTCRAFT_LAUNCHER_URL" /tmp/artcraft-launcher.deb
apt install -y /tmp/artcraft-launcher.deb
rm -f /tmp/artcraft-launcher.deb
echo "ArtCraft Launcher installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_artcraft_launcher() -> u64 {
    start_pkexec_job("apt remove -y artcraft-launcher && echo 'ArtCraft Launcher removed.'".to_string())
}

// Audacity 4 dropped its Debian package in favor of an official AppImage
// only (confirmed on audacityteam.org/download/Linux and the GitHub
// releases page — no .deb asset exists), so this follows the same
// AppImage pattern as Double Commander above instead of apt. The release
// tag changes per version (Audacity-X.Y.Z), so the asset URL is resolved
// from the GitHub API rather than hardcoded, same as Concat/Glassy Music.
// %VERSION% is filled in at install time from the release actually
// resolved (see AUDACITY_VERSION below) — there's no fixed value to hardcode
// here since install_audacity always grabs whatever is currently latest.
const AUDACITY_DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=Audacity\nExec=%HOME%/AppImages/audacity-linux-x86_64.AppImage\nIcon=%HOME%/AppImages/.icons/audacity.png\nCategories=AudioVideo;Audio;\nComment=Free, open source, cross-platform audio editor\nX-AppImage-Version=%VERSION%\nX-AppImage-UpdateUrl=https://github.com/audacity/audacity\nX-AppImage-Integrate=true\n";

#[tauri::command]
pub fn install_audacity(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("resources/appimage-icons/audacity.png", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    Ok(start_job(
        "bash",
        vec![
            "-c".to_string(),
            format!(
                r#"{VERIFY_DOWNLOAD_SH}
mkdir -p "$HOME/AppImages/.icons"
mkdir -p "$HOME/.local/share/applications"
echo "Downloading Audacity..."
AUDACITY_URL=$(curl -s https://api.github.com/repos/audacity/audacity/releases/latest | grep browser_download_url | grep linux-.*-x86_64.AppImage | cut -d\" -f4)
AUDACITY_VERSION=$(echo "$AUDACITY_URL" | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' | head -1)
verify_and_download "$AUDACITY_URL" "$HOME/AppImages/audacity-linux-x86_64.AppImage"
chmod +x "$HOME/AppImages/audacity-linux-x86_64.AppImage"
cp "{icon}" "$HOME/AppImages/.icons/audacity.png"
printf '%s' '{AUDACITY_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g; s|%VERSION%|$AUDACITY_VERSION|g" > "$HOME/.local/share/applications/audacity.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Audacity installed.""#
            ),
        ],
    ))
}

#[tauri::command]
pub fn uninstall_audacity() -> u64 {
    start_job(
        "bash",
        vec![
            "-c".to_string(),
            r#"echo "Removing Audacity..."
rm -f "$HOME/AppImages/audacity-linux-x86_64.AppImage"
rm -f "$HOME/AppImages/.icons/audacity.png"
rm -f "$HOME/.local/share/applications/audacity.desktop"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Audacity removed.""#
                .to_string(),
        ],
    )
}

