// Kernel management commands — ported from soplos-welcome 2.x's kernels_tab.py
// and soplos-kernel-installer's "Soplos Kernels" tab.

use std::process::{Command, Stdio};
use std::io::Write;

use serde::Serialize;

use super::drivers::{detect_gpus, nvidia_dkms_patch_commands};
use super::software::start_pkexec_job;
use crate::utils::is_valid_pkg_name;

fn run_stdout(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

// === System information ===

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelInfo {
    kernel: String,
    kernel_type: String,
    arch: String,
    uptime: String,
}

#[tauri::command]
pub fn get_kernel_info() -> KernelInfo {
    let kernel = run_stdout("uname", &["-r"]);
    let arch = run_stdout("uname", &["-m"]);
    let uptime = run_stdout("uptime", &["-p"])
        .strip_prefix("up ")
        .unwrap_or("")
        .to_string();

    let kernel_type = if kernel.contains("liquorix") {
        "Liquorix Kernel"
    } else if kernel.contains("soplos") {
        "Soplos Kernel"
    } else if kernel.contains("xanmod") {
        "XanMod Kernel"
    } else {
        "Standard Kernel"
    }
    .to_string();

    KernelInfo {
        kernel,
        kernel_type,
        arch,
        uptime,
    }
}

#[tauri::command]
pub fn is_kernel_in_use(needle: String) -> bool {
    run_stdout("uname", &["-r"]).contains(&needle)
}

// === Broken XanMod repository (leftover from earlier Soplos versions) ===
// A previous version wrote a Debian suite XanMod no longer publishes, which
// makes every `apt update` fail system-wide. Ported from
// _has_broken_xanmod_repo / _on_repair_xanmod_repo_clicked.

#[tauri::command]
pub fn has_broken_xanmod_repo() -> bool {
    std::fs::read_to_string("/etc/apt/sources.list.d/xanmod-release.list")
        .map(|content| {
            content
                .lines()
                .any(|l| l.contains("deb.xanmod.org") && l.split_whitespace().any(|w| w == "releases"))
        })
        .unwrap_or(false)
}

#[tauri::command]
pub fn repair_xanmod_repo() -> u64 {
    let script = r#"set -e
echo "=== Repairing XanMod repository ==="
rm -f /etc/apt/sources.list.d/xanmod-release.list
rm -f /etc/apt/keyrings/xanmod-archive-keyring.gpg
apt update -q || true
echo "[+] Broken repository removed."
"#
    .to_string();
    start_pkexec_job(script)
}

// === CPU microcode ===
// Install/uninstall status is checked from the frontend via the generic
// is_apt_package_installed / install_apt_packages / uninstall_apt_packages
// commands (intel-microcode / amd64-microcode) — only vendor detection is
// specific to this tab.

#[tauri::command]
pub fn get_cpu_vendor() -> Option<String> {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    if cpuinfo.contains("GenuineIntel") {
        Some("intel".to_string())
    } else if cpuinfo.contains("AuthenticAMD") {
        Some("amd".to_string())
    } else {
        None
    }
}

// === XanMod kernel variants ===
// Ported from _get_cpu_psabi_level / _get_xanmod_package / on_install_xanmod_clicked.

fn get_cpu_psabi_level() -> u8 {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let flags: std::collections::HashSet<&str> = cpuinfo
        .lines()
        .find(|l| l.starts_with("flags"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.split_whitespace().collect())
        .unwrap_or_default();

    if flags.is_empty() {
        return 1;
    }

    let v2 = ["cx16", "lahf_lm", "popcnt", "sse4_1", "sse4_2", "ssse3"];
    let v3 = ["avx", "avx2", "bmi1", "bmi2", "f16c", "fma", "abm", "movbe", "xsave"];
    let v4 = ["avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl"];

    if !v2.iter().all(|f| flags.contains(f)) {
        return 1;
    }
    if !v3.iter().all(|f| flags.contains(f)) {
        return 2;
    }
    if !v4.iter().all(|f| flags.contains(f)) {
        return 3;
    }
    4
}

fn xanmod_package(kernel_type: &str) -> Option<String> {
    let tier = get_cpu_psabi_level().min(3);
    if kernel_type == "xanmod-lts" {
        return Some(format!("linux-xanmod-lts-x64v{tier}"));
    }
    if tier < 2 {
        return None;
    }
    let prefix = match kernel_type {
        "xanmod-main" => "linux-xanmod",
        "xanmod-rt" => "linux-xanmod-rt",
        "xanmod-edge" => "linux-xanmod-edge",
        _ => return None,
    };
    Some(format!("{prefix}-x64v{tier}"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XanmodVariant {
    key: String,
    package: Option<String>,
    installed: bool,
    in_use: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XanmodStatus {
    psabi_level: u8,
    variants: Vec<XanmodVariant>,
}

#[tauri::command]
pub fn get_xanmod_status() -> XanmodStatus {
    let running = run_stdout("uname", &["-r"]);
    let in_use = running.contains("xanmod");
    let keys = ["xanmod-main", "xanmod-rt", "xanmod-edge", "xanmod-lts"];

    let variants = keys
        .iter()
        .map(|&key| {
            let package = xanmod_package(key);
            let installed = package
                .as_ref()
                .map(|p| super::drivers::is_package_installed(p))
                .unwrap_or(false);
            XanmodVariant {
                key: key.to_string(),
                package,
                installed,
                in_use,
            }
        })
        .collect();

    XanmodStatus {
        psabi_level: get_cpu_psabi_level(),
        variants,
    }
}

#[tauri::command]
pub fn install_xanmod(kernel_type: String) -> Result<u64, String> {
    let package = xanmod_package(&kernel_type).ok_or("This CPU does not support this XanMod variant")?;
    let script = format!(
        r#"set -e
echo "Installing XanMod {kernel_type}..."
mkdir -p /etc/apt/keyrings
# Earlier versions wrote the retired "releases" suite, which fails on every
# apt update. Reinstalling must repair that state.
rm -f /etc/apt/sources.list.d/xanmod-release.list
wget -qO - https://dl.xanmod.org/archive.key | gpg --batch --yes --dearmor -o /etc/apt/keyrings/xanmod-archive-keyring.gpg
# XanMod publishes one suite per Debian codename, taken from the Debian
# repositories actually configured on this system.
CODENAME=$(apt-cache policy | grep o=Debian | grep -o "n=[a-z]*" | cut -d= -f2 | sort | uniq -c | sort -rn | head -1 | tr -dc "a-z") || true
case "$CODENAME" in
    bookworm|trixie|forky|sid) ;;
    *)
        echo "Could not determine the Debian codename of this system."
        echo "XanMod publishes one suite per Debian codename, so the repository cannot be configured."
        exit 1
        ;;
esac
echo "deb [signed-by=/etc/apt/keyrings/xanmod-archive-keyring.gpg] http://deb.xanmod.org $CODENAME main" > /etc/apt/sources.list.d/xanmod-release.list
apt update
# XanMod kernels are built with LLVM/Clang, so DKMS has to build its modules
# with the same toolchain or they fail — only for systems that actually have
# DKMS modules, so nobody else gets LLVM for nothing.
if command -v dkms >/dev/null 2>&1 && [ -n "$(dkms status 2>/dev/null)" ]; then
    echo "DKMS modules found: installing the LLVM toolchain they need"
    apt install -y clang lld llvm
fi
apt install -y {package}
echo "Installation complete."
"#
    );
    Ok(start_pkexec_job(script))
}

#[tauri::command]
pub fn uninstall_xanmod(kernel_type: String) -> Result<u64, String> {
    let package = xanmod_package(&kernel_type).ok_or("This CPU does not support this XanMod variant")?;
    let script = format!(
        r#"set -e
echo "Uninstalling XanMod..."
apt remove -y {package}
echo "Uninstallation complete."
"#
    );
    Ok(start_pkexec_job(script))
}

// === Liquorix kernel ===
// Ported from on_install_liquorix_clicked / on_uninstall_liquorix_clicked.

#[tauri::command]
pub fn install_liquorix() -> u64 {
    let script = r#"set -e
echo "Installing Liquorix Kernel..."
INSTALLER=$(mktemp /tmp/liquorix-upstream.XXXXXX.sh)
if ! curl -fsSL https://liquorix.net/install-liquorix.sh -o "$INSTALLER"; then
    rm -f "$INSTALLER"
    echo "Could not download the Liquorix installer. Check your internet connection."
    exit 1
fi
bash "$INSTALLER"
STATUS=$?
rm -f "$INSTALLER"
if [ $STATUS -ne 0 ]; then
    echo "Installation failed."
    exit $STATUS
fi
echo "Installation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_liquorix() -> u64 {
    let script = r#"set -e
echo "Uninstalling Liquorix Kernel..."
apt remove -y linux-image-liquorix-amd64 linux-headers-liquorix-amd64
rm -f /etc/apt/sources.list.d/liquorix.list
rm -f /etc/apt/keyrings/liquorix-keyring.gpg
apt update -q || true
echo "Uninstallation complete."
"#
    .to_string();
    start_pkexec_job(script)
}

// === Soplos Kernel Installer (external app) ===

#[tauri::command]
pub fn open_kernel_installer() -> Result<(), String> {
    Command::new("soplos-kernel-installer")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// === System maintenance: clean old kernels / update GRUB ===
// Ported from on_clean_kernels_clicked / on_update_grub_clicked.

fn version_sort(pkgs: &[String]) -> Vec<String> {
    if pkgs.is_empty() {
        return vec![];
    }
    let sorted = (|| -> Option<Vec<String>> {
        let mut child = Command::new("sort")
            .arg("-V")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .ok()?;
        child.stdin.as_mut()?.write_all(pkgs.join("\n").as_bytes()).ok()?;
        let output = child.wait_with_output().ok()?;
        Some(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect(),
        )
    })();
    sorted.unwrap_or_else(|| {
        let mut v = pkgs.to_vec();
        v.sort();
        v
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanKernelsPreview {
    current: String,
    keep: Vec<String>,
    remove: Vec<String>,
}

#[tauri::command]
pub fn preview_clean_kernels() -> CleanKernelsPreview {
    let current = run_stdout("uname", &["-r"]);
    let dpkg_out = Command::new("dpkg")
        .args(["-l", "linux-image-*"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let meta_patterns = [
        "linux-image-amd64",
        "linux-image-liquorix-amd64",
        "linux-image-xanmod",
        "linux-image-686",
    ];

    let mut installed: Vec<String> = Vec::new();
    for line in dpkg_out.lines() {
        if !line.starts_with("ii") {
            continue;
        }
        let pkg = line.split_whitespace().nth(1).unwrap_or("").to_string();
        if pkg.is_empty() || !pkg.contains("linux-image-") {
            continue;
        }
        // Skip the unversioned metapackages themselves (e.g. "linux-image-amd64",
        // which always depends on the latest real numbered kernel) so they never
        // get mixed into the version comparison below. Must be an exact match:
        // these names already contain digits ("amd64", "686"), so a substring +
        // "no digits in the name" check never actually filters anything out.
        if meta_patterns.contains(&pkg.as_str()) {
            continue;
        }
        installed.push(pkg);
    }

    if installed.is_empty() {
        return CleanKernelsPreview {
            current,
            keep: vec![],
            remove: vec![],
        };
    }

    let family = |pkg: &str, tags: &[&str]| tags.iter().any(|t| pkg.contains(t));

    let base_kernels = version_sort(
        &installed
            .iter()
            .filter(|p| !family(p, &["liquorix", "xanmod", "soplos"]))
            .cloned()
            .collect::<Vec<_>>(),
    );
    let soplos_kernels = version_sort(&installed.iter().filter(|p| p.contains("soplos")).cloned().collect::<Vec<_>>());
    let liquorix_kernels = version_sort(&installed.iter().filter(|p| p.contains("liquorix")).cloned().collect::<Vec<_>>());
    let xanmod_kernels = version_sort(&installed.iter().filter(|p| p.contains("xanmod")).cloned().collect::<Vec<_>>());

    let mut keep: std::collections::HashSet<String> = std::collections::HashSet::new();
    for pkg in &installed {
        if pkg.contains(&current) {
            keep.insert(pkg.clone());
        }
    }
    if let Some(p) = base_kernels.last() {
        keep.insert(p.clone());
    }
    if let Some(p) = soplos_kernels.last() {
        keep.insert(p.clone());
    }
    if let Some(p) = liquorix_kernels.last() {
        keep.insert(p.clone());
    }
    if let Some(p) = xanmod_kernels.last() {
        keep.insert(p.clone());
    }

    let mut to_remove: Vec<String> = installed.iter().filter(|p| !keep.contains(*p)).cloned().collect();

    let mut headers_to_remove = Vec::new();
    for pkg in &to_remove {
        let header_pkg = pkg.replacen("linux-image-", "linux-headers-", 1);
        if let Ok(out) = Command::new("dpkg").args(["-l", &header_pkg]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if out.status.success() && stdout.contains("ii") {
                headers_to_remove.push(header_pkg);
            }
        }
    }
    to_remove.extend(headers_to_remove);
    to_remove.sort();

    let mut keep_vec: Vec<String> = keep.into_iter().collect();
    keep_vec.sort();

    CleanKernelsPreview {
        current,
        keep: keep_vec,
        remove: to_remove,
    }
}

#[tauri::command]
pub fn clean_kernels(packages: Vec<String>) -> Result<u64, String> {
    if packages.is_empty() || !packages.iter().all(|p| is_valid_pkg_name(p)) {
        return Err("Invalid package name in cleanup list".to_string());
    }
    let packages_str = packages.join(" ");
    let script = format!(
        r#"set -e
echo "Purging old kernels and headers..."
apt purge -y {packages_str}
echo "Updating GRUB..."
update-grub
echo "Cleanup complete."
"#
    );
    Ok(start_pkexec_job(script))
}

#[tauri::command]
pub fn update_grub() -> u64 {
    let script = r#"set -e
echo "Updating GRUB..."
if [ -x /usr/sbin/update-grub ]; then
    /usr/sbin/update-grub
else
    update-grub
fi
echo "GRUB update complete."
"#
    .to_string();
    start_pkexec_job(script)
}

// === Soplos Kernels repository + package list ===
// Ported from soplos-kernel-installer's main_window.py "Soplos Kernels" tab.

const SOPLOS_KERNELS_REPO_FILE: &str = "/etc/apt/sources.list.d/soplos-kernels.sources";
const SOPLOS_KERNELS_REPO_URL: &str = "https://raw.githubusercontent.com/SoplosLinux/soplos-kernels/main/";
const SOPLOS_KERNELS_GPG_URL: &str = "https://raw.githubusercontent.com/SoplosLinux/soplos-kernels/main/public.key";
const SOPLOS_KERNELS_GPG_FILE: &str = "/usr/share/keyrings/soplos-kernels.gpg";

#[tauri::command]
pub fn is_soplos_kernels_repo_installed() -> bool {
    std::path::Path::new(SOPLOS_KERNELS_REPO_FILE).exists()
}

#[tauri::command]
pub fn add_soplos_kernels_repo() -> u64 {
    let script = format!(
        r#"set -e
wget -qO- "{SOPLOS_KERNELS_GPG_URL}" | gpg --dearmor -o "{SOPLOS_KERNELS_GPG_FILE}"
mkdir -p /etc/apt/sources.list.d
printf "Types: deb\nURIs: {SOPLOS_KERNELS_REPO_URL}\nSuites: stable\nComponents: main\nSigned-By: {SOPLOS_KERNELS_GPG_FILE}\n" > "{SOPLOS_KERNELS_REPO_FILE}"
apt-get update -qq
echo "[+] Soplos Kernels repository added."
"#
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn remove_soplos_kernels_repo() -> u64 {
    let script = format!(
        r#"set -e
rm -f "{SOPLOS_KERNELS_REPO_FILE}" "{SOPLOS_KERNELS_GPG_FILE}"
apt-get update -qq
echo "[+] Soplos Kernels repository removed."
"#
    );
    start_pkexec_job(script)
}

fn is_valid_soplos_pkg_name(pkg: &str) -> bool {
    // Matches ^linux-soplos(-[a-z0-9]+)*$
    match pkg.strip_prefix("linux-soplos") {
        Some(rest) if rest.is_empty() => true,
        Some(rest) => rest
            .strip_prefix('-')
            .map(|r| {
                r.split('-')
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
            })
            .unwrap_or(false),
        None => false,
    }
}

fn soplos_pkg_flavor_and_march(pkg: &str) -> (String, Option<String>) {
    let suffix = match pkg.strip_prefix("linux-soplos") {
        Some(s) => s,
        None => return ("Stock".to_string(), None),
    };
    if suffix.is_empty() {
        return ("Stock".to_string(), None);
    }
    let parts: Vec<&str> = suffix.trim_start_matches('-').split('-').collect();
    let marches = ["v1", "v2", "v3", "v4"];
    let last = *parts.last().unwrap_or(&"");
    let has_march = marches.contains(&last);
    let march = if has_march { Some(last.to_uppercase()) } else { None };
    let flavor_parts: &[&str] = if has_march { &parts[..parts.len() - 1] } else { &parts[..] };

    let label = if flavor_parts.is_empty() {
        "Stock".to_string()
    } else {
        let joined = flavor_parts.join("-");
        if joined.len() <= 5 {
            joined.to_uppercase()
        } else {
            joined
                .replace('-', " ")
                .split(' ')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    };
    (label, march)
}

fn soplos_pkg_display_name(pkg: &str) -> String {
    let (label, march) = soplos_pkg_flavor_and_march(pkg);
    match march {
        Some(m) => format!("{label} {m}"),
        None => label,
    }
}

fn boot_vmlinuz_matching(suffix: &str) -> Option<String> {
    if suffix.is_empty() {
        return None;
    }
    std::fs::read_dir("/boot").ok()?.flatten().find_map(|entry| {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("vmlinuz-") && name.ends_with(suffix) {
            Some(name)
        } else {
            None
        }
    })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SoplosKernelPkg {
    package: String,
    name: String,
    description: String,
    suffix: String,
    version: String,
    installed: bool,
    has_update: bool,
}

// Parses a batched `apt-cache show pkg1 pkg2 ...` output into {pkg -> (desc,
// version, suffix)}. Splitting once per stanza and indexing by the
// `Package:` field lets a single subprocess call cover every package,
// instead of spawning `apt-cache show` once per package in a loop — with
// ~20 Soplos kernel packages and a large apt cache (many extra repos
// enabled, as most Soplos machines have), the old per-package loop measured
// close to 10 seconds on a real laptop.
fn parse_apt_show_batch(output: &str) -> std::collections::HashMap<String, (String, String, String)> {
    let mut map = std::collections::HashMap::new();
    for stanza in output.split("\n\n") {
        let mut pkg_name = String::new();
        let mut desc = String::new();
        let mut suffix = String::new();
        let mut version = String::new();
        for line in stanza.lines() {
            if let Some(v) = line.strip_prefix("Package:") {
                pkg_name = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Description:") {
                desc = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Version:") {
                version = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Depends:") {
                for dep in v.split(',') {
                    let dep = dep.trim().split_whitespace().next().unwrap_or("");
                    if dep.starts_with("linux-image-") {
                        if let Some(idx) = dep.find("-soplos") {
                            suffix = dep[idx..].to_string();
                        }
                        break;
                    }
                }
            }
        }
        if !pkg_name.is_empty() && !desc.is_empty() {
            map.insert(pkg_name, (desc, version, suffix));
        }
    }
    map
}

// Parses a batched `apt-cache policy pkg1 pkg2 ...` output into {pkg ->
// (installed_version, candidate_version)}. Each package's block starts with
// an unindented "<pkgname>:" header line. Run with LC_ALL=C — apt-cache
// policy's "Installed:"/"Candidate:" labels are themselves localized (e.g.
// "Instalados:"/"Candidato:" under a Spanish locale), so without forcing the
// C locale this parsing silently finds nothing on any non-English system.
fn parse_apt_policy_batch(output: &str) -> std::collections::HashMap<String, (String, String)> {
    let mut map = std::collections::HashMap::new();
    let mut current_pkg: Option<String> = None;
    let mut installed_ver = String::new();
    let mut candidate_ver = String::new();
    for line in output.lines() {
        if !line.starts_with(' ') && !line.starts_with('\t') && line.trim().ends_with(':') && !line.trim().is_empty() {
            if let Some(pkg) = current_pkg.take() {
                map.insert(pkg, (installed_ver.clone(), candidate_ver.clone()));
            }
            current_pkg = Some(line.trim().trim_end_matches(':').to_string());
            installed_ver.clear();
            candidate_ver.clear();
            continue;
        }
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("Installed:") {
            let v = v.trim();
            if v != "(none)" {
                installed_ver = v.to_string();
            }
        } else if let Some(v) = trimmed.strip_prefix("Candidate:") {
            let v = v.trim();
            if v != "(none)" {
                candidate_ver = v.to_string();
            }
        }
    }
    if let Some(pkg) = current_pkg.take() {
        map.insert(pkg, (installed_ver, candidate_ver));
    }
    map
}

#[tauri::command]
pub fn list_soplos_kernels() -> Vec<SoplosKernelPkg> {
    let names = match Command::new("apt-cache").args(["pkgnames", "linux-soplos"]).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return vec![],
    };

    let mut pkg_names: Vec<String> = names
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|p| !p.is_empty() && is_valid_soplos_pkg_name(p))
        .collect();
    pkg_names.sort();
    pkg_names.dedup();

    if pkg_names.is_empty() {
        return vec![];
    }

    let show_output = Command::new("apt-cache")
        .arg("show")
        .args(&pkg_names)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let show_map = parse_apt_show_batch(&show_output);

    let policy_output = Command::new("apt-cache")
        .env("LC_ALL", "C")
        .arg("policy")
        .args(&pkg_names)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let policy_map = parse_apt_policy_batch(&policy_output);

    let mut result = Vec::new();
    for pkg in pkg_names {
        let Some((desc, version, suffix)) = show_map.get(&pkg).cloned() else {
            continue;
        };

        let (installed_ver, candidate_ver) = policy_map.get(&pkg).cloned().unwrap_or_default();
        let apt_registered = !installed_ver.is_empty();
        let mut has_update = apt_registered && !candidate_ver.is_empty() && installed_ver != candidate_ver;

        // The actual install status is whether the kernel is on disk (matches
        // soplos-kernel-installer's _is_soplos_kernel_installed), not whether
        // apt considers the package installed.
        let installed = !suffix.is_empty() && boot_vmlinuz_matching(&suffix).is_some();

        if !has_update && installed && !apt_registered {
            if let Some(vmlinuz) = boot_vmlinuz_matching(&suffix) {
                if let Some(boot_ver) = vmlinuz.strip_prefix("vmlinuz-").and_then(|v| v.strip_suffix(&suffix)) {
                    if !candidate_ver.is_empty() && !boot_ver.is_empty() && boot_ver != candidate_ver {
                        has_update = true;
                    }
                }
            }
        }

        let name = soplos_pkg_display_name(&pkg);
        result.push(SoplosKernelPkg {
            package: pkg,
            name,
            description: desc,
            suffix,
            version,
            installed,
            has_update,
        });
    }

    result
}

#[tauri::command]
pub fn install_soplos_kernel(package: String) -> Result<u64, String> {
    if !is_valid_soplos_pkg_name(&package) {
        return Err(format!("Invalid package name: {package}"));
    }
    let has_nvidia = detect_gpus().iter().any(|g| g.vendor == "NVIDIA");
    let patch = if has_nvidia {
        format!("{} && ", nvidia_dkms_patch_commands())
    } else {
        String::new()
    };
    let script = format!(
        r#"set -e
echo "Installing {package}..."
{patch}apt install -y {package}
echo "Installation complete."
"#
    );
    Ok(start_pkexec_job(script))
}

#[tauri::command]
pub fn uninstall_soplos_kernel(package: String, suffix: String) -> Result<u64, String> {
    if !is_valid_soplos_pkg_name(&package) {
        return Err(format!("Invalid package name: {package}"));
    }
    if !is_valid_pkg_name(&suffix) {
        return Err(format!("Invalid suffix: {suffix}"));
    }
    let script = format!(
        r#"set -e
echo "Removing {package}..."
EXTRA=$(dpkg -l | awk '$1=="ii"{{print $2}}' | grep -E '^(linux-image-|linux-headers-).*{suffix}$' || true)
apt remove -y {package} $EXTRA
echo "Removal complete."
"#
    );
    Ok(start_pkexec_job(script))
}

#[tauri::command]
pub fn update_soplos_kernel(package: String, suffix: String) -> Result<u64, String> {
    if !is_valid_soplos_pkg_name(&package) {
        return Err(format!("Invalid package name: {package}"));
    }
    if !is_valid_pkg_name(&suffix) {
        return Err(format!("Invalid suffix: {suffix}"));
    }
    let has_nvidia = detect_gpus().iter().any(|g| g.vendor == "NVIDIA");
    let patch = if has_nvidia {
        format!("{} && ", nvidia_dkms_patch_commands())
    } else {
        String::new()
    };
    let script = format!(
        r#"set -e
echo "Updating {package}..."
{patch}apt install -y --only-upgrade {package}
RUNNING=$(uname -r)
dpkg -l 'linux-image-*{suffix}' 2>/dev/null | awk '/^ii/{{print $2}}' | sort -V | head -n -1 | grep -v "$RUNNING" | xargs -r apt purge -y 2>/dev/null || true
echo "Update complete."
"#
    );
    Ok(start_pkexec_job(script))
}

// === Debian Official Kernels ===

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebianKernelPackage {
    pub package_name: String,
    pub label: String,
    pub description: String,
    pub installed: bool,
    pub in_use: bool,
}

#[tauri::command]
pub fn get_debian_kernels() -> Vec<DebianKernelPackage> {
    let running = run_stdout("uname", &["-r"]);
    let debian_meta = [
        ("linux-image-amd64", "Debian Linux Standard (amd64)", "Standard 64-bit official Debian kernel"),
        ("linux-image-rt-amd64", "Debian Linux PREEMPT_RT (amd64)", "Real-time low latency 64-bit official Debian kernel"),
    ];

    let mut result = Vec::new();
    for (pkg, label, desc) in debian_meta {
        let is_installed = Command::new("dpkg-query")
            .args(["-W", "-f=${Status}", pkg])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("installed"))
            .unwrap_or(false);
        let in_use = is_installed && running.contains("amd64") && !running.contains("soplos") && !running.contains("liquorix") && !running.contains("xanmod");
        result.push(DebianKernelPackage {
            package_name: pkg.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            installed: is_installed,
            in_use,
        });
    }
    result
}

#[tauri::command]
pub fn install_debian_kernel(package: String) -> Result<u64, String> {
    if !package.starts_with("linux-image-") || !is_valid_pkg_name(&package) {
        return Err(format!("Invalid package name: {package}"));
    }
    let script = format!(
        r#"set -e
echo "Installing Debian official kernel {package}..."
apt update -q || true
apt install -y {package}
echo "Debian kernel {package} installed."
echo "IMPORTANT: Restart the system to boot into the new kernel."
"#
    );
    Ok(start_pkexec_job(script))
}

#[tauri::command]
pub fn uninstall_debian_kernel(package: String) -> Result<u64, String> {
    if !package.starts_with("linux-image-") || !is_valid_pkg_name(&package) {
        return Err(format!("Invalid package name: {package}"));
    }
    let script = format!(
        r#"set -e
echo "Uninstalling Debian official kernel {package}..."
apt purge -y {package}
echo "Debian kernel {package} uninstalled."
"#
    );
    Ok(start_pkexec_job(script))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soplos_pkg_name_accepts_real_names() {
        assert!(is_valid_soplos_pkg_name("linux-soplos"));
        assert!(is_valid_soplos_pkg_name("linux-soplos-bore-v3"));
        assert!(is_valid_soplos_pkg_name("linux-soplos-rt-v4"));
    }

    #[test]
    fn soplos_pkg_name_rejects_injection_and_other_packages() {
        assert!(!is_valid_soplos_pkg_name(""));
        assert!(!is_valid_soplos_pkg_name("firefox"));
        assert!(!is_valid_soplos_pkg_name("linux-soplos; rm -rf /"));
        assert!(!is_valid_soplos_pkg_name("linux-soplos-BORE"));
        assert!(!is_valid_soplos_pkg_name("linux-soplos-"));
    }
}
