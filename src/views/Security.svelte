<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { api } from "../lib/api";
  import { runTracked, terminalExpanded, terminalLines } from "../lib/progress";
  import TerminalOutput from "../components/TerminalOutput.svelte";
  import Modal from "../components/Modal.svelte";
  import FlatpakScopeModal from "../components/FlatpakScopeModal.svelte";
  import { _ } from "svelte-i18n";
  import { ChevronDown } from "lucide-svelte";

  interface SecurityItem {
    key: string;
    label: string;
    hintKey: string;
    icon: string;
    badgeKey?: "recommended" | "advanced" | "alternative" | "kde";
    kind: "apt" | "flatpak" | "custom";
    packages?: string;
    checkPackage?: string;
    flatpakId?: string;
    openCommand?: string;
    openArgs?: string[];
    openPrivileged?: boolean;
    customCheckCmd?: string;
    customInstallCmd?: string;
    customUninstallCmd?: string;
    onlyBtrfs?: boolean;
    extra?: "clamtk" | "rkhunter";
  }

  interface Family {
    key: string;
    icon: string;
    items: SecurityItem[];
  }

  let filesystem = "unknown";

  const families: Family[] = [
    {
      key: "backups",
      icon: "/assets/icons/security/timeshift.png",
      items: [
        { key: "timeshift", label: "Timeshift", hintKey: "timeshift", icon: "/assets/icons/security/timeshift.png", badgeKey: "recommended", kind: "apt", packages: "timeshift", openCommand: "timeshift-gtk", openPrivileged: true },
        { key: "grub-btrfs", label: "Grub BTRFS", hintKey: "grubBtrfs", icon: "/assets/icons/security/grub-btrfs.png", kind: "apt", packages: "grub-btrfs", onlyBtrfs: true },
        { key: "deja-dup", label: "Deja Dup", hintKey: "dejaDup", icon: "/assets/icons/security/dejadup.png", kind: "apt", packages: "deja-dup" },
        { key: "btrfs-assistant", label: "BTRFS Assistant", hintKey: "btrfsAssistant", icon: "/assets/icons/security/btrfs-assistant.png", kind: "apt", packages: "btrfs-assistant", onlyBtrfs: true, openCommand: "btrfs-assistant", openPrivileged: true },
      ],
    },
    {
      key: "firewall",
      icon: "/assets/icons/security/gufw.png",
      items: [
        { key: "gufw", label: "GUFW", hintKey: "gufw", icon: "/assets/icons/security/gufw.png", badgeKey: "recommended", kind: "apt", packages: "gufw", openCommand: "gufw" },
        { key: "portmaster", label: "Portmaster", hintKey: "portmaster", icon: "/assets/icons/security/portmaster.png", badgeKey: "advanced", kind: "custom", customCheckCmd: "is_portmaster_installed", customInstallCmd: "install_portmaster", customUninstallCmd: "uninstall_portmaster", openCommand: "portmaster" },
      ],
    },
    {
      key: "vpn",
      icon: "/assets/icons/security/proton-vpn.png",
      items: [
        { key: "protonvpn", label: "Proton VPN", hintKey: "protonvpn", icon: "/assets/icons/security/proton-vpn.png", kind: "flatpak", flatpakId: "com.protonvpn.www" },
        { key: "surfshark", label: "Surfshark", hintKey: "surfshark", icon: "/assets/icons/security/surfshark.png", kind: "flatpak", flatpakId: "com.surfshark.Surfshark" },
        { key: "mozillavpn", label: "Mozilla VPN", hintKey: "mozillavpn", icon: "/assets/icons/security/mozilla-vpn.png", kind: "flatpak", flatpakId: "org.mozilla.vpn" },
        { key: "vpnunlimited", label: "VPN Unlimited", hintKey: "vpnunlimited", icon: "/assets/icons/security/vpn-unlimited.png", kind: "flatpak", flatpakId: "com.keepsolid.VpnUnlimited" },
        { key: "cloudflarewarp", label: "Cloudflare WARP", hintKey: "cloudflarewarp", icon: "/assets/icons/security/one.png", kind: "custom", checkPackage: "cloudflare-warp", customInstallCmd: "install_cloudflare_warp", customUninstallCmd: "uninstall_cloudflare_warp", openCommand: "warp-taskbar" },
        { key: "nordvpn", label: "NordVPN", hintKey: "nordvpn", icon: "/assets/icons/security/nordvpn.png", kind: "custom", checkPackage: "nordvpn-gui", customInstallCmd: "install_nordvpn", customUninstallCmd: "uninstall_nordvpn", openCommand: "nordvpn-gui" },
      ],
    },
    {
      key: "cleaning",
      icon: "/assets/icons/security/bleachbit.png",
      items: [
        { key: "bleachbit", label: "BleachBit", hintKey: "bleachbit", icon: "/assets/icons/security/bleachbit.png", kind: "apt", packages: "bleachbit", openCommand: "bleachbit" },
        { key: "stacer", label: "Stacer", hintKey: "stacer", icon: "/assets/icons/security/stacer.png", badgeKey: "alternative", kind: "custom", customCheckCmd: "is_stacer_installed", customInstallCmd: "install_stacer", customUninstallCmd: "uninstall_stacer", openCommand: "open_stacer" },
        { key: "sweeper", label: "Sweeper", hintKey: "sweeper", icon: "/assets/icons/security/sweeper.png", badgeKey: "kde", kind: "apt", packages: "sweeper", openCommand: "sweeper" },
        { key: "soplos-sys-cleaner", label: "Soplos Sys Cleaner", hintKey: "soplosSysCleaner", icon: "/assets/icons/security/soplos-sys-cleaner.png", badgeKey: "recommended", kind: "apt", packages: "soplos-sys-cleaner", openCommand: "soplos-sys-cleaner" },
        { key: "kudu", label: "Kudu", hintKey: "kudu", icon: "/assets/icons/security/kudu.png", badgeKey: "alternative", kind: "custom", customCheckCmd: "is_kudu_installed", customInstallCmd: "install_kudu", customUninstallCmd: "uninstall_kudu", openCommand: "/opt/Kudu/kudu" },
      ],
    },
    {
      key: "antivirus",
      icon: "/assets/icons/security/clamtk.png",
      items: [
        { key: "clamtk", label: "ClamTk", hintKey: "clamtk", icon: "/assets/icons/security/clamtk.png", badgeKey: "recommended", kind: "apt", packages: "clamav clamtk", checkPackage: "clamtk", extra: "clamtk" },
        { key: "clamui", label: "ClamUI", hintKey: "clamui", icon: "/assets/icons/security/clamui.png", kind: "flatpak", flatpakId: "io.github.linx_systems.ClamUI" },
        { key: "rkhunter", label: "rkhunter", hintKey: "rkhunter", icon: "/assets/icons/security/rkhunter.png", badgeKey: "advanced", kind: "apt", packages: "rkhunter", extra: "rkhunter" },
      ],
    },
  ];

  const installedMap: Record<string, boolean> = {};
  let installedVersion = 0;
  let infoMessage = "";
  let flatpakScopeChoice: { name: string; resolve: (scope: "user" | "system") => void } | null = null;

  function chooseFlatpakScope(name: string): Promise<"user" | "system"> {
    return new Promise((resolve) => {
      flatpakScopeChoice = { name, resolve };
    });
  }

  async function refreshItemStatus(item: SecurityItem) {
    let installed = false;
    if (item.kind === "apt") {
      installed = await api.invoke<boolean>("is_apt_package_installed", { package: item.checkPackage ?? item.packages!.split(" ")[0] });
    } else if (item.kind === "flatpak") {
      installed = await api.invoke<boolean>("is_flatpak_installed", { appId: item.flatpakId });
    } else if (item.kind === "custom") {
      installed = item.customCheckCmd
        ? await api.invoke<boolean>(item.customCheckCmd)
        : await api.invoke<boolean>("is_apt_package_installed", { package: item.checkPackage });
    }
    installedMap[item.key] = installed;
    installedVersion++;
  }

  // Used on mount, in place of looping every item with its own invoke (~28
  // separate dpkg-query/flatpak processes firing almost simultaneously on a
  // VM with few cores is exactly what made this tab feel slow on every
  // visit). apt and flatpak checks collapse into the same single batched
  // call Software/Recommended already use; only the handful of truly custom
  // checks (no apt/flatpak equivalent) still run individually, in parallel
  // via Promise.all instead of a loop.
  async function refreshAllStatuses() {
    const apt: string[] = [];
    const flatpak: string[] = [];
    const customItems: SecurityItem[] = [];
    for (const family of families) {
      for (const item of family.items) {
        if (item.kind === "apt") {
          apt.push(item.checkPackage ?? item.packages!.split(" ")[0]);
        } else if (item.kind === "flatpak" && item.flatpakId) {
          flatpak.push(item.flatpakId);
        } else if (item.kind === "custom" && !item.customCheckCmd && item.checkPackage) {
          apt.push(item.checkPackage);
        } else if (item.kind === "custom") {
          customItems.push(item);
        }
      }
    }
    try {
      const result = await api.invoke<{ apt: Record<string, boolean>; flatpak: Record<string, boolean>; paths: Record<string, boolean> }>(
        "get_package_statuses",
        { query: { apt, flatpak, paths: [] } },
      );
      for (const family of families) {
        for (const item of family.items) {
          if (item.kind === "apt") {
            installedMap[item.key] = result.apt[item.checkPackage ?? item.packages!.split(" ")[0]] ?? false;
          } else if (item.kind === "flatpak" && item.flatpakId) {
            installedMap[item.key] = result.flatpak[item.flatpakId] ?? false;
          } else if (item.kind === "custom" && !item.customCheckCmd && item.checkPackage) {
            installedMap[item.key] = result.apt[item.checkPackage] ?? false;
          }
        }
      }
      installedVersion++;
    } catch (error) {
      console.error("Failed to read package statuses", error);
    }
    await Promise.all(customItems.map((item) => refreshItemStatus(item)));
  }

  async function toggleItem(item: SecurityItem) {
    const installed = installedMap[item.key];
    if (item.kind === "apt") {
      await runTracked(installed ? "uninstall_apt_packages" : "install_apt_packages", { packages: item.packages });
    } else if (item.kind === "flatpak") {
      if (installed) {
        const scope = (await api.invoke<string | null>("flatpak_app_scope", { appId: item.flatpakId })) ?? "user";
        await runTracked("uninstall_flatpak_scoped", { appId: item.flatpakId, scope });
      } else {
        const state = await api.invoke<string>("flatpak_state");
        if (state === "missing") {
          infoMessage = $_("softwareTile.flatpakMissing", { values: { name: item.label } });
          return;
        }
        let scope: "user" | "system";
        if (state === "both") {
          scope = await chooseFlatpakScope(item.label);
        } else if (state === "system") {
          scope = "system";
          infoMessage = $_("softwareTile.flatpakSystemOnly", { values: { name: item.label } });
        } else {
          scope = "user";
        }
        await runTracked("install_flatpak_scoped", { appId: item.flatpakId, scope });
      }
    } else if (item.kind === "custom") {
      const command = installed ? item.customUninstallCmd : item.customInstallCmd;
      if (command) await runTracked(command);
    }
    await refreshItemStatus(item);
  }

  async function openItem(item: SecurityItem) {
    if (!item.openCommand) return;
    try {
      if (item.openCommand === "open_stacer") {
        await api.invoke("open_stacer");
      } else if (item.openPrivileged) {
        await api.invoke("launch_privileged_app", { program: item.openCommand });
      } else {
        await api.invoke("launch_app", { program: item.openCommand, args: item.openArgs ?? [] });
      }
    } catch (error) {
      console.error(`Failed to launch ${item.openCommand}`, error);
    }
  }

  async function openFlatpak(item: SecurityItem) {
    if (!item.flatpakId) return;
    await api.invoke("launch_app", { program: "flatpak", args: ["run", item.flatpakId] });
  }

  let expanded: string | null = null;
  function toggleFamily(key: string) {
    expanded = expanded === key ? null : key;
  }

  // === UFW live status, polled every 3s while the tab is open ===
  let ufwActive = false;
  let ufwTimer: ReturnType<typeof setInterval> | undefined;

  async function refreshUfwStatus() {
    ufwActive = await api.invoke<boolean>("is_ufw_active");
  }

  async function toggleUfw() {
    await runTracked(ufwActive ? "disable_ufw" : "enable_ufw");
    await refreshUfwStatus();
  }

  async function updateClamavDefinitions() {
    await runTracked("update_clamav_definitions");
  }

  async function scanRkhunter() {
    await runTracked("scan_rkhunter");
  }

  onMount(() => {
    api.invoke<string>("detect_filesystem").then((fs) => (filesystem = fs));
    refreshAllStatuses();
    refreshUfwStatus();
    ufwTimer = setInterval(refreshUfwStatus, 3000);
  });

  onDestroy(() => {
    if (ufwTimer) clearInterval(ufwTimer);
  });
</script>

<div
  class="relative rounded border border-border overflow-hidden mb-5 flex flex-col items-start justify-end text-left p-4 min-h-[200px]"
  style="background: linear-gradient(rgba(20, 20, 22, 0.35), rgba(20, 20, 22, 0.85)), url('/assets/hero/security-background.png'); background-size: cover; background-position: center;"
>
  <p class="text-white font-semibold text-[1rem]">{$_("security.hero.title")}</p>
  <p class="text-[0.8rem] text-white/85 mt-1">{$_("security.hero.description")}</p>
</div>

<div class="grid grid-cols-1 md:grid-cols-2 gap-3.5">
  {#each families as family}
    <div
      class="bg-surface border border-border rounded p-4 transition-all {expanded === family.key ? 'md:col-span-2' : ''}"
    >
      <button class="w-full flex items-center justify-between gap-3" on:click={() => toggleFamily(family.key)}>
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
            <img src={family.icon} alt="" class="w-full h-full object-contain" />
          </div>
          <div class="text-left">
            <div class="text-text-bright font-semibold text-[0.9rem]">{$_(`security.families.${family.key}`)}</div>
            {#if family.key === "firewall"}
              <div class="text-[0.75rem] {ufwActive ? 'text-success' : 'text-danger'}">
                UFW: {ufwActive ? $_("security.active") : $_("security.inactive")}
              </div>
            {:else}
              <div class="text-[0.75rem] text-text-muted">{$_("security.toolsAvailable", { values: { count: family.items.length } })}</div>
            {/if}
          </div>
        </div>
        <ChevronDown size={18} class="text-text-muted transition-transform {expanded === family.key ? 'rotate-180' : ''}" />
      </button>

      {#if expanded === family.key}
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5 mt-4 pt-4 border-t border-border">
          {#if family.key === "firewall"}
            <div class="border border-border rounded p-3 sm:col-span-2 flex items-center justify-between gap-2">
              <div>
                <p class="text-[0.85rem] font-semibold text-text-bright">{$_("security.firewallStatus")}</p>
                <p class="text-[0.72rem] {ufwActive ? 'text-success' : 'text-danger'} font-semibold">
                  {ufwActive ? $_("security.active") : $_("security.inactive")}
                </p>
              </div>
              <button
                class="px-3 py-1.5 rounded text-[0.78rem] font-semibold shrink-0 {ufwActive
                  ? 'border border-danger text-danger hover:bg-danger/10'
                  : 'bg-primary hover:bg-primary-hover text-text-bright'}"
                on:click={toggleUfw}
              >
                {ufwActive ? $_("security.deactivate") : $_("security.activate")}
              </button>
            </div>
          {/if}

          {#each family.items as item}
            {@const wired = !item.onlyBtrfs || filesystem === "btrfs"}
            {@const installed = installedVersion >= 0 && installedMap[item.key]}
            <div
              class="border rounded p-3 transition-colors {wired ? 'border-border' : 'border-border opacity-50'}"
            >
              <div class="flex items-center justify-between gap-2 flex-wrap">
                <div class="flex items-center gap-2 min-w-0">
                  <div class="w-7 h-7 rounded bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
                    <img src={item.icon} alt="" class="w-full h-full object-contain" />
                  </div>
                  <span class="text-[0.85rem] font-semibold text-text-bright">
                    {item.label}
                    {#if item.badgeKey}<span class="text-text-muted font-normal">({$_(`security.badges.${item.badgeKey}`)})</span>{/if}
                  </span>
                </div>
                {#if !wired}
                  <span class="text-[0.68rem] px-2 py-0.5 rounded-full bg-surface-2 text-text shrink-0">
                    {$_("security.notAvailableOn", { values: { fs: filesystem.toUpperCase() } })}
                  </span>
                {:else}
                  <span
                    class="text-[0.68rem] px-2 py-0.5 rounded-full shrink-0 {installed
                      ? 'bg-success/15 text-success'
                      : 'bg-surface-2 text-text'}"
                  >
                    {installed ? $_("security.installed") : $_("security.notInstalled")}
                  </span>
                {/if}
              </div>
              <p class="text-[0.72rem] text-text mt-1">{$_(`security.items.${item.hintKey}`)}</p>

              {#if wired}
                <div class="flex flex-wrap gap-2 mt-2">
                  <button
                    class="px-2.5 py-1.5 rounded text-[0.75rem] font-semibold {installed
                      ? 'border border-danger text-danger hover:bg-danger/10'
                      : 'bg-primary hover:bg-primary-hover text-text-bright'}"
                    on:click={() => toggleItem(item)}
                  >
                    {installed ? $_("common.uninstall") : $_("common.install")}
                  </button>
                  {#if installed && item.openCommand}
                    <button
                      class="px-2.5 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.75rem] font-semibold"
                      on:click={() => openItem(item)}
                    >
                      {$_("common.open")}
                    </button>
                  {/if}
                  {#if installed && item.kind === "flatpak"}
                    <button
                      class="px-2.5 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.75rem] font-semibold"
                      on:click={() => openFlatpak(item)}
                    >
                      {$_("common.open")}
                    </button>
                  {/if}
                  {#if installed && item.extra === "clamtk"}
                    <button
                      class="px-2.5 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.75rem] font-semibold"
                      on:click={updateClamavDefinitions}
                    >
                      {$_("security.updateDefinitions")}
                    </button>
                  {/if}
                  {#if installed && item.extra === "rkhunter"}
                    <button
                      class="px-2.5 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.75rem] font-semibold"
                      on:click={scanRkhunter}
                    >
                      {$_("security.scanSystem")}
                    </button>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
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
