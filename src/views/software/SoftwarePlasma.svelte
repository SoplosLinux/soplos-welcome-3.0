<script lang="ts">
  import FlathubModal from "../../components/FlathubModal.svelte";
  import { onMount } from "svelte";
  import { api } from "../../lib/api";
  import { _ } from "svelte-i18n";
  import SoftwareTile from "../../components/SoftwareTile.svelte";
  import { runTracked, terminalExpanded } from "../../lib/progress";

  interface SoftwareApp {
    name: string;
    packages: string;
    icon: string;
    warning?: string;
  }

  const appsBase: Omit<SoftwareApp, "warning">[] = [
    { name: "Repo Selector", packages: "soplos-repo-selector", icon: "/assets/icons/software/reposelector.png" },
    {
      name: "Soplos Software Center",
      packages: "soplos-software-center",
      icon: "/assets/icons/software/soplos-software-center.png",
    },
    { name: "Synaptic", packages: "synaptic", icon: "/assets/icons/software/synaptic.png" },
    { name: "Gdebi", packages: "gdebi gdebi-core", icon: "/assets/icons/software/gdebi.png" },
    {
      name: "GNOME Software",
      packages: "gnome-software gnome-software-plugin-flatpak gnome-software-plugin-snap",
      icon: "/assets/icons/software/gnome-software.png",
    },
    { name: "Snap Store", packages: "snap:snap-store", icon: "/assets/icons/software/snap-store.png" },
    {
      name: "Flatpak",
      packages: "flatpak plasma-discover-backend-flatpak",
      icon: "/assets/icons/software/flatpak.png",
    },
    {
      name: "Snap",
      packages: "snapd plasma-discover-backend-snap",
      icon: "/assets/icons/software/snap.png",
    },
    { name: "Bazaar", packages: "flatpak:io.github.kolunmi.Bazaar", icon: "/assets/icons/software/bazaar.png" },
    { name: "Warehouse", packages: "flatpak:io.github.flattool.Warehouse", icon: "/assets/icons/software/warehouse.png" },
    { name: "Easy Flatpak", packages: "flatpak:org.dupot.easyflatpak", icon: "/assets/icons/software/easyflatpak.png" },
    {
      name: "Discover",
      packages: "plasma-discover plasma-discover-backend-flatpak",
      icon: "/assets/icons/software/discover.png",
    },
  ];

  let apps: SoftwareApp[];
  $: apps = appsBase.map((app) => (app.name === "Discover" ? { ...app, warning: $_("software.discoverWarningPlasma") } : app));

  let busyAction = "";
  let showFlathubChoice = false;
  let flathubState = "missing";

  async function refreshFlathubState() {
    flathubState = await api.invoke<string>("flatpak_state");
  }

  function flathubLabel(state: string): string {
    if (state === "both") return $_("software.flathubBoth");
    if (state === "user") return $_("software.flathubUser");
    if (state === "system") return $_("software.flathubSystem");
    return $_("software.addFlathub");
  }

  // The terminal output panel (toggled via "Show output" below) is already
  // the source of truth for what a command did — a separate status line
  // here just duplicated it and made the layout shift every time one ran.
  async function run(action: string, command: string) {
    busyAction = action;
    await runTracked(command);
    busyAction = "";
  }

  // The scope buttons ("Just for me" / "For all users" / "Both") represent
  // the desired END STATE, not "also add this" — so switching from "both"
  // to "user" must remove the system remote too, not just leave it there.
  async function addFlathub(scope: "user" | "system" | "both") {
    showFlathubChoice = false;
    busyAction = "flathub";
    const wantUser = scope === "user" || scope === "both";
    const wantSystem = scope === "system" || scope === "both";
    const hasUser = flathubState === "user" || flathubState === "both";
    const hasSystem = flathubState === "system" || flathubState === "both";
    if (wantUser && !hasUser) {
      await runTracked("add_flathub_remote");
    }
    if (wantSystem && !hasSystem) {
      await runTracked("add_flathub_remote_system");
    }
    if (!wantUser && hasUser) {
      await runTracked("remove_flathub_remote_user");
    }
    if (!wantSystem && hasSystem) {
      await runTracked("remove_flathub_remote_system");
    }
    busyAction = "";
    await refreshFlathubState();
  }

  async function removeFlathub() {
    showFlathubChoice = false;
    busyAction = "flathub";
    if (flathubState === "user" || flathubState === "both") {
      await runTracked("remove_flathub_remote_user");
    }
    if (flathubState === "system" || flathubState === "both") {
      await runTracked("remove_flathub_remote_system");
    }
    busyAction = "";
    await refreshFlathubState();
  }

  onMount(refreshFlathubState);
</script>

<div class="flex flex-col items-center gap-6">
  <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 w-full">
    {#each apps as app}
      <SoftwareTile name={app.name} packages={app.packages} icon={app.icon} warning={app.warning} />
    {/each}
  </div>

  <div class="flex flex-col items-center gap-2">
    <div class="flex gap-2.5">
      <button
        class="px-4 py-2 rounded border text-[0.85rem] transition-colors {flathubState === 'missing'
          ? 'border-border text-text hover:bg-surface hover:text-text-bright'
          : 'border-success/40 bg-success/10 text-success hover:bg-success/15'}"
        disabled={busyAction === "flathub"}
        on:click={() => (showFlathubChoice = true)}
      >
        {busyAction === "flathub" ? $_("software.adding") : flathubLabel(flathubState)}
      </button>
      <button
        class="px-4 py-2 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.85rem] transition-colors"
        disabled={busyAction === "clean"}
        on:click={() => run("clean", "clean_system")}
      >
        {busyAction === "clean" ? $_("software.cleaning") : $_("software.cleanSystem")}
      </button>
      <button
        class="px-4 py-2 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.85rem] transition-colors"
        on:click={() => ($terminalExpanded = !$terminalExpanded)}
      >
        {$terminalExpanded ? $_("common.hideOutput") : $_("common.showOutput")}
      </button>
    </div>
  </div>
</div>

{#if showFlathubChoice}
  <FlathubModal
    {flathubState}
    onAdd={addFlathub}
    onRemove={removeFlathub}
    onClose={() => (showFlathubChoice = false)}
  />
{/if}
