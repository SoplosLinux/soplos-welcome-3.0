// Desktop customization commands — ported from soplos-welcome 2.x's
// customization_tab.py. Launching Soplos tools and native DE settings apps
// reuses the generic launch_app / is_apt_package_installed / install_apt_packages
// commands (from security.rs / software.rs) — only LucidGlyph, which is
// installed straight from its GitHub releases rather than through apt, gets
// commands of its own here.

use std::process::Command;

use super::software::start_pkexec_job;

#[tauri::command]
pub fn is_lucidglyph_installed() -> bool {
    std::path::Path::new("/usr/local/share/lucidglyph/version").exists()
}

#[tauri::command]
pub fn install_lucidglyph() -> u64 {
    let script = r#"set -e
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading LucidGlyph..."
TARBALL_URL=$(curl -fsSL -H "User-Agent: soplos-welcome" https://api.github.com/repos/maximilionus/lucidglyph/releases/latest | grep -o '"tarball_url": *"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -z "$TARBALL_URL" ]; then
    echo "Error: could not get LucidGlyph release URL"
    exit 1
fi
wget -q -O "$TMPDIR/lucidglyph.tar.gz" "$TARBALL_URL"
tar -xzf "$TMPDIR/lucidglyph.tar.gz" -C "$TMPDIR"

SRC_DIR=$(find "$TMPDIR" -maxdepth 1 -type d -name 'maximilionus-lucidglyph-*' | head -1)
if [ -z "$SRC_DIR" ]; then
    echo "Error: extracted directory not found"
    exit 1
fi

echo "Installing LucidGlyph..."
cd "$SRC_DIR" && bash ./lucidglyph.sh install
echo "Installation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_lucidglyph() -> u64 {
    let script = r#"set -e
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading LucidGlyph..."
TARBALL_URL=$(curl -fsSL -H "User-Agent: soplos-welcome" https://api.github.com/repos/maximilionus/lucidglyph/releases/latest | grep -o '"tarball_url": *"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -z "$TARBALL_URL" ]; then
    echo "Error: could not get LucidGlyph release URL"
    exit 1
fi
wget -q -O "$TMPDIR/lucidglyph.tar.gz" "$TARBALL_URL"
tar -xzf "$TMPDIR/lucidglyph.tar.gz" -C "$TMPDIR"

SRC_DIR=$(find "$TMPDIR" -maxdepth 1 -type d -name 'maximilionus-lucidglyph-*' | head -1)
if [ -z "$SRC_DIR" ]; then
    echo "Error: extracted directory not found"
    exit 1
fi

echo "Removing LucidGlyph..."
cd "$SRC_DIR" && bash ./lucidglyph.sh remove
echo "Uninstallation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

// gsettings has to run as the logged-in user (not through pkexec, which
// would write the root user's dconf instead), so these run unprivileged —
// called from the frontend right after a successful GNOME install/uninstall,
// matching the 2.x tab's separate (non-pkexec'd) gsettings calls.

#[tauri::command]
pub fn set_gnome_font_antialiasing_grayscale() -> Result<(), String> {
    Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "font-antialiasing", "grayscale"])
        .status()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_gnome_font_antialiasing() -> Result<(), String> {
    Command::new("gsettings")
        .args(["reset", "org.gnome.desktop.interface", "font-antialiasing"])
        .status()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
