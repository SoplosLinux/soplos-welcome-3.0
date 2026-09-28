<script lang="ts">
  import { onMount, getContext } from "svelte";
  import { open } from "@tauri-apps/plugin-shell";
  import { _ } from "svelte-i18n";
  import SystemCard from "../components/SystemCard.svelte";
  import AppCard from "../components/AppCard.svelte";
  import { api } from "../lib/api";
  import {
    Cpu,
    Monitor,
    Settings,
    LayoutGrid,
    Package,
    Shield,
    Globe,
    MessageCircle,
    BookOpen,
    Heart,
    CircuitBoard,
  } from "lucide-svelte";

  const navigate = getContext<(id: string) => void>("navigate");

  interface SystemInfo {
    cpu: string;
    gpu: string;
    kernel: string;
    desktop: string;
  }

  let systemInfo: SystemInfo = { cpu: "—", gpu: "—", kernel: "—", desktop: "—" };

  $: system = [
    { icon: Cpu, label: $_("home.system.processor"), value: systemInfo.cpu },
    { icon: Monitor, label: $_("home.system.graphics"), value: systemInfo.gpu },
    { icon: Settings, label: $_("home.system.kernel"), value: systemInfo.kernel },
    { icon: LayoutGrid, label: $_("home.system.desktop"), value: systemInfo.desktop },
  ];

  interface GpuInfo {
    vendor: string;
    model: string;
    driverStatus: string;
  }

  let gpus: GpuInfo[] = [];

  // Driven by detect_gpus (same source as Drivers and Gaming) instead of
  // re-parsing a raw lspci string here: the old substring match on "ati" hit
  // "Corporation" in every Intel line and reported AMD, and the single string
  // only ever held the first VGA controller, never a hybrid laptop's NVIDIA.
  function graphicsCardContent(list: GpuInfo[]) {
    const has = (vendor: string) => list.find((g) => g.vendor === vendor);
    const title = $_("home.drivers.title");

    const nvidia = has("NVIDIA");
    if (nvidia) {
      if (nvidia.driverStatus === "installed") {
        return {
          title,
          description: $_("home.drivers.nvidiaInstalled"),
          badge: $_("home.drivers.badgeOk"),
          badgeClass: "bg-success/15 text-success",
          action: $_("common.details"),
        };
      }
      return {
        title,
        description: $_("home.drivers.nvidiaPending"),
        badge: $_("home.drivers.badgePending"),
        badgeClass: "bg-warning/15 text-warning",
        action: $_("home.drivers.actionInstall"),
      };
    }

    if (has("AMD")) {
      return {
        title,
        description: $_("home.drivers.amd"),
        badge: $_("home.drivers.badgeOk"),
        badgeClass: "bg-success/15 text-success",
        action: $_("common.details"),
      };
    }

    if (has("Intel")) {
      return {
        title,
        description: $_("home.drivers.intel"),
        badge: $_("home.drivers.badgeOk"),
        badgeClass: "bg-success/15 text-success",
        action: $_("common.details"),
      };
    }

    if (has("Virtual")) {
      return {
        title,
        description: $_("home.drivers.virtual"),
        badge: $_("home.drivers.badgeVirtual"),
        badgeClass: "bg-surface-2 text-text-muted",
        action: $_("common.details"),
      };
    }

    return {
      title,
      description: $_("home.drivers.unknown"),
      badge: $_("home.drivers.badgePending"),
      badgeClass: "bg-warning/15 text-warning",
      action: $_("home.drivers.actionInstall"),
    };
  }

  $: quickActions = [
    {
      icon: CircuitBoard,
      ...graphicsCardContent(gpus),
      target: "drivers",
    },
    {
      icon: Package,
      title: $_("home.recommendedApps.title"),
      description: $_("home.recommendedApps.description"),
      badge: $_("home.recommendedApps.badge"),
      badgeClass: "bg-surface-2 text-text-muted",
      action: $_("common.explore"),
      target: "recommended",
    },
    {
      icon: Shield,
      title: $_("home.security.title"),
      description: $_("home.security.description"),
      badge: $_("home.security.badge"),
      badgeClass: "bg-success/15 text-success",
      action: $_("common.configure"),
      target: "security",
    },
  ];

  $: links = [
    { label: $_("home.links.web"), icon: Globe, url: "https://soplos.org" },
    { label: $_("home.links.forums"), icon: MessageCircle, url: "https://soplos.org/forums/" },
    { label: $_("home.links.wiki"), icon: BookOpen, url: "https://soplos.org/wiki" },
    { label: $_("home.links.donate"), icon: Heart, url: "https://soplos.org/community/donate/" },
  ];

  onMount(async () => {
    try {
      systemInfo = await api.invoke<SystemInfo>("get_system_info");
    } catch (error) {
      console.error("Failed to read system info", error);
    }
    try {
      gpus = await api.invoke<GpuInfo[]>("detect_gpus");
    } catch (error) {
      console.error("Failed to detect GPUs", error);
    }
  });
</script>

<div class="flex flex-col min-h-full">
<div
  class="rounded p-4 mb-4 flex flex-col items-center text-center border border-border"
  style="background: linear-gradient(rgba(37, 37, 39, 0.85), rgba(239, 90, 51, 0.75)),
    url('/assets/hero/background.jpg'); background-size: cover; background-position: center;"
>
  <img src="/assets/icons/soplos-logo.png" alt="Soplos" class="w-14 h-14 object-contain mb-2" />
  <h2 class="text-[1.2rem] font-bold text-white mb-0.5">{$_("home.title")}</h2>
  <p class="text-primary text-[0.8rem] mb-1.5">{$_("home.subtitle")}</p>
  <p class="text-white/85 text-[0.78rem] leading-snug max-w-[520px] mb-2.5">
    {$_("home.description")}
  </p>
  <button
    class="bg-primary hover:bg-primary-hover text-text-bright px-4 py-1.5 rounded font-semibold text-[0.82rem]"
    on:click={() => navigate("software")}
  >
    {$_("home.startSetup")}
  </button>
</div>

<div class="grid grid-cols-2 gap-2.5 mb-4">
  {#each system as item}
    <SystemCard icon={item.icon} label={item.label} value={item.value} />
  {/each}
</div>

<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3.5 mb-4">
  {#each quickActions as item}
    <AppCard icon={item.icon} title={item.title} description={item.description}>
      <svelte:fragment slot="footer">
        <span class="text-[0.72rem] px-2.5 py-1 rounded-full font-semibold {item.badgeClass}">
          {item.badge}
        </span>
        <button
          class="bg-primary hover:bg-primary-hover text-text-bright px-3 py-1.5 rounded font-semibold text-[0.82rem]"
          on:click={() => navigate(item.target)}
        >
          {item.action}
        </button>
      </svelte:fragment>
    </AppCard>
  {/each}
</div>

<div class="flex flex-wrap items-center justify-center gap-2.5 mt-auto">
  {#each links as link}
    <button
      class="flex items-center gap-2 px-4 py-2 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.88rem] transition-colors"
      on:click={() => open(link.url)}
    >
      <svelte:component this={link.icon} size={16} />
      <span>{link.label}</span>
    </button>
  {/each}
</div>
</div>
