<script lang="ts">
  import Modal from "./Modal.svelte";
  import { api } from "../lib/api";
  import { _ } from "svelte-i18n";
  import { runConfirmableUpdate, runTracked, type UpgradeSelection } from "../lib/progress";

  interface UpgradableSet {
    apt: string[];
    flatpakUser: string[];
    flatpakSystem: string[];
    snap: string[];
  }

  type Source = "apt" | "flatpakUser" | "flatpakSystem" | "snap";

  let sourceLabel: Record<Source, string>;
  $: sourceLabel = {
    apt: $_("update.sourceApt"),
    flatpakUser: $_("update.sourceFlatpakUser"),
    flatpakSystem: $_("update.sourceFlatpakSystem"),
    snap: $_("update.sourceSnap"),
  };

  interface UpdateEntry {
    key: string;
    source: Source;
    name: string;
  }

  let busy = "";
  let updates: UpgradableSet = { apt: [], flatpakUser: [], flatpakSystem: [], snap: [] };
  let entries: UpdateEntry[] = [];
  let selected = new Set<string>();
  let showList = false;
  let infoMessage = "";
  // Resolves the popup with the root-side selection plus the user-scope
  // Flatpak ids (those run afterwards, unprivileged), or null when cancelled.
  let confirmResolver: ((result: { selection: UpgradeSelection; flatpakUser: string[] } | null) => void) | null = null;

  function buildEntries(set: UpgradableSet): UpdateEntry[] {
    const list: UpdateEntry[] = [];
    for (const source of ["apt", "flatpakUser", "flatpakSystem", "snap"] as Source[]) {
      for (const name of set[source]) list.push({ key: `${source}:${name}`, source, name });
    }
    return list;
  }

  // Runs "apt update" and then, under that SAME pkexec authentication
  // (started by start_update_session(), so confirming never asks for the
  // password again), the root-side updates the user picked: APT (plain
  // "apt upgrade -y" if nothing was deselected, like 2.x — apt itself holds
  // back conflicting packages on a plain upgrade but rejects a whole explicit
  // list over one unresolvable package), Snap and system Flatpaks. User-scope
  // Flatpaks then update unprivileged, appended to the same terminal.
  async function onUpdate() {
    busy = "update";
    let attempted = false;
    let flatpakUserPicked: string[] = [];

    const success = await runConfirmableUpdate("start_update_session", async () => {
      try {
        updates = await api.invoke<UpgradableSet>("check_upgradable");
      } catch (error) {
        infoMessage = $_("update.failed", { values: { error: String(error) } });
        return null;
      }
      entries = buildEntries(updates);
      if (entries.length === 0) {
        infoMessage = $_("update.upToDate");
        return null;
      }
      selected = new Set(entries.map((e) => e.key));
      showList = true;
      const result = await new Promise<{ selection: UpgradeSelection; flatpakUser: string[] } | null>((resolve) => {
        confirmResolver = resolve;
      });
      if (!result) return null;
      attempted = true;
      flatpakUserPicked = result.flatpakUser;
      return result.selection;
    });

    let flatpakUserOk = true;
    if (attempted && flatpakUserPicked.length > 0) {
      flatpakUserOk = await runTracked("update_flatpak_user", { ids: flatpakUserPicked }, { append: true });
    }

    showList = false;
    busy = "";
    if (attempted) {
      const ok = success && flatpakUserOk;
      infoMessage = ok ? $_("update.selectedInstalled") : $_("update.selectedFailed");
      if (ok) {
        entries = [];
      }
    }
  }

  function toggleSelected(key: string) {
    if (selected.has(key)) {
      selected.delete(key);
    } else {
      selected.add(key);
    }
    selected = selected;
  }

  function cancelConfirmation() {
    showList = false;
    confirmResolver?.(null);
    confirmResolver = null;
  }

  function confirmSelected() {
    if (selected.size === 0) {
      cancelConfirmation();
      return;
    }
    showList = false;
    const pick = (source: Source) => updates[source].filter((name) => selected.has(`${source}:${name}`));
    const apt = pick("apt");
    const selection: UpgradeSelection = {
      aptAll: updates.apt.length > 0 && apt.length === updates.apt.length,
      apt: apt.length === updates.apt.length ? [] : apt,
      snap: pick("snap"),
      flatpakSystem: pick("flatpakSystem"),
    };
    confirmResolver?.({ selection, flatpakUser: pick("flatpakUser") });
    confirmResolver = null;
  }

  // Standalone upgrade, independent of the Update flow above — its own
  // pkexec authentication, since it isn't chained from an "apt update" step.
  // Root part: APT + Snap + system Flatpak; then user Flatpaks unprivileged.
  async function onUpgradeDirect() {
    busy = "upgrade";
    let success = await runTracked("upgrade_system");
    try {
      if ((await api.invoke<string>("flatpak_state")) !== "missing") {
        const ok = await runTracked("update_flatpak_user", { ids: null }, { append: true });
        success = success && ok;
      }
    } catch (error) {
      console.error("Failed to update user Flatpaks", error);
      success = false;
    }
    infoMessage = success ? $_("update.systemUpgraded") : $_("update.upgradeFailed");
    busy = "";
  }
</script>

<div class="contents">
  <button
    class="absolute bottom-4 left-4 bg-primary hover:bg-primary-hover text-text-bright px-3.5 py-1.5 rounded font-semibold text-[0.82rem]"
    disabled={busy === "update"}
    on:click={onUpdate}
  >
    {busy === "update" ? $_("update.updating") : $_("update.update")}
  </button>
  <button
    class="absolute bottom-4 right-4 bg-primary hover:bg-primary-hover text-text-bright px-3.5 py-1.5 rounded font-semibold text-[0.82rem]"
    disabled={busy === "upgrade"}
    on:click={onUpgradeDirect}
  >
    {busy === "upgrade" ? $_("update.upgrading") : $_("update.upgrade")}
  </button>
</div>

{#if infoMessage}
  <Modal onClose={() => (infoMessage = "")}>
    <p class="text-text-bright">{infoMessage}</p>
  </Modal>
{/if}

{#if showList}
  <Modal
    variant="confirm"
    title={$_("update.updatesAvailable", { values: { count: entries.length } })}
    confirmLabel={$_("update.upgradeSelected", { values: { count: selected.size } })}
    confirmDisabled={selected.size === 0}
    maxWidth={480}
    onClose={cancelConfirmation}
    onConfirm={confirmSelected}
  >
    <div class="content-scroll overflow-y-auto max-h-[260px] flex flex-col gap-1 rounded border border-border p-2">
      {#each entries as entry (entry.key)}
        <label class="flex items-center gap-2.5 text-[0.82rem] text-text cursor-pointer py-1 px-1 rounded hover:bg-surface-2">
          <input type="checkbox" checked={selected.has(entry.key)} on:change={() => toggleSelected(entry.key)} />
          <span class="min-w-0 truncate">{entry.name}</span>
          {#if entry.source !== "apt"}
            <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted shrink-0">{sourceLabel[entry.source]}</span>
          {/if}
        </label>
      {/each}
    </div>
  </Modal>
{/if}
