// Gaming Center commands — ported from soplos-welcome 2.x's gaming_tab.py.
// Plain apt items (GameMode, CPU Power) and Flatpak launchers reuse the
// generic commands from software.rs — this module covers the tools with
// bespoke install methods, static config files, or system-wide state.

use std::process::Command;

use super::drivers::detect_gpus;
use super::software::{start_job, start_pkexec_job};
use crate::utils::{is_valid_https_url, is_valid_pkg_name, VERIFY_DOWNLOAD_SH};

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

// === Generic helpers reused by launchers (apt/flatpak already exist in
// software.rs; deb_url downloads and AppImage/custom scripts land here) ===

#[tauri::command]
pub fn check_path_exists(path: String) -> bool {
    std::path::Path::new(&expand_home(&path)).exists()
}

#[tauri::command]
pub fn install_deb_url(url: String, package: String) -> Result<u64, String> {
    if !is_valid_pkg_name(&package) {
        return Err(format!("Invalid package name: {package}"));
    }
    if !is_valid_https_url(&url) {
        return Err(format!("Invalid URL: {url}"));
    }
    let filename = format!("/tmp/{package}.deb");
    let script = format!(
        r#"set -e
{VERIFY_DOWNLOAD_SH}
verify_and_download "{url}" "{filename}"
dpkg -i "{filename}" || apt-get install -f -y
rm -f "{filename}"
echo "Installation complete."
"#
    );
    Ok(start_pkexec_job(script))
}

// === Performance Mode ===
// Wraps game launches to switch the CPU to the "performance" power profile.

const GAME_PERFORMANCE_SCRIPT: &str = r#"#!/usr/bin/env bash
# Soplos Gaming Performance Wrapper
# Temporarily switches the power profile to "performance" while a game runs.

if ! command -v powerprofilesctl &>/dev/null; then
    echo "Error: powerprofilesctl not found" >&2
    echo "Install: apt install power-profiles-daemon" >&2
    exit 1
fi

if ! powerprofilesctl list | grep -q 'performance:'; then
    echo "Warning: your CPU does not support the performance profile" >&2
    echo "Running without power profile optimization..." >&2
    exec "$@"
fi

exec systemd-inhibit --why "Soplos Gaming Mode active" powerprofilesctl launch \
    -p performance -r "Launched with Soplos Gaming Performance" -- "$@"
"#;

#[tauri::command]
pub fn is_performance_mode_installed() -> bool {
    std::path::Path::new("/usr/local/bin/soplos-game-performance").exists()
}

#[tauri::command]
pub fn install_performance_mode() -> u64 {
    let script = format!(
        r#"set -e
cat > /usr/local/bin/soplos-game-performance << 'SCRIPT_EOF'
{GAME_PERFORMANCE_SCRIPT}SCRIPT_EOF
chmod +x /usr/local/bin/soplos-game-performance
echo "Performance Mode installed."
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_performance_mode() -> u64 {
    start_pkexec_job("rm -f /usr/local/bin/soplos-game-performance\necho \"Performance Mode removed.\"".to_string())
}

// === Gaming Sysctl ===

const GAMING_SYSCTL_CONF: &str = r#"# Soplos Gaming Sysctl Profile
# Adapted from CachyOS-Settings

# === MEMORY ===
vm.max_map_count = 2147483642
vm.swappiness = 10
vm.page-cluster = 0

# === I/O AND CACHE ===
vm.vfs_cache_pressure = 50
vm.dirty_bytes = 268435456
vm.dirty_background_bytes = 134217728
vm.dirty_writeback_centisecs = 1500

# === NETWORK ===
net.ipv4.tcp_fastopen = 3
net.core.netdev_max_backlog = 16384
net.core.rmem_default = 1048576
net.core.rmem_max = 16777216
net.core.wmem_default = 1048576
net.core.wmem_max = 16777216

# === SYSTEM ===
fs.file-max = 2097152
kernel.nmi_watchdog = 0
kernel.split_lock_mitigate = 0
"#;

const SYSCTL_CONF_PATH: &str = "/etc/sysctl.d/99-soplos-gaming.conf";

#[tauri::command]
pub fn is_gaming_sysctl_active() -> bool {
    std::path::Path::new(SYSCTL_CONF_PATH).exists()
}

#[tauri::command]
pub fn apply_gaming_sysctl() -> u64 {
    let script = format!(
        r#"set -e
cat > {SYSCTL_CONF_PATH} << 'CONF_EOF'
{GAMING_SYSCTL_CONF}CONF_EOF
sysctl --system
echo "Gaming sysctl tweaks applied."
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn revert_gaming_sysctl() -> u64 {
    let script = format!(
        r#"set -e
rm -f {SYSCTL_CONF_PATH}
sysctl --system
echo "Gaming sysctl tweaks reverted."
"#
    );
    start_pkexec_job(script)
}

// === Optimize GPU ===

const AMD_ENV_CONF: &str = "RADV_PERFTEST=gpl\nAMD_VULKAN_ICD=RADV\nmesa_glthread=true\nMESA_LOADER_DRIVER_OVERRIDE=radeonsi\n";
const INTEL_ENV_CONF: &str = "mesa_glthread=true\nMESA_LOADER_DRIVER_OVERRIDE=iris\n";
const NVIDIA_ENV_CONF: &str = "__GL_SHADER_DISK_CACHE=1\n__GL_THREADED_OPTIMIZATION=1\n__GL_SYNC_TO_VBLANK=0\nPROTON_ENABLE_NVAPI=1\n";
const PRIME_RUN_SCRIPT: &str = "#!/bin/sh\n# NVIDIA PRIME Render Offload Script\nexec env __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia \"$@\"\n";

#[tauri::command]
pub fn is_gpu_optimized() -> bool {
    std::fs::read_dir("/etc/environment.d")
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("50-soplos-") && name.ends_with("-gaming.conf")
            })
        })
        .unwrap_or(false)
}

#[tauri::command]
pub fn apply_gpu_optimization() -> Result<u64, String> {
    let gpus = detect_gpus();
    if gpus.is_empty() {
        return Err("No supported GPU (NVIDIA, AMD or Intel) was detected".to_string());
    }
    let is_hybrid = gpus.len() > 1;
    let has_nvidia = gpus.iter().any(|g| g.vendor == "NVIDIA");
    let primary = gpus
        .iter()
        .find(|g| g.vendor == "NVIDIA")
        .or_else(|| gpus.iter().find(|g| g.vendor == "AMD"))
        .unwrap_or(&gpus[0]);

    let (vendor_key, env_conf) = match primary.vendor.as_str() {
        "NVIDIA" => ("nvidia", NVIDIA_ENV_CONF),
        "AMD" => ("amd", AMD_ENV_CONF),
        _ => ("intel", INTEL_ENV_CONF),
    };

    let prime_run_block = if is_hybrid && has_nvidia {
        format!(
            r#"echo "Installing prime-run script..."
cat > /usr/local/bin/prime-run << 'PRIME_EOF'
{PRIME_RUN_SCRIPT}PRIME_EOF
chmod +x /usr/local/bin/prime-run
"#
        )
    } else {
        String::new()
    };

    let script = format!(
        r#"set -e
mkdir -p /etc/environment.d
cat > /etc/environment.d/50-soplos-{vendor_key}-gaming.conf << 'ENV_EOF'
{env_conf}ENV_EOF
{prime_run_block}echo "GPU optimizations applied for {vendor_key}."
"#
    );
    Ok(start_pkexec_job(script))
}

#[tauri::command]
pub fn revert_gpu_optimization() -> u64 {
    let script = r#"set -e
rm -f /etc/environment.d/50-soplos-*-gaming.conf
echo "GPU optimizations reverted."
"#
    .to_string();
    start_pkexec_job(script)
}

// === Disk I/O schedulers ===

const IOSCHEDULERS_RULES: &str = r#"# Soplos I/O Schedulers Gaming Optimization
ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="1", ATTR{queue/scheduler}="bfq"
ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="0", ATTR{queue/scheduler}="mq-deadline"
ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/scheduler}="none"
"#;

const IOSCHEDULERS_PATH: &str = "/etc/udev/rules.d/60-soplos-ioschedulers.rules";

#[tauri::command]
pub fn is_disk_io_optimized() -> bool {
    std::path::Path::new(IOSCHEDULERS_PATH).exists()
}

#[tauri::command]
pub fn apply_disk_io_optimization() -> u64 {
    let script = format!(
        r#"set -e
cat > {IOSCHEDULERS_PATH} << 'RULES_EOF'
{IOSCHEDULERS_RULES}RULES_EOF
udevadm control --reload-rules
udevadm trigger
echo "Disk I/O optimizations applied."
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn revert_disk_io_optimization() -> u64 {
    let script = format!(
        r#"set -e
rm -f {IOSCHEDULERS_PATH}
udevadm control --reload-rules
udevadm trigger
echo "Disk I/O optimizations reverted."
"#
    );
    start_pkexec_job(script)
}

// === MangoHud + overlay tools (Flatpak) ===

const MANGOHUD_FLATPAKS: &str = "io.github.benjamimgois.goverlay//stable \
org.freedesktop.Platform.VulkanLayer.MangoHud//25.08 \
org.freedesktop.Platform.VulkanInfo//25.08 \
org.freedesktop.Platform.VulkanLayer.gamescope//25.08 \
org.winehq.Wine.DLLs.dxvk//stable-23.08 \
org.freedesktop.Platform.VulkanLayer.vkBasalt//25.08";

#[tauri::command]
pub fn is_mangohud_installed() -> bool {
    Command::new("flatpak")
        .args(["info", "io.github.benjamimgois.goverlay"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn install_mangohud() -> u64 {
    let script = format!(
        r#"set -e
echo "Installing MangoHud and gaming overlay tools via Flatpak..."
flatpak install -y flathub {MANGOHUD_FLATPAKS}
echo "MangoHud + tools installed."
"#
    );
    start_job("bash", vec!["-c".to_string(), script])
}

// === RyzenAdj (compiled from source — AMD Mini PC thermal fix) ===

#[tauri::command]
pub fn is_ryzenadj_installed() -> bool {
    std::path::Path::new("/usr/local/bin/ryzenadj").exists()
        || std::path::Path::new("/etc/systemd/system/ryzenadj.service").exists()
}

#[tauri::command]
pub fn install_ryzenadj() -> u64 {
    let script = r#"set -e
apt install -y cmake libpci-dev git
rm -rf /tmp/ryzenadj-src
git clone https://github.com/FlyGoat/RyzenAdj.git /tmp/ryzenadj-src
cd /tmp/ryzenadj-src
mkdir build && cd build
cmake ..
make
cp ryzenadj /usr/local/bin/ryzenadj
find /tmp/ryzenadj-src/build -name "libryzenadj.so" -exec cp {} /usr/local/lib/libryzenadj.so \;
ldconfig
tee /etc/systemd/system/ryzenadj.service > /dev/null << 'EOF'
[Unit]
Description=RyzenAdj thermal limits for AMD Mini PCs
After=multi-user.target

[Service]
ExecStart=/usr/local/bin/ryzenadj --tctl-temp=85 --stapm-limit=35000 --fast-limit=35000 --slow-limit=35000
Type=oneshot

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable --now ryzenadj
rm -rf /tmp/ryzenadj-src
echo "RyzenAdj installed."
"#
    .to_string();
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_ryzenadj() -> u64 {
    let script = r#"set -e
systemctl stop ryzenadj || true
systemctl disable ryzenadj || true
rm -f /etc/systemd/system/ryzenadj.service
systemctl daemon-reload
rm -f /usr/local/bin/ryzenadj
rm -f /usr/local/lib/libryzenadj.so
ldconfig
echo "RyzenAdj removed."
"#
    .to_string();
    start_pkexec_job(script)
}

// === Lutris Vulkan Fix ===
// The bug only exists inside the Flatpak sandbox, where gpu.py looks for
// vulkaninfo at /usr/bin (the host path) instead of /app/bin (where the
// Flatpak's own VulkanInfo extension puts it). A native apt install sees the
// real /usr/bin/vulkaninfo, so it never has this bug — it just needs
// vulkan-tools present to provide that binary at all.

#[tauri::command]
pub fn is_lutris_flatpak_user_installed() -> bool {
    Command::new("flatpak")
        .args(["info", "--user", "net.lutris.Lutris"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn is_lutris_flatpak_system_installed() -> bool {
    Command::new("flatpak")
        .args(["info", "--system", "net.lutris.Lutris"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn lutris_gpu_py_patch_script(base_dir: &str) -> String {
    format!(
        r#"GPU_PY=$(find -L "{base_dir}" -path "*/active/*" -name gpu.py 2>/dev/null | head -1)
if [ -z "$GPU_PY" ]; then
    echo "gpu.py not found under {base_dir} — skipping."
else
    if grep -q '/app/bin/vulkaninfo' "$GPU_PY"; then
        echo "Patch already applied."
    else
        sed -i 's|/usr/bin/vulkaninfo|/app/bin/vulkaninfo|g' "$GPU_PY"
        echo "Lutris Vulkan patch applied to $GPU_PY."
    fi
fi
"#
    )
}

#[tauri::command]
pub fn fix_lutris_flatpak_user() -> u64 {
    let home = std::env::var("HOME").unwrap_or_default();
    let base = format!("{home}/.local/share/flatpak/app/net.lutris.Lutris");
    let script = format!("set -e\n{}", lutris_gpu_py_patch_script(&base));
    start_job("bash", vec!["-c".to_string(), script])
}

#[tauri::command]
pub fn fix_lutris_flatpak_system() -> u64 {
    let script = format!(
        "set -e\n{}",
        lutris_gpu_py_patch_script("/var/lib/flatpak/app/net.lutris.Lutris")
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn fix_lutris_apt() -> u64 {
    let script = r#"set -e
if command -v vulkaninfo >/dev/null 2>&1; then
    echo "vulkan-tools already provides vulkaninfo — nothing to do."
else
    apt install -y vulkan-tools
    echo "vulkan-tools installed."
fi
"#
    .to_string();
    start_pkexec_job(script)
}

// === Revert All ===

#[tauri::command]
pub fn revert_all_gaming_optimizations() -> u64 {
    let script = r#"set -e
if [ -f /etc/sysctl.d/99-soplos-gaming.conf ]; then
    echo "Reverting gaming sysctl tweaks..."
    rm -f /etc/sysctl.d/99-soplos-gaming.conf
    sysctl --system
fi
if ls /etc/environment.d/50-soplos-*-gaming.conf >/dev/null 2>&1; then
    echo "Removing GPU optimizations..."
    rm -f /etc/environment.d/50-soplos-*-gaming.conf
fi
if [ -f /etc/udev/rules.d/60-soplos-ioschedulers.rules ]; then
    echo "Reverting disk I/O optimizations..."
    rm -f /etc/udev/rules.d/60-soplos-ioschedulers.rules
    udevadm control --reload-rules
    udevadm trigger
fi
if [ -f /usr/local/bin/soplos-game-performance ]; then
    echo "Removing performance mode script..."
    rm -f /usr/local/bin/soplos-game-performance
fi
if [ -f /usr/local/bin/prime-run ]; then
    echo "Removing prime-run script..."
    rm -f /usr/local/bin/prime-run
fi
echo "All optimizations reverted."
"#
    .to_string();
    start_pkexec_job(script)
}

// === Gaming Wallpapers ===
// The archive is bundled as a Tauri resource (like VBoxLinuxAdditions.run)
// since public/assets only ships inside the webview bundle, not as a real
// filesystem path the backend can extract from after install.

#[tauri::command]
pub fn install_gaming_wallpapers(app: tauri::AppHandle, desktop: String) -> Result<u64, String> {
    use tauri::Manager;
    let archive = app
        .path()
        .resolve(
            "resources/wallpapers/wallpapers.tar.xz",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| e.to_string())?;
    let archive = archive.to_string_lossy().into_owned();

    let (dest_dir, is_xfce, is_gnome) = match desktop.as_str() {
        "kde" => ("/usr/share/wallpapers/soplos".to_string(), false, false),
        "xfce" => ("/usr/share/backgrounds/soplos".to_string(), true, false),
        "gnome" => ("/usr/share/backgrounds/soplos".to_string(), false, true),
        _ => ("/usr/share/backgrounds/soplos".to_string(), false, false),
    };

    let xfce_block = if is_xfce {
        r#"echo "Creating XFCE symlinks..."
mkdir -p /usr/share/backgrounds/xfce
for file in "$DEST"/*; do
    is_image "$file" || continue
    [ -f "$file" ] || continue
    filename=$(basename "$file")
    ln -sf "$DEST/$filename" "/usr/share/backgrounds/xfce/$filename"
done
"#
    } else {
        ""
    };

    let gnome_block = if is_gnome {
        r#"echo "Generating GNOME wallpaper list..."
XML="/usr/share/gnome-background-properties/soplos-gaming-wallpapers.xml"
mkdir -p /usr/share/gnome-background-properties
{
    echo '<?xml version="1.0" encoding="UTF-8"?>'
    echo '<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">'
    echo '<wallpapers>'
    for file in "$DEST"/*; do
        is_image "$file" || continue
        [ -f "$file" ] || continue
        filename=$(basename "$file")
        name="${filename%.*}"
        echo '  <wallpaper deleted="false">'
        echo "    <name>Soplos Gaming $name</name>"
        echo "    <filename>$DEST/$filename</filename>"
        echo '    <options>zoom</options>'
        echo '  </wallpaper>'
    done
    echo '</wallpapers>'
} > "$XML"
"#
    } else {
        ""
    };

    // The script runs under `sh` (dash), which has no brace expansion, so the
    // image extensions are matched with `case` instead of a `*.{jpg,png}` glob.
    let image_fn = r#"is_image() {
    case "$1" in
        *.jpg|*.jpeg|*.png|*.webp|*.JPG|*.JPEG|*.PNG|*.WEBP) return 0 ;;
    esac
    return 1
}
"#;

    let script = format!(
        r#"set -e
DEST="{dest_dir}"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
{image_fn}
echo "Extracting wallpapers..."
tar -xf "{archive}" -C "$TMPDIR"

echo "Installing wallpapers to $DEST..."
mkdir -p "$DEST"
COUNT=0
for file in "$TMPDIR"/*; do
    is_image "$file" || continue
    [ -f "$file" ] || continue
    cp "$file" "$DEST/"
    COUNT=$((COUNT + 1))
done
if [ "$COUNT" -eq 0 ]; then
    echo "ERROR: no wallpapers were found in the archive."
    exit 1
fi

{xfce_block}{gnome_block}echo "Gaming wallpapers installed to $DEST ($COUNT files)."
"#
    );
    Ok(start_pkexec_job(script))
}
