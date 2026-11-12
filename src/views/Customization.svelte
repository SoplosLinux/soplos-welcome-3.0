<script lang="ts">
  import Modal from "../components/Modal.svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { runTracked, terminalExpanded, terminalLines } from "../lib/progress";
  import TerminalOutput from "../components/TerminalOutput.svelte";
  import { _ } from "svelte-i18n";

  type DesktopKind = "gnome" | "kde" | "xfce";

  interface SoplosTool {
    key: string;
    label: string;
    icon: string;
    command: string;
    package: string;
  }

  interface NativeTool {
    key: string;
    icon: string;
    command?: string;
    desktopFile?: string;
  }

  let desktop: DesktopKind = "xfce";
  let loaded = false;

  function detectDesktopKind(raw: string): DesktopKind {
    const lower = raw.toLowerCase();
    if (lower.includes("gnome")) return "gnome";
    if (lower.includes("kde") || lower.includes("plasma")) return "kde";
    return "xfce";
  }

  const soplosToolsByDesktop: Record<DesktopKind, SoplosTool[]> = {
    xfce: [
      { key: "theme-manager", label: "Theme Manager", icon: "/assets/icons/soplos/soplos-theme-manager.png", command: "soplos-theme-manager", package: "soplos-theme-manager" },
      { key: "grub-editor", label: "GRUB Editor", icon: "/assets/icons/soplos/soplos-grub-editor.png", command: "soplos-grub-editor", package: "soplos-grub-editor" },
      { key: "plymouth-manager", label: "Plymouth Manager", icon: "/assets/icons/soplos/soplos-plymouth-manager.png", command: "soplos-plymouth-manager", package: "soplos-plymouth-manager" },
    ],
    gnome: [
      { key: "layout-switcher", label: "Layout Switcher", icon: "/assets/icons/soplos/soplos-layout-switcher.png", command: "soplos-layout-switcher", package: "soplos-layout-switcher" },
      { key: "grub-editor", label: "GRUB Editor", icon: "/assets/icons/soplos/soplos-grub-editor.png", command: "soplos-grub-editor", package: "soplos-grub-editor" },
      { key: "plymouth-manager", label: "Plymouth Manager", icon: "/assets/icons/soplos/soplos-plymouth-manager.png", command: "soplos-plymouth-manager", package: "soplos-plymouth-manager" },
    ],
    kde: [
      { key: "look-manager", label: "Look Manager", icon: "/assets/icons/soplos/soplos-look-manager.png", command: "soplos-look-manager", package: "soplos-look-manager" },
      { key: "grub-editor", label: "GRUB Editor", icon: "/assets/icons/soplos/soplos-grub-editor.png", command: "soplos-grub-editor", package: "soplos-grub-editor" },
      { key: "plymouth-manager", label: "Plymouth Manager", icon: "/assets/icons/soplos/soplos-plymouth-manager.png", command: "soplos-plymouth-manager", package: "soplos-plymouth-manager" },
    ],
  };

  const nativeToolsByDesktop: Record<DesktopKind, NativeTool[]> = {
    xfce: [
      { key: "appearance", icon: "/assets/icons/customization/appearance.svg", command: "xfce4-appearance-settings" },
      { key: "desktop", icon: "/assets/icons/customization/wallpaper.svg", command: "xfdesktop-settings" },
      { key: "wm", icon: "/assets/icons/customization/window-manager.svg", command: "xfwm4-settings" },
      { key: "keyboard", icon: "/assets/icons/customization/keyboard.svg", command: "xfce4-keyboard-settings" },
      { key: "mouse", icon: "/assets/icons/customization/mouse.svg", command: "xfce4-mouse-settings" },
      { key: "notifications", icon: "/assets/icons/customization/notifications.svg", command: "xfce4-notifyd-config" },
      { key: "settings-editor", icon: "/assets/icons/customization/settings.svg", command: "xfce4-settings-editor" },
    ],
    gnome: [
      { key: "wallpaper", icon: "/assets/icons/customization/wallpaper.svg", command: "gnome-control-center background" },
      { key: "display", icon: "/assets/icons/customization/display.svg", command: "gnome-control-center display" },
      { key: "power", icon: "/assets/icons/customization/power.svg", command: "gnome-control-center power" },
      { key: "sound", icon: "/assets/icons/customization/sound.svg", command: "gnome-control-center sound" },
      { key: "settings", icon: "/assets/icons/customization/settings.svg", command: "gnome-control-center" },
      { key: "tweaks", icon: "/assets/icons/customization/tweaks.svg", command: "gnome-tweaks" },
      { key: "extensions", icon: "/assets/icons/customization/extensions.svg", command: "gnome-extensions-app" },
      { key: "dconf", icon: "/assets/icons/customization/dconf.svg", command: "dconf-editor" },
    ],
    kde: [
      { key: "look-and-feel", icon: "/assets/icons/customization/theme-global.svg", desktopFile: "kcm_lookandfeel.desktop" },
      { key: "login-screen", icon: "/assets/icons/customization/login.svg", desktopFile: "kcm_plasmalogin.desktop" },
      { key: "plymouth", icon: "/assets/icons/customization/splash.svg", desktopFile: "kcm_plymouth.desktop" },
      { key: "wallpaper", icon: "/assets/icons/customization/wallpaper.svg", command: "systemsettings kcm_wallpaper" },
      { key: "icons", icon: "/assets/icons/customization/icons-theme.svg", command: "systemsettings kcm_icons" },
      { key: "cursor", icon: "/assets/icons/customization/cursors.svg", command: "systemsettings kcm_cursortheme" },
      { key: "fonts", icon: "/assets/icons/customization/fonts.svg", command: "systemsettings kcm_fonts" },
      { key: "system-settings", icon: "/assets/icons/customization/systemsettings.svg", command: "systemsettings" },
    ],
  };

  const installedMap: Record<string, boolean> = {};
  let installedVersion = 0;

  async function refreshSoplosToolStatus(tool: SoplosTool) {
    installedMap[tool.key] = await api.invoke<boolean>("is_apt_package_installed", { package: tool.package });
    installedVersion++;
  }

  async function onSoplosToolClick(tool: SoplosTool) {
    if (installedMap[tool.key]) {
      await api.invoke("launch_app", { program: tool.command, args: [] });
      return;
    }
    await runTracked("install_apt_packages", { packages: tool.package });
    await refreshSoplosToolStatus(tool);
    if (installedMap[tool.key]) {
      await api.invoke("launch_app", { program: tool.command, args: [] });
    }
  }

  async function onNativeToolClick(tool: NativeTool) {
    if (tool.desktopFile) {
      await api.invoke("launch_app", { program: "gtk-launch", args: [tool.desktopFile] });
      return;
    }
    if (!tool.command) return;
    const [program, ...args] = tool.command.split(" ");
    await api.invoke("launch_app", { program, args });
  }

  let lucidglyphInstalled = false;
  let lucidglyphBusy = false;
  let infoMessage = "";

  async function refreshLucidglyph() {
    lucidglyphInstalled = await api.invoke<boolean>("is_lucidglyph_installed");
  }

  async function toggleLucidglyph() {
    lucidglyphBusy = true;
    const wasInstalled = lucidglyphInstalled;
    const success = await runTracked(wasInstalled ? "uninstall_lucidglyph" : "install_lucidglyph");
    lucidglyphBusy = false;
    if (!success) {
      infoMessage = wasInstalled ? $_("customization.lucidGlyph.removeFailed") : $_("customization.lucidGlyph.installFailed");
      return;
    }
    await refreshLucidglyph();
    if (desktop === "gnome") {
      try {
        await api.invoke(wasInstalled ? "reset_gnome_font_antialiasing" : "set_gnome_font_antialiasing_grayscale");
      } catch (error) {
        console.error("gsettings font-antialiasing update failed", error);
      }
    }
    if (wasInstalled) {
      infoMessage = $_("customization.lucidGlyph.removed");
    } else if (desktop === "kde") {
      infoMessage = $_("customization.lucidGlyph.installedKde");
    } else {
      infoMessage = $_("customization.lucidGlyph.installed");
    }
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
    for (const tool of soplosToolsByDesktop[desktop]) refreshSoplosToolStatus(tool);
    refreshLucidglyph();
  });
</script>

<div
  class="relative rounded border border-border overflow-hidden mb-5 flex flex-col items-start justify-end text-left p-4 min-h-[200px]"
  style="background: linear-gradient(rgba(20, 20, 22, 0.35), rgba(20, 20, 22, 0.85)), url('/assets/hero/customization-background.png'); background-size: cover; background-position: center;"
>
  <p class="text-white font-semibold text-[1rem]">{$_("customization.hero.title")}</p>
  <p class="text-[0.8rem] text-white/85 mt-1">{$_("customization.hero.description")}</p>
</div>

{#if loaded}
  <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">{$_("customization.soplosTools")}</div>
  <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3 mb-5">
    {#each soplosToolsByDesktop[desktop] as tool}
      {@const installed = installedVersion >= 0 && installedMap[tool.key]}
      <button
        class="flex flex-col items-center text-center gap-1.5 bg-surface border border-border rounded p-4 hover:bg-surface-2 transition-colors"
        on:click={() => onSoplosToolClick(tool)}
      >
        <img src={tool.icon} alt="" class="w-12 h-12 object-contain" />
        <span class="text-[0.85rem] font-semibold text-text-bright">{tool.label}</span>
        <span class="text-[0.7rem] text-text">{$_(`customization.tools.${tool.key}`)}</span>
        <span
          class="text-[0.68rem] px-2 py-0.5 rounded-full mt-1 {installed
            ? 'bg-success/15 text-success'
            : 'bg-surface-2 text-text'}"
        >
          {installed ? $_("customization.clickToOpen") : $_("customization.clickToInstallOpen")}
        </span>
      </button>
    {/each}

    <button
      class="flex flex-col items-center text-center gap-1.5 bg-surface border border-border rounded p-4 hover:bg-surface-2 transition-colors disabled:opacity-60"
      disabled={lucidglyphBusy}
      on:click={toggleLucidglyph}
    >
      <img src="/assets/icons/customization/fonts.svg" alt="" class="w-12 h-12 object-contain" />
      <span class="text-[0.85rem] font-semibold text-text-bright">LucidGlyph</span>
      <span class="text-[0.7rem] text-text">{$_("customization.lucidGlyph.description")}</span>
      <span
        class="text-[0.68rem] px-2 py-0.5 rounded-full mt-1 {lucidglyphInstalled
          ? 'bg-success/15 text-success'
          : 'bg-surface-2 text-text'}"
      >
        {lucidglyphBusy ? $_("gaming.working") : lucidglyphInstalled ? $_("drivers.clickToUninstall") : $_("drivers.clickToInstall")}
      </span>
    </button>
  </div>

  <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-2">
    {desktop === "xfce" ? $_("customization.xfceSettings") : desktop === "gnome" ? $_("customization.gnomeSettings") : $_("customization.kdeSettings")}
  </div>
  <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
    {#each nativeToolsByDesktop[desktop] as tool}
      <button
        class="flex flex-col items-center text-center gap-1.5 bg-surface border border-border rounded p-4 hover:bg-surface-2 transition-colors"
        on:click={() => onNativeToolClick(tool)}
      >
        <img src={tool.icon} alt="" class="w-10 h-10 object-contain" />
        <span class="text-[0.85rem] font-semibold text-text-bright">{$_(`customization.native.${desktop}.${tool.key}.label`)}</span>
        <span class="text-[0.7rem] text-text">{$_(`customization.native.${desktop}.${tool.key}.hint`)}</span>
        <span class="text-[0.68rem] px-2 py-0.5 rounded-full bg-surface-2 text-text mt-1">{$_("customization.clickToOpen")}</span>
      </button>
    {/each}
  </div>
{/if}

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
