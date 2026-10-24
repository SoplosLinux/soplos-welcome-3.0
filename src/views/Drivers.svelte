<script lang="ts">
  import Modal from "../components/Modal.svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { runTracked, terminalExpanded, terminalLines } from "../lib/progress";
  import TerminalOutput from "../components/TerminalOutput.svelte";
  import MacCard from "../components/MacCard.svelte";
  import { _ } from "svelte-i18n";
  import { CircuitBoard, ChevronDown, AlertTriangle, Cpu, MemoryStick, HardDrive, Network } from "lucide-svelte";

  interface GpuInfo {
    vendor: string;
    model: string;
    gpuType: string;
    recommendedDriver: string | null;
    driverStatus: string;
    installedDriverVersion: string | null;
  }

  interface SimpleDeviceInfo {
    model: string;
    driverStatus: string;
    missingPackages: string[];
    requiredPackages: string[];
  }

  interface UnnecessaryFinding {
    name: string;
    detail: string;
    packages: string[];
    action: "nvidia" | "generic" | "vbox";
  }

  interface ScanResults {
    cpu: { model: string; cores: number; threads: number };
    memory: { total: string; available: string };
    storage: { name: string; size: string }[];
    network: { name: string; ifaceType: string; status: string }[];
    gpus: GpuInfo[];
    hybridGpu: { isHybrid: boolean; integrated: string | null; dedicated: string | null };
    wifi: { model: string; vendor: string; firmwarePackage: string | null; driverStatus: string }[];
    audio: SimpleDeviceInfo[];
    bluetooth: SimpleDeviceInfo | null;
    printers: SimpleDeviceInfo | null;
    vmDetection: { isVm: boolean; vmType: string | null; driverStatus: string; missingPackages: string[]; requiredPackages: string[] };
    mac: { isMac: boolean; model: string; items: { id: string; installed: boolean; removable: boolean }[] };
    unnecessarySoftware: UnnecessaryFinding[];
  }

  let scanning = false;
  let scanned = false;
  let scan: ScanResults | null = null;
  let showScanResults = false;
  $: gpus = scan?.gpus ?? [];
  $: nvidiaGpu = gpus.find((g) => g.vendor === "NVIDIA") ?? null;

  async function scanHardware() {
    scanning = true;
    try {
      scan = await api.invoke<ScanResults>("scan_hardware");
      scanned = true;
    } finally {
      scanning = false;
    }
  }

  // Only the button click opens the results popup — not the initial
  // auto-scan on mount, and not the re-scans after an install/uninstall.
  async function onScanButtonClick() {
    await scanHardware();
    showScanResults = true;
  }

  onMount(scanHardware);

  // Closes the results popup and re-scans, matching the 2.x flow: every
  // action closes the dialog, runs, then the button can be pressed again.
  async function runAndRescan(command: string, args?: Record<string, unknown>) {
    showScanResults = false;
    await runTracked(command, args);
    await scanHardware();
  }

  // What the scan offers on a Mac: the fan daemon, TLP and the camera driver.
  async function macInstall(id: string) {
    if (id === "mbpfan") await runAndRescan("install_apt_packages", { packages: "mbpfan" });
    else if (id === "tlp") await runAndRescan("install_tlp");
    else if (id === "facetimehd") await runAndRescan("install_facetimehd");
  }

  async function macRemove(id: string) {
    if (id === "mbpfan") await runAndRescan("uninstall_apt_packages", { packages: "mbpfan" });
    else if (id === "tlp") await runAndRescan("uninstall_apt_packages", { packages: "tlp tlp-rdw" });
    else if (id === "facetimehd") await runAndRescan("uninstall_facetimehd");
  }

  async function installGpuRecommended(gpu: GpuInfo) {
    const rec = gpu.recommendedDriver;
    if (!rec) return;
    if (gpu.vendor === "NVIDIA") {
      const cudaMatch = rec.match(/^nvidia-driver-(610|590|580)$/);
      if (cudaMatch) {
        await runAndRescan("install_nvidia_cuda", { version: cudaMatch[1] });
      } else if (rec === "nouveau") {
        // Nothing to install: nouveau ships with the kernel, only the
        // NVIDIA blacklist stands in the way.
        await runAndRescan("uninstall_nvidia");
      } else {
        await runAndRescan("install_nvidia_repo", { package: rec });
      }
    } else {
      // AMD/Intel: recommendedDriver is already a space-joined package list.
      await runAndRescan("install_apt_packages", { packages: rec });
    }
  }

  function removeUnnecessary(finding: UnnecessaryFinding) {
    if (finding.action === "nvidia") {
      return runAndRescan("uninstall_nvidia");
    }
    if (finding.action === "vbox") {
      return runAndRescan("uninstall_vbox_guest");
    }
    return runAndRescan("uninstall_apt_packages", { packages: finding.packages.join(" ") });
  }

  async function removeAllUnnecessary() {
    if (!scan) return;
    showScanResults = false;
    for (const finding of scan.unnecessarySoftware) {
      if (finding.action === "nvidia") {
        await runTracked("uninstall_nvidia");
      } else if (finding.action === "vbox") {
        await runTracked("uninstall_vbox_guest");
      } else {
        await runTracked("uninstall_apt_packages", { packages: finding.packages.join(" ") });
      }
    }
    await scanHardware();
  }

  // Status line for the scan results popup — mirrors the 2.x dialog's
  // colored ✓ / ⚠ / ✗ lines (green installed, orange different-version or
  // partial, red missing).
  //
  // These return i18n KEYS, not translated text: $_() is called directly at
  // each template call site instead, because a $_() call hidden inside a
  // plain helper function isn't visible to Svelte's per-expression reactive
  // dependency tracking — the text wouldn't update on a locale switch until
  // something else (like `scan`) also happened to change.
  function statusInfo(status: string): { icon: string; key: string; class: string } {
    if (status === "installed") return { icon: "✓", key: "drivers.status.installed", class: "text-success" };
    if (status === "different_version") return { icon: "⚠", key: "drivers.status.differentVersion", class: "text-warning" };
    if (status === "partial") return { icon: "⚠", key: "drivers.status.partial", class: "text-warning" };
    return { icon: "✗", key: "drivers.status.notInstalled", class: "text-danger" };
  }

  function actionButtonInfo(status: string, name: string): { key: string; values: Record<string, string> } {
    if (status === "different_version") return { key: "drivers.actionSwitchTo", values: { name } };
    if (status === "partial") return { key: "drivers.actionCompleteInstallation", values: {} };
    return { key: "drivers.actionInstall", values: { name } };
  }

  // A plain function's result isn't tracked by Svelte unless the reactive
  // variables it reads (`scanned`, `scan`) are also referenced directly in
  // the template — hidden inside a function call, they never trigger a
  // re-render, so this has to be a reactive statement instead.
  $: familySubtitles = {
    amd: (() => {
      if (!scanned) return $_("drivers.detecting");
      if (!scan) return "";
      const gpu = scan.gpus.find((g) => g.vendor === "AMD");
      return gpu ? `${gpu.model} (${gpu.driverStatus})` : $_("drivers.noAmdDetected");
    })(),
    wifi: (() => {
      if (!scanned) return $_("drivers.detecting");
      if (!scan) return "";
      if (scan.wifi.length === 0) return $_("drivers.noWifiDetected");
      return scan.wifi.map((w) => `${w.vendor} (${w.driverStatus})`).join(", ");
    })(),
    vm: (() => {
      if (!scanned) return $_("drivers.detecting");
      if (!scan) return "";
      return scan.vmDetection.isVm
        ? $_("drivers.runningInVm", { values: { type: scan.vmDetection.vmType ?? "", status: scan.vmDetection.driverStatus } })
        : $_("drivers.notInVm");
    })(),
  } as Record<string, string>;

  let expanded: string | null = null;
  function toggleFamily(key: string) {
    expanded = expanded === key ? null : key;
  }

  const nvidiaBranchIds = [
    "610",
    "590",
    "580",
    "nvidia-driver",
    "nvidia-tesla-470-driver",
    "nvidia-legacy-390xx-driver",
    "nvidia-legacy-340xx-driver",
  ] as const;
  const nvidiaBranchKind: Record<(typeof nvidiaBranchIds)[number], "cuda" | "repo"> = {
    "610": "cuda",
    "590": "cuda",
    "580": "cuda",
    "nvidia-driver": "repo",
    "nvidia-tesla-470-driver": "repo",
    "nvidia-legacy-390xx-driver": "repo",
    "nvidia-legacy-340xx-driver": "repo",
  };
  let nvidiaBranches: { id: string; label: string; hint: string; kind: "cuda" | "repo" }[];
  $: nvidiaBranches = nvidiaBranchIds.map((id) => ({
    id,
    label: $_(`drivers.nvidiaBranches.${id}.label`),
    hint: $_(`drivers.nvidiaBranches.${id}.hint`),
    kind: nvidiaBranchKind[id],
  }));

  let confirmBranch: (typeof nvidiaBranches)[number] | null = null;
  let confirmUninstall = false;

  async function runInstall(branch: (typeof nvidiaBranches)[number]) {
    confirmBranch = null;
    if (branch.kind === "cuda") {
      await runTracked("install_nvidia_cuda", { version: branch.id });
    } else {
      await runTracked("install_nvidia_repo", { package: branch.id });
    }
    await scanHardware();
  }

  async function runUninstall() {
    confirmUninstall = false;
    await runTracked("uninstall_nvidia");
    await scanHardware();
  }

  function statusBadge(status: string) {
    if (status === "installed") return { key: "drivers.badge.installed", class: "bg-success/15 text-success" };
    if (status === "different_version") return { key: "drivers.badge.differentVersion", class: "bg-warning/15 text-warning" };
    return { key: "drivers.badge.notInstalled", class: "bg-surface-2 text-text" };
  }

  // Items with `packages` are wired up via the same apt install/uninstall
  // commands the Software tab already uses. `special` items run their own
  // dedicated command instead (wifi-repair, the two ROCm modes, and vbox).
  interface DriverItem {
    label: string;
    hint: string;
    packages?: string;
    special?: "wifi-repair" | "rocm-opencl" | "rocm-full" | "vbox";
  }

  const amdItemsBase: Omit<DriverItem, "hint">[] = [
    { label: "AMD Radeon (Open Source)", packages: "firmware-amd-graphics libgl1-mesa-dri libglx-mesa0 mesa-vulkan-drivers xserver-xorg-video-all" },
    { label: "ROCm OpenCL", packages: "rocm-opencl-runtime", special: "rocm-opencl" },
    { label: "ROCm Full Suite", packages: "rocm", special: "rocm-full" },
  ];
  const amdItemHintKeys = ["amdRadeon", "rocmOpencl", "rocmFull"];
  let amdItems: DriverItem[];
  $: amdItems = amdItemsBase.map((item, i) => ({ ...item, hint: $_(`drivers.items.${amdItemHintKeys[i]}`) }));

  const wifiItemsBase: Omit<DriverItem, "hint">[] = [
    { label: "Intel Wi-Fi", packages: "firmware-iwlwifi" },
    { label: "Realtek Wi-Fi", packages: "firmware-realtek" },
    { label: "Broadcom Wi-Fi", packages: "firmware-b43-installer" },
    { label: "Broadcom Wi-Fi (proprietary)", packages: "broadcom-sta-dkms" },
    { label: "Repair Wi-Fi", special: "wifi-repair" },
    { label: "Bluetooth", packages: "bluetooth bluez bluez-tools blueman" },
  ];
  const wifiItemHintKeys = ["intelWifi", "realtekWifi", "broadcomWifi", "broadcomWifiProprietary", "repairWifi", "bluetoothHint"];
  let wifiItems: DriverItem[];
  $: wifiItems = wifiItemsBase.map((item, i) => ({ ...item, hint: $_(`drivers.items.${wifiItemHintKeys[i]}`) }));

  const vmItemsBase: Omit<DriverItem, "hint">[] = [
    { label: "VMware Tools", packages: "open-vm-tools-desktop" },
    { label: "QEMU/KVM Tools", packages: "qemu-guest-agent spice-vdagent spice-webdavd xserver-xspice" },
    { label: "VirtualBox Guest", special: "vbox" },
  ];
  const vmItemHintKeys = ["vmwareTools", "qemuTools", "vboxGuest"];
  let vmItems: DriverItem[];
  $: vmItems = vmItemsBase.map((item, i) => ({ ...item, hint: $_(`drivers.items.${vmItemHintKeys[i]}`) }));

  let printerItem: DriverItem;
  $: printerItem = {
    label: $_("drivers.printerDriversLabel"),
    hint: $_("drivers.printerDriversHint"),
    packages: "printer-driver-all",
  };

  // NVIDIA Extras — separate from the driver branches above, matches the
  // 2.x "NVIDIA Extras" section.
  const nvidiaExtraItemsBase: Omit<DriverItem, "hint">[] = [
    { label: "DaVinci Resolve Extras", packages: "nvidia-opencl-icd libcuda1 libglu1-mesa libnvidia-encode1" },
    { label: "Blender CUDA Toolkit", packages: "nvidia-cuda-toolkit" },
  ];
  const nvidiaExtraItemHintKeys = ["davinciExtras", "blenderCuda"];
  let nvidiaExtraItems: DriverItem[];
  $: nvidiaExtraItems = nvidiaExtraItemsBase.map((item, i) => ({ ...item, hint: $_(`drivers.items.${nvidiaExtraItemHintKeys[i]}`) }));

  const installedMap: Record<string, boolean> = {};
  let installedVersion = 0; // bumped to force the template to re-read installedMap

  async function refreshItemStatus(item: DriverItem) {
    if (!item.packages) return;
    // Items like Radeon, Bluetooth or DaVinci Extras list several packages —
    // checking only the first one let the badge/toggle say "Installed" while
    // the rest of the group was actually missing. All of them must be
    // present for this to count as installed.
    const packages = item.packages.split(" ").filter(Boolean);
    const results = await Promise.all(packages.map((pkg) => api.invoke<boolean>("is_apt_package_installed", { package: pkg })));
    installedMap[item.label] = results.every(Boolean);
    installedVersion++;
  }

  function refreshFamilyStatus(items: DriverItem[]) {
    for (const item of items) refreshItemStatus(item);
  }

  let oneapiInstalled = false;
  async function refreshOneapiStatus() {
    oneapiInstalled = await api.invoke<boolean>("is_oneapi_installed");
  }

  async function toggleOneapi() {
    await runTracked(oneapiInstalled ? "uninstall_oneapi" : "install_oneapi");
    await refreshOneapiStatus();
  }

  let cuda12Installed = false;
  async function refreshCuda12Status() {
    cuda12Installed = await api.invoke<boolean>("is_cuda12_installed");
  }

  async function toggleCuda12() {
    await runTracked(cuda12Installed ? "uninstall_cuda12" : "install_cuda12");
    await refreshCuda12Status();
  }

  let vboxInstalled = false;
  async function refreshVboxStatus() {
    vboxInstalled = await api.invoke<boolean>("is_vbox_installed");
  }

  interface PrimeStatus {
    installed: boolean;
    activeProfile: string;
  }
  let primeStatus: PrimeStatus | null = null;
  async function refreshPrimeStatus() {
    try {
      primeStatus = await api.invoke<PrimeStatus>("get_prime_status");
    } catch {
      primeStatus = null;
    }
  }

  async function switchPrimeProfile(profile: string) {
    await runTracked("set_prime_profile", { profile });
    await refreshPrimeStatus();
  }

  async function installNvidiaPrime() {
    await runTracked("install_nvidia_prime");
    await refreshPrimeStatus();
  }

  interface NvidiaUpdateStatus {
    installedVersion: string;
    latestVersion: string;
    hasUpdate: boolean;
  }
  let checkingNvidiaUpdate = false;
  let nvidiaUpdateStatus: NvidiaUpdateStatus | null = null;
  let nvidiaUpdateError: string | null = null;
  let nvidiaUpdateResultOpen = false;
  async function runCheckNvidiaUpdate() {
    checkingNvidiaUpdate = true;
    nvidiaUpdateError = null;
    nvidiaUpdateStatus = null;
    try {
      nvidiaUpdateStatus = await api.invoke<NvidiaUpdateStatus>("check_nvidia_update", { branch: "" });
    } catch (e: unknown) {
      nvidiaUpdateError = e instanceof Error ? e.message : String(e);
    } finally {
      checkingNvidiaUpdate = false;
      nvidiaUpdateResultOpen = true;
    }
  }

  onMount(() => {
    refreshFamilyStatus(amdItems);
    refreshFamilyStatus(wifiItems);
    refreshFamilyStatus(vmItems);
    refreshFamilyStatus(nvidiaExtraItems);
    refreshItemStatus(printerItem);
    refreshOneapiStatus();
    refreshCuda12Status();
    refreshVboxStatus();
    refreshPrimeStatus();
  });

  async function toggleItem(item: DriverItem) {
    if (!item.packages) return;
    const installed = installedMap[item.label];
    await runTracked(installed ? "uninstall_apt_packages" : "install_apt_packages", { packages: item.packages });
    await refreshItemStatus(item);
  }

  async function toggleVbox() {
    await runTracked(vboxInstalled ? "uninstall_vbox_guest" : "install_vbox_guest");
    await refreshVboxStatus();
  }

  // ROCm install needs a confirmation dialog (GPU compatibility + download
  // size warning, matches drivers_tab.py's _on_rocm_clicked); uninstall does
  // too, since it purges every rocm/hip/hsa package system-wide.
  let confirmRocm: "opencl" | "full" | null = null;
  let confirmRocmUninstall = false;

  async function runRocmInstall(mode: "opencl" | "full") {
    confirmRocm = null;
    await runTracked("install_rocm", { mode });
    refreshFamilyStatus(amdItems);
  }

  async function runRocmUninstall() {
    confirmRocmUninstall = false;
    await runTracked("uninstall_rocm");
    refreshFamilyStatus(amdItems);
  }

  function handleFamilyItemClick(item: DriverItem) {
    if (item.special === "wifi-repair") return startWifiRepair();
    if (item.special === "rocm-opencl" || item.special === "rocm-full") {
      if (installedMap[item.label]) {
        confirmRocmUninstall = true;
      } else {
        confirmRocm = item.special === "rocm-opencl" ? "opencl" : "full";
      }
      return;
    }
    if (item.special === "vbox") return toggleVbox();
    return toggleItem(item);
  }

  let wifiConfirm: { iface: string | null; driver: string } | null = null;
  let wifiNoHardware = false;

  async function startWifiRepair() {
    const info = await api.invoke<{ iface: string | null; driver: string | null }>("detect_wifi_driver");
    if (!info.driver) {
      wifiConfirm = null;
      wifiNoHardware = true;
      return;
    }
    wifiConfirm = { iface: info.iface, driver: info.driver };
  }

  async function confirmWifiRepair() {
    if (!wifiConfirm) return;
    const { iface, driver } = wifiConfirm;
    wifiConfirm = null;
    await runTracked("repair_wifi", { iface, driver });
  }
</script>

<div
  class="relative rounded border border-border overflow-hidden mb-5 flex flex-col items-center justify-end text-center p-4 min-h-[200px]"
  style="background: linear-gradient(rgba(20, 20, 22, 0.35), rgba(20, 20, 22, 0.85)), url('/assets/hero/drivers-background.png'); background-size: cover; background-position: center;"
>
  <button
    class="flex items-center gap-2 bg-primary hover:bg-primary-hover text-text-bright px-5 py-2.5 rounded font-semibold text-[0.9rem]"
    disabled={scanning}
    on:click={onScanButtonClick}
  >
    <CircuitBoard size={18} />
    {scanning ? $_("drivers.scanning") : $_("drivers.scanHardware")}
  </button>
  {#if scanned && nvidiaGpu}
    <p class="text-[0.78rem] text-white/85 mt-2">
      {$_("drivers.detectedGpu", { values: { model: nvidiaGpu.model, status: $_(statusBadge(nvidiaGpu.driverStatus).key) } })}
    </p>
  {:else if scanned}
    <p class="text-[0.78rem] text-white/85 mt-2">{$_("drivers.noGpuDetected")}</p>
  {/if}
</div>

{#if showScanResults && scan}
  <div class="w-full bg-surface border border-border rounded shadow-lg mb-4 flex flex-col">
    <div class="px-4 py-3 border-b border-border flex items-center justify-between shrink-0">
      <h3 class="text-text-bright font-semibold text-[0.95rem]">{$_("drivers.hardwareDetectionTitle")}</h3>
      <button class="text-text-muted hover:text-text-bright text-[0.8rem]" on:click={() => (showScanResults = false)}>
        {$_("common.close")}
      </button>
    </div>

    <div class="content-scroll overflow-y-auto overscroll-contain max-h-[60vh] p-4 flex flex-col gap-3">
      <!-- CPU -->
      <div class="border border-border rounded p-3">
        <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.processor")}</div>
        <p class="text-[0.82rem] text-text-bright">{scan.cpu.model}</p>
        <p class="text-[0.78rem] text-text">{$_("drivers.coresThreads", { values: { cores: scan.cpu.cores, threads: scan.cpu.threads } })}</p>
      </div>

      <!-- RAM -->
      <div class="border border-border rounded p-3">
        <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.ramMemory")}</div>
        <p class="text-[0.82rem] text-text-bright">{$_("drivers.totalAvailable", { values: { total: scan.memory.total, available: scan.memory.available } })}</p>
      </div>

      <!-- VM -->
      {#if scan.vmDetection.isVm}
        <div class="border border-border rounded p-3">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.virtualMachine")}</div>
          <p class="text-[0.82rem] text-text-bright mb-1">{$_("drivers.vmType", { values: { type: scan.vmDetection.vmType ?? "" } })}</p>
          {#if scan.vmDetection.requiredPackages.length > 0}
            {@const vmPackages = scan.vmDetection.requiredPackages.join(" ")}
            <p class="text-[0.78rem] {statusInfo(scan.vmDetection.driverStatus).class} mb-1.5">
              {statusInfo(scan.vmDetection.driverStatus).icon} {$_(statusInfo(scan.vmDetection.driverStatus).key)}
            </p>
            {#if scan.vmDetection.driverStatus !== "installed"}
              {@const vmAction = actionButtonInfo(scan.vmDetection.driverStatus, $_("drivers.guestTools"))}
              <button
                class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
                on:click={() => runAndRescan("install_apt_packages", { packages: vmPackages })}
              >
                {$_(vmAction.key, { values: vmAction.values })}
              </button>
            {:else}
              <button
                class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold"
                on:click={() => runAndRescan("uninstall_apt_packages", { packages: vmPackages })}
              >
                {$_("common.uninstall")}
              </button>
            {/if}
          {:else}
            <p class="text-[0.78rem] text-text">
              {scan.vmDetection.driverStatus === "installed" ? $_("drivers.guestToolsActive") : $_("drivers.guestToolsMissing")}
            </p>
          {/if}
        </div>
      {/if}

      <!-- GPUs -->
      {#each scan.gpus as gpu}
        <div class="border border-border rounded p-3">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">
            {$_("drivers.graphicsCard", { values: { vendor: gpu.vendor, model: gpu.model } })}
          </div>
          {#if gpu.recommendedDriver}
            <p class="text-[0.82rem] text-text-bright mb-1">{$_("drivers.recommended", { values: { driver: gpu.recommendedDriver } })}</p>
            <p class="text-[0.78rem] {statusInfo(gpu.driverStatus).class} mb-1.5">
              {statusInfo(gpu.driverStatus).icon} {$_(statusInfo(gpu.driverStatus).key)}
            </p>
            <div class="flex gap-2">
              {#if gpu.driverStatus !== "installed"}
                {@const gpuAction = actionButtonInfo(gpu.driverStatus, gpu.recommendedDriver)}
                <button
                  class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
                  on:click={() => installGpuRecommended(gpu)}
                >
                  {$_(gpuAction.key, { values: gpuAction.values })}
                </button>
              {/if}
              {#if gpu.vendor === "NVIDIA" && gpu.driverStatus === "installed"}
                <button
                  class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold"
                  on:click={() => runAndRescan("uninstall_nvidia")}
                >
                  {$_("common.uninstall")}
                </button>
              {/if}
            </div>
          {:else}
            <p class="text-[0.78rem] text-text">{$_("drivers.noProprietaryNeeded")}</p>
          {/if}
        </div>
      {/each}

      <!-- Hybrid graphics (informational only for now) -->
      {#if scan.hybridGpu.isHybrid}
        <div class="border border-border rounded p-3">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.hybridGraphics")}</div>
          <p class="text-[0.82rem] text-text-bright">{$_("drivers.integrated", { values: { name: scan.hybridGpu.integrated ?? "" } })}</p>
          <p class="text-[0.82rem] text-text-bright">{$_("drivers.dedicated", { values: { name: scan.hybridGpu.dedicated ?? "" } })}</p>
        </div>
      {/if}

      <!-- Wi-Fi -->
      {#each scan.wifi as adapter}
        <div class="border border-border rounded p-3">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.wifi")}</div>
          <p class="text-[0.82rem] text-text-bright mb-1">{adapter.vendor} — {adapter.model}</p>
          {#if adapter.firmwarePackage}
            <p class="text-[0.78rem] {statusInfo(adapter.driverStatus).class} mb-1.5">
              {statusInfo(adapter.driverStatus).icon} {$_(statusInfo(adapter.driverStatus).key)}
            </p>
            {#if adapter.driverStatus !== "installed"}
              {@const wifiAction = actionButtonInfo(adapter.driverStatus, adapter.firmwarePackage ?? "")}
              <button
                class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
                on:click={() => runAndRescan("install_apt_packages", { packages: adapter.firmwarePackage })}
              >
                {$_(wifiAction.key, { values: wifiAction.values })}
              </button>
            {:else}
              <button
                class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold"
                on:click={() => runAndRescan("uninstall_apt_packages", { packages: adapter.firmwarePackage })}
              >
                {$_("common.uninstall")}
              </button>
            {/if}
          {/if}
        </div>
      {/each}

      <!-- Audio -->
      {#each scan.audio as device}
        <div class="border border-border rounded p-3">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.audio")}</div>
          <p class="text-[0.82rem] text-text-bright mb-1">{device.model}</p>
          <p class="text-[0.78rem] {statusInfo(device.driverStatus).class} mb-1.5">
            {statusInfo(device.driverStatus).icon} {$_(statusInfo(device.driverStatus).key)}
          </p>
          {#if device.driverStatus !== "installed"}
            {@const audioAction = actionButtonInfo(device.driverStatus, $_("drivers.audioDrivers"))}
            <button
              class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
              on:click={() => runAndRescan("install_apt_packages", { packages: device.requiredPackages.join(" ") })}
            >
              {$_(audioAction.key, { values: audioAction.values })}
            </button>
          {:else}
            <button
              class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold"
              on:click={() => runAndRescan("uninstall_apt_packages", { packages: device.requiredPackages.join(" ") })}
            >
              {$_("common.uninstall")}
            </button>
          {/if}
        </div>
      {/each}

      <!-- Bluetooth -->
      {#if scan.bluetooth}
        {@const bluetoothPackages = scan.bluetooth.requiredPackages.join(" ")}
        <div class="border border-border rounded p-3">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.bluetooth")}</div>
          <p class="text-[0.82rem] text-text-bright mb-1">{scan.bluetooth.model}</p>
          <p class="text-[0.78rem] {statusInfo(scan.bluetooth.driverStatus).class} mb-1.5">
            {statusInfo(scan.bluetooth.driverStatus).icon} {$_(statusInfo(scan.bluetooth.driverStatus).key)}
          </p>
          {#if scan.bluetooth.driverStatus !== "installed"}
            {@const btAction = actionButtonInfo(scan.bluetooth.driverStatus, $_("drivers.bluetoothStack"))}
            <button
              class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
              on:click={() => runAndRescan("install_apt_packages", { packages: bluetoothPackages })}
            >
              {$_(btAction.key, { values: btAction.values })}
            </button>
          {:else}
            <button
              class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold"
              on:click={() => runAndRescan("uninstall_apt_packages", { packages: bluetoothPackages })}
            >
              {$_("common.uninstall")}
            </button>
          {/if}
        </div>
      {/if}

      <!-- Printers -->
      {#if scan.printers}
        {@const printerPackages = scan.printers.requiredPackages.join(" ")}
        <div class="border border-border rounded p-3">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.printers")}</div>
          <p class="text-[0.82rem] text-text-bright mb-1">{scan.printers.model}</p>
          <p class="text-[0.78rem] {statusInfo(scan.printers.driverStatus).class} mb-1.5">
            {statusInfo(scan.printers.driverStatus).icon} {$_(statusInfo(scan.printers.driverStatus).key)}
          </p>
          {#if scan.printers.driverStatus !== "installed"}
            {@const printerAction = actionButtonInfo(scan.printers.driverStatus, $_("drivers.printerDrivers"))}
            <button
              class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
              on:click={() => runAndRescan("install_apt_packages", { packages: printerPackages })}
            >
              {$_(printerAction.key, { values: printerAction.values })}
            </button>
          {:else}
            <button
              class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold"
              on:click={() => runAndRescan("uninstall_apt_packages", { packages: printerPackages })}
            >
              {$_("common.uninstall")}
            </button>
          {/if}
        </div>
      {/if}

      <!-- Apple Mac -->
      <MacCard mac={scan.mac} on:install={(e) => macInstall(e.detail)} on:remove={(e) => macRemove(e.detail)} />

      <!-- Storage -->
      <div class="border border-border rounded p-3">
        <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.storage")}</div>
        {#each scan.storage as disk}
          <p class="text-[0.82rem] text-text-bright">{disk.name}: {disk.size}</p>
        {/each}
      </div>

      <!-- Network -->
      <div class="border border-border rounded p-3">
        <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.network")}</div>
        {#each scan.network as iface}
          <p class="text-[0.82rem] text-text-bright">{iface.name}: {iface.ifaceType} — {iface.status}</p>
        {/each}
      </div>

      <!-- Unnecessary software -->
      {#if scan.unnecessarySoftware.length > 0}
        <div class="border border-warning/30 bg-warning/5 rounded p-3">
          <div class="text-[0.7rem] font-semibold text-warning uppercase tracking-wide mb-2">
            {$_("drivers.unnecessarySoftwareDetected")}
          </div>
          {#if scan.unnecessarySoftware.length > 1}
            <button
              class="w-full mb-2 px-3 py-1.5 rounded bg-danger hover:bg-danger/85 text-text-bright text-[0.78rem] font-semibold"
              on:click={removeAllUnnecessary}
            >
              {$_("drivers.removeAllDetected")}
            </button>
          {/if}
          {#each scan.unnecessarySoftware as finding}
            <div class="flex items-center justify-between gap-2 py-1.5 border-t border-warning/20 first:border-t-0">
              <div class="min-w-0">
                <p class="text-[0.8rem] text-text-bright font-medium">{finding.name}</p>
                <p class="text-[0.72rem] text-text">{finding.detail}</p>
              </div>
              <button
                class="px-2.5 py-1 rounded border border-warning text-warning hover:bg-warning/10 text-[0.75rem] font-semibold shrink-0"
                on:click={() => removeUnnecessary(finding)}
              >
                {$_("common.uninstall")}
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if scan}
  <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5 mb-4">
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <Cpu size={13} />
        </div>
        {$_("drivers.cpuGpu")}
      </div>
      <div class="text-[0.8rem] text-text-bright font-medium truncate mt-1">{scan.cpu.model}</div>
      <div class="text-[0.7rem] text-text-muted">{$_("drivers.coresThreads", { values: { cores: scan.cpu.cores, threads: scan.cpu.threads } })}</div>
      {#each scan.gpus as gpu}
        <div class="text-[0.8rem] text-text-bright font-medium truncate mt-1.5">{gpu.vendor} {gpu.model}</div>
      {/each}
    </div>
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <MemoryStick size={13} />
        </div>
        {$_("drivers.memory")}
      </div>
      <div class="text-[0.8rem] text-text-bright font-medium mt-1">{scan.memory.total}</div>
      <div class="text-[0.7rem] text-text-muted">{$_("drivers.availableSuffix", { values: { available: scan.memory.available } })}</div>
    </div>
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <HardDrive size={13} />
        </div>
        {$_("drivers.storage")}
      </div>
      {#each scan.storage as disk}
        <div class="text-[0.8rem] text-text-bright font-medium mt-1">{disk.name} — {disk.size}</div>
      {:else}
        <div class="text-[0.8rem] text-text-muted mt-1">{$_("drivers.noneDetected")}</div>
      {/each}
    </div>
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <Network size={13} />
        </div>
        {$_("drivers.network")}
      </div>
      {#each scan.network as iface}
        <div class="text-[0.8rem] text-text-bright font-medium truncate mt-1">{iface.name} ({iface.ifaceType}) — {iface.status}</div>
      {:else}
        <div class="text-[0.8rem] text-text-muted mt-1">{$_("drivers.noneDetected")}</div>
      {/each}
    </div>
  </div>

  {#each scan.unnecessarySoftware as finding}
    <div class="bg-warning/10 border border-warning/30 rounded p-3 mb-3 flex items-center justify-between gap-3">
      <div class="flex items-center gap-2.5 min-w-0">
        <AlertTriangle size={18} class="text-warning shrink-0" />
        <div class="min-w-0">
          <div class="text-[0.85rem] font-semibold text-text-bright">{finding.name}</div>
          <div class="text-[0.72rem] text-text-muted truncate">{finding.detail}</div>
        </div>
      </div>
      <button
        class="px-3 py-1.5 rounded border border-warning text-warning hover:bg-warning/10 text-[0.78rem] font-semibold shrink-0"
        on:click={() => removeUnnecessary(finding)}
      >
        {$_("drivers.remove")}
      </button>
    </div>
  {/each}
{/if}

<div class="grid grid-cols-1 md:grid-cols-2 gap-3.5">
  <!-- NVIDIA family -->
  <div
    class="bg-surface border border-border rounded p-4 transition-all {expanded === 'nvidia' ? 'md:col-span-2' : ''}"
  >
    <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily("nvidia")}>
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
          <img src="/assets/icons/hardware/nvidia.png" alt="" class="w-full h-full object-contain" />
        </div>
        <div class="text-left">
          <div class="text-text-bright font-semibold text-[0.9rem]">NVIDIA</div>
          <div class="text-[0.75rem] text-text-muted">
            {!scanned ? $_("drivers.detecting") : nvidiaGpu ? nvidiaGpu.model : $_("drivers.noNvidiaDetected")}
          </div>
        </div>
      </div>
      <ChevronDown size={18} class="text-text-muted transition-transform {expanded === 'nvidia' ? 'rotate-180' : ''}" />
    </button>

    {#if expanded === "nvidia"}
      <div class="mt-4 pt-3 border-t border-border flex items-center justify-end gap-3">
        <button
          class="px-3 py-1.5 rounded border border-border bg-surface-2 hover:bg-surface text-text-bright text-[0.78rem] font-medium transition-colors shrink-0 disabled:opacity-50"
          disabled={checkingNvidiaUpdate}
          on:click={runCheckNvidiaUpdate}
        >
          {checkingNvidiaUpdate ? $_("drivers.checkingUpdate") : $_("drivers.checkNvidiaUpdate")}
        </button>
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5 mt-3">
        {#each nvidiaBranches as branch}
          {@const branchDriverName = branch.kind === "cuda" ? `nvidia-driver-${branch.id}` : branch.id}
          {@const active = nvidiaGpu?.recommendedDriver === branchDriverName && nvidiaGpu?.driverStatus === "installed"}
          <button
            class="text-left border rounded p-3 transition-colors {active
              ? 'border-primary bg-primary/10'
              : 'border-border hover:bg-surface-2'}"
            on:click={() => (confirmBranch = branch)}
          >
            <div class="flex items-center justify-between gap-2">
              <span class="text-[0.85rem] font-semibold text-text-bright">{branch.label}</span>
              {#if active}
                <span class="text-[0.68rem] px-2 py-0.5 rounded-full bg-success/15 text-success shrink-0">{$_("drivers.active")}</span>
              {/if}
            </div>
            <p class="text-[0.72rem] text-text mt-1">{branch.hint}</p>
          </button>
        {/each}
      </div>

      <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mt-4 mb-2">{$_("drivers.nvidiaExtras")}</div>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
        {#each nvidiaExtraItems as item}
          {@const installed = installedVersion >= 0 && installedMap[item.label]}
          <button
            class="text-left border rounded p-3 transition-colors {installed
              ? 'border-primary bg-primary/10'
              : 'border-border hover:bg-surface-2'}"
            on:click={() => toggleItem(item)}
          >
            <div class="flex items-center justify-between gap-2">
              <span class="text-[0.85rem] font-semibold text-text-bright">{item.label}</span>
              <span
                class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {installed
                  ? 'bg-success/15 text-success'
                  : 'bg-surface-2 text-text'}"
              >
                {installed ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
              </span>
            </div>
            <p class="text-[0.72rem] text-text mt-1">{item.hint}</p>
            <p class="text-[0.68rem] text-primary mt-1">
              {installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
            </p>
          </button>
        {/each}
        <button
          class="text-left border rounded p-3 transition-colors {cuda12Installed
            ? 'border-primary bg-primary/10'
            : 'border-border hover:bg-surface-2'}"
          on:click={toggleCuda12}
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-[0.85rem] font-semibold text-text-bright">CUDA 12 Toolkit</span>
            <span
              class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {cuda12Installed
                ? 'bg-success/15 text-success'
                : 'bg-surface-2 text-text'}"
            >
              {cuda12Installed ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
            </span>
          </div>
          <p class="text-[0.72rem] text-text mt-1">
            {$_("drivers.cuda12Description")}
          </p>
          <p class="text-[0.68rem] text-primary mt-1">
            {cuda12Installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
          </p>
        </button>
      </div>

      <button
        class="w-full mt-4 px-3 py-2 rounded border border-danger text-danger hover:bg-danger/10 text-[0.82rem] font-semibold"
        on:click={() => (confirmUninstall = true)}
      >
        {$_("drivers.uninstallNvidiaDrivers")}
      </button>

      <div class="mt-4 pt-4 border-t border-border">
        <div class="flex items-center justify-between gap-2 mb-2">
          <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide">
            {$_("drivers.prime.title")}
          </div>
          <button
            class="text-[0.72rem] text-primary hover:underline font-medium flex items-center gap-1"
            on:click={onScanButtonClick}
          >
            {$_("drivers.prime.detectHybrid")}
          </button>
        </div>
        <p class="text-[0.72rem] text-text mb-3">{$_("drivers.prime.subtitle")}</p>

        {#if primeStatus && primeStatus.installed}
          <p class="text-[0.72rem] text-primary mb-3">
            {$_("drivers.prime.activeProfile", { values: { profile: primeStatus.activeProfile } })}
          </p>
          {@const profiles = [
            { id: "nvidia", labelKey: "drivers.prime.profileNvidia", hintKey: "drivers.prime.hintNvidia" },
            { id: "intel",  labelKey: "drivers.prime.profileIntel",  hintKey: "drivers.prime.hintIntel" },
            { id: "amd",    labelKey: "drivers.prime.profileAmd",    hintKey: "drivers.prime.hintAmd" },
            { id: "on-demand", labelKey: "drivers.prime.profileOnDemand", hintKey: "drivers.prime.hintOnDemand" },
          ]}
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {#each profiles as p}
              {@const active = primeStatus.activeProfile === p.id}
              <button
                class="text-left border rounded p-3 transition-colors {active
                  ? 'border-primary bg-primary/10'
                  : 'border-border hover:bg-surface-2'}"
                on:click={() => switchPrimeProfile(p.id)}
              >
                <div class="flex items-center justify-between gap-2">
                  <span class="text-[0.85rem] font-semibold text-text-bright">{$_(p.labelKey)}</span>
                  {#if active}
                    <span class="text-[0.68rem] px-2 py-0.5 rounded-full bg-success/15 text-success shrink-0">{$_("drivers.active")}</span>
                  {/if}
                </div>
                <p class="text-[0.72rem] text-text mt-1">{$_(p.hintKey)}</p>
              </button>
            {/each}
          </div>
          <p class="text-[0.68rem] text-text-muted mt-2">{$_("drivers.prime.rebootRequired")}</p>
        {:else}
          <p class="text-[0.72rem] text-text-muted mb-2">{$_("drivers.prime.notInstalled")}</p>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <button
              class="text-left border border-border rounded p-3 hover:bg-surface-2 transition-colors"
              on:click={installNvidiaPrime}
            >
              <div class="text-[0.85rem] font-semibold text-text-bright">{$_("drivers.prime.installPackage")}</div>
              <p class="text-[0.72rem] text-text mt-1">{$_("drivers.prime.installHint")}</p>
            </button>
            <button
              class="text-left border border-border rounded p-3 hover:bg-surface-2 transition-colors"
              on:click={() => switchPrimeProfile("offload")}
            >
              <div class="text-[0.85rem] font-semibold text-text-bright">PRIME Render Offload</div>
              <p class="text-[0.72rem] text-text mt-1">{$_("drivers.prime.hintOnDemand")}</p>
            </button>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Intel family (single item: oneAPI) -->
  <div class="bg-surface border border-border rounded p-4">
    <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily("intel")}>
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
          <img src="/assets/icons/hardware/intel.png" alt="" class="w-full h-full object-contain" />
        </div>
        <div class="text-left">
          <div class="text-text-bright font-semibold text-[0.9rem]">Intel</div>
          <div class="text-[0.75rem] text-text-muted">
            {!scanned
              ? $_("drivers.detecting")
              : scan?.gpus.find((g) => g.vendor === "Intel")?.model ?? $_("drivers.noIntelDetected")}
          </div>
        </div>
      </div>
      <ChevronDown size={18} class="text-text-muted transition-transform {expanded === 'intel' ? 'rotate-180' : ''}" />
    </button>

    {#if expanded === "intel"}
      <div class="mt-4 pt-4 border-t border-border">
        <button
          class="w-full text-left border rounded p-3 transition-colors {oneapiInstalled
            ? 'border-primary bg-primary/10'
            : 'border-border hover:bg-surface-2'}"
          on:click={toggleOneapi}
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-[0.85rem] font-semibold text-text-bright">Intel oneAPI Base Toolkit</span>
            <span
              class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {oneapiInstalled
                ? 'bg-success/15 text-success'
                : 'bg-surface-2 text-text'}"
            >
              {oneapiInstalled ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
            </span>
          </div>
          <p class="text-[0.72rem] text-text mt-1">
            {$_("drivers.oneapiDescription")}
          </p>
          <p class="text-[0.68rem] text-primary mt-1">
            {oneapiInstalled ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
          </p>
        </button>
      </div>
    {/if}
  </div>

  <!-- Other families: layout in place, not wired to the backend yet -->
  {#each [{ key: "amd", label: "AMD", icon: "/assets/icons/hardware/amd.png", items: amdItems }, { key: "wifi", label: $_("drivers.familyWifi"), icon: "/assets/icons/hardware/wifi.png", items: wifiItems }, { key: "vm", label: $_("drivers.familyVm"), icon: "/assets/icons/hardware/vms.png", items: vmItems }] as family}
    <div
      class="bg-surface border border-border rounded p-4 transition-all {expanded === family.key ? 'md:col-span-2' : ''}"
    >
      <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily(family.key)}>
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
            <img src={family.icon} alt="" class="w-full h-full object-contain" />
          </div>
          <div class="text-left">
            <div class="text-text-bright font-semibold text-[0.9rem]">{family.label}</div>
            <div class="text-[0.75rem] text-text-muted truncate">
              {familySubtitles[family.key] ?? ""}
            </div>
          </div>
        </div>
        <ChevronDown
          size={18}
          class="text-text-muted transition-transform {expanded === family.key ? 'rotate-180' : ''}"
        />
      </button>

      {#if expanded === family.key}
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5 mt-4 pt-4 border-t border-border">
          {#each family.items as item}
            {@const wired = Boolean(item.packages || item.special)}
            {@const installed = installedVersion >= 0 && installedMap[item.label]}
            <button
              class="text-left border rounded p-3 transition-colors {wired
                ? 'border-border hover:bg-surface-2'
                : 'border-border opacity-50 cursor-not-allowed'}"
              disabled={!wired}
              on:click={() => handleFamilyItemClick(item)}
            >
              <div class="flex items-center justify-between gap-2">
                <span class="text-[0.85rem] font-semibold text-text-bright">{item.label}</span>
                {#if !wired}
                  <span class="text-[0.68rem] px-2 py-0.5 rounded-full bg-surface-2 text-text shrink-0">{$_("drivers.soon")}</span>
                {:else if item.packages}
                  <span
                    class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {installed
                      ? 'bg-success/15 text-success'
                      : 'bg-surface-2 text-text'}"
                  >
                    {installed ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
                  </span>
                {:else if item.special === "vbox"}
                  <span
                    class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {vboxInstalled
                      ? 'bg-success/15 text-success'
                      : 'bg-surface-2 text-text'}"
                  >
                    {vboxInstalled ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
                  </span>
                {/if}
              </div>
              <p class="text-[0.72rem] text-text mt-1">{item.hint}</p>
              {#if item.packages}
                <p class="text-[0.68rem] text-primary mt-1">
                  {installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
                </p>
              {:else if item.special === "vbox"}
                <p class="text-[0.68rem] text-primary mt-1">
                  {vboxInstalled ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
                </p>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/each}

  <!-- Printers (single item, own card for symmetry) -->
  <div class="bg-surface border border-border rounded p-4">
    <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily("printers")}>
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
          <img src="/assets/icons/hardware/print.png" alt="" class="w-full h-full object-contain" />
        </div>
        <div class="text-left">
          <div class="text-text-bright font-semibold text-[0.9rem]">{$_("drivers.printers")}</div>
          <div class="text-[0.75rem] text-text-muted truncate">
            {!scanned ? $_("drivers.detecting") : scan?.printers ? `${scan.printers.model} (${scan.printers.driverStatus})` : $_("drivers.noPrinterDetected")}
          </div>
        </div>
      </div>
      <ChevronDown size={18} class="text-text-muted transition-transform {expanded === 'printers' ? 'rotate-180' : ''}" />
    </button>

    {#if expanded === "printers"}
      {@const installed = installedVersion >= 0 && installedMap[printerItem.label]}
      <div class="mt-4 pt-4 border-t border-border">
        <button
          class="w-full text-left border rounded p-3 transition-colors {installed
            ? 'border-primary bg-primary/10'
            : 'border-border hover:bg-surface-2'}"
          on:click={() => toggleItem(printerItem)}
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-[0.85rem] font-semibold text-text-bright">{printerItem.label}</span>
            <span
              class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {installed
                ? 'bg-success/15 text-success'
                : 'bg-surface-2 text-text'}"
            >
              {installed ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
            </span>
          </div>
          <p class="text-[0.72rem] text-text mt-1">{printerItem.hint}</p>
          <p class="text-[0.68rem] text-primary mt-1">
            {installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
          </p>
        </button>
      </div>
    {/if}
  </div>

  <!-- Apple Mac: only when the scan recognised one -->
  {#if scan?.mac.isMac}
    <div class="bg-surface border border-border rounded p-4">
      <MacCard mac={scan.mac} on:install={(e) => macInstall(e.detail)} on:remove={(e) => macRemove(e.detail)} />
    </div>
  {/if}
</div>

{#if $terminalLines.length > 0 && $terminalExpanded}
  <div class="mt-4" id="job-terminal-anchor">
    <div class="flex justify-end mb-2">
      <button
        class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.78rem] transition-colors"
        on:click={() => ($terminalExpanded = !$terminalExpanded)}
      >
        {$terminalExpanded ? $_("common.hideOutput") : $_("common.showOutput")}
      </button>
    </div>
    <TerminalOutput fullWidth />
  </div>
{/if}

{#if nvidiaUpdateResultOpen}
  <Modal variant="info" title={$_("drivers.checkNvidiaUpdate")} onClose={() => (nvidiaUpdateResultOpen = false)}>
    {#if nvidiaUpdateError}
      <p class="text-danger">{$_("drivers.updateCheckFailed")}</p>
    {:else if nvidiaUpdateStatus?.hasUpdate}
      <p class="text-primary font-semibold">
        {$_("drivers.updateAvailable", { values: { version: nvidiaUpdateStatus.latestVersion } })}
      </p>
    {:else if nvidiaUpdateStatus && !nvidiaUpdateStatus.hasUpdate && nvidiaUpdateStatus.installedVersion}
      <p class="text-success font-medium">{$_("drivers.latestVersionInstalled")}</p>
    {:else if nvidiaUpdateStatus && !nvidiaUpdateStatus.installedVersion}
      <p class="text-text-muted">{$_("drivers.noNvidiaDriverInstalled")}</p>
    {/if}
  </Modal>
{/if}

{#if wifiNoHardware}
  <Modal variant="info" title={$_("drivers.wifiRepair.title")} onClose={() => (wifiNoHardware = false)}>
    {$_("drivers.wifiRepair.noHardware")}
  </Modal>
{/if}

{#if wifiConfirm}
  <Modal
    variant="warning"
    title={$_("drivers.wifiRepair.title")}
    confirmLabel={$_("drivers.wifiRepair.repair")}
    onClose={() => (wifiConfirm = null)}
    onConfirm={confirmWifiRepair}
  >
    <p class="mb-3">
      {$_("drivers.wifiRepair.detectedDriver")} <strong class="text-text-bright">{wifiConfirm.driver}</strong>
      {#if wifiConfirm.iface}{$_("drivers.wifiRepair.interface", { values: { iface: wifiConfirm.iface } })}{/if}
    </p>
    <p class="text-text-muted">{$_("drivers.wifiRepair.description")}</p>
  </Modal>
{/if}

{#if confirmBranch}
  <Modal
    variant="confirm"
    title={$_("drivers.confirmBranch.title", { values: { label: confirmBranch.label } })}
    confirmLabel={$_("common.install")}
    onClose={() => (confirmBranch = null)}
    onConfirm={() => confirmBranch && runInstall(confirmBranch)}
  >
    {$_("drivers.confirmBranch.description", { values: { label: confirmBranch.label } })}
  </Modal>
{/if}

{#if confirmUninstall}
  <Modal
    variant="danger"
    title={$_("drivers.confirmUninstall.title")}
    confirmLabel={$_("common.uninstall")}
    onClose={() => (confirmUninstall = false)}
    onConfirm={runUninstall}
  >
    {$_("drivers.confirmUninstall.description")}
  </Modal>
{/if}

{#if confirmRocm}
  {@const rocmName = confirmRocm === "opencl" ? $_("drivers.rocm.openclName") : $_("drivers.rocm.fullName")}
  {@const rocmSize = confirmRocm === "opencl" ? $_("drivers.rocm.sizeOpencl") : $_("drivers.rocm.sizeFull")}
  <Modal
    variant="confirm"
    title={$_("drivers.rocm.installTitle", { values: { name: rocmName } })}
    confirmLabel={$_("common.install")}
    maxWidth={480}
    onClose={() => (confirmRocm = null)}
    onConfirm={() => confirmRocm && runRocmInstall(confirmRocm)}
  >
    {$_("drivers.rocm.description", { values: { name: rocmName, sizeNote: rocmSize } })}
  </Modal>
{/if}

{#if confirmRocmUninstall}
  <Modal
    variant="danger"
    title={$_("drivers.rocm.removeTitle")}
    confirmLabel={$_("common.uninstall")}
    onClose={() => (confirmRocmUninstall = false)}
    onConfirm={runRocmUninstall}
  >
    {$_("drivers.rocm.removeDescription")}
  </Modal>
{/if}
