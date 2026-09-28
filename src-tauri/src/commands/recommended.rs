// Recommended tab — one Tauri command per "custom" catalog entry in
// Recommended.svelte, replacing the old run_privileged_script/run_user_script
// generic shell executor (security plan point 1). Each script is fixed here
// in Rust; the frontend only ever names a command, never sends script text.

use super::software::{start_job, start_pkexec_job};
use crate::utils::{VERIFY_DOWNLOAD_SH, WEBAPP_INSTALL_SH};

// === Browsers ===

#[tauri::command]
pub fn install_chrome() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb" /tmp/google-chrome.deb
apt install -y /tmp/google-chrome.deb
rm -f /tmp/google-chrome.deb"#
    ))
}

#[tauri::command]
pub fn install_brave() -> u64 {
    start_pkexec_job(
        r#"apt install -y curl
curl -fsSLo /usr/share/keyrings/brave-browser-archive-keyring.gpg https://brave-browser-apt-release.s3.brave.com/brave-browser-archive-keyring.gpg
curl -fsSLo /etc/apt/sources.list.d/brave-browser-release.sources https://brave-browser-apt-release.s3.brave.com/brave-browser.sources
apt update
apt install -y brave-browser"#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_brave() -> u64 {
    start_pkexec_job(
        r#"apt purge -y brave-browser
rm -f /etc/apt/sources.list.d/brave-browser-release.sources /usr/share/keyrings/brave-browser-archive-keyring.gpg"#
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
rm -f /tmp/zen-browser.deb"#
    ))
}

#[tauri::command]
pub fn install_helium() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
HELIUM_URL=$(curl -s https://api.github.com/repos/imputnet/helium-linux/releases/latest | grep browser_download_url | grep amd64.deb | cut -d\" -f4)
verify_and_download "$HELIUM_URL" /tmp/helium.deb
apt install -y /tmp/helium.deb
rm -f /tmp/helium.deb"#
    ))
}

#[tauri::command]
pub fn install_midori() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://github.com/goastian/midori-desktop/releases/download/v11.6/midori_11.6-1_amd64.deb" /tmp/midori.deb
apt install -y /tmp/midori.deb
rm -f /tmp/midori.deb"#
    ))
}

#[tauri::command]
pub fn install_brave_origin() -> u64 {
    start_pkexec_job("curl -fsS https://dl.brave.com/install.sh | FLAVOR=origin sh".to_string())
}

// === Office ===

#[tauri::command]
pub fn install_genoffice() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://genoffice.ai/download/linux-deb" /tmp/genoffice.deb
apt install -y /tmp/genoffice.deb
rm -f /tmp/genoffice.deb"#
    ))
}

#[tauri::command]
pub fn install_jopdf() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://cdn.jopdf.com/download/jopdf/jopdf-linux-amd64_setup.deb" /tmp/jopdf.deb
apt install -y /tmp/jopdf.deb
rm -f /tmp/jopdf.deb"#
    ))
}

// === Graphics: Affinity Suite (AppImage, user-scoped) ===

const AFFINITY_DESKTOP_ENTRY: &str = "[Desktop Entry]\nName=Affinity Suite\nExec=%HOME%/AppImages/Affinity-3.0.2-x86_64.AppImage\nIcon=%HOME%/AppImages/.icons/affinity.png\nType=Application\nCategories=Graphics;\nComment=Professional photo editing, design and publishing suite\nX-AppImage-Integrate=true\n";

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
apt install -y code"#
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
rm -f /etc/apt/sources.list.d/vscode.list"#
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
apt install -y sublime-text"#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_sublime_text() -> u64 {
    start_pkexec_job(
        r#"apt purge -y sublime-text
rm -f /etc/apt/sources.list.d/sublime-text.sources /etc/apt/keyrings/sublimehq-pub.asc"#
            .to_string(),
    )
}

#[tauri::command]
pub fn install_cursor() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
verify_and_download "https://api2.cursor.sh/updates/download/golden/linux-x64-deb/cursor/0.43.3" /tmp/cursor.deb
apt install -y /tmp/cursor.deb
rm -f /tmp/cursor.deb"#
    ))
}

#[tauri::command]
pub fn install_waydroid() -> u64 {
    start_pkexec_job(
        r#"curl -fsSL https://repo.waydro.id/waydroid.gpg -o /usr/share/keyrings/waydroid.gpg
echo 'deb [signed-by=/usr/share/keyrings/waydroid.gpg] https://repo.waydro.id/ trixie main' > /etc/apt/sources.list.d/waydroid.list
apt update
apt install -y waydroid"#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_waydroid() -> u64 {
    start_pkexec_job(
        r#"apt purge -y waydroid 2>/dev/null || true
rm -f /etc/apt/sources.list.d/waydroid.list /usr/share/keyrings/waydroid.gpg"#
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
flatpak install --user -y GeForceNOW com.nvidia.geforcenow"#
                .to_string(),
        ],
    )
}

#[tauri::command]
pub fn uninstall_geforcenow() -> u64 {
    start_job("bash", vec!["-c".to_string(), "flatpak uninstall --user -y com.nvidia.geforcenow".to_string()])
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
verify_and_download "https://github.com/SethRobinson/Patchy/releases/download/v1.00/PatchyLinux.flatpak" /tmp/PatchyLinux.flatpak
flatpak install --user -y --noninteractive --bundle /tmp/PatchyLinux.flatpak
rm -f /tmp/PatchyLinux.flatpak
echo "Patchy installed.""#
            ),
        ],
    )
}

#[tauri::command]
pub fn uninstall_patchy() -> u64 {
    start_job("bash", vec!["-c".to_string(), "flatpak uninstall --user -y com.rtsoft.patchy".to_string()])
}

const ESDE_DESKTOP_ENTRY: &str = "[Desktop Entry]\nName=ES-DE\nExec=%HOME%/AppImages/ES-DE_x64.AppImage\nIcon=%HOME%/AppImages/.icons/es-de.png\nType=Application\nCategories=Game;\nComment=Frontend for emulators with a modern interface\nX-AppImage-Integrate=true\n";

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
verify_and_download "https://gitlab.com/es-de/emulationstation-de/-/package_files/246875981/download" "$HOME/AppImages/ES-DE_x64.AppImage"
chmod +x "$HOME/AppImages/ES-DE_x64.AppImage"
cp "{icon}" "$HOME/AppImages/.icons/es-de.png"
printf '%s' '{ESDE_DESKTOP_ENTRY}' | sed "s|%HOME%|$HOME|g" > "$HOME/.local/share/applications/es-de.desktop"
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
    start_pkexec_job("apt install -y cpupower-gui linux-cpupower".to_string())
}

#[tauri::command]
pub fn uninstall_cpupower_gui() -> u64 {
    start_pkexec_job("apt remove -y cpupower-gui linux-cpupower".to_string())
}

#[tauri::command]
pub fn install_amdgpu_top() -> u64 {
    start_pkexec_job(format!(
        r#"{VERIFY_DOWNLOAD_SH}
AMDGPU_TOP_URL=$(curl -s https://api.github.com/repos/Umio-Yasuno/amdgpu_top/releases/latest | grep browser_download_url | grep amd64.deb | grep -v without_gui | cut -d\" -f4)
verify_and_download "$AMDGPU_TOP_URL" /tmp/amdgpu-top.deb
apt install -y /tmp/amdgpu-top.deb
rm -f /tmp/amdgpu-top.deb"#
    ))
}

// CoolerControl is two things, ported verbatim from 2.x's
// services/coolercontrol-install.sh: the AppImage daemon itself (needs root
// to control fans, hence Exec=pkexec) and a companion "WebApp" .desktop
// entry that opens its local web UI (http://localhost:11987/) in whichever
// browser is available, in app/kiosk mode with its own taskbar identity.
const COOLERCONTROL_DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=CoolerControl\nExec=pkexec %HOME%/AppImages/CoolerControlD-x86_64.AppImage\nTryExec=%HOME%/AppImages/CoolerControlD-x86_64.AppImage\nIcon=%HOME%/AppImages/.icons/coolercontrol.png\nCategories=System;\nComment=Fan and cooling control daemon\nX-AppImage-RunAsRoot=true\nX-AppImage-Integrate=true\n";

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

const DOUBLECMD_DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=Double Commander\nExec=%HOME%/AppImages/doublecmd-gtk-latest-x86_64.AppImage\nIcon=%HOME%/AppImages/.icons/doublecmd.png\nCategories=FileManager;\nComment=Twin-panel file manager with advanced features\nX-AppImage-Integrate=true\n";

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
apt install -y virtualbox-7.2"#
            .to_string(),
    )
}

#[tauri::command]
pub fn uninstall_virtualbox() -> u64 {
    start_pkexec_job(
        r#"apt purge -y 'virtualbox-*' 2>/dev/null || true
rm -f /etc/apt/sources.list.d/virtualbox.list /etc/apt/sources.list.d/virtualbox-trixie-security.sources /usr/share/keyrings/oracle-virtualbox-2016.gpg"#
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
VBOX_VERSION=$(VBoxManage --version 2>/dev/null | grep -oE '^[0-9]+\.[0-9]+\.[0-9]+')
if [ -z "$VBOX_VERSION" ]; then
    echo "Could not detect an installed VirtualBox version. Install VirtualBox first."
    exit 1
fi
echo "Downloading the Extension Pack for VirtualBox $VBOX_VERSION..."
verify_and_download "https://download.virtualbox.org/virtualbox/$VBOX_VERSION/Oracle_VirtualBox_Extension_Pack-$VBOX_VERSION.vbox-extpack" /tmp/vbox-extpack.vbox-extpack
VBoxManage extpack install --replace --accept-license=eb31505e56e9b4d0fbca139104da41ac6f6b98f8e78968bdf01b1f3da3c4f9ae /tmp/vbox-extpack.vbox-extpack
rm -f /tmp/vbox-extpack.vbox-extpack
echo "VirtualBox Extension Pack installed.""#
    ))
}

#[tauri::command]
pub fn uninstall_vbox_extpack() -> u64 {
    start_pkexec_job("VBoxManage extpack uninstall \"Oracle VirtualBox Extension Pack\"".to_string())
}

