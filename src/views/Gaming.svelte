<script lang="ts">
  import Modal from "../components/Modal.svelte";
  import FlatpakScopeModal from "../components/FlatpakScopeModal.svelte";
  import { _ } from "svelte-i18n";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { runTracked, terminalExpanded, terminalLines } from "../lib/progress";
  import TerminalOutput from "../components/TerminalOutput.svelte";
  import { Undo2, ChevronDown, Cpu, MemoryStick, CircuitBoard, Zap } from "lucide-svelte";

  type DesktopKind = "gnome" | "kde" | "xfce";

  interface HardwareSummary {
    cpu: { model: string; cores: number; threads: number };
    memory: { total: string; available: string };
    gpus: { vendor: string; model: string; driverStatus: string }[];
  }

  let hardware: HardwareSummary | null = null;

  interface OptimizationItem {
    key: string;
    icon: string;
    badge?: string;
    noUninstall?: boolean;
  }

  interface LauncherItem {
    key: string;
    name: string;
    icon: string;
    kind: "apt" | "flatpak" | "deb" | "custom";
    package?: string;
    flatpakId?: string;
    debUrl?: string;
    checkPath?: string;
    // Fixed Tauri command names for "custom" entries — the script lives in
    // Rust (src-tauri/src/commands/recommended.rs), never sent from the
    // frontend. See security plan point 1.
    customInstall?: string;
    customUninstall?: string;
    official?: boolean;
    // What the source badge shows — defaults to `kind` when omitted; only
    // needed when it differs (ES-DE is an AppImage though it's kind "custom",
    // GeForce NOW is really Flatpak though it's installed via custom commands).
    badgeKind?: "apt" | "flatpak" | "deb" | "appimage" | "snap";
  }

  const badgeLabels: Record<string, string> = {
    apt: "APT",
    flatpak: "Flatpak",
    deb: "Deb",
    appimage: "AppImage",
    snap: "Snap",
  };

  function launcherBadgeKind(item: LauncherItem): string {
    return item.badgeKind ?? item.kind;
  }

  const optimizations: OptimizationItem[] = [
    { key: "gamemode", icon: "/assets/icons/gaming/gamemode.png" },
    { key: "performance-mode", icon: "/assets/icons/gaming/performance.png" },
    { key: "sysctl", icon: "/assets/icons/gaming/sysctl.svg" },
    { key: "gpu", icon: "/assets/icons/gaming/gpu.svg" },
    { key: "disk-io", icon: "/assets/icons/gaming/disk-io.svg" },
    { key: "mangohud", icon: "/assets/icons/gaming/mangohud.svg", noUninstall: true },
    { key: "cpu-power", icon: "/assets/icons/gaming/cpupower.png" },
    { key: "ryzenadj", icon: "/assets/icons/gaming/ryzenadj.png" },
  ];

  const launchers: LauncherItem[] = [
    // The gaming tool of Soplos itself, from its repository, first.
    { key: "soplos-gaming-panel", name: "Soplos Gaming Panel", icon: "/assets/icons/gaming/soplos-gaming-panel.png", kind: "apt", package: "soplos-gaming-panel" },
    { key: "steam", name: "Steam", icon: "/assets/icons/gaming/steam.png", kind: "flatpak", flatpakId: "com.valvesoftware.Steam" },
    { key: "lutris", name: "Lutris", icon: "/assets/icons/gaming/lutris.png", kind: "flatpak", flatpakId: "net.lutris.Lutris", official: true },
    { key: "heroic", name: "Heroic Games Launcher", icon: "/assets/icons/gaming/heroic.png", kind: "flatpak", flatpakId: "com.heroicgameslauncher.hgl" },
    { key: "heronite", name: "Heronite", icon: "/assets/icons/gaming/heronite.png", kind: "deb", package: "heronite", debUrl: "https://github.com/RafaelGoulartB/Heronite/releases/download/v0.14.0/heronite_0.14.0-1_amd64.deb" },
    { key: "bottles", name: "Bottles", icon: "/assets/icons/gaming/bottles.png", kind: "flatpak", flatpakId: "com.usebottles.bottles" },
    { key: "r2modman", name: "R2ModMan", icon: "/assets/icons/gaming/r2modman.png", kind: "deb", package: "r2modman", debUrl: "https://github.com/ebkr/r2modmanPlus/releases/download/v3.2.11/r2modman_3.2.11_amd64.deb" },
    { key: "vinegar", name: "Vinegar (Roblox)", icon: "/assets/icons/gaming/vinegar.png", kind: "flatpak", flatpakId: "org.vinegarhq.Vinegar" },
    { key: "sober", name: "Sober", icon: "/assets/icons/gaming/sober.png", kind: "flatpak", flatpakId: "org.vinegarhq.Sober" },
    { key: "mocktail", name: "Mocktail", icon: "/assets/icons/gaming/mocktail.png", kind: "flatpak", flatpakId: "space.bigrat.mocktail" },
    { key: "prism", name: "Prism Launcher", icon: "/assets/icons/gaming/prism.png", kind: "flatpak", flatpakId: "org.prismlauncher.PrismLauncher" },
    { key: "itch", name: "Itch.io", icon: "/assets/icons/gaming/itch-io.png", kind: "flatpak", flatpakId: "io.itch.itch" },
    { key: "minigalaxy", name: "Minigalaxy", icon: "/assets/icons/gaming/gog.png", kind: "apt", package: "minigalaxy", official: true },
    { key: "retroarch", name: "RetroArch", icon: "/assets/icons/gaming/retroarch.png", kind: "apt", package: "retroarch", official: true },
    {
      key: "es-de",
      name: "ES-DE",
      icon: "/assets/icons/gaming/ES-DE.png",
      kind: "custom",
      badgeKind: "appimage",
      checkPath: "~/AppImages/ES-DE_x64.AppImage",
      customInstall: "install_esde",
      customUninstall: "uninstall_esde",
    },
    { key: "moonlight", name: "Moonlight", icon: "/assets/icons/gaming/moonlight.png", kind: "flatpak", flatpakId: "com.moonlight_stream.Moonlight" },
    { key: "chiaki", name: "Chiaki (PS4/PS5)", icon: "/assets/icons/gaming/chiaki.png", kind: "flatpak", flatpakId: "io.github.streetpea.Chiaki4deck" },
    { key: "discord", name: "Discord", icon: "/assets/icons/gaming/discord.svg", kind: "flatpak", flatpakId: "com.discordapp.Discord" },
    { key: "protonup", name: "ProtonUp-Qt", icon: "/assets/icons/gaming/protonup.png", kind: "flatpak", flatpakId: "net.davidotek.pupgui2" },
    { key: "protonplus", name: "ProtonPlus", icon: "/assets/icons/gaming/proton+.png", kind: "flatpak", flatpakId: "com.vysp3r.ProtonPlus" },
    { key: "ppsspp", name: "PPSSPP", icon: "/assets/icons/gaming/ppsspp.png", kind: "flatpak", flatpakId: "org.ppsspp.PPSSPP" },
    {
      key: "geforcenow",
      name: "GeForce NOW",
      icon: "/assets/icons/gaming/geforcenow.png",
      kind: "custom",
      badgeKind: "flatpak",
      checkPath: "~/.local/share/flatpak/app/com.nvidia.geforcenow",
      customInstall: "install_geforcenow",
      customUninstall: "uninstall_geforcenow",
    },
    {
      key: "xodus-nexus",
      name: "Xodus Nexus",
      icon: "/assets/icons/gaming/xodus-nexus.png",
      kind: "custom",
      badgeKind: "appimage",
      checkPath: "~/AppImages/Xodus-Nexus.AppImage",
      customInstall: "install_xodus_nexus",
      customUninstall: "uninstall_xodus_nexus",
    },
    {
      key: "epic-games",
      name: "Epic Games Launcher",
      icon: "/assets/icons/gaming/epic-games-launcher.png",
      kind: "custom",
      badgeKind: "deb",
      checkPath: "/usr/bin/EpicGamesLauncher",
      customInstall: "install_epic_games_launcher",
      customUninstall: "uninstall_epic_games_launcher",
    },
  ];

  const customizationApps: LauncherItem[] = [
    {
      key: "openrgb",
      name: "OpenRGB",
      icon: "/assets/icons/gaming/openrgb.png",
      kind: "custom",
      badgeKind: "apt",
      checkPath: "/usr/bin/openrgb",
      customInstall: "install_openrgb",
      customUninstall: "uninstall_openrgb",
    },
    { key: "msi-keyboard-gui", name: "MSI Keyboard RGB", icon: "/assets/icons/gaming/msi-keyboard.png", kind: "apt", package: "msi-keyboard-gui" },
  ];

  const installedMap: Record<string, boolean> = {};
  let installedVersion = 0;

  async function refreshOptimizationStatus(key: string) {
    let installed = false;
    switch (key) {
      case "gamemode":
        installed = await api.invoke<boolean>("is_apt_package_installed", { package: "gamemode" });
        break;
      case "performance-mode":
        installed = await api.invoke<boolean>("is_performance_mode_installed");
        break;
      case "sysctl":
        installed = await api.invoke<boolean>("is_gaming_sysctl_active");
        break;
      case "gpu":
        installed = await api.invoke<boolean>("is_gpu_optimized");
        break;
      case "disk-io":
        installed = await api.invoke<boolean>("is_disk_io_optimized");
        break;
      case "mangohud":
        installed = await api.invoke<boolean>("is_mangohud_installed");
        break;
      case "cpu-power":
        installed = await api.invoke<boolean>("is_apt_package_installed", { package: "cpupower-gui" });
        break;
      case "ryzenadj":
        installed = await api.invoke<boolean>("is_ryzenadj_installed");
        break;
    }
    installedMap[key] = installed;
    installedVersion++;
  }

  async function toggleOptimization(item: OptimizationItem) {
    const installed = installedMap[item.key];
    switch (item.key) {
      case "gamemode":
        await runTracked(installed ? "uninstall_apt_packages" : "install_apt_packages", { packages: "gamemode libgamemode0" });
        break;
      case "performance-mode":
        await runTracked(installed ? "uninstall_performance_mode" : "install_performance_mode");
        break;
      case "sysctl":
        await runTracked(installed ? "revert_gaming_sysctl" : "apply_gaming_sysctl");
        break;
      case "gpu":
        await runTracked(installed ? "revert_gpu_optimization" : "apply_gpu_optimization");
        break;
      case "disk-io":
        await runTracked(installed ? "revert_disk_io_optimization" : "apply_disk_io_optimization");
        break;
      case "mangohud":
        if (!installed) await runTracked("install_mangohud");
        break;
      case "cpu-power":
        await runTracked(installed ? "uninstall_apt_packages" : "install_apt_packages", { packages: "linux-cpupower cpupower-gui" });
        break;
      case "ryzenadj":
        await runTracked(installed ? "uninstall_ryzenadj" : "install_ryzenadj");
        break;
    }
    await refreshOptimizationStatus(item.key);
  }

  let lutrisFixBusy = false;

  async function applyLutrisFix() {
    lutrisFixBusy = true;
    try {
      const [userFlatpak, systemFlatpak, apt] = await Promise.all([
        api.invoke<boolean>("is_lutris_flatpak_user_installed"),
        api.invoke<boolean>("is_lutris_flatpak_system_installed"),
        api.invoke<boolean>("is_apt_package_installed", { package: "lutris" }),
      ]);
      if (!userFlatpak && !systemFlatpak && !apt) {
        infoMessage = $_("gaming.lutrisNotInstalled");
        return;
      }
      if (userFlatpak) await runTracked("fix_lutris_flatpak_user");
      if (systemFlatpak) await runTracked("fix_lutris_flatpak_system");
      if (apt) await runTracked("fix_lutris_apt");
    } finally {
      lutrisFixBusy = false;
    }
  }

  async function revertAll() {
    await runTracked("revert_all_gaming_optimizations");
    for (const item of optimizations) await refreshOptimizationStatus(item.key);
  }

  // === Launchers ===
  async function refreshLauncherStatus(item: LauncherItem) {
    let installed = false;
    if (item.kind === "apt" || item.kind === "deb") {
      installed = await api.invoke<boolean>("is_apt_package_installed", { package: item.package });
    } else if (item.kind === "flatpak") {
      installed = await api.invoke<boolean>("is_flatpak_installed", { appId: item.flatpakId });
    } else if (item.kind === "custom") {
      installed = await api.invoke<boolean>("check_path_exists", { path: item.checkPath });
    }
    installedMap[item.key] = installed;
    installedVersion++;
  }

  // Used only on mount, in place of looping every launcher/customization
  // item with its own invoke — one batched call for all of them (same
  // get_package_statuses Software/Recommended already use) instead of ~15
  // separate dpkg-query/flatpak/path-check processes firing at once.
  async function refreshAllLauncherStatuses() {
    const items = [...launchers, ...customizationApps];
    const apt = new Set<string>();
    const flatpak = new Set<string>();
    const paths = new Set<string>();
    for (const item of items) {
      if ((item.kind === "apt" || item.kind === "deb") && item.package) apt.add(item.package);
      else if (item.kind === "flatpak" && item.flatpakId) flatpak.add(item.flatpakId);
      else if (item.kind === "custom" && item.checkPath) paths.add(item.checkPath);
    }
    try {
      const result = await api.invoke<{
        apt: Record<string, boolean>;
        flatpak: Record<string, boolean>;
        paths: Record<string, boolean>;
      }>("get_package_statuses", { query: { apt: [...apt], flatpak: [...flatpak], paths: [...paths] } });
      for (const item of items) {
        let installed = false;
        if ((item.kind === "apt" || item.kind === "deb") && item.package) installed = result.apt[item.package] ?? false;
        else if (item.kind === "flatpak" && item.flatpakId) installed = result.flatpak[item.flatpakId] ?? false;
        else if (item.kind === "custom" && item.checkPath) installed = result.paths[item.checkPath] ?? false;
        installedMap[item.key] = installed;
      }
      installedVersion++;
    } catch (error) {
      console.error("Failed to read package statuses", error);
    }
  }

  async function toggleLauncher(item: LauncherItem) {
    const installed = installedMap[item.key];
    if (item.kind === "apt") {
      await runTracked(installed ? "uninstall_apt_packages" : "install_apt_packages", { packages: item.package });
    } else if (item.kind === "deb") {
      if (installed) {
        await runTracked("uninstall_apt_packages", { packages: item.package });
      } else {
        await runTracked("install_deb_url", { url: item.debUrl, package: item.package });
      }
    } else if (item.kind === "flatpak") {
      if (installed) {
        const scope = (await api.invoke<string | null>("flatpak_app_scope", { appId: item.flatpakId })) ?? "user";
        await runTracked("uninstall_flatpak_scoped", { appId: item.flatpakId, scope });
      } else {
        const state = await api.invoke<string>("flatpak_state");
        if (state === "missing") {
          infoMessage = $_("softwareTile.flatpakMissing", { values: { name: item.name } });
          return;
        }
        let scope: "user" | "system";
        if (state === "both") {
          scope = await chooseFlatpakScope(item.name);
        } else if (state === "system") {
          scope = "system";
          infoMessage = $_("softwareTile.flatpakSystemOnly", { values: { name: item.name } });
        } else {
          scope = "user";
        }
        await runTracked("install_flatpak_scoped", { appId: item.flatpakId, scope });
        if (item.key === "ppsspp") {
          await runTracked("link_ppsspp_lutris_runner");
        }
      }
    } else if (item.kind === "custom") {
      const command = installed ? item.customUninstall : item.customInstall;
      if (command) await runTracked(command);
    }
    await refreshLauncherStatus(item);
  }

  // === Wallpapers ===
  let desktop: DesktopKind = "xfce";
  let infoMessage = "";
  let flatpakScopeChoice: { name: string; resolve: (scope: "user" | "system") => void } | null = null;

  function chooseFlatpakScope(name: string): Promise<"user" | "system"> {
    return new Promise((resolve) => {
      flatpakScopeChoice = { name, resolve };
    });
  }
  let expanded: string | null = null;
  function toggleFamily(key: string) {
    expanded = expanded === key ? null : key;
  }

  function detectDesktopKind(raw: string): DesktopKind {
    const lower = raw.toLowerCase();
    if (lower.includes("gnome")) return "gnome";
    if (lower.includes("kde") || lower.includes("plasma")) return "kde";
    return "xfce";
  }

  async function installWallpapers() {
    const success = await runTracked("install_gaming_wallpapers", { desktop });
    infoMessage = success
      ? $_("gaming.wallpapersInstalled")
      : $_("gaming.wallpapersFailed");
  }

  onMount(async () => {
    try {
      const info = await api.invoke<{ desktop: string }>("get_system_info");
      desktop = detectDesktopKind(info.desktop);
    } catch (error) {
      console.error("Failed to detect desktop environment", error);
    }
    try {
      hardware = await api.invoke<HardwareSummary>("scan_hardware");
    } catch (error) {
      console.error("Failed to scan hardware", error);
    }
    for (const item of optimizations) refreshOptimizationStatus(item.key);
    refreshAllLauncherStatuses();
  });

  $: activeOptimizationsCount = installedVersion >= 0 && optimizations.filter((item) => installedMap[item.key]).length;
</script>

<div
  class="relative rounded border border-border overflow-hidden mb-5 flex flex-col items-start justify-end text-left p-4 min-h-[200px]"
  style="background: linear-gradient(rgba(20, 20, 22, 0.35), rgba(20, 20, 22, 0.85)), url('/assets/hero/gaming-background.png'); background-size: cover; background-position: center;"
>
  <p class="text-white font-semibold text-[1rem]">{$_("gaming.hero.title")}</p>
  <p class="text-[0.8rem] text-white/85 mt-1">{$_("gaming.hero.description")}</p>
</div>

{#if hardware}
  <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5 mb-4">
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <Cpu size={13} />
        </div>
        CPU
      </div>
      <div class="text-[0.8rem] text-text-bright font-medium truncate mt-1">{hardware.cpu.model}</div>
      <div class="text-[0.7rem] text-text-muted">{$_("drivers.coresThreads", { values: { cores: hardware.cpu.cores, threads: hardware.cpu.threads } })}</div>
    </div>
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <CircuitBoard size={13} />
        </div>
        GPU
      </div>
      {#each hardware.gpus as gpu}
        <div class="text-[0.8rem] text-text-bright font-medium truncate mt-1">{gpu.vendor} {gpu.model}</div>
        <div class="text-[0.7rem] text-text-muted">{gpu.driverStatus}</div>
      {:else}
        <div class="text-[0.8rem] text-text-muted mt-1">{$_("drivers.noneDetected")}</div>
      {/each}
    </div>
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <MemoryStick size={13} />
        </div>
        {$_("drivers.memory")}
      </div>
      <div class="text-[0.8rem] text-text-bright font-medium mt-1">{hardware.memory.total}</div>
      <div class="text-[0.7rem] text-text-muted">{$_("drivers.availableSuffix", { values: { available: hardware.memory.available } })}</div>
    </div>
    <div class="bg-surface border border-border rounded p-3">
      <div class="flex items-center gap-1.5 text-[0.72rem] text-text-muted">
        <div class="w-5 h-5 rounded bg-surface-2 grid place-items-center shrink-0">
          <Zap size={13} />
        </div>
        {$_("gaming.optimizationsTitle")}
      </div>
      <div class="text-[0.8rem] text-text-bright font-medium mt-1">{$_("gaming.optimizationsCount", { values: { active: activeOptimizationsCount, total: optimizations.length } })}</div>
    </div>
  </div>
{/if}

<div class="grid grid-cols-1 md:grid-cols-2 gap-3.5">
  <!-- Optimizations -->
  <div class="bg-surface border border-border rounded p-4 transition-all {expanded === 'optimizations' ? 'md:col-span-2' : ''}">
    <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily("optimizations")}>
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
          <img src="/assets/icons/gaming/gamemode.png" alt="" class="w-full h-full object-contain" />
        </div>
        <div class="text-left">
          <div class="text-text-bright font-semibold text-[0.9rem]">{$_("gaming.optimizationsTitle")}</div>
          <div class="text-[0.75rem] text-text-muted">{$_("gaming.optimizationsSubtitle")}</div>
        </div>
      </div>
      <ChevronDown size={18} class="text-text-muted transition-transform {expanded === 'optimizations' ? 'rotate-180' : ''}" />
    </button>

    {#if expanded === "optimizations"}
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5 mt-4 pt-4 border-t border-border">
        {#each optimizations as item}
          {@const installed = installedVersion >= 0 && installedMap[item.key]}
          <button
            class="text-left border rounded p-3 transition-colors border-border hover:bg-surface-2"
            on:click={() => toggleOptimization(item)}
          >
            <div class="flex items-center gap-2">
              <div class="w-7 h-7 rounded bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
                <img src={item.icon} alt="" class="w-full h-full object-contain" />
              </div>
              <span class="text-[0.85rem] font-semibold text-text-bright">{$_(`gaming.optimizations.${item.key}.label`)}</span>
              <span
                class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 ml-auto {installed
                  ? 'bg-success/15 text-success'
                  : 'bg-surface-2 text-text'}"
              >
                {installed ? $_("drivers.badge.installed") : $_("drivers.badge.notInstalled")}
              </span>
            </div>
            <p class="text-[0.72rem] text-text mt-1.5">{$_(`gaming.optimizations.${item.key}.hint`)}</p>
            {#if !item.noUninstall || !installed}
              <p class="text-[0.68rem] text-primary mt-1">
                {installed ? $_("gaming.clickToRevert") : $_("drivers.clickToInstall")}
              </p>
            {/if}
          </button>
        {/each}

        <!-- Lutris Vulkan Fix (own row: not a toggle, applies to whatever's installed) -->
        <button
          class="text-left border rounded p-3 transition-colors border-border hover:bg-surface-2 disabled:opacity-60"
          disabled={lutrisFixBusy}
          on:click={applyLutrisFix}
        >
          <div class="flex items-center gap-2">
            <div class="w-7 h-7 rounded bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
              <img src="/assets/icons/gaming/lutris.png" alt="" class="w-full h-full object-contain" />
            </div>
            <span class="text-[0.85rem] font-semibold text-text-bright">{$_("gaming.lutrisFixTitle")}</span>
          </div>
          <p class="text-[0.72rem] text-text mt-1.5">{$_("gaming.lutrisFixDescription")}</p>
          <p class="text-[0.68rem] text-primary mt-1">{lutrisFixBusy ? $_("gaming.working") : $_("gaming.clickToApply")}</p>
        </button>
      </div>

      <button
        class="w-full mt-4 flex items-center justify-center gap-2 px-3 py-2 rounded border border-danger text-danger hover:bg-danger/10 text-[0.82rem] font-semibold"
        on:click={revertAll}
      >
        <Undo2 size={16} />
        {$_("gaming.revertAll")}
      </button>
    {/if}
  </div>

  <!-- Launchers -->
  <div class="bg-surface border border-border rounded p-4 transition-all {expanded === 'launchers' ? 'md:col-span-2' : ''}">
    <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily("launchers")}>
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
          <img src="/assets/icons/gaming/steam.png" alt="" class="w-full h-full object-contain" />
        </div>
        <div class="text-left">
          <div class="text-text-bright font-semibold text-[0.9rem]">{$_("gaming.launchersTitle")}</div>
          <div class="text-[0.75rem] text-text-muted">{$_("gaming.launchersSubtitle", { values: { count: launchers.length } })}</div>
        </div>
      </div>
      <ChevronDown size={18} class="text-text-muted transition-transform {expanded === 'launchers' ? 'rotate-180' : ''}" />
    </button>

    {#if expanded === "launchers"}
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3 mt-4 pt-4 border-t border-border">
        {#each launchers as item}
          {@const installed = installedVersion >= 0 && installedMap[item.key]}
          <button
            class="flex flex-col items-center text-center gap-1.5 border border-border rounded p-3 hover:bg-surface-2 transition-colors"
            on:click={() => toggleLauncher(item)}
          >
            <img src={item.icon} alt="" class="w-10 h-10 object-contain" />
            <span class="text-[0.8rem] font-semibold text-text-bright">{item.name}</span>
            <span class="text-[0.6rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted">{badgeLabels[launcherBadgeKind(item)]}</span>
            <span class="text-[0.68rem] text-text">{$_(`gaming.launchers.${item.key}`)}</span>
            <span
              class="text-[0.66rem] px-2 py-0.5 rounded-full mt-1 {installed
                ? 'bg-success/15 text-success'
                : 'bg-surface-2 text-text'}"
            >
              {installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Customization -->
  <div class="bg-surface border border-border rounded p-4 transition-all {expanded === 'customization' ? 'md:col-span-2' : ''}">
    <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily("customization")}>
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
          <img src="/assets/icons/gaming/openrgb.png" alt="" class="w-full h-full object-contain" />
        </div>
        <div class="text-left">
          <div class="text-text-bright font-semibold text-[0.9rem]">{$_("app.tabs.customization.label")}</div>
          <div class="text-[0.75rem] text-text-muted">{$_("gaming.customizationSubtitle")}</div>
        </div>
      </div>
      <ChevronDown size={18} class="text-text-muted transition-transform {expanded === 'customization' ? 'rotate-180' : ''}" />
    </button>

    {#if expanded === "customization"}
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3 mt-4 pt-4 border-t border-border">
        <button
          class="flex flex-col items-center text-center gap-1.5 border border-border rounded p-3 hover:bg-surface-2 transition-colors"
          on:click={installWallpapers}
        >
          <img src="/assets/icons/customization/wallpaper.svg" alt="" class="w-10 h-10 object-contain" />
          <span class="text-[0.8rem] font-semibold text-text-bright">{$_("gaming.wallpapersTitle")}</span>
          <span class="text-[0.68rem] text-text">{$_("gaming.wallpapersDescription")}</span>
          <span class="text-[0.66rem] px-2 py-0.5 rounded-full mt-1 bg-surface-2 text-text">{$_("drivers.clickToInstall")}</span>
        </button>

        {#each customizationApps as item}
            {@const installed = installedVersion >= 0 && installedMap[item.key]}
            <button
              class="flex flex-col items-center text-center gap-1.5 border border-border rounded p-3 hover:bg-surface-2 transition-colors"
              on:click={() => toggleLauncher(item)}
            >
              <img src={item.icon} alt="" class="w-10 h-10 object-contain" />
              <span class="text-[0.8rem] font-semibold text-text-bright">{item.name}</span>
              <span class="text-[0.6rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted">{badgeLabels[launcherBadgeKind(item)]}</span>
              <span class="text-[0.68rem] text-text">{$_(`gaming.launchers.${item.key}`)}</span>
              <span
                class="text-[0.66rem] px-2 py-0.5 rounded-full mt-1 {installed
                  ? 'bg-success/15 text-success'
                  : 'bg-surface-2 text-text'}"
              >
                {installed ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
              </span>
            </button>
          {/each}
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
  <Modal onClose={() => (infoMessage = "")}>
    <p class="whitespace-pre-line text-text-bright">{infoMessage}</p>
  </Modal>
{/if}

{#if flatpakScopeChoice}
  <FlatpakScopeModal
    name={flatpakScopeChoice.name}
    onChoose={(scope) => {
      flatpakScopeChoice?.resolve(scope);
      flatpakScopeChoice = null;
    }}
    onClose={() => (flatpakScopeChoice = null)}
  />
{/if}
