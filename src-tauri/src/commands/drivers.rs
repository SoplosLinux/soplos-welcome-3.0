use std::process::Command;

use serde::Serialize;

use super::software::start_pkexec_job;
use crate::utils::{is_valid_identifier, is_valid_pkg_name};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub(crate) vendor: String,
    pub(crate) model: String,
    gpu_type: String,
    recommended_driver: Option<String>,
    driver_status: String,
    installed_driver_version: Option<String>,
}

pub(crate) fn is_package_installed(package: &str) -> bool {
    Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", package])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("install ok installed"))
        .unwrap_or(false)
}

pub(crate) fn get_lspci_output() -> String {
    Command::new("lspci")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

fn detect_installed_nvidia_version() -> Option<String> {
    if let Ok(output) = Command::new("nvidia-smi")
        .args(["--query-gpu=driver_version", "--format=csv,noheader"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(major) = text.trim().split('.').next() {
                if !major.is_empty() && major.chars().all(|c| c.is_ascii_digit()) {
                    return Some(major.to_string());
                }
            }
        }
    }

    if let Ok(output) = Command::new("dpkg").arg("-l").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if !line.starts_with("ii") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            if let Some(rest) = parts[1].strip_prefix("nvidia-driver-") {
                if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                    return Some(rest.to_string());
                }
            }
        }
    }

    None
}

// Marker substrings for GPUs of the Turing generation or newer — the only
// cards the 610/590 driver branches support. Installing them on an older
// card leaves the machine with no working module, and since the NVIDIA
// package blacklists nouveau, with no graphics driver at all. Ported as-is
// from hardware_detector.py's NVIDIA_TURING_PLUS_MARKERS.
const NVIDIA_TURING_PLUS_MARKERS: &[&str] = &[
    "rtx",
    "gtx 16",
    "gtx16",
    "mx450",
    "mx 450",
    "mx550",
    "mx 550",
    "mx570",
    "mx 570",
    "t400",
    "t500",
    "t550",
    "t600",
    "t1000",
    "t1200",
    "t2000",
    "tesla t4",
];

// Mirrors Python's `\b` regex word boundary: a plain substring check would
// let a short marker like "ati" match inside an unrelated word like
// "Corporation", or "t550" match inside "gtx550" (a Fermi card, not Turing).
// Ported from hardware_detector.py, which hit exactly these two collisions.
pub(crate) fn contains_word_boundary(haystack: &str, needle: &str) -> bool {
    let is_word_char = |c: char| c.is_alphanumeric();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs_pos = start + pos;
        let before_ok = haystack[..abs_pos].chars().last().map_or(true, |c| !is_word_char(c));
        let after_idx = abs_pos + needle.len();
        let after_ok = haystack[after_idx..].chars().next().map_or(true, |c| !is_word_char(c));
        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
        if start >= haystack.len() {
            break;
        }
    }
    false
}

pub(crate) fn is_nvidia_turing_or_newer(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    NVIDIA_TURING_PLUS_MARKERS.iter().any(|marker| contains_word_boundary(&model_lower, marker))
}

fn extract_bracketed_model(line: &str) -> Option<String> {
    let start = line.rfind('[')?;
    let end = line[start..].find(']')? + start;
    Some(line[start + 1..end].trim().to_string())
}

// Fallback regex-free extraction for lspci lines with no bracketed marketing
// name (rare, but hardware_detector.py hits it and has a per-vendor fallback
// for each). Looks for the first occurrence of any of `patterns` and returns
// everything from that point to the next '(' or end of line.
fn extract_model_fallback(line: &str, patterns: &[&str]) -> Option<String> {
    let lower = line.to_lowercase();
    for pattern in patterns {
        if let Some(pos) = lower.find(pattern) {
            let rest = &line[pos..];
            let end = rest.find('(').unwrap_or(rest.len());
            let candidate = rest[..end].trim();
            // The UI prefixes the vendor itself ("{vendor} {model}"), so a
            // model that already starts with it rendered as "Intel Intel ...".
            let candidate = ["intel ", "amd ", "nvidia "]
                .iter()
                .find(|v| candidate.to_lowercase().starts_with(*v))
                .map(|v| candidate[v.len()..].trim())
                .unwrap_or(candidate);
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

// Ported from hardware_detector.py's _recommend_nvidia_driver: the same
// branch cutoffs, in the same order, for the same reasons documented there.
fn recommend_nvidia_driver(model: &str) -> String {
    let model_lower = model.to_lowercase();

    if is_nvidia_turing_or_newer(&model_lower) {
        return "nvidia-driver-610".to_string();
    }
    let pascal_maxwell = [
        "gtx 10", "gtx10", "gtx 1080", "gtx 1070", "gtx 1060", "gtx 1050", "gt 1030", "mx350",
        "mx 350", "mx330", "mx 330", "mx250", "mx 250", "mx150", "mx 150", "gtx 9", "gtx 980",
        "gtx 970", "gtx 960", "gtx 950", "gtx 8", "gtx 880", "gtx 870", "gtx 860", "gtx 850",
        "920m", "930m", "940m", "920mx", "930mx", "940mx", "910m", "820m", "840m", "960m",
        "970m", "980m",
    ];
    if pascal_maxwell.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-driver-580".to_string();
    }
    let kepler = [
        "gtx 7", "gtx 780", "gtx 770", "gtx 760", "gtx 750", "gtx 6", "gtx 680", "gtx 670",
        "gtx 660", "gtx 650", "gt 710", "gt 720", "gt 730", "gt 740", "mx230", "mx 230",
        "mx130", "mx 130", "mx110", "mx 110", "650m", "750m",
    ];
    if kepler.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-tesla-470-driver".to_string();
    }
    let fermi = [
        "gtx 580", "gtx 570", "gtx 560", "gtx 550", "gtx 480", "gtx 470", "gtx 460", "gtx 450",
        "gtx 555", "gtx 520", "gtx 510", "gt 640", "gt 630", "gt 620", "gt 610", "gt 545",
        "gt 530", "gt 520", "gt 440", "gt 430", "gts 450",
    ];
    if fermi.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-legacy-390xx-driver".to_string();
    }
    let tesla_gen = [
        "geforce 8", "geforce 9", "geforce 2", "geforce 3", "8400", "8600", "8800", "9400",
        "9500", "9600", "9800", "gts ", "320m", "330m", "quadro fx", "quadro nvs", "nforce",
    ];
    if tesla_gen.iter().any(|s| model_lower.contains(s)) {
        return "nouveau".to_string();
    }
    let professional = ["quadro", "tesla", "a100", "a40", "a30", "a10", "rtx a"];
    if professional.iter().any(|s| model_lower.contains(s)) {
        return "nvidia-driver".to_string();
    }
    // Unrecognised card: never fall back to a proprietary branch here. The
    // Debian nvidia-driver package only covers Maxwell to Ada, so anything
    // older would get a module that cannot build while the package
    // blacklists nouveau, leaving the machine with no driver at all.
    "nouveau".to_string()
}

/// Returns ("installed" | "partial" | "missing", missing_packages). Ported
/// from hardware_detector.py's _packages_status.
fn packages_status(required: &[&str]) -> (&'static str, Vec<String>) {
    let missing: Vec<String> = required.iter().filter(|p| !is_package_installed(p)).map(|s| s.to_string()).collect();
    if missing.is_empty() {
        ("installed", missing)
    } else if missing.len() == required.len() {
        ("missing", missing)
    } else {
        ("partial", missing)
    }
}

#[tauri::command]
pub fn detect_gpus() -> Vec<GpuInfo> {
    detect_gpus_from_lspci(&get_lspci_output())
}

// Split out so scan_hardware() (hardware_scan.rs) can reuse the lspci output
// it already fetched once, instead of detect_gpus() spawning its own
// redundant second `lspci` process on every hardware scan.
pub(crate) fn detect_gpus_from_lspci(lspci: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let mut nvidia_installed_version: Option<String> = None;
    let mut nvidia_checked = false;

    for line in lspci.lines() {
        let lower = line.to_lowercase();
        if !["vga", "3d", "display"].iter().any(|k| lower.contains(k)) {
            continue;
        }

        // "virtio" alone, not "virtio gpu": QEMU/KVM's virtio-gpu reports as
        // "Red Hat, Inc. Virtio 1.0 GPU" in lspci — the version number in the
        // middle means "virtio gpu" as a contiguous phrase never matches.
        // Verified against a real Proxmox/QEMU VM.
        if ["vmware svga", "virtualbox graphics", "vbox graphics", "qxl paravirtual", "bochs vbe", "virtio", "red hat"]
            .iter()
            .any(|k| lower.contains(k))
        {
            let model = if lower.contains("vmware") {
                "VMware SVGA II"
            } else if lower.contains("virtualbox") || lower.contains("vbox") {
                "VirtualBox VGA"
            } else if lower.contains("virtio") {
                "VirtIO GPU"
            } else if lower.contains("qxl") || lower.contains("red hat") {
                "QXL (QEMU/KVM)"
            } else {
                "Virtual GPU"
            };
            gpus.push(GpuInfo {
                vendor: "Virtual".to_string(),
                model: model.to_string(),
                gpu_type: "Virtual".to_string(),
                recommended_driver: None,
                driver_status: "installed".to_string(),
                installed_driver_version: None,
            });
        } else if contains_word_boundary(&lower, "nvidia") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["geforce rtx", "geforce gtx", "geforce", "nvidia"]))
                .unwrap_or_else(|| "NVIDIA Unknown".to_string());
            let recommended = recommend_nvidia_driver(&model);
            if !nvidia_checked {
                nvidia_installed_version = detect_installed_nvidia_version();
                nvidia_checked = true;
            }
            let status = if recommended == "nouveau" {
                if is_package_installed("xserver-xorg-video-nouveau") { "installed" } else { "missing" }
            } else if let Some(ver) = recommended.strip_prefix("nvidia-driver-") {
                match &nvidia_installed_version {
                    Some(installed) if installed == ver => "installed",
                    Some(_) => "different_version",
                    None => "missing",
                }
            } else if is_package_installed(&recommended) {
                "installed"
            } else if nvidia_installed_version.is_some() {
                "different_version"
            } else {
                "missing"
            };
            gpus.push(GpuInfo {
                vendor: "NVIDIA".to_string(),
                model,
                gpu_type: "Dedicated".to_string(),
                recommended_driver: Some(recommended),
                driver_status: status.to_string(),
                installed_driver_version: nvidia_installed_version.clone(),
            });
        } else if contains_word_boundary(&lower, "amd") || contains_word_boundary(&lower, "ati") || contains_word_boundary(&lower, "radeon") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["radeon rx", "radeon r", "radeon", "amd"]))
                .unwrap_or_else(|| "AMD Unknown".to_string());
            let required = ["firmware-amd-graphics", "libgl1-mesa-dri", "libglx-mesa0", "mesa-vulkan-drivers", "xserver-xorg-video-all"];
            let (status, _missing) = packages_status(&required);
            gpus.push(GpuInfo {
                vendor: "AMD".to_string(),
                model,
                gpu_type: "Dedicated/Integrated".to_string(),
                recommended_driver: Some(required.join(" ")),
                driver_status: status.to_string(),
                installed_driver_version: None,
            });
        } else if contains_word_boundary(&lower, "intel") {
            let model = extract_bracketed_model(line)
                .or_else(|| extract_model_fallback(line, &["uhd graphics", "hd graphics", "iris xe", "iris", "intel"]))
                .unwrap_or_else(|| "Intel Unknown".to_string());
            let required = ["intel-media-va-driver", "mesa-vulkan-drivers"];
            let (status, _missing) = packages_status(&required);
            gpus.push(GpuInfo {
                vendor: "Intel".to_string(),
                model,
                gpu_type: "Integrated".to_string(),
                recommended_driver: Some(required.join(" ")),
                driver_status: status.to_string(),
                installed_driver_version: None,
            });
        }
    }

    gpus
}

// === NVIDIA DKMS compatibility patches for Soplos kernel 7.x ===
// Ported verbatim from drivers_tab.py (NV_MMAP_VMA_LOCK_PATCH / NV_STRING_H_PATCH /
// get_nvidia_dkms_patch_commands / LLVM_DKMS_TOOLCHAIN). Without these, DKMS
// fails to build the NVIDIA module against this kernel and, since the driver
// package blacklists nouveau, the machine boots with no graphics driver at all.

const NV_MMAP_VMA_LOCK_PATCH: &str = r#"--- a/nvidia/nv-mmap.c
+++ b/nvidia/nv-mmap.c
@@ -868,17 +868,24 @@

     nvl->safe_to_mmap = safe_to_mmap;
 }
+#ifndef VM_REFCNT_EXCLUDE_READERS_FLAG
+#define VM_REFCNT_EXCLUDE_READERS_FLAG VMA_LOCK_OFFSET
+#else
+#define NV_VMA_WRITE_LOCKED_ONE_ARG 1
+#endif
+
+

 #if !NV_CAN_CALL_VMA_START_WRITE
 static NvBool nv_vma_enter_locked(struct vm_area_struct *vma, NvBool detaching)
 {
-    NvU32 tgt_refcnt = VMA_LOCK_OFFSET;
+    NvU32 tgt_refcnt = VM_REFCNT_EXCLUDE_READERS_FLAG;
     NvBool interrupted = NV_FALSE;
     if (!detaching)
     {
         tgt_refcnt++;
     }
-    if (!refcount_add_not_zero(VMA_LOCK_OFFSET, &vma->vm_refcnt))
+    if (!refcount_add_not_zero(VM_REFCNT_EXCLUDE_READERS_FLAG, &vma->vm_refcnt))
     {
         return NV_FALSE;
     }
@@ -908,7 +915,7 @@
     if (interrupted)
     {
         // Clean up on error: release refcount and dep_map
-        refcount_sub_and_test(VMA_LOCK_OFFSET, &vma->vm_refcnt);
+        refcount_sub_and_test(VM_REFCNT_EXCLUDE_READERS_FLAG, &vma->vm_refcnt);
         rwsem_release(&vma->vmlock_dep_map, _RET_IP_);
         return NV_FALSE;
     }
@@ -924,7 +931,11 @@
 {
     NvU32 mm_lock_seq;
     NvBool locked;
+#ifdef NV_VMA_WRITE_LOCKED_ONE_ARG
+    if (__is_vma_write_locked(vma))
+#else
     if (__is_vma_write_locked(vma, &mm_lock_seq))
+#endif
         return;

     locked = nv_vma_enter_locked(vma, NV_FALSE);
@@ -933,7 +944,7 @@
     if (locked)
     {
         NvBool detached;
-        detached = refcount_sub_and_test(VMA_LOCK_OFFSET, &vma->vm_refcnt);
+        detached = refcount_sub_and_test(VM_REFCNT_EXCLUDE_READERS_FLAG, &vma->vm_refcnt);
         rwsem_release(&vma->vmlock_dep_map, _RET_IP_);
         WARN_ON_ONCE(detached);
     }
"#;

const NV_STRING_H_PATCH: &str = r#"--- a/nvidia/os-interface.c
+++ b/nvidia/os-interface.c
@@ -35,6 +35,7 @@
 #include <linux/mmzone.h>
 #include <linux/numa.h>
 #include <linux/cpuset.h>
 #include <linux/sys_soc.h>
+#include <linux/string.h>

 extern char *NVreg_TemporaryFilePath;
"#;

const LLVM_DKMS_TOOLCHAIN: &str = r#"
if grep -q "CONFIG_CC_IS_CLANG=y" "/boot/config-$(uname -r)" 2>/dev/null; then
    echo "This kernel was built with LLVM/Clang: installing the toolchain DKMS needs..."
    apt install -y clang lld llvm
fi
"#;

pub(crate) fn nvidia_dkms_patch_commands() -> String {
    let mmap_patch = NV_MMAP_VMA_LOCK_PATCH.replace('\'', "'\\''");
    let string_h_patch = NV_STRING_H_PATCH.replace('\'', "'\\''");

    format!(
        "for NVIDIA_SRC in /usr/src/nvidia-*/; do \
  for NV_SUB in '' 'kernel-open/'; do \
    NV_ROOT=\"${{NVIDIA_SRC}}${{NV_SUB}}\"; \
    NV_MMAP=\"${{NV_ROOT}}nvidia/nv-mmap.c\"; \
    if [ -f \"$NV_MMAP\" ]; then \
      if grep -q 'VM_REFCNT_EXCLUDE_READERS_FLAG' \"$NV_MMAP\" 2>/dev/null; then \
        echo \"NVIDIA VMA patch already applied in ${{NV_ROOT}} - skipping.\"; \
      elif ! grep -q 'VMA_LOCK_OFFSET' \"$NV_MMAP\" 2>/dev/null; then \
        echo \"${{NV_ROOT}} does not use VMA_LOCK_OFFSET - patch not needed.\"; \
      else \
        echo \"Applying NVIDIA VMA lock patch to ${{NV_ROOT}}...\"; \
        printf '%s' '{mmap_patch}' | patch --fuzz=5 -p1 -d \"$NV_ROOT\" && \
        echo \"NVIDIA VMA patch applied.\" || \
        echo \"Warning: NVIDIA VMA patch failed for ${{NV_ROOT}} - DKMS may fail.\"; \
      fi; \
    fi; \
    NV_OSIF=\"${{NV_ROOT}}nvidia/os-interface.c\"; \
    if [ -f \"$NV_OSIF\" ]; then \
      if grep -q '#include <linux/string.h>' \"$NV_OSIF\" 2>/dev/null; then \
        echo \"NVIDIA string.h patch already applied in ${{NV_ROOT}} - skipping.\"; \
      else \
        echo \"Applying NVIDIA string.h patch to ${{NV_ROOT}}...\"; \
        printf '%s' '{string_h_patch}' | patch --fuzz=5 -p1 -d \"$NV_ROOT\" && \
        echo \"NVIDIA string.h patch applied.\" || \
        echo \"Warning: NVIDIA string.h patch failed for ${{NV_ROOT}} - DKMS may fail.\"; \
      fi; \
    fi; \
  done; \
done"
    )
}

// Installs an NVIDIA driver package straight from the Debian repositories —
// used by the 550 branch and (after the user enables Sid) the legacy
// 470/390/340 branches. Ported from drivers_tab.py's _on_nvidia_repo_clicked.
#[tauri::command]
pub fn install_nvidia_repo(package: String) -> Result<u64, String> {
    if !is_valid_pkg_name(&package) {
        return Err(format!("Invalid package name: {package}"));
    }
    let patch = nvidia_dkms_patch_commands();
    let llvm = LLVM_DKMS_TOOLCHAIN;
    let script = format!(
        r#"set -e
echo "Installing NVIDIA driver from repository..."
echo "Removing NVIDIA DKMS modules from all kernels..."
for entry in $(dkms status | grep -i nvidia | awk -F'[:,]' '{{print $1"/"$2}}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
for kver in $(ls /lib/modules/); do
    rm -f /lib/modules/$kver/updates/dkms/nvidia*.ko*
    depmod -a "$kver" 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
echo "Removing existing NVIDIA packages to prevent conflicts..."
apt purge -y 'nvidia-driver*' 'nvidia-kernel*' 'libnvidia*' 'nvidia-modprobe' 'nvidia-settings' 'nvidia-smi' 'nvidia-opencl*' 'nvidia-cuda*' 'cuda-drivers*' 'xserver-xorg-video-nvidia*' 2>/dev/null || true
apt update
apt install -y dkms build-essential linux-headers-$(uname -r)
{llvm}
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "Kernel build tree missing or broken, reinstalling headers..."
    apt install -y --reinstall linux-headers-$(uname -r) || true
fi
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "ERROR: no usable kernel build tree for $(uname -r)."
    echo "DKMS cannot build the NVIDIA module, so the driver is NOT installed."
    exit 1
fi
apt install -y {package} nvidia-smi nvidia-settings nvidia-modprobe libglu1-mesa
echo "Applying NVIDIA DKMS compatibility patch for Soplos 7.x kernels..."
{patch}
echo "Rebuilding NVIDIA DKMS modules..."
dkms autoinstall || true
if ! dkms status | grep -qi "nvidia.*installed"; then
    echo "WARNING: the NVIDIA DKMS module did not build. Run 'dkms autoinstall' manually before rebooting."
fi
echo "Configuring GRUB with nvidia-drm.modeset=1..."
if ! grep -q "nvidia-drm.modeset=1" /etc/default/grub; then
    sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\([^"]*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 nvidia-drm.modeset=1"/' /etc/default/grub
    update-grub
fi
if command -v dracut >/dev/null 2>&1; then
    mkdir -p /etc/dracut.conf.d
    echo 'omit_drivers+=" nouveau "' > /etc/dracut.conf.d/blacklist-nouveau.conf
    rm -f /etc/dracut.conf.d/nvidia.conf
    dracut --force
elif command -v update-initramfs >/dev/null 2>&1; then
    update-initramfs -u
fi
echo "=== Installation completed ==="
echo "NVIDIA driver installed successfully. Restart the system to apply the changes."
"#
    );
    Ok(start_pkexec_job(script))
}

// Installs an NVIDIA driver from the official NVIDIA CUDA repository — used
// by the 580/590/610 branches, each pinned to the Debian release NVIDIA
// actually publishes packages for. Ported from
// drivers_tab.py's _on_nvidia_cuda_repo_clicked.
#[tauri::command]
pub fn install_nvidia_cuda(version: String) -> Result<u64, String> {
    if !is_valid_identifier(&version) {
        return Err(format!("Invalid version: {version}"));
    }
    let (repo_setup, install_driver, repo_cleanup): (String, String, String) = match version.as_str() {
        "580" => (
            r#"echo "deb [trusted=yes] https://developer.download.nvidia.com/compute/cuda/repos/debian12/x86_64/ /" \
    > /etc/apt/sources.list.d/cuda-debian12-x86_64.list
trap 'rm -f /etc/apt/sources.list.d/cuda-debian12-x86_64.list' EXIT"#
                .to_string(),
            "apt install -y nvidia-driver-pinning-580\napt install -y --allow-downgrades cuda-drivers-580".to_string(),
            "trap - EXIT\nrm -f /etc/apt/sources.list.d/cuda-debian12-x86_64.list\napt update -q".to_string(),
        ),
        "590" => (
            r#"TEMP_DIR=$(mktemp -d)
wget -q -O "$TEMP_DIR/cuda-keyring.deb" \
    https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
if [ ! -s "$TEMP_DIR/cuda-keyring.deb" ]; then
    echo "ERROR: Failed to download cuda-keyring."
    rm -rf "$TEMP_DIR"
    exit 1
fi
dpkg -i "$TEMP_DIR/cuda-keyring.deb"
rm -rf "$TEMP_DIR""#
                .to_string(),
            "apt install -y nvidia-driver-pinning-590\napt install -y nvidia-open-590".to_string(),
            "apt purge -y cuda-keyring\nrm -f /etc/apt/sources.list.d/cuda-debian13-x86_64.list\napt update -q".to_string(),
        ),
        _ => (
            r#"TEMP_DIR=$(mktemp -d)
wget -q -O "$TEMP_DIR/cuda-keyring.deb" \
    https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
if [ ! -s "$TEMP_DIR/cuda-keyring.deb" ]; then
    echo "ERROR: Failed to download cuda-keyring."
    rm -rf "$TEMP_DIR"
    exit 1
fi
dpkg -i "$TEMP_DIR/cuda-keyring.deb"
rm -rf "$TEMP_DIR""#
                .to_string(),
            "apt install -y nvidia-driver-pinning-610\napt install -y nvidia-open-610".to_string(),
            "apt purge -y cuda-keyring\nrm -f /etc/apt/sources.list.d/cuda-debian13-x86_64.list\napt update -q".to_string(),
        ),
    };
    let distro = if version == "580" { "debian12" } else { "debian13" };
    let patch = nvidia_dkms_patch_commands();
    let llvm = LLVM_DKMS_TOOLCHAIN;

    let script = format!(
        r#"set -e
echo "=== NVIDIA {version} Official CUDA Repository ({distro}) ==="
echo "[0/6] Removing NVIDIA DKMS modules from all kernels..."
for entry in $(dkms status | grep -i nvidia | awk -F'[:,]' '{{print $1"/"$2}}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
for kver in $(ls /lib/modules/); do
    rm -f /lib/modules/$kver/updates/dkms/nvidia*.ko*
    depmod -a "$kver" 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
echo "[1/6] Removing existing NVIDIA/CUDA packages..."
rm -f /etc/dracut.conf.d/nvidia.conf
rm -f /etc/dracut.conf.d/blacklist-nouveau.conf
rm -f /etc/apt/apt.conf.d/99nvidia-sha1-exception
apt purge -y 'nvidia*' 'cuda*' 'libnvidia*' 2>/dev/null || true
apt -f install -y 2>/dev/null || true
echo "[2/6] Installing kernel headers and DKMS build dependencies..."
apt install -y dkms build-essential
apt install -y linux-headers-$(uname -r) || true
{llvm}
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "Kernel build tree missing or broken, reinstalling headers..."
    apt install -y --reinstall linux-headers-$(uname -r) || true
fi
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "ERROR: no usable kernel build tree for $(uname -r)."
    echo "DKMS cannot build the NVIDIA module, so the driver is NOT installed."
    exit 1
fi
echo "[3/6] Setting up NVIDIA CUDA repository ({distro})..."
{repo_setup}
echo "[4/6] Installing NVIDIA Driver {version}..."
apt update
{install_driver}
{repo_cleanup}
echo "[+] Applying NVIDIA DKMS compatibility patch for Soplos 7.x kernels..."
{patch}
echo "[+] Rebuilding NVIDIA DKMS modules..."
dkms autoinstall || true
if ! dkms status | grep -qi "nvidia.*installed"; then
    echo "WARNING: the NVIDIA DKMS module did not build. Run 'dkms autoinstall' manually before rebooting."
fi
echo "[5/6] Configuring dracut and regenerating initramfs..."
if command -v dracut >/dev/null 2>&1; then
    mkdir -p /etc/dracut.conf.d
    echo 'omit_drivers+=" nouveau "' > /etc/dracut.conf.d/blacklist-nouveau.conf
    rm -f /etc/dracut.conf.d/nvidia.conf
    dracut --force
elif command -v update-initramfs >/dev/null 2>&1; then
    update-initramfs -u
fi
echo "[+] Configuring GRUB with nvidia-drm.modeset=1..."
if ! grep -q "nvidia-drm.modeset=1" /etc/default/grub; then
    sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\([^"]*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 nvidia-drm.modeset=1"/' /etc/default/grub
fi
update-grub
echo "[6/6] Done."
echo "NVIDIA driver {version} installed. Restart to apply changes."
"#
    );
    Ok(start_pkexec_job(script))
}

// Fully removes NVIDIA/CUDA — shared by every "uninstall" driver button and
// by switching to nouveau. Ported from drivers_tab.py's
// _build_nvidia_uninstall_script.
#[tauri::command]
pub fn uninstall_nvidia() -> u64 {
    let script = r#"echo "=== Removing all NVIDIA drivers ==="
echo "[0/7] Removing NVIDIA DKMS modules from all kernels..."
for entry in $(dkms status | grep -i nvidia | awk -F'[:,]' '{print $1"/"$2}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
for kver in $(ls /lib/modules/); do
    rm -f /lib/modules/$kver/updates/dkms/nvidia*.ko*
    rm -f /lib/modules/$kver/updates/dkms/nvidia-*.ko*
    depmod -a "$kver" 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
echo "[1/7] Fixing any interrupted dpkg state..."
dpkg --configure -a 2>/dev/null || true
echo "[2/7] Removing NVIDIA and CUDA packages..."
NVIDIA_PKGS=$(dpkg -l | grep -iE '^[a-z]+[[:space:]]+(nvidia|libnvidia|cuda)' | awk '{print $2}' | tr '\n' ' ')
if [ -n "$NVIDIA_PKGS" ]; then
    apt purge -y $NVIDIA_PKGS 2>/dev/null || dpkg --purge --force-depends $NVIDIA_PKGS
else
    echo "No NVIDIA/CUDA packages found."
fi
echo "[3/7] Removing NVIDIA CUDA repository sources..."
rm -f /etc/apt/sources.list.d/cuda-*.list
rm -f /etc/apt/sources.list.d/nvidia*.list
rm -f /usr/share/keyrings/cuda-*.gpg
rm -f /usr/share/keyrings/nvidia*.gpg
apt update -q
echo "[4/7] Removing NVIDIA dracut configuration..."
rm -f /etc/dracut.conf.d/nvidia.conf
rm -f /etc/dracut.conf.d/blacklist-nouveau.conf
rm -f /etc/apt/apt.conf.d/99nvidia-sha1-exception
echo "[5/7] Restoring GRUB defaults..."
sed -i 's/ nvidia-drm.modeset=1//' /etc/default/grub 2>/dev/null || true
update-grub 2>/dev/null || true
echo "[6/7] Regenerating initramfs..."
if command -v dracut >/dev/null 2>&1; then
    dracut --force
elif command -v update-initramfs >/dev/null 2>&1; then
    update-initramfs -u
fi
echo "[7/7] Done. Restart the system before installing a new driver version."
"#
    .to_string();
    start_pkexec_job(script)
}

// === Wi-Fi repair ===
// Ported from drivers_tab.py's _detect_wifi_driver / _on_repair_wifi_clicked:
// three fallback strategies so the driver is found even when the interface
// is down or the module isn't loaded at all — the exact "no Wi-Fi after
// reboot" case this button exists to fix.

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiDriverInfo {
    iface: Option<String>,
    driver: Option<String>,
}

fn driver_link_target(iface: &str) -> Option<String> {
    let link = format!("/sys/class/net/{iface}/device/driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn detect_wifi_driver() -> WifiDriverInfo {
    // Strategy 1: sysfs wireless subdir + device/driver symlink.
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let iface = entry.file_name().to_string_lossy().into_owned();
            if std::path::Path::new(&format!("/sys/class/net/{iface}/wireless")).exists() {
                if let Some(driver) = driver_link_target(&iface) {
                    return WifiDriverInfo { iface: Some(iface), driver: Some(driver) };
                }
            }
        }
    }

    // Strategy 2: `iw dev` — interface exists but may be down.
    if let Ok(output) = Command::new("iw").arg("dev").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let line = line.trim();
            if let Some(iface) = line.strip_prefix("Interface ") {
                if let Some(driver) = driver_link_target(iface) {
                    return WifiDriverInfo { iface: Some(iface.to_string()), driver: Some(driver) };
                }
            }
        }
    }

    // Strategy 3: `lspci -k` — detects the module even when not loaded, the
    // critical fallback for the "no Wi-Fi after reboot" case.
    if let Ok(output) = Command::new("lspci").arg("-k").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut in_wifi = false;
        for line in text.lines() {
            let lower = line.to_lowercase();
            if ["network controller", "wireless", "wifi", "wi-fi"].iter().any(|k| lower.contains(k)) {
                in_wifi = true;
            } else if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                in_wifi = false;
            }
            if in_wifi {
                if let Some(rest) = lower.split_once("kernel modules:") {
                    if let Some(driver) = rest.1.split(',').next() {
                        let driver = driver.trim();
                        if !driver.is_empty() {
                            return WifiDriverInfo { iface: None, driver: Some(driver.to_string()) };
                        }
                    }
                } else if let Some(rest) = lower.split_once("kernel driver in use:") {
                    let driver = rest.1.trim();
                    if !driver.is_empty() {
                        return WifiDriverInfo { iface: None, driver: Some(driver.to_string()) };
                    }
                }
            }
        }
    }

    WifiDriverInfo { iface: None, driver: None }
}

#[tauri::command]
pub fn repair_wifi(iface: Option<String>, driver: String) -> Result<u64, String> {
    if !is_valid_identifier(&driver) {
        return Err(format!("Invalid driver name: {driver}"));
    }
    let iface_value = iface.unwrap_or_default();
    if !iface_value.is_empty() && !is_valid_identifier(&iface_value) {
        return Err(format!("Invalid interface name: {iface_value}"));
    }
    let script = format!(
        r#"IFACE="{iface_value}"
if [ -n "$IFACE" ]; then
    ip link set "$IFACE" down 2>/dev/null || true
fi
modprobe -r {driver} 2>/dev/null || true
sleep 1
modprobe {driver}
systemctl restart NetworkManager
echo "Wi-Fi repair completed. Driver: {driver}"
"#
    );
    Ok(start_pkexec_job(script))
}

// === Intel oneAPI Base Toolkit ===
// Ported from drivers_tab.py's _on_oneapi_clicked.

#[tauri::command]
pub fn is_oneapi_installed() -> bool {
    std::path::Path::new("/opt/intel/oneapi/setvars.sh").exists()
}

#[tauri::command]
pub fn install_oneapi() -> u64 {
    let script = r#"set -e
echo "=== Intel oneAPI Base Toolkit Installation ==="
echo "[1/3] Setting up Intel oneAPI repository..."
mkdir -p /etc/apt/keyrings
curl -fsSL https://apt.repos.intel.com/intel-gpg-keys/GPG-PUB-KEY-INTEL-SW-PRODUCTS.PUB \
    | gpg --dearmor -o /etc/apt/keyrings/intel-oneapi.gpg
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/intel-oneapi.gpg] https://apt.repos.intel.com/oneapi all main" \
    > /etc/apt/sources.list.d/intel-oneapi.list
cat > /etc/apt/preferences.d/intel-oneapi-pin-600 <<'PINEOF'
Package: *
Pin: release o=Intel Corporation
Pin-Priority: 600
PINEOF
echo "[2/3] Updating package lists..."
apt update
echo "[3/3] Installing Intel oneAPI Toolkit..."
apt install -y intel-oneapi-toolkit
echo "=== Installation completed successfully ==="
echo "Run 'source /opt/intel/oneapi/setvars.sh' to activate the environment."
"#
    .to_string();
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_oneapi() -> u64 {
    let script = r#"set -e
echo "=== Removing Intel oneAPI Base Toolkit ==="
apt purge -y 'intel-basekit*' 'intel-oneapi-*' 2>/dev/null || true
rm -f /etc/apt/sources.list.d/intel-oneapi.list
rm -f /etc/apt/keyrings/intel-oneapi.gpg
rm -f /etc/apt/preferences.d/intel-oneapi-pin-600
apt update -q
echo "[+] Intel oneAPI removed successfully."
"#
    .to_string();
    start_pkexec_job(script)
}

// === CUDA 12 Toolkit (NVIDIA Extras) ===
// Ported from drivers_tab.py's _on_cuda12_clicked.

#[tauri::command]
pub fn is_cuda12_installed() -> bool {
    std::path::Path::new("/usr/local/cuda/bin/nvcc").exists()
}

#[tauri::command]
pub fn install_cuda12() -> u64 {
    let script = r#"set -e
echo "=== CUDA 12 Toolkit Installation ==="
echo "[1/3] Setting up NVIDIA CUDA repository..."
echo "deb [trusted=yes] https://developer.download.nvidia.com/compute/cuda/repos/debian12/x86_64/ /" \
    > /etc/apt/sources.list.d/cuda-debian12-x86_64.list
echo "[2/3] Updating package lists..."
apt update
echo "[3/3] Installing CUDA 12 Toolkit..."
apt install -y cuda-toolkit-12
rm -f /etc/apt/sources.list.d/cuda-debian12-x86_64.list
apt update -q
echo "=== Installation completed successfully ==="
echo "CUDA 12 Toolkit installed. Run 'nvcc --version' to verify."
"#
    .to_string();
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_cuda12() -> u64 {
    let script = r#"set -e
echo "=== Removing CUDA 12 Toolkit ==="
apt purge -y 'cuda-toolkit-12*' 'cuda-compiler-12*' 'cuda-libraries-12*' 'cuda-tools-12*' 'cuda-documentation-12*' 'cuda-nvml-dev-12*' 2>/dev/null || true
rm -f /etc/apt/sources.list.d/cuda-debian12-x86_64.list
apt update -q
echo "[+] CUDA 12 Toolkit removed successfully."
"#
    .to_string();
    start_pkexec_job(script)
}

// === AMD ROCm (OpenCL runtime / Full Suite) ===
// Ported from drivers_tab.py's _on_rocm_clicked. Installed status for both
// modes is checked via the generic is_apt_package_installed command, since
// rocm-opencl-runtime / rocm are real apt packages once the repo is added.

#[tauri::command]
pub fn install_rocm(mode: String) -> u64 {
    let (pkg, label) = if mode == "opencl" {
        ("rocm-opencl-runtime", "ROCm OpenCL")
    } else {
        ("rocm", "ROCm Full Suite")
    };
    let script = format!(
        r#"set -e
echo "=== AMD ROCm Installation ({label}) ==="
echo "[1/5] Removing any leftover ROCm repository..."
rm -f /etc/apt/sources.list.d/rocm.list
rm -f /etc/apt/keyrings/rocm.gpg
rm -f /etc/apt/preferences.d/rocm-pin-600

echo "[2/5] Setting up AMD ROCm repository..."
mkdir -p /etc/apt/keyrings
curl -fsSL https://repo.radeon.com/rocm/rocm.gpg.key | gpg --dearmor -o /etc/apt/keyrings/rocm.gpg
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/rocm.gpg] https://repo.radeon.com/rocm/apt/latest noble main" \
    > /etc/apt/sources.list.d/rocm.list

cat > /etc/apt/preferences.d/rocm-pin-600 <<'PINEOF'
Package: *
Pin: release o=repo.radeon.com
Pin-Priority: 600
PINEOF

echo "[3/5] Updating package lists..."
apt update

echo "[4/5] Installing {label}..."
apt install -y {pkg}

echo "[5/5] Adding user to render and video groups..."
REAL_USER=$(getent passwd $PKEXEC_UID | cut -d: -f1)
usermod -aG render,video "$REAL_USER"

echo "=== Installation completed successfully ==="
echo "ROCm installed. Please restart your session to apply group membership."
"#,
        label = label,
        pkg = pkg
    );
    start_pkexec_job(script)
}

#[tauri::command]
pub fn uninstall_rocm() -> u64 {
    let script = r#"set -e
echo "=== Removing ROCm ==="
apt purge -y 'rocm*' 'hip*' 'hsa*' 'comgr*' 'rocblas*' 'rocsolver*' 2>/dev/null || true
rm -f /etc/apt/sources.list.d/rocm.list
rm -f /etc/apt/keyrings/rocm.gpg
rm -f /etc/apt/preferences.d/rocm-pin-600
apt update -q
echo "[+] ROCm removed successfully."
"#
    .to_string();
    start_pkexec_job(script)
}

// === VirtualBox Guest Additions ===
// Ported from drivers_tab.py's _on_vbox_clicked / _on_vbox_uninstall_clicked.
// Installed via the official .run installer (bundled as a Tauri resource),
// never as an apt package, so status and uninstall both work differently
// from every other driver here.

#[tauri::command]
pub fn is_vbox_installed() -> bool {
    Command::new("systemctl")
        .args(["is-enabled", "vboxadd"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn install_vbox_guest(app: tauri::AppHandle) -> Result<u64, String> {
    use tauri::Manager;
    let vbox_run = app
        .path()
        .resolve(
            "resources/vbox/VBoxLinuxAdditions.run",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| e.to_string())?;
    let vbox_run = vbox_run.to_string_lossy().into_owned();
    let llvm = LLVM_DKMS_TOOLCHAIN;

    let script = format!(
        r#"set -e
echo "Installing VirtualBox Guest Additions..."
apt update
apt install -y build-essential dkms linux-headers-$(uname -r)
{llvm}
# Run installer (non-zero exit expected when not running inside VirtualBox)
"{vbox_run}" || true

echo "=== Installation completed ==="
echo "VirtualBox Guest Additions installed."
echo "IMPORTANT: Restart the system to apply the changes."
"#,
        llvm = llvm,
        vbox_run = vbox_run
    );
    Ok(start_pkexec_job(script))
}


#[tauri::command]
pub fn uninstall_vbox_guest() -> u64 {
    let script = r#"
UNINSTALLER=$(ls -d /opt/VBoxGuestAdditions-*/uninstall.sh 2>/dev/null | head -1)
if [ -n "$UNINSTALLER" ] && [ -x "$UNINSTALLER" ]; then
    echo "Running official VirtualBox Guest Additions uninstaller..."
    "$UNINSTALLER" || true
elif command -v rcvboxadd >/dev/null 2>&1; then
    echo "Running rcvboxadd cleanup..."
    rcvboxadd cleanup || true
else
    echo "No official VirtualBox Guest Additions uninstaller found on disk."
fi

apt purge -y virtualbox-guest-utils virtualbox-guest-x11 virtualbox-guest-dkms 2>/dev/null || true

echo "VirtualBox Guest Additions removal completed."
"#
    .to_string();
    start_pkexec_job(script)
}

// ── NVIDIA PRIME ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimeStatus {
    /// Whether the nvidia-prime package is installed.
    pub installed: bool,
    /// Current active profile: "nvidia", "intel", "amd" or "on-demand".
    /// Empty string when nvidia-prime is not installed.
    pub active_profile: String,
}

#[tauri::command]
pub fn get_prime_status() -> PrimeStatus {
    let installed = is_package_installed("nvidia-prime");
    if !installed {
        return PrimeStatus { installed: false, active_profile: String::new() };
    }
    let active_profile = Command::new("prime-select")
        .arg("query")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_lowercase())
        .unwrap_or_default();
    PrimeStatus { installed, active_profile }
}

/// Switches the active PRIME profile or configures hybrid graphics modes. `profile` must be one of:
/// "nvidia", "intel", "amd", "on-demand", "offload".
#[tauri::command]
pub fn set_prime_profile(profile: String) -> u64 {
    let mode = if profile == "on-demand" { "offload" } else { profile.as_str() };
    match mode {
        "offload" => {
            let script = r#"echo "=== Configuring PRIME Render Offload ==="
if command -v prime-select &>/dev/null; then
    prime-select on-demand 2>/dev/null || true
fi

rm -f /etc/X11/xorg.conf.d/10-nvidia-prime.conf
rm -f /etc/X11/xorg.conf
rm -f /etc/environment.d/10-nvidia-primary.conf
rm -f /etc/udev/rules.d/61-nvidia-prime.rules
rm -f /etc/udev/rules.d/61-gdm-nvidia.rules

cat > /usr/local/bin/prime-run << 'PRIMERUN'
#!/bin/bash
export __NV_PRIME_RENDER_OFFLOAD=1
export __NV_PRIME_RENDER_OFFLOAD_PROVIDER=NVIDIA-G0
export __GLX_VENDOR_LIBRARY_NAME=nvidia
export __VK_LAYER_NV_optimus=NVIDIA_only
export __EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/10_nvidia.json
export GBM_BACKEND=nvidia-drm
exec "$@"
PRIMERUN
chmod +x /usr/local/bin/prime-run

mkdir -p /usr/share/applications
cat > /usr/share/applications/prime-run.desktop << 'DESKTOP'
[Desktop Entry]
Name=Run with NVIDIA GPU
Comment=Run application using the NVIDIA GPU
Exec=prime-run %f
Icon=nvidia
Terminal=false
Type=Application
NoDisplay=true
DESKTOP

echo "PRIME Render Offload configured."
echo "Use 'prime-run <command>' to execute apps with NVIDIA."
"#.to_string();
            start_pkexec_job(script)
        }
        "nvidia" => {
            let script = r#"echo "=== Configuring NVIDIA as Primary GPU ==="
if command -v prime-select &>/dev/null; then
    prime-select nvidia 2>/dev/null || true
fi

if ! grep -q "nvidia-drm.modeset=1" /etc/default/grub 2>/dev/null; then
    echo "Adding nvidia-drm.modeset=1 to GRUB..."
    sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\([^"]*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 nvidia-drm.modeset=1"/' /etc/default/grub
    update-grub 2>/dev/null || true
fi

mkdir -p /etc/environment.d
cat > /etc/environment.d/10-nvidia-primary.conf << 'ENVCONF'
__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/10_nvidia.json
__GLX_VENDOR_LIBRARY_NAME=nvidia
ENVCONF

mkdir -p /etc/X11/xorg.conf.d
cat > /etc/X11/xorg.conf.d/10-nvidia-prime.conf << 'XORGCONF'
Section "OutputClass"
    Identifier "nvidia"
    MatchDriver "nvidia-drm"
    Driver "nvidia"
    Option "AllowEmptyInitialConfiguration"
    Option "PrimaryGPU" "yes"
    ModulePath "/usr/lib/x86_64-linux-gnu/nvidia/xorg"
EndSection
XORGCONF

echo "NVIDIA configured as Primary GPU."
echo "IMPORTANT: Restart the system to apply changes."
"#.to_string();
            start_pkexec_job(script)
        }
        "intel" | "amd" => {
            let script = format!(
                r#"echo "=== Configuring Integrated GPU Mode ({target}) ==="
if command -v prime-select &>/dev/null; then
    prime-select {target} 2>/dev/null || true
fi

rm -f /etc/X11/xorg.conf.d/10-nvidia-prime.conf
rm -f /etc/environment.d/10-nvidia-primary.conf

echo "Integrated GPU mode ({target}) configured."
echo "IMPORTANT: Restart the system to apply changes."
"#,
                target = mode
            );
            start_pkexec_job(script)
        }
        _ => 0,
    }
}

/// Installs the nvidia-prime package so prime-select becomes available.
#[tauri::command]
pub fn install_nvidia_prime() -> u64 {
    start_pkexec_job(
        "apt-get install -y nvidia-prime\necho \"nvidia-prime installed.\"".to_string(),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaUpdateStatus {
    pub installed_version: String,
    pub latest_version: String,
    pub has_update: bool,
}

/// Semantic version comparison for NVIDIA version strings like "580.100.00".
/// Returns true if `a` is strictly greater than `b`.
fn nvidia_ver_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect()
    };
    let av = parse(a);
    let bv = parse(b);
    let len = av.len().max(bv.len());
    for i in 0..len {
        let ai = av.get(i).copied().unwrap_or(0);
        let bi = bv.get(i).copied().unwrap_or(0);
        if ai != bi {
            return ai > bi;
        }
    }
    false
}

/// Checks online if a newer NVIDIA driver release exists.
/// Returns an error string on network/parse failure so the frontend can show a
/// proper message instead of silently showing nothing.
#[tauri::command]
pub async fn check_nvidia_update(branch: String) -> Result<NvidiaUpdateStatus, String> {
    // Detect installed version via nvidia-smi.
    let installed = Command::new("nvidia-smi")
        .arg("--query-gpu=driver_version")
        .arg("--format=csv,noheader")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if installed.is_empty() {
        return Ok(NvidiaUpdateStatus {
            installed_version: String::new(),
            latest_version: String::new(),
            has_update: false,
        });
    }

    // Determine major version to select the right repo.
    let major = installed.split('.').next().unwrap_or("").to_string();
    let target_prefix = if branch.is_empty() { major.clone() } else { branch.clone() };

    // For 580 use debian12 repo; for 590/610 use debian13.
    let repo = if target_prefix == "580" {
        "debian12"
    } else {
        "debian13"
    };
    let url = format!(
        "https://developer.download.nvidia.com/compute/cuda/repos/{repo}/x86_64/Packages"
    );

    // Fetch in a blocking thread so the async Tauri runtime is not stalled.
    // The debian12 (580 branch) Packages index alone is ~8.8 MB, debian13
    // ~1.9 MB — a 3 MB cap silently failed every debian12 check: ureq's
    // limit() doesn't truncate, it returns Err(BodyExceedsLimit) once the
    // body exceeds it (confirmed in ureq's own test suite), which the
    // .ok() below turned into an empty body and a generic "network error"
    // that had nothing to do with actual connectivity. 16 MB leaves
    // headroom for the index to keep growing.
    let body = tauri::async_runtime::spawn_blocking(move || -> Option<String> {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(20)))
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let mut response = agent.get(&url).call().ok()?;
        response
            .body_mut()
            .with_config()
            .limit(16 * 1024 * 1024)
            .read_to_string()
            .ok()
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default();

    if body.is_empty() {
        return Err("Network error: could not fetch NVIDIA package index".to_string());
    }

    let mut latest = installed.clone();
    for line in body.lines() {
        if line.starts_with("Version:") {
            let ver = line.trim_start_matches("Version:").trim();
            // Strip Debian package epoch/revision: "2:580.100.00-1" → "580.100.00"
            let ver = ver.split(':').last().unwrap_or(ver);
            let clean_ver = ver.split('-').next().unwrap_or(ver).trim();
            if clean_ver.starts_with(&target_prefix) && nvidia_ver_gt(clean_ver, &latest) {
                latest = clean_ver.to_string();
            }
        }
    }

    let has_update = latest != installed;
    Ok(NvidiaUpdateStatus {
        installed_version: installed,
        latest_version: latest,
        has_update,
    })
}



