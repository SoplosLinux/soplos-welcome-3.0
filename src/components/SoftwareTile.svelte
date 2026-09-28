<script lang="ts">
  import Modal from "./Modal.svelte";
  import FlatpakScopeModal from "./FlatpakScopeModal.svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { runTracked } from "../lib/progress";
  import { _ } from "svelte-i18n";

  export let name: string;
  export let packages: string;
  export let icon: string;
  // Shown as a confirm popup before installing (not uninstalling) — for
  // apps whose real dependency pull is much heavier than their name
  // suggests (see Discover: 62 packages including KDE Connect, kaccounts
  // and a full QtWebEngine, verified with a real `apt install -s` run).
  export let warning: string | undefined = undefined;

  let confirmingWarning = false;
  let infoMessage = "";
  let flatpakScopeChoice: { resolve: (scope: "user" | "system") => void } | null = null;

  function chooseFlatpakScope(): Promise<"user" | "system"> {
    return new Promise((resolve) => {
      flatpakScopeChoice = { resolve };
    });
  }

  const mainPackage = packages.split(" ")[0];
  const isRepoSelector = mainPackage === "soplos-repo-selector";
  const isSnap = mainPackage.startsWith("snap:");
  const isFlatpak = mainPackage.startsWith("flatpak:");
  const snapName = isSnap ? mainPackage.slice(5) : "";
  const flatpakId = isFlatpak ? mainPackage.slice(8) : "";

  let installed = false;
  let checked = false;
  let busy = false;

  async function refresh() {
    try {
      if (isSnap) {
        installed = await api.invoke<boolean>("is_snap_installed", { name: snapName });
      } else if (isFlatpak) {
        installed = await api.invoke<boolean>("is_flatpak_installed", { appId: flatpakId });
      } else {
        installed = await api.invoke<boolean>("is_apt_package_installed", { package: mainPackage });
      }
    } catch (error) {
      console.error(`Failed to check status for ${name}`, error);
    } finally {
      checked = true;
    }
  }

  // Flathub can have a user remote, a system remote, or both — flatpak
  // install/uninstall without --user/--system then has to ask interactively
  // which one to use, and since this runs as a non-interactive subprocess
  // with no stdin attached, it just hangs forever instead of ever finishing.
  async function toggleFlatpak() {
    if (installed) {
      const scope = (await api.invoke<string | null>("flatpak_app_scope", { appId: flatpakId })) ?? "user";
      await runTracked("uninstall_flatpak_scoped", { appId: flatpakId, scope });
      return;
    }
    const state = await api.invoke<string>("flatpak_state");
    if (state === "missing") {
      infoMessage = $_("softwareTile.flatpakMissing", { values: { name } });
      return;
    }
    let scope: "user" | "system";
    if (state === "both") {
      scope = await chooseFlatpakScope();
    } else if (state === "system") {
      scope = "system";
      infoMessage = $_("softwareTile.flatpakSystemOnly", { values: { name } });
    } else {
      scope = "user";
    }
    await runTracked("install_flatpak_scoped", { appId: flatpakId, scope });
  }

  async function toggle() {
    busy = true;
    try {
      if (isSnap) {
        await runTracked(installed ? "uninstall_snap" : "install_snap", { name: snapName });
      } else if (isFlatpak) {
        await toggleFlatpak();
      } else {
        await runTracked(installed ? "uninstall_apt_packages" : "install_apt_packages", { packages });
      }
      await refresh();
    } finally {
      busy = false;
    }
  }

  function onInstallClick() {
    if (!installed && warning) {
      confirmingWarning = true;
      return;
    }
    toggle();
  }

  function confirmWarning() {
    confirmingWarning = false;
    toggle();
  }

  async function openRepoSelector() {
    try {
      await api.invoke("open_repo_selector");
    } catch (error) {
      console.error("Failed to open repo selector", error);
    }
  }

  onMount(() => {
    if (!isRepoSelector) {
      refresh();
    }
  });
</script>

<div class="h-full flex flex-col items-center gap-2 bg-surface border border-border rounded p-3 text-center">
  <img src={icon} alt={name} class="w-11 h-11 object-contain" />
  <span class="text-[0.8rem] text-text-bright font-medium leading-snug">{name}</span>

  <div class="w-full mt-auto">
    {#if isRepoSelector}
      <button
        class="w-full bg-primary hover:bg-primary-hover text-text-bright px-3 py-1.5 rounded font-semibold text-[0.78rem]"
        on:click={openRepoSelector}
      >
        {$_("common.open")}
      </button>
    {:else if !checked}
      <span class="text-[0.72rem] text-text-muted py-1.5 block">{$_("common.checking")}</span>
    {:else}
      <button
        class="w-full px-3 py-1.5 rounded font-semibold text-[0.78rem] {installed
          ? 'bg-danger/15 text-danger hover:bg-danger/25'
          : 'bg-primary hover:bg-primary-hover text-text-bright'}"
        disabled={busy}
        on:click={onInstallClick}
      >
        {busy ? "…" : installed ? $_("common.uninstall") : $_("common.install")}
      </button>
    {/if}
  </div>
</div>

{#if confirmingWarning}
  <Modal
    variant="warning"
    title={$_("softwareTile.installQuestion", { values: { name } })}
    confirmLabel={$_("softwareTile.installAnyway")}
    onClose={() => (confirmingWarning = false)}
    onConfirm={confirmWarning}
  >
    {warning}
  </Modal>
{/if}

{#if infoMessage}
  <Modal onClose={() => (infoMessage = "")}>
    <p class="whitespace-pre-line text-text-bright">{infoMessage}</p>
  </Modal>
{/if}

{#if flatpakScopeChoice}
  <FlatpakScopeModal
    {name}
    onChoose={(scope) => {
      flatpakScopeChoice?.resolve(scope);
      flatpakScopeChoice = null;
    }}
    onClose={() => (flatpakScopeChoice = null)}
  />
{/if}
