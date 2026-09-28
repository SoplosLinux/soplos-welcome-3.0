<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import SoftwareXfce from "./software/SoftwareXfce.svelte";
  import SoftwareGnome from "./software/SoftwareGnome.svelte";
  import SoftwarePlasma from "./software/SoftwarePlasma.svelte";
  import TerminalOutput from "../components/TerminalOutput.svelte";
  import UpdateActions from "../components/UpdateActions.svelte";

  type DesktopKind = "gnome" | "kde" | "xfce";

  let desktop: DesktopKind = "xfce";
  let loaded = false;

  function detectDesktopKind(raw: string): DesktopKind {
    const lower = raw.toLowerCase();
    if (lower.includes("gnome")) return "gnome";
    if (lower.includes("kde") || lower.includes("plasma")) return "kde";
    return "xfce";
  }

  onMount(async () => {
    try {
      const info = await api.invoke<{ desktop: string }>("get_system_info");
      desktop = detectDesktopKind(info.desktop);
    } catch (error) {
      console.error("Failed to detect desktop environment", error);
    } finally {
      loaded = true;
    }
  });
</script>

{#if loaded}
  <div class="flex flex-col gap-5 w-full items-stretch">
    <div
      class="relative rounded border border-border overflow-hidden flex flex-col items-center justify-end text-center p-4 min-h-[200px]"
      style="background: linear-gradient(rgba(20, 20, 22, 0.35), rgba(20, 20, 22, 0.85)), url('/assets/hero/software-background.png'); background-size: cover; background-position: center;"
    >
      <UpdateActions />
    </div>

    {#if desktop === "gnome"}
      <SoftwareGnome />
    {:else if desktop === "kde"}
      <SoftwarePlasma />
    {:else}
      <SoftwareXfce />
    {/if}

    <div id="job-terminal-anchor">
      <TerminalOutput fullWidth />
    </div>
  </div>
{/if}
