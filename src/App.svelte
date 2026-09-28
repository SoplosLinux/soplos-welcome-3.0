<script lang="ts">
  import { setContext, type ComponentType } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { _ } from "svelte-i18n";
  import { isRunning, progressFraction } from "./lib/progress";
  import Sidebar from "./components/Sidebar.svelte";
  import Header from "./components/Header.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import Home from "./views/Home.svelte";
  import Software from "./views/Software.svelte";
  import Drivers from "./views/Drivers.svelte";
  import Kernels from "./views/Kernels.svelte";
  import Security from "./views/Security.svelte";
  import Recommended from "./views/Recommended.svelte";
  import Customization from "./views/Customization.svelte";
  import Gaming from "./views/Gaming.svelte";
  import {
    Home as HomeIcon,
    Package,
    CircuitBoard,
    Settings,
    Shield,
    Star,
    Palette,
    Gamepad2,
    ChevronRight,
    ChevronLeft,
  } from "lucide-svelte";

  $: tabs = [
    { id: "home", icon: HomeIcon, label: $_("app.tabs.home.label"), title: $_("app.tabs.home.title") },
    { id: "software", icon: Package, label: $_("app.tabs.software.label"), title: $_("app.tabs.software.title") },
    { id: "drivers", icon: CircuitBoard, label: $_("app.tabs.drivers.label"), title: $_("app.tabs.drivers.title") },
    { id: "kernels", icon: Settings, label: $_("app.tabs.kernels.label"), title: $_("app.tabs.kernels.title") },
    { id: "security", icon: Shield, label: $_("app.tabs.security.label"), title: $_("app.tabs.security.title") },
    { id: "recommended", icon: Star, label: $_("app.tabs.recommended.label"), title: $_("app.tabs.recommended.title") },
    { id: "customization", icon: Palette, label: $_("app.tabs.customization.label"), title: $_("app.tabs.customization.title") },
    { id: "gaming", icon: Gamepad2, label: $_("app.tabs.gaming.label"), title: $_("app.tabs.gaming.title") },
  ];

  const views: Record<string, ComponentType> = {
    home: Home,
    software: Software,
    drivers: Drivers,
    kernels: Kernels,
    security: Security,
    recommended: Recommended,
    customization: Customization,
    gaming: Gaming,
  };

  let active = "home";

  function select(id: string) {
    active = id;
    document.getElementById("main-scroll")?.scrollTo({ top: 0 });
  }

  // Lets any view (e.g. Home's quick actions) switch tabs without prop-drilling.
  setContext("navigate", select);
  setContext("openAbout", () => (aboutOpen = true));

  $: activeTab = tabs.find((t) => t.id === active)!;
  $: ActiveView = views[active];
  $: activeIndex = tabs.findIndex((t) => t.id === active);
  $: nextTab = activeIndex >= 0 && activeIndex < tabs.length - 1 ? tabs[activeIndex + 1] : null;
  $: prevTab = activeIndex > 0 ? tabs[activeIndex - 1] : null;

  // === Global keyboard shortcuts, ported from soplos-welcome 2.x ===
  // F1 -> About, Ctrl+Q -> quit, Ctrl+Tab / Ctrl+Shift+Tab -> cycle tabs.
  let aboutOpen = false;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "F1") {
      e.preventDefault();
      aboutOpen = true;
      return;
    }
    if (!e.ctrlKey) return;
    if (e.key === "q" || e.key === "Q") {
      e.preventDefault();
      getCurrentWindow().close();
    } else if (e.code === "Tab") {
      e.preventDefault();
      const dir = e.shiftKey ? -1 : 1;
      const idx = tabs.findIndex((t) => t.id === active);
      select(tabs[(idx + dir + tabs.length) % tabs.length].id);
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="flex h-screen overflow-hidden">
  <Sidebar {tabs} {active} onSelect={select} />

  <div class="flex-1 flex flex-col overflow-hidden min-w-0">
    {#if active !== "home"}
      <Header title={activeTab.title}>
        <svelte:fragment slot="back">
          {#if prevTab}
            <button
              class="flex items-center gap-1 px-3 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.82rem] transition-colors"
              on:click={() => select(prevTab.id)}
            >
              <ChevronLeft size={14} />
              {prevTab.label}
            </button>
          {/if}
        </svelte:fragment>
        <svelte:fragment slot="actions">
          {#if nextTab}
            <button
              class="flex items-center gap-1 px-3 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.82rem] transition-colors"
              on:click={() => select(nextTab.id)}
            >
              {nextTab.label}
              <ChevronRight size={14} />
            </button>
          {:else}
            <button
              class="flex items-center gap-1 px-3 py-1.5 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.82rem] transition-colors"
              on:click={() => select("home")}
            >
              <HomeIcon size={14} />
              {$_("app.tabs.home.label")}
            </button>
          {/if}
        </svelte:fragment>
      </Header>
    {/if}
    <div id="main-scroll" class="content-scroll flex-1 overflow-y-auto p-7">
      <svelte:component this={ActiveView} />
    </div>

    <div class="h-1.5 bg-surface-2 overflow-hidden shrink-0" class:invisible={!$isRunning}>
      {#if $progressFraction !== null}
        <div class="h-full bg-primary transition-[width]" style="width: {$progressFraction * 100}%"></div>
      {:else}
        <div class="h-full w-1/3 bg-primary progress-fill"></div>
      {/if}
    </div>
  </div>
</div>

{#if aboutOpen}
  <AboutDialog onClose={() => (aboutOpen = false)} />
{/if}

<style>
  .progress-fill {
    animation: progress-slide 1.1s ease-in-out infinite;
  }

  @keyframes progress-slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(300%);
    }
  }
</style>
