use std::process::Command;

use serde::Serialize;

use super::drivers::{detect_gpus_from_lspci, get_lspci_output, GpuInfo};

fn get_lsusb_output() -> String {
    Command::new("lsusb")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    model: String,
    cores: u32,
    threads: u32,
}

fn detect_cpu() -> CpuInfo {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let model = cpuinfo
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let threads = cpuinfo.lines().filter(|l| l.starts_with("processor")).count() as u32;
    let mut core_ids = std::collections::HashSet::new();
    for line in cpuinfo.lines() {
        if let Some(rest) = line.strip_prefix("core id") {
            if let Some(id) = rest.split(':').nth(1) {
                core_ids.insert(id.trim().to_string());
            }
        }
    }
    let cores = if !core_ids.is_empty() { core_ids.len() as u32 } else { threads };
    CpuInfo { model, cores, threads }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    total: String,
    available: String,
}

fn detect_memory() -> MemoryInfo {
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let read_gb = |key: &str| -> u64 {
        meminfo
            .lines()
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .map(|kb| kb / 1024 / 1024)
            .unwrap_or(0)
    };
    MemoryInfo {
        total: format!("{} GB", read_gb("MemTotal:")),
        available: format!("{} GB", read_gb("MemAvailable:")),
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDevice {
    name: String,
    size: String,
}

fn detect_storage() -> Vec<StorageDevice> {
    let output = Command::new("lsblk").args(["-d", "-o", "NAME,SIZE,TYPE"]).output();
    let Ok(output) = output else { return Vec::new() };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == "disk" {
                Some(StorageDevice { name: parts[0].to_string(), size: parts[1].to_string() })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    name: String,
    iface_type: String,
    status: String,
}

fn detect_network() -> Vec<NetworkInterface> {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "lo" {
                return None;
            }
            let iface_type = if name.starts_with("wl") {
                "Wi-Fi"
            } else if name.starts_with("en") || name.starts_with("eth") {
                "Ethernet"
            } else {
                "Other"
            };
            let state = std::fs::read_to_string(format!("/sys/class/net/{name}/operstate"))
                .unwrap_or_default()
                .trim()
                .to_string();
            let status = if state == "up" { "Connected" } else { "Disconnected" };
            Some(NetworkInterface { name, iface_type: iface_type.to_string(), status: status.to_string() })
        })
        .collect()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HybridGpuInfo {
    is_hybrid: bool,
    integrated: Option<String>,
    dedicated: Option<String>,
}

fn detect_hybrid_gpu(gpus: &[GpuInfo]) -> HybridGpuInfo {
    let nvidia = gpus.iter().find(|g| g.vendor == "NVIDIA");
    // A discrete AMD card (RX/Vega/VII/Radeon Pro) is not an "integrated" GPU
    // even though the AMD branch covers both — ported from
    // hardware_detector.py's detect_hybrid_gpu, which excludes these same
    // model markers so a dual-dGPU machine isn't mislabeled as hybrid.
    let is_discrete_amd = |model: &str| {
        let lower = model.to_lowercase();
        ["rx ", "vega", "vii", "radeon pro"].iter().any(|m| lower.contains(m))
    };
    let integrated = gpus
        .iter()
        .find(|g| g.vendor == "Intel" || (g.vendor == "AMD" && !is_discrete_amd(&g.model)));
    match (nvidia, integrated) {
        (Some(n), Some(i)) => HybridGpuInfo {
            is_hybrid: true,
            integrated: Some(format!("{} {}", i.vendor, i.model)),
            dedicated: Some(format!("{} {}", n.vendor, n.model)),
        },
        _ => HybridGpuInfo { is_hybrid: false, integrated: None, dedicated: None },
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiAdapterInfo {
    model: String,
    vendor: String,
    firmware_package: Option<String>,
    driver_status: String,
}

fn identify_wifi_vendor(line_lower: &str) -> (Option<&'static str>, Option<&'static str>) {
    if line_lower.contains("intel") {
        (Some("Intel"), Some("firmware-iwlwifi"))
    } else if line_lower.contains("realtek") {
        (Some("Realtek"), Some("firmware-realtek"))
    } else if line_lower.contains("broadcom") || line_lower.contains("bcm") {
        (Some("Broadcom"), Some("firmware-b43-installer"))
    } else if line_lower.contains("atheros") || line_lower.contains("qualcomm") {
        (Some("Atheros/Qualcomm"), Some("firmware-atheros"))
    } else if line_lower.contains("ralink") || line_lower.contains("mediatek") {
        // firmware-ralink has no candidate in Debian testing/forky — MediaTek
        // bought Ralink and firmware-mediatek absorbed its blobs (rt2561.bin,
        // rt2860.bin, rt2870.bin, rt3070.bin, etc.), confirmed via `apt-cache
        // policy` and the real .deb's contents. Same fix already applied in
        // 2.x.
        (Some("Ralink/MediaTek"), Some("firmware-mediatek"))
    } else if line_lower.contains("marvell") {
        (Some("Marvell"), Some("firmware-libertas"))
    } else {
        (None, None)
    }
}

fn detect_wifi(lspci: &str, lsusb: &str, installed: &[String]) -> Vec<WifiAdapterInfo> {
    let keywords = ["wireless", "wifi", "802.11", "wlan", "wi-fi"];
    let mut seen = std::collections::HashSet::new();
    let mut adapters = Vec::new();

    for line in lspci.lines().chain(lsusb.lines()) {
        let lower = line.to_lowercase();
        if !keywords.iter().any(|k| lower.contains(k)) {
            continue;
        }
        let (vendor, firmware) = identify_wifi_vendor(&lower);
        let vendor = vendor.unwrap_or("Unknown");
        if !seen.insert(vendor.to_string()) {
            continue;
        }
        let driver_status = match firmware {
            Some(pkg) if !is_installed(installed, pkg) => "missing",
            _ => "installed",
        };
        let model: String = line.rsplit(':').next().unwrap_or(line).trim().chars().take(60).collect();
        adapters.push(WifiAdapterInfo {
            model,
            vendor: vendor.to_string(),
            firmware_package: firmware.map(|s| s.to_string()),
            driver_status: driver_status.to_string(),
        });
    }
    adapters
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleDeviceInfo {
    model: String,
    driver_status: String,
    missing_packages: Vec<String>,
    required_packages: Vec<String>,
}

fn detect_audio(lspci: &str, installed: &[String]) -> Vec<SimpleDeviceInfo> {
    let keywords = ["audio", "sound", "multimedia audio", "high definition audio", "ac97"];
    let has_alsa = is_installed(installed, "alsa-utils");
    let has_server =
        is_installed(installed, "pulseaudio") || is_installed(installed, "pipewire-pulse") || is_installed(installed, "pipewire");
    let required_packages = vec!["alsa-utils".to_string(), "pulseaudio".to_string()];

    lspci
        .lines()
        .filter(|line| keywords.iter().any(|k| line.to_lowercase().contains(k)))
        .map(|line| {
            let model: String = line.rsplit(':').next().unwrap_or(line).trim().chars().take(60).collect();
            let (status, missing) = if has_alsa && has_server {
                ("installed", vec![])
            } else if has_alsa {
                ("partial", vec!["pulseaudio".to_string()])
            } else {
                ("missing", required_packages.clone())
            };
            SimpleDeviceInfo { model, driver_status: status.to_string(), missing_packages: missing, required_packages: required_packages.clone() }
        })
        .collect()
}

fn detect_bluetooth(lsusb: &str, lspci: &str, installed: &[String]) -> Option<SimpleDeviceInfo> {
    let line = lsusb
        .lines()
        .find(|l| l.to_lowercase().contains("bluetooth"))
        .or_else(|| lspci.lines().find(|l| l.to_lowercase().contains("bluetooth")))?;
    let model: String = line.rsplit(':').next().unwrap_or(line).trim().chars().take(50).collect();
    let required = ["bluetooth", "bluez"];
    let missing: Vec<String> = required.iter().filter(|p| !is_installed(installed, p)).map(|s| s.to_string()).collect();
    let status = if missing.is_empty() { "installed" } else if missing.len() == required.len() { "missing" } else { "partial" };
    Some(SimpleDeviceInfo {
        model: if model.is_empty() { "Bluetooth Controller".to_string() } else { model },
        driver_status: status.to_string(),
        missing_packages: missing,
        required_packages: required.iter().map(|s| s.to_string()).collect(),
    })
}

fn detect_printers(lsusb: &str, installed: &[String]) -> Option<SimpleDeviceInfo> {
    let line = lsusb.lines().find(|l| l.to_lowercase().contains("printer"))?;
    let model: String = line.rsplit(':').next().unwrap_or(line).trim().chars().take(50).collect();
    let required = ["cups", "printer-driver-all"];
    let missing: Vec<String> = required.iter().filter(|p| !is_installed(installed, p)).map(|s| s.to_string()).collect();
    let status = if missing.is_empty() { "installed" } else if missing.len() == required.len() { "missing" } else { "partial" };
    Some(SimpleDeviceInfo {
        model: if model.is_empty() { "USB Printer".to_string() } else { model },
        driver_status: status.to_string(),
        missing_packages: missing,
        required_packages: required.iter().map(|s| s.to_string()).collect(),
    })
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmInfo {
    is_vm: bool,
    vm_type: Option<String>,
    driver_status: String,
    missing_packages: Vec<String>,
    required_packages: Vec<String>,
}

// VirtualBox Guest Additions are installed via the official .run installer,
// never as an apt package, so presence is checked via the systemd service it
// registers — same check hardware_detector.py uses in both detect_vm() and
// detect_unnecessary_software().
fn is_vboxadd_enabled() -> bool {
    Command::new("systemctl")
        .args(["is-enabled", "vboxadd"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn detect_vm(installed: &[String]) -> VmInfo {
    let mut vm_raw: Option<String> = None;

    if let Ok(output) = Command::new("systemd-detect-virt").output() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        if output.status.success() && text != "none" && !text.is_empty() {
            vm_raw = Some(text);
        }
    }
    if vm_raw.is_none() {
        for dmi_file in ["/sys/class/dmi/id/sys_vendor", "/sys/class/dmi/id/product_name"] {
            if let Ok(content) = std::fs::read_to_string(dmi_file) {
                let content = content.to_lowercase();
                if content.contains("vmware") {
                    vm_raw = Some("vmware".to_string());
                } else if content.contains("virtualbox") {
                    vm_raw = Some("oracle".to_string());
                } else if content.contains("qemu") || content.contains("kvm") {
                    vm_raw = Some("kvm".to_string());
                }
                if vm_raw.is_some() {
                    break;
                }
            }
        }
    }

    let Some(vm_raw) = vm_raw else {
        return VmInfo { is_vm: false, vm_type: None, driver_status: "installed".to_string(), missing_packages: vec![], required_packages: vec![] };
    };

    if vm_raw.contains("oracle") || vm_raw.contains("virtualbox") {
        let vbox_ok = is_vboxadd_enabled();
        return VmInfo {
            is_vm: true,
            vm_type: Some("VirtualBox".to_string()),
            driver_status: if vbox_ok { "installed".to_string() } else { "missing".to_string() },
            missing_packages: if vbox_ok { vec![] } else { vec!["VirtualBox Guest Additions".to_string()] },
            required_packages: vec![],
        };
    }

    // Any other hypervisor systemd-detect-virt/DMI can report (Xen, Bochs,
    // Parallels, ...) still gets reported as a VM with its real raw name,
    // never a generic placeholder — ported from hardware_detector.py, which
    // titlecases vm_raw itself instead of hiding it.
    let mut display = vm_raw.clone();
    let mut required: Vec<&str> = vec![];
    if vm_raw.contains("vmware") {
        display = "VMware".to_string();
        required = vec!["open-vm-tools", "open-vm-tools-desktop"];
    } else if vm_raw.contains("kvm") || vm_raw.contains("qemu") {
        display = "QEMU/KVM".to_string();
        required = vec!["qemu-guest-agent", "spice-vdagent"];
    } else if vm_raw.contains("microsoft") {
        display = "Hyper-V".to_string();
        required = vec!["hyperv-daemons"];
    } else if let Some(first_char) = vm_raw.get(0..1) {
        display = format!("{}{}", first_char.to_uppercase(), &vm_raw[1..]);
    }

    let (status, missing) = if required.is_empty() {
        ("installed", vec![])
    } else {
        let missing: Vec<String> = required.iter().filter(|p| !is_installed(installed, p)).map(|s| s.to_string()).collect();
        if missing.is_empty() {
            ("installed", missing)
        } else if missing.len() == required.len() {
            ("missing", missing)
        } else {
            ("partial", missing)
        }
    };
    VmInfo {
        is_vm: true,
        vm_type: Some(display.to_string()),
        driver_status: status.to_string(),
        missing_packages: missing,
        required_packages: required.iter().map(|s| s.to_string()).collect(),
    }
}

// One thing a Mac can have installed: "mbpfan", "tlp" or "facetimehd".
// `removable` is whether a package of the system holds it, which is what
// apt can remove.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacItem {
    id: String,
    installed: bool,
    removable: bool,
}

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MacInfo {
    is_mac: bool,
    model: String,
    items: Vec<MacItem>,
}

fn read_dmi(name: &str) -> String {
    std::fs::read_to_string(format!("/sys/class/dmi/id/{name}")).map(|s| s.trim().to_string()).unwrap_or_default()
}

// The FaceTime HD camera is the Broadcom 1570 PCIe device (14e4:1570).
fn facetime_camera_present() -> bool {
    let Ok(devices) = std::fs::read_dir("/sys/bus/pci/devices") else { return false };
    devices.flatten().any(|device| {
        let read = |file: &str| std::fs::read_to_string(device.path().join(file)).unwrap_or_default().trim().to_lowercase();
        read("vendor") == "0x14e4" && read("device") == "0x1570"
    })
}

// An Apple computer is told by the vendor in its DMI data. The fan daemon and
// the battery manager are offered on MacBooks only; the camera driver on any
// Mac that has the camera.
fn detect_mac(installed: &[String]) -> MacInfo {
    let vendor = read_dmi("sys_vendor").to_lowercase();
    if !vendor.starts_with("apple") {
        return MacInfo::default();
    }
    let model = read_dmi("product_name");
    let mut items = Vec::new();
    if model.starts_with("MacBook") {
        for (id, package) in [("mbpfan", "mbpfan"), ("tlp", "tlp")] {
            let installed = is_installed(installed, package);
            items.push(MacItem { id: id.to_string(), installed, removable: installed });
        }
    }
    if facetime_camera_present() {
        let packaged = is_installed(installed, "facetimehd-dkms");
        let loaded = std::path::Path::new("/sys/module/facetimehd").exists();
        items.push(MacItem { id: "facetimehd".to_string(), installed: packaged || loaded, removable: packaged });
    }
    MacInfo { is_mac: true, model, items }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnnecessaryFinding {
    name: String,
    detail: String,
    packages: Vec<String>,
    // "nvidia" needs the dedicated uninstall_nvidia flow (DKMS/GRUB/initramfs
    // cleanup), everything else is a plain apt purge of `packages`.
    action: String,
}

// One `dpkg -l` dump, shared by every check in this file that used to spawn
// its own `dpkg-query`/`dpkg -l` process per package name — this is what
// made scan_hardware() launch 15-30+ subprocesses in a row on every visit to
// the Drivers/Gaming tabs, all to answer questions this single listing
// already covers.
fn installed_package_names() -> Vec<String> {
    let Ok(output) = Command::new("dpkg").arg("-l").output() else { return Vec::new() };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| l.starts_with("ii"))
        .filter_map(|l| l.split_whitespace().nth(1))
        .map(|s| s.to_string())
        .collect()
}

fn is_installed(installed: &[String], pkg: &str) -> bool {
    installed.iter().any(|p| p.eq_ignore_ascii_case(pkg))
}

fn installed_matching(installed: &[String], pattern_prefixes: &[&str]) -> Vec<String> {
    installed
        .iter()
        .filter(|pkg| {
            let lower = pkg.to_lowercase();
            pattern_prefixes.iter().any(|p| lower.starts_with(p))
        })
        .cloned()
        .collect()
}

fn installed_from_list(installed: &[String], names: &[&str]) -> Vec<String> {
    names.iter().filter(|n| is_installed(installed, n)).map(|s| s.to_string()).collect()
}

// Ported from hardware_detector.py's detect_unnecessary_software: installed
// software that doesn't match hardware actually present. Deliberately skips
// printers/Bluetooth/generic Intel packages — those are commonly kept
// installed without hardware currently attached, so flagging them would be a
// false positive.
fn detect_unnecessary_software(gpus: &[GpuInfo], wifi: &[WifiAdapterInfo], vm: &VmInfo, installed: &[String]) -> Vec<UnnecessaryFinding> {
    let mut findings = Vec::new();
    let gpu_vendors: std::collections::HashSet<&str> = gpus.iter().map(|g| g.vendor.as_str()).collect();
    let wifi_vendors: std::collections::HashSet<&str> = wifi.iter().map(|w| w.vendor.as_str()).collect();

    if !gpu_vendors.contains("NVIDIA") {
        let pkgs = installed_matching(installed, &["nvidia", "libnvidia", "cuda"]);
        if !pkgs.is_empty() {
            findings.push(UnnecessaryFinding {
                name: "NVIDIA / CUDA packages".to_string(),
                detail: format!("{} package(s) installed, no NVIDIA GPU detected", pkgs.len()),
                packages: pkgs,
                action: "nvidia".to_string(),
            });
        }
    }

    if !gpu_vendors.contains("AMD") {
        let mut pkgs = installed_from_list(
            installed,
            &["xserver-xorg-video-amdgpu", "xserver-xorg-video-ati", "xserver-xorg-video-radeon", "radeontop"],
        );
        pkgs.extend(installed_matching(installed, &["rocm"]));
        if !pkgs.is_empty() {
            findings.push(UnnecessaryFinding {
                name: "AMD GPU / ROCm packages".to_string(),
                detail: format!("{} package(s) installed, no AMD GPU detected", pkgs.len()),
                packages: pkgs,
                action: "generic".to_string(),
            });
        }
    }

    if !wifi_vendors.contains("Broadcom") {
        let pkgs = installed_from_list(installed, &["broadcom-sta-dkms", "bcmwl-kernel-source", "firmware-b43-installer"]);
        if !pkgs.is_empty() {
            findings.push(UnnecessaryFinding {
                name: "Broadcom Wi-Fi driver".to_string(),
                detail: format!("{} package(s) installed, no Broadcom Wi-Fi adapter detected", pkgs.len()),
                packages: pkgs,
                action: "generic".to_string(),
            });
        }
    }

    if !vm.is_vm {
        let pkgs = installed_from_list(installed, &[
            "open-vm-tools", "open-vm-tools-desktop", "qemu-guest-agent", "spice-vdagent", "spice-webdavd", "hyperv-daemons",
        ]);
        if !pkgs.is_empty() {
            findings.push(UnnecessaryFinding {
                name: "Virtual machine guest tools".to_string(),
                detail: format!("{} package(s) installed, but this machine is not a virtual machine", pkgs.len()),
                packages: pkgs,
                action: "generic".to_string(),
            });
        }
    }

    // Checked regardless of vm.is_vm — it used to only fire when this
    // machine wasn't a VM at all, which meant it never fired on ANY VM
    // (Proxmox, VMware, VirtualBox itself), since is_vm is true there too.
    // Ported from hardware_detector.py's fix for the same bug.
    if is_vboxadd_enabled() {
        let detail = if vm.is_vm {
            "Active — uninstall it if you no longer need shared clipboard/folders with the host".to_string()
        } else {
            "Active, but this machine is not running inside a virtual machine".to_string()
        };
        findings.push(UnnecessaryFinding {
            name: "VirtualBox Guest Additions".to_string(),
            detail,
            packages: vec![],
            action: "vbox".to_string(),
        });
    }

    findings
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResults {
    cpu: CpuInfo,
    memory: MemoryInfo,
    storage: Vec<StorageDevice>,
    network: Vec<NetworkInterface>,
    gpus: Vec<GpuInfo>,
    hybrid_gpu: HybridGpuInfo,
    wifi: Vec<WifiAdapterInfo>,
    audio: Vec<SimpleDeviceInfo>,
    bluetooth: Option<SimpleDeviceInfo>,
    printers: Option<SimpleDeviceInfo>,
    vm_detection: VmInfo,
    mac: MacInfo,
    unnecessary_software: Vec<UnnecessaryFinding>,
}

// Used to spawn 15-30+ subprocesses one after another (every check below
// used to run its own `lspci`/`lsusb`/`dpkg-query`), which is what made the
// Drivers and Gaming tabs slow on every single visit — nothing here was
// cached, so the whole chain re-ran from scratch each time. Fixed in two
// passes: first the three raw data sources everything else reads from
// (lspci, lsusb, one shared `dpkg -l` dump instead of ~15 separate package
// lookups), then every independent detector that only needs that raw data,
// all run concurrently via thread::scope instead of one after another.
#[tauri::command]
pub fn scan_hardware() -> ScanResults {
    let (lspci, lsusb, installed) = std::thread::scope(|s| {
        let lspci = s.spawn(get_lspci_output);
        let lsusb = s.spawn(get_lsusb_output);
        let installed = s.spawn(installed_package_names);
        (lspci.join().unwrap_or_default(), lsusb.join().unwrap_or_default(), installed.join().unwrap_or_default())
    });

    let (cpu, memory, storage, network, vm_detection, gpus, wifi, audio, bluetooth, printers) = std::thread::scope(|s| {
        let cpu = s.spawn(detect_cpu);
        let memory = s.spawn(detect_memory);
        let storage = s.spawn(detect_storage);
        let network = s.spawn(detect_network);
        let vm_detection = s.spawn(|| detect_vm(&installed));
        let gpus = s.spawn(|| detect_gpus_from_lspci(&lspci));
        let wifi = s.spawn(|| detect_wifi(&lspci, &lsusb, &installed));
        let audio = s.spawn(|| detect_audio(&lspci, &installed));
        let bluetooth = s.spawn(|| detect_bluetooth(&lsusb, &lspci, &installed));
        let printers = s.spawn(|| detect_printers(&lsusb, &installed));
        (
            cpu.join().unwrap_or_else(|_| detect_cpu()),
            memory.join().unwrap_or_else(|_| detect_memory()),
            storage.join().unwrap_or_default(),
            network.join().unwrap_or_default(),
            vm_detection.join().unwrap_or(VmInfo {
                is_vm: false,
                vm_type: None,
                driver_status: "installed".to_string(),
                missing_packages: vec![],
                required_packages: vec![],
            }),
            gpus.join().unwrap_or_default(),
            wifi.join().unwrap_or_default(),
            audio.join().unwrap_or_default(),
            bluetooth.join().unwrap_or_default(),
            printers.join().unwrap_or_default(),
        )
    });

    // No new subprocesses below — both just filter the data already fetched
    // above.
    let hybrid_gpu = detect_hybrid_gpu(&gpus);
    let unnecessary_software = detect_unnecessary_software(&gpus, &wifi, &vm_detection, &installed);
    let mac = detect_mac(&installed);

    ScanResults { cpu, memory, storage, network, gpus, hybrid_gpu, wifi, audio, bluetooth, printers, vm_detection, mac, unnecessary_software }
}
