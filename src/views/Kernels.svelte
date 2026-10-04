<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { runTracked, terminalExpanded, terminalLines } from "../lib/progress";
  import TerminalOutput from "../components/TerminalOutput.svelte";
  import Modal from "../components/Modal.svelte";
  import { _ } from "svelte-i18n";
  import { ChevronDown, AlertTriangle, Info } from "lucide-svelte";

  interface KernelInfo {
    kernel: string;
    kernelType: string;
    arch: string;
    uptime: string;
  }

  interface XanmodVariant {
    key: string;
    package: string | null;
    installed: boolean;
    inUse: boolean;
  }

  interface XanmodStatus {
    psabiLevel: number;
    variants: XanmodVariant[];
  }

  interface CleanKernelsPreview {
    current: string;
    keep: string[];
    remove: string[];
  }

  interface SoplosKernelPkg {
    package: string;
    name: string;
    description: string;
    suffix: string;
    version: string;
    installed: boolean;
    hasUpdate: boolean;
  }

  let xanmodLabels: Record<string, { title: string; hint: string; badge?: string }>;
  $: xanmodLabels = {
    "xanmod-main": { title: $_("kernels.xanmod.main.title"), hint: $_("kernels.xanmod.main.hint") },
    "xanmod-rt": { title: $_("kernels.xanmod.rt.title"), hint: $_("kernels.xanmod.rt.hint") },
    "xanmod-edge": { title: $_("kernels.xanmod.edge.title"), hint: $_("kernels.xanmod.edge.hint"), badge: $_("kernels.xanmod.edge.badge") },
    "xanmod-lts": { title: $_("kernels.xanmod.lts.title"), hint: $_("kernels.xanmod.lts.hint") },
  };

  let kernelInfo: KernelInfo | null = null;
  let brokenXanmodRepo = false;
  let cpuVendor: string | null = null;
  let microcodePackage = "";
  let microcodeInstalled = false;
  let kernelInstallerInstalled = false;
  let liquorixInstalled = false;
  let liquorixInUse = false;
  let xanmodStatus: XanmodStatus | null = null;
  let xanmodExpanded = false;
  let infoMessage = "";
  let archInfo: { cpu: string; level: number; gpu: string; ram: string } | null = null;

  async function refreshKernelInfo() {
    kernelInfo = await api.invoke<KernelInfo>("get_kernel_info");
  }

  async function refreshBrokenRepo() {
    brokenXanmodRepo = await api.invoke<boolean>("has_broken_xanmod_repo");
  }

  async function refreshMicrocode() {
    cpuVendor = await api.invoke<string | null>("get_cpu_vendor");
    microcodePackage = cpuVendor === "intel" ? "intel-microcode" : cpuVendor === "amd" ? "amd64-microcode" : "";
    if (microcodePackage) {
      microcodeInstalled = await api.invoke<boolean>("is_apt_package_installed", { package: microcodePackage });
    }
  }

  async function refreshKernelInstaller() {
    kernelInstallerInstalled = await api.invoke<boolean>("is_apt_package_installed", { package: "soplos-kernel-installer" });
  }

  async function showArchInfo() {
    const [scan, status] = await Promise.all([
      api.invoke<{
        cpu: { model: string };
        memory: { total: string };
        gpus: { vendor: string; model: string }[];
      }>("scan_hardware"),
      xanmodStatus ?? api.invoke<XanmodStatus>("get_xanmod_status"),
    ]);
    archInfo = {
      cpu: scan.cpu.model,
      level: Math.min(status.psabiLevel, 4),
      gpu: scan.gpus.map((g) => `${g.vendor} ${g.model}`).join(", "),
      ram: scan.memory.total,
    };
  }

  async function refreshLiquorix() {
    liquorixInstalled = await api.invoke<boolean>("is_apt_package_installed", { package: "linux-image-liquorix-amd64" });
    liquorixInUse = await api.invoke<boolean>("is_kernel_in_use", { needle: "liquorix" });
  }

  async function refreshXanmod() {
    xanmodStatus = await api.invoke<XanmodStatus>("get_xanmod_status");
  }

  interface DebianKernelPackage {
    packageName: string;
    label: string;
    description: string;
    installed: boolean;
    inUse: boolean;
  }

  let debianKernels: DebianKernelPackage[] = [];
  let debianExpanded = false;

  async function refreshDebianKernels() {
    debianKernels = await api.invoke<DebianKernelPackage[]>("get_debian_kernels");
  }

  async function toggleDebianKernel(pkg: DebianKernelPackage) {
    if (pkg.installed && pkg.inUse) {
      infoMessage = $_("kernels.cannotUninstallLiquorix");
      return;
    }
    await runTracked(pkg.installed ? "uninstall_debian_kernel" : "install_debian_kernel", { package: pkg.packageName });
    await refreshDebianKernels();
    await refreshKernelInfo();
  }

  onMount(() => {
    refreshKernelInfo();
    refreshBrokenRepo();
    refreshMicrocode();
    refreshKernelInstaller();
    refreshLiquorix();
    refreshXanmod();
    refreshDebianKernels();
    refreshRepoStatus();
  });

  async function repairXanmodRepo() {
    await runTracked("repair_xanmod_repo");
    await refreshBrokenRepo();
  }

  async function toggleMicrocode() {
    if (!microcodePackage) return;
    await runTracked(microcodeInstalled ? "uninstall_apt_packages" : "install_apt_packages", { packages: microcodePackage });
    await refreshMicrocode();
  }

  async function toggleKernelInstaller() {
    await runTracked(kernelInstallerInstalled ? "uninstall_apt_packages" : "install_apt_packages", { packages: "soplos-kernel-installer" });
    await refreshKernelInstaller();
  }

  async function openKernelInstaller() {
    try {
      await api.invoke("open_kernel_installer");
    } catch (error) {
      infoMessage = $_("kernels.launchFailed", { values: { error: String(error) } });
    }
  }

  async function toggleLiquorix() {
    if (liquorixInstalled && liquorixInUse) {
      infoMessage = $_("kernels.cannotUninstallLiquorix");
      return;
    }
    await runTracked(liquorixInstalled ? "uninstall_liquorix" : "install_liquorix");
    await refreshLiquorix();
    await refreshKernelInfo();
  }

  async function toggleXanmodVariant(variant: XanmodVariant) {
    if (!variant.package) return;
    if (variant.installed && variant.inUse) {
      infoMessage = $_("kernels.cannotUninstallXanmod");
      return;
    }
    await runTracked(variant.installed ? "uninstall_xanmod" : "install_xanmod", { kernelType: variant.key });
    await refreshXanmod();
    await refreshKernelInfo();
  }

  // === Maintenance ===
  let cleanPreview: CleanKernelsPreview | null = null;

  async function openCleanPreview() {
    cleanPreview = await api.invoke<CleanKernelsPreview>("preview_clean_kernels");
    if (cleanPreview.remove.length === 0) {
      infoMessage = $_("kernels.noOldKernels");
      cleanPreview = null;
    }
  }

  async function confirmClean() {
    if (!cleanPreview) return;
    const packages = cleanPreview.remove;
    cleanPreview = null;
    await runTracked("clean_kernels", { packages });
    await refreshKernelInfo();
  }

  async function updateGrub() {
    await runTracked("update_grub");
  }

  // === Soplos Kernels repository + list ===
  let repoInstalled = false;
  let kernels: SoplosKernelPkg[] = [];
  let flavorFilter = "all";
  let marchFilter = "all";
  let confirmRemoveRepo = false;

  async function refreshRepoStatus() {
    repoInstalled = await api.invoke<boolean>("is_soplos_kernels_repo_installed");
    if (repoInstalled) {
      await refreshKernelsList();
    }
  }

  async function refreshKernelsList() {
    kernels = await api.invoke<SoplosKernelPkg[]>("list_soplos_kernels");
  }

  async function addRepo() {
    await runTracked("add_soplos_kernels_repo");
    await refreshRepoStatus();
  }

  async function refreshRepo() {
    await runTracked("update_repositories");
    await refreshKernelsList();
  }

  async function removeRepo() {
    confirmRemoveRepo = false;
    await runTracked("remove_soplos_kernels_repo");
    kernels = [];
    await refreshRepoStatus();
  }

  async function installKernel(pkg: SoplosKernelPkg) {
    await runTracked("install_soplos_kernel", { package: pkg.package });
    await refreshKernelsList();
    await refreshKernelInfo();
  }

  async function uninstallKernel(pkg: SoplosKernelPkg) {
    await runTracked("uninstall_soplos_kernel", { package: pkg.package, suffix: pkg.suffix });
    await refreshKernelsList();
    await refreshKernelInfo();
  }

  async function updateKernel(pkg: SoplosKernelPkg) {
    await runTracked("update_soplos_kernel", { package: pkg.package, suffix: pkg.suffix });
    await refreshKernelsList();
    await refreshKernelInfo();
  }

  $: flavorOptions = Array.from(new Set(kernels.map((k) => k.name.replace(/\s+(V[1-4])$/, "")))).sort();
  $: marchOptions = Array.from(
    new Set(kernels.map((k) => k.name.match(/\s+(V[1-4])$/)?.[1]).filter((v): v is string => Boolean(v))),
  ).sort();
  $: visibleKernels = kernels.filter((k) => {
    const march = k.name.match(/\s+(V[1-4])$/)?.[1] ?? null;
    const flavor = k.name.replace(/\s+(V[1-4])$/, "");
    if (flavorFilter !== "all" && flavor !== flavorFilter) return false;
    if (marchFilter !== "all" && march !== marchFilter) return false;
    return true;
  });
</script>

<div
  class="relative rounded border border-border overflow-hidden mb-5 flex flex-col items-start justify-end text-left p-4 min-h-[200px]"
  style="background: linear-gradient(rgba(20, 20, 22, 0.35), rgba(20, 20, 22, 0.85)), url('/assets/hero/kernels-background.png'); background-size: cover; background-position: center;"
>
  {#if kernelInfo}
    <p class="text-white font-semibold text-[1rem]">{kernelInfo.kernel}</p>
    <p class="text-[0.8rem] text-white/85 mt-1">
      {kernelInfo.kernelType} — {kernelInfo.arch} — up {kernelInfo.uptime}
    </p>
  {/if}
</div>

<div class="grid grid-cols-1 lg:grid-cols-2 gap-3.5 items-start">
  <!-- Left column -->
  <div class="flex flex-col gap-3.5">
    {#if brokenXanmodRepo}
      <div class="bg-warning/10 border border-warning/30 rounded p-3 flex items-start gap-2.5">
        <AlertTriangle size={18} class="text-warning shrink-0 mt-0.5" />
        <div class="min-w-0">
          <p class="text-[0.85rem] font-semibold text-warning">{$_("kernels.repairRepoTitle")}</p>
          <p class="text-[0.75rem] text-text mt-1">
            {$_("kernels.repairRepoDescription")}
          </p>
          <button
            class="mt-2 px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
            on:click={repairXanmodRepo}
          >
            {$_("kernels.repairRepoButton")}
          </button>
        </div>
      </div>
    {/if}

    <!-- CPU Microcode -->
    <div class="bg-surface border border-border rounded p-4">
      <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("kernels.cpuMicrocode")}</div>
      {#if !cpuVendor}
        <p class="text-[0.78rem] text-text-muted italic">{$_("kernels.cpuVendorNotDetected")}</p>
      {:else}
        <div class="flex items-center justify-between gap-2">
          <div>
            <p class="text-[0.85rem] font-semibold text-text-bright">{$_("kernels.microcodeTitle", { values: { vendor: cpuVendor === "intel" ? "Intel" : "AMD" } })}</p>
            <p class="text-[0.72rem] text-text">{$_("kernels.microcodeDescription")}</p>
          </div>
          <button
            class="px-3 py-1.5 rounded text-[0.78rem] font-semibold shrink-0 {microcodeInstalled
              ? 'border border-danger text-danger hover:bg-danger/10'
              : 'bg-primary hover:bg-primary-hover text-text-bright'}"
            on:click={toggleMicrocode}
          >
            {microcodeInstalled ? $_("common.uninstall") : $_("common.install")}
          </button>
        </div>
      {/if}
    </div>

    <!-- Soplos Kernel Installer -->
    <div class="bg-surface border border-border rounded p-4">
      <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("kernels.kernelInstallerTitle")}</div>
      <div class="flex items-center justify-between gap-2">
        <div>
          <p class="text-[0.85rem] font-semibold text-text-bright">{$_("kernels.kernelInstallerTitle")}</p>
          <p class="text-[0.72rem] text-text">{$_("kernels.kernelInstallerDescription")}</p>
        </div>
        <div class="flex gap-2 shrink-0">
          {#if kernelInstallerInstalled}
            <button
              class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem] font-semibold"
              on:click={openKernelInstaller}
            >
              {$_("common.open")}
            </button>
          {/if}
          <button
            class="px-3 py-1.5 rounded text-[0.78rem] font-semibold {kernelInstallerInstalled
              ? 'border border-danger text-danger hover:bg-danger/10'
              : 'bg-primary hover:bg-primary-hover text-text-bright'}"
            on:click={toggleKernelInstaller}
          >
            {kernelInstallerInstalled ? $_("common.uninstall") : $_("common.install")}
          </button>
        </div>
      </div>
    </div>

    <!-- Liquorix -->
    <div class="bg-surface border border-border rounded p-4">
      <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("kernels.liquorixTitle")}</div>
      <div class="flex items-center justify-between gap-2">
        <div>
          <p class="text-[0.85rem] font-semibold text-text-bright">
            Liquorix {liquorixInUse ? $_("kernels.inUseParen") : liquorixInstalled ? $_("kernels.installedParen") : ""}
          </p>
          <p class="text-[0.72rem] text-text">{$_("kernels.liquorixDescription")}</p>
        </div>
        <button
          class="px-3 py-1.5 rounded text-[0.78rem] font-semibold shrink-0 {liquorixInstalled
            ? 'border border-danger text-danger hover:bg-danger/10'
            : 'bg-primary hover:bg-primary-hover text-text-bright'}"
          on:click={toggleLiquorix}
        >
          {liquorixInstalled ? $_("common.uninstall") : $_("common.install")}
        </button>
      </div>
    </div>

    <!-- Debian Official Kernels -->
    <div class="bg-surface border border-border rounded p-4">
      <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("kernels.debianKernels")}</div>
      <button class="w-full flex items-center justify-between gap-3" on:click={() => (debianExpanded = !debianExpanded)}>
        <div class="text-left">
          <p class="text-[0.85rem] font-semibold text-text-bright">{$_("kernels.debianKernels")}</p>
          <p class="text-[0.72rem] text-text">{$_("kernels.debianKernelsHint")}</p>
        </div>
        <ChevronDown size={18} class="text-text-muted transition-transform {debianExpanded ? 'rotate-180' : ''}" />
      </button>

      {#if debianExpanded}
        <div class="flex flex-col gap-2 mt-4 pt-4 border-t border-border">
          {#each debianKernels as pkg}
            <button
              class="text-left border rounded p-3 transition-colors border-border hover:bg-surface-2"
              on:click={() => toggleDebianKernel(pkg)}
            >
              <div class="flex items-center justify-between gap-2 flex-wrap">
                <span class="text-[0.85rem] font-semibold text-text-bright">{pkg.label}</span>
                <span
                  class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {pkg.installed
                    ? 'bg-success/15 text-success'
                    : 'bg-surface-2 text-text'}"
                >
                  {pkg.inUse && pkg.installed ? $_("kernels.inUseBadge") : pkg.installed ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
                </span>
              </div>
              <p class="text-[0.72rem] text-text mt-1">{pkg.description}</p>
              <p class="text-[0.68rem] text-primary mt-1">
                {pkg.installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
              </p>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- XanMod (expandable, Drivers-style) -->
    <div class="bg-surface border border-border rounded p-4">
      <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("kernels.xanmodVariantsTitle")}</div>
      <button class="w-full flex items-center justify-between gap-3" on:click={() => (xanmodExpanded = !xanmodExpanded)}>
        <div class="text-left">
          <p class="text-[0.85rem] font-semibold text-text-bright">{$_("kernels.highPerformanceKernel")}</p>
          <p class="text-[0.72rem] text-text">
            {xanmodStatus ? $_("kernels.archBuilds", { values: { level: Math.min(xanmodStatus.psabiLevel, 3) } }) : $_("drivers.detecting")}
          </p>
        </div>
        <ChevronDown size={18} class="text-text-muted transition-transform {xanmodExpanded ? 'rotate-180' : ''}" />
      </button>

      {#if xanmodExpanded && xanmodStatus}
        <div class="flex flex-col gap-2 mt-4 pt-4 border-t border-border">
          {#each xanmodStatus.variants as variant}
            {@const labels = xanmodLabels[variant.key]}
            <button
              class="text-left border rounded p-3 transition-colors {variant.package
                ? 'border-border hover:bg-surface-2'
                : 'border-border opacity-50 cursor-not-allowed'}"
              disabled={!variant.package}
              on:click={() => toggleXanmodVariant(variant)}
            >
              <div class="flex items-center justify-between gap-2 flex-wrap">
                <span class="text-[0.85rem] font-semibold text-text-bright">{labels.title}</span>
                {#if !variant.package}
                  <span class="text-[0.68rem] px-2 py-0.5 rounded-full bg-surface-2 text-text shrink-0">{$_("kernels.notCompatible")}</span>
                {:else}
                  <span
                    class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {variant.installed
                      ? 'bg-success/15 text-success'
                      : 'bg-surface-2 text-text'}"
                  >
                    {variant.inUse && variant.installed ? $_("kernels.inUseBadge") : variant.installed ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
                  </span>
                {/if}
              </div>
              <p class="text-[0.72rem] text-text mt-1">{labels.hint}</p>
              {#if variant.package}
                <p class="text-[0.68rem] text-primary mt-1">
                  {variant.installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
                </p>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- System Maintenance -->
    <div class="bg-surface border border-border rounded p-4">
      <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("kernels.systemMaintenance")}</div>
      <p class="text-[0.72rem] text-text mb-2.5">{$_("kernels.maintenanceDescription")}</p>
      <div class="flex gap-2">
        <button
          class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem] font-semibold"
          on:click={openCleanPreview}
        >
          {$_("kernels.cleanOldKernels")}
        </button>
        <button
          class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem] font-semibold"
          on:click={updateGrub}
        >
          {$_("kernels.updateGrub")}
        </button>
      </div>
    </div>
  </div>

  <!-- Right column: Soplos Kernels repository -->
  <div class="flex flex-col gap-3.5">
    <div class="bg-surface border border-border rounded p-4">
      <div class="flex items-center justify-between gap-2 mb-2">
        <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide">{$_("kernels.repoTitle")}</div>
        <button
          class="text-text-muted hover:text-text-bright"
          title={$_("kernels.archInfoTooltip")}
          on:click={showArchInfo}
        >
          <Info size={15} />
        </button>
      </div>
      <div class="flex items-center justify-between gap-2">
        <div>
          <p class="text-[0.85rem] font-semibold text-text-bright">
            {repoInstalled ? $_("kernels.repoInstalled") : $_("kernels.repoNotConfigured")}
          </p>
          <p class="text-[0.72rem] text-text">{$_("kernels.repoDescription")}</p>
        </div>
        <div class="flex gap-2 shrink-0">
          {#if repoInstalled}
            <button
              class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem] font-semibold"
              on:click={refreshRepo}
            >
              {$_("kernels.refresh")}
            </button>
            <button
              class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold"
              on:click={() => (confirmRemoveRepo = true)}
            >
              {$_("kernels.remove")}
            </button>
          {:else}
            <button
              class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold"
              on:click={addRepo}
            >
              {$_("kernels.addRepository")}
            </button>
          {/if}
        </div>
      </div>
    </div>

    {#if repoInstalled}
      <div class="bg-surface border border-border rounded p-4">
        <div class="flex items-center gap-3 flex-wrap">
          <label class="flex items-center gap-1.5 text-[0.78rem] text-text">
            {$_("kernels.profileLabel")}
            <select
              bind:value={flavorFilter}
              class="appearance-none bg-surface-2 border border-border rounded px-2 py-1 text-[0.78rem] text-text-bright cursor-pointer"
            >
              <option value="all">{$_("kernels.all")}</option>
              {#each flavorOptions as opt}
                <option value={opt}>{opt}</option>
              {/each}
            </select>
          </label>
          <label class="flex items-center gap-1.5 text-[0.78rem] text-text">
            {$_("kernels.versionLabel")}
            <select
              bind:value={marchFilter}
              class="appearance-none bg-surface-2 border border-border rounded px-2 py-1 text-[0.78rem] text-text-bright cursor-pointer"
            >
              <option value="all">{$_("kernels.all")}</option>
              {#each marchOptions as opt}
                <option value={opt}>{opt}</option>
              {/each}
            </select>
          </label>
        </div>
      </div>

      <div class="bg-surface border border-border rounded p-4">
        <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("kernels.availableKernels")}</div>
        <div
          class="content-scroll overflow-y-auto overscroll-contain max-h-[50vh] pr-3 flex flex-col"
          style="scrollbar-gutter: stable;"
        >
          {#if visibleKernels.length === 0}
            <p class="text-[0.78rem] text-text-muted italic py-2">
              {kernels.length === 0 ? $_("kernels.noKernelsInRepo") : $_("kernels.noKernelsMatchFilters")}
            </p>
          {/if}
          {#each visibleKernels as pkg, i}
            <div class="flex items-center justify-between gap-2 py-2 {i > 0 ? 'border-t border-border' : ''}">
              <div class="min-w-0">
                <p class="text-[0.82rem] text-text-bright font-semibold">
                  {pkg.name} <span class="text-text-muted font-normal text-[0.72rem]">{pkg.version}</span>
                </p>
                <p class="text-[0.72rem] text-text truncate">{pkg.description}</p>
              </div>
              <div class="flex gap-2 shrink-0">
                {#if pkg.installed && pkg.hasUpdate}
                  <button
                    class="px-2.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.75rem] font-semibold"
                    on:click={() => updateKernel(pkg)}
                  >
                    {$_("kernels.update")}
                  </button>
                {/if}
                <button
                  class="px-2.5 py-1.5 rounded text-[0.75rem] font-semibold {pkg.installed
                    ? 'border border-danger text-danger hover:bg-danger/10'
                    : 'bg-primary hover:bg-primary-hover text-text-bright'}"
                  on:click={() => (pkg.installed ? uninstallKernel(pkg) : installKernel(pkg))}
                >
                  {pkg.installed ? $_("kernels.remove") : $_("common.install")}
                </button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
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

{#if infoMessage}
  <Modal variant="info" maxWidth={400} onClose={() => (infoMessage = "")}>
    <p class="whitespace-pre-line text-text-bright">{infoMessage}</p>
  </Modal>
{/if}

{#if archInfo}
  <Modal title={$_("kernels.archInfoTooltip")} maxWidth={460} onClose={() => (archInfo = null)}>
    <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 mb-3">
      <dt class="text-text-muted">CPU</dt>
      <dd class="text-text-bright font-medium">{archInfo.cpu}</dd>
      <dt class="text-text-muted">{$_("kernels.archInfo.levelLabel")}</dt>
      <dd>
        <span class="px-2 py-0.5 rounded-full bg-primary/15 text-primary font-semibold text-[0.8rem]">V{archInfo.level}</span>
      </dd>
      {#if archInfo.gpu}
        <dt class="text-text-muted">GPU</dt>
        <dd class="text-text-bright font-medium">{archInfo.gpu}</dd>
      {/if}
      <dt class="text-text-muted">RAM</dt>
      <dd class="text-text-bright font-medium">{archInfo.ram}</dd>
    </dl>
    <p class="text-text-muted text-[0.8rem]">{$_("kernels.archInfo.chooseHint")}</p>
  </Modal>
{/if}

{#if cleanPreview}
  <Modal
    variant="danger"
    title={$_("kernels.cleanOldKernels")}
    confirmLabel={$_("kernels.continue")}
    maxWidth={500}
    onClose={() => (cleanPreview = null)}
    onConfirm={confirmClean}
  >
    <p class="text-text-muted text-[0.8rem] mb-3">{$_("kernels.currentKernel", { values: { current: cleanPreview.current } })}</p>
    <div class="content-scroll overflow-y-auto overscroll-contain max-h-[240px] flex flex-col gap-3 text-[0.8rem]">
      <div class="rounded border border-success/30 bg-success/5 p-3">
        <p class="text-success font-semibold mb-1">{$_("kernels.kernelsToKeep")}</p>
        {#each cleanPreview.keep as pkg}
          <p class="text-text">✓ {pkg}</p>
        {/each}
      </div>
      <div class="rounded border border-danger/30 bg-danger/5 p-3">
        <p class="text-danger font-semibold mb-1">{$_("kernels.packagesToPurge", { values: { count: cleanPreview.remove.length } })}</p>
        {#each cleanPreview.remove as pkg}
          <p class="text-text">✗ {pkg}</p>
        {/each}
      </div>
    </div>
    <p class="text-text-muted text-[0.78rem] mt-3">{$_("kernels.removeWillDelete")}</p>
  </Modal>
{/if}

{#if confirmRemoveRepo}
  <Modal
    variant="danger"
    title={$_("kernels.removeRepoTitle")}
    confirmLabel={$_("kernels.remove")}
    onClose={() => (confirmRemoveRepo = false)}
    onConfirm={removeRepo}
  >
    {$_("kernels.removeRepoDescription")}
  </Modal>
{/if}
