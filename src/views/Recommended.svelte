<script lang="ts">
  import { onMount, onDestroy, beforeUpdate, tick } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { _ } from "svelte-i18n";
  import { api } from "../lib/api";
  import {
    runTracked,
    terminalExpanded,
    terminalLines,
    isRunning,
    progressFraction,
  } from "../lib/progress";
  import TerminalOutput from "../components/TerminalOutput.svelte";
  import Modal from "../components/Modal.svelte";
  import FlatpakScopeModal from "../components/FlatpakScopeModal.svelte";
  import { ChevronDown, LoaderCircle, Package, Search, X } from "lucide-svelte";

  interface RecPackage {
    name: string;
    icon: string;
    official?: boolean;
    kind: "apt" | "flatpak" | "custom" | "manual";
    package?: string;
    flatpak?: string;
    checkPath?: string;
    // Overrides the source badge; without it, checkPath entries under ~/AppImages show "AppImage" and others none.
    badge?: string;
    // Fixed Tauri command names for "custom" entries — the script itself
    // lives in Rust (src-tauri/src/commands/recommended.rs), never sent
    // from the frontend. See security plan point 1.
    customInstall?: string;
    customUninstall?: string;
    postInstall?: string;
    // Shows a license-acceptance confirmation before running customInstall —
    // for non-free components (e.g. VirtualBox Extension Pack's PUEL).
    licenseConfirm?: boolean;
    disabled?: string; // tooltip reason when the item can't be installed (e.g. Waydroid without Binder)
  }

  interface RecCategory {
    key: string;
    packages: RecPackage[];
  }

  // Order matches the 2.x "recommended_categories" list.
  const categories: RecCategory[] = [
    {
      key: "browsers",
      packages: [
        {
          name: "Firefox",
          icon: "firefox.png",
          official: true,
          kind: "apt",
          package: "firefox",
        },
        {
          name: "Google Chrome",
          icon: "chrome.png",
          kind: "custom",
          package: "google-chrome-stable",
          customInstall: "install_chrome",
        },
        {
          name: "Chromium",
          icon: "chromium.png",
          official: true,
          kind: "apt",
          package: "chromium",
        },
        {
          name: "Brave",
          icon: "brave.png",
          kind: "custom",
          package: "brave-browser",
          customInstall: "install_brave",
          customUninstall: "uninstall_brave",
        },
        {
          name: "LibreWolf",
          icon: "librewolf.png",
          kind: "flatpak",
          flatpak: "io.gitlab.librewolf-community",
        },
        {
          name: "Vivaldi",
          icon: "vivaldi.png",
          kind: "flatpak",
          flatpak: "com.vivaldi.Vivaldi",
        },
        {
          name: "Opera GX",
          icon: "opera-gx.png",
          kind: "flatpak",
          flatpak: "com.opera.opera-gx",
        },
        {
          name: "Opera",
          icon: "opera.png",
          kind: "flatpak",
          flatpak: "com.opera.Opera",
        },
        {
          name: "Zen Browser",
          icon: "zen-browser.png",
          kind: "custom",
          package: "zen-browser",
          customInstall: "install_zen_browser",
        },
        {
          name: "Helium",
          icon: "helium.png",
          kind: "custom",
          package: "helium-bin",
          customInstall: "install_helium",
        },
        {
          name: "Midori",
          icon: "midori.png",
          kind: "custom",
          package: "midori",
          customInstall: "install_midori",
        },
        {
          name: "Brave Origin",
          icon: "brave-origin.png",
          kind: "custom",
          package: "brave-origin",
          customInstall: "install_brave_origin",
        },
      ],
    },
    {
      key: "comunications",
      packages: [
        {
          name: "Thunderbird",
          icon: "thunderbird.png",
          official: true,
          kind: "apt",
          package: "thunderbird",
        },
        {
          name: "Discord",
          icon: "discord.png",
          kind: "flatpak",
          flatpak: "com.discordapp.Discord",
        },
        {
          name: "Telegram",
          icon: "telegram.png",
          kind: "apt",
          package: "telegram-desktop",
        },
        {
          name: "Signal",
          icon: "signal.png",
          kind: "flatpak",
          flatpak: "org.signal.Signal",
        },
        {
          name: "Element",
          icon: "element.png",
          kind: "flatpak",
          flatpak: "im.riot.Riot",
        },
        {
          name: "WhatsApp",
          icon: "whatsapp.png",
          kind: "custom",
          badge: "WebApp",
          checkPath:
            "~/.local/share/applications/soplos-webapp-whatsapp.desktop",
          customInstall: "install_whatsapp",
          customUninstall: "uninstall_whatsapp",
        },
        {
          name: "Slack",
          icon: "slack.png",
          kind: "flatpak",
          flatpak: "com.slack.Slack",
        },
        {
          name: "Zoom",
          icon: "zoom.png",
          kind: "flatpak",
          flatpak: "us.zoom.Zoom",
        },
      ],
    },
    {
      key: "office",
      packages: [
        {
          name: "LibreOffice",
          icon: "libreoffice.png",
          official: true,
          kind: "flatpak",
          flatpak: "org.libreoffice.LibreOffice",
        },
        {
          name: "OnlyOffice",
          icon: "onlyoffice.png",
          kind: "flatpak",
          flatpak: "org.onlyoffice.desktopeditors",
        },
        {
          name: "WPS Office",
          icon: "wpsoffice.png",
          kind: "flatpak",
          flatpak: "com.wps.Office",
        },
        {
          name: "Calligra",
          icon: "calligra.png",
          official: true,
          kind: "flatpak",
          flatpak: "org.kde.calligra",
        },
        {
          name: "Collabora Office",
          icon: "collabora.png",
          official: true,
          kind: "flatpak",
          flatpak: "com.collaboraoffice.Office",
        },
        {
          name: "GenOffice",
          icon: "genoffice.png",
          kind: "custom",
          package: "genoffice",
          customInstall: "install_genoffice",
        },
        {
          name: "Adobe Reader",
          icon: "reader.png",
          kind: "flatpak",
          flatpak: "com.adobe.Reader",
        },
        {
          name: "JoPDF",
          icon: "jopdf.png",
          kind: "custom",
          package: "jopdf",
          customInstall: "install_jopdf",
        },
        {
          name: "PrintCraft",
          icon: "printcraft.png",
          kind: "custom",
          package: "printcraft",
          customInstall: "install_printcraft",
          customUninstall: "uninstall_printcraft",
        },
        {
          name: "WordCraft",
          icon: "wordcraft.png",
          kind: "custom",
          package: "wordcraft",
          customInstall: "install_wordcraft",
          customUninstall: "uninstall_wordcraft",
        },
        {
          name: "GridCraft",
          icon: "gridcraft.png",
          kind: "custom",
          package: "gridcraft",
          customInstall: "install_gridcraft",
          customUninstall: "uninstall_gridcraft",
        },
        {
          name: "DeckCraft",
          icon: "deckcraft.png",
          kind: "custom",
          package: "deckcraft",
          customInstall: "install_deckcraft",
          customUninstall: "uninstall_deckcraft",
        },
      ],
    },
    {
      key: "multimedia",
      packages: [
        {
          name: "VLC Media Player",
          icon: "vlc.png",
          official: true,
          kind: "apt",
          package: "vlc",
        },
        {
          name: "MPV",
          icon: "mpv.png",
          official: true,
          kind: "apt",
          package: "mpv",
        },
        {
          name: "Kodi",
          icon: "kodi.png",
          official: true,
          kind: "apt",
          package: "kodi",
        },
        {
          name: "Spotify",
          icon: "spotify.png",
          kind: "flatpak",
          flatpak: "com.spotify.Client",
        },
        {
          name: "Glassy Music",
          icon: "glassy-music.png",
          kind: "custom",
          package: "glassy-music-nankill-mod",
          customInstall: "install_glassy_music",
          customUninstall: "uninstall_glassy_music",
        },
        {
          name: "Orchard",
          icon: "orchard.png",
          kind: "custom",
          badge: "Flatpak",
          checkPath: "~/.local/share/flatpak/app/dev.sfg.orchard",
          customInstall: "install_orchard",
          customUninstall: "uninstall_orchard",
        },
        {
          name: "Sonora",
          icon: "sonora.png",
          kind: "custom",
          badge: "Flatpak",
          checkPath: "~/.local/share/flatpak/app/io.github.nolight132.sonora",
          customInstall: "install_sonora",
          customUninstall: "uninstall_sonora",
        },
        {
          name: "OBS Studio",
          icon: "obs-studio.png",
          official: true,
          kind: "flatpak",
          flatpak: "com.obsproject.Studio",
        },
        {
          name: "Kdenlive",
          icon: "kdenlive.png",
          official: true,
          kind: "flatpak",
          flatpak: "org.kde.kdenlive",
        },
        {
          name: "OpenShot",
          icon: "openshot.png",
          official: true,
          kind: "flatpak",
          flatpak: "org.openshot.OpenShot",
        },
        {
          name: "HandBrake",
          icon: "handbrake.png",
          official: true,
          kind: "flatpak",
          flatpak: "fr.handbrake.ghb",
        },
        {
          name: "Concat",
          icon: "concat.png",
          kind: "custom",
          package: "concat",
          customInstall: "install_concat",
          customUninstall: "uninstall_concat",
        },
        {
          name: "FilmCraft",
          icon: "filmcraft.png",
          kind: "custom",
          package: "filmcraft",
          customInstall: "install_filmcraft",
          customUninstall: "uninstall_filmcraft",
        },
        {
          name: "EffectCraft",
          icon: "effectcraft.png",
          kind: "custom",
          package: "effectcraft",
          customInstall: "install_effectcraft",
          customUninstall: "uninstall_effectcraft",
        },
        {
          name: "DaVinci Resolve",
          icon: "davinci-resolve.png",
          kind: "manual",
          checkPath: "/opt/resolve/bin/resolve",
        },
        {
          name: "Drift",
          icon: "drift.png",
          kind: "flatpak",
          flatpak: "org.cutwire.Drift",
        },
        {
          name: "Prism",
          icon: "prism.png",
          kind: "flatpak",
          flatpak: "org.cutwire.Prism",
        },
        {
          name: "Audacity",
          icon: "audacity.png",
          kind: "custom",
          checkPath: "~/AppImages/audacity-linux-x86_64.AppImage",
          customInstall: "install_audacity",
          customUninstall: "uninstall_audacity",
        },
        {
          name: "KutEditor",
          icon: "kuteditor.png",
          official: true,
          kind: "apt",
          package: "kuteditor",
        },
        {
          name: "SoundCraft",
          icon: "soundcraft.png",
          kind: "custom",
          package: "soundcraft",
          customInstall: "install_soundcraft",
          customUninstall: "uninstall_soundcraft",
        },
        {
          name: "LMMS",
          icon: "lmms.png",
          kind: "flatpak",
          flatpak: "io.lmms.LMMS",
        },
        {
          name: "Mixxx",
          icon: "mixxx.png",
          official: true,
          kind: "apt",
          package: "mixxx",
        },
        {
          name: "Bitwig Studio",
          icon: "bitwig-studio.png",
          kind: "flatpak",
          flatpak: "com.bitwig.BitwigStudio",
        },
        {
          name: "Reaper",
          icon: "reaper.png",
          kind: "flatpak",
          flatpak: "fm.reaper.Reaper",
        },
        {
          name: "Zrythm",
          icon: "zrythm.png",
          kind: "flatpak",
          flatpak: "org.zrythm.Zrythm",
        },
        {
          name: "Ardour",
          icon: "ardour.png",
          kind: "flatpak",
          flatpak: "org.ardour.Ardour",
        },
      ],
    },
    {
      key: "graphics",
      packages: [
        {
          name: "GIMP",
          icon: "gimp.png",
          official: true,
          kind: "apt",
          package: "gimp",
        },
        {
          name: "Krita",
          icon: "krita.png",
          official: true,
          kind: "apt",
          package: "krita",
        },
        {
          name: "Affinity Suite",
          icon: "affinity.png",
          kind: "custom",
          checkPath: "~/AppImages/Affinity-3.0.2-x86_64.AppImage",
          customInstall: "install_affinity",
          customUninstall: "uninstall_affinity",
        },
        {
          name: "Fog Panther",
          icon: "pantherfog.png",
          kind: "flatpak",
          flatpak: "com.fogpanther.FogPanther",
        },
        {
          name: "Patchy",
          icon: "patchy.png",
          kind: "custom",
          badge: "Flatpak",
          checkPath: "~/.local/share/flatpak/app/com.rtsoft.patchy",
          customInstall: "install_patchy",
          customUninstall: "uninstall_patchy",
        },
        {
          name: "PhotoCraft",
          icon: "photocraft.png",
          kind: "custom",
          package: "photocraft",
          customInstall: "install_photocraft",
          customUninstall: "uninstall_photocraft",
        },
        {
          name: "Inkscape",
          icon: "inkscape.png",
          official: true,
          kind: "apt",
          package: "inkscape",
        },
        {
          name: "VectorCraft",
          icon: "vectorcraft.png",
          kind: "custom",
          package: "vectorcraft",
          customInstall: "install_vectorcraft",
          customUninstall: "uninstall_vectorcraft",
        },
        {
          name: "DesignCraft",
          icon: "designcraft.png",
          kind: "custom",
          package: "designcraft",
          customInstall: "install_designcraft",
          customUninstall: "uninstall_designcraft",
        },
        {
          name: "FigCraft",
          icon: "figcraft.png",
          kind: "custom",
          package: "figcraft",
          customInstall: "install_figcraft",
          customUninstall: "uninstall_figcraft",
        },
        {
          name: "Blender",
          icon: "blender.png",
          official: true,
          kind: "apt",
          package: "blender",
        },
        {
          name: "darktable",
          icon: "darktable.png",
          official: true,
          kind: "apt",
          package: "darktable",
        },
        {
          name: "RapidRAW",
          icon: "rapidraw.png",
          kind: "flatpak",
          flatpak: "io.github.CyberTimon.RapidRAW",
        },
        {
          name: "RawTherapee",
          icon: "rawtherapee.png",
          official: true,
          kind: "apt",
          package: "rawtherapee",
        },
        {
          name: "ART",
          icon: "art.png",
          kind: "flatpak",
          flatpak: "us.pixls.art.ART",
        },
        {
          name: "LightCraft",
          icon: "lightcraft.png",
          kind: "custom",
          package: "lightcraft",
          customInstall: "install_lightcraft",
          customUninstall: "uninstall_lightcraft",
        },
        {
          name: "Hugin",
          icon: "hugin.png",
          official: true,
          kind: "apt",
          package: "hugin",
        },
      ],
    },
    {
      key: "developer",
      packages: [
        {
          name: "Visual Studio Code",
          icon: "vscode.png",
          kind: "custom",
          package: "code",
          customInstall: "install_vscode",
          customUninstall: "uninstall_vscode",
        },
        {
          name: "VSCodium",
          icon: "vscodium.png",
          kind: "flatpak",
          flatpak: "com.vscodium.codium",
        },
        {
          name: "Google Antigravity",
          icon: "antigravity.png",
          kind: "custom",
          checkPath: "/opt/antigravity/antigravity-ide",
          customInstall: "install_antigravity",
          customUninstall: "uninstall_antigravity",
        },
        {
          name: "Sublime Text",
          icon: "sublime-text.png",
          kind: "custom",
          package: "sublime-text",
          customInstall: "install_sublime_text",
          customUninstall: "uninstall_sublime_text",
        },
        {
          name: "Cursor",
          icon: "cursor.png",
          kind: "custom",
          package: "cursor",
          customInstall: "install_cursor",
        },
        {
          name: "Zed",
          icon: "zed.png",
          kind: "flatpak",
          flatpak: "dev.zed.Zed",
        },
        {
          name: "Pulsar",
          icon: "pulsar.png",
          kind: "flatpak",
          flatpak: "dev.pulsar_edit.Pulsar",
        },
        {
          name: "Geany",
          icon: "geany.png",
          official: true,
          kind: "apt",
          package: "geany",
        },
        {
          name: "Bluefish",
          icon: "bluefish.png",
          official: true,
          kind: "apt",
          package: "bluefish",
        },
        {
          name: "Postman",
          icon: "postman.png",
          kind: "flatpak",
          flatpak: "com.getpostman.Postman",
        },
      ],
    },
    {
      key: "app_management",
      packages: [
        {
          name: "Soplos WebApp Manager",
          icon: "soplos-webapps-manager.png",
          official: true,
          kind: "apt",
          package: "soplos-webapp-manager",
        },
        {
          name: "Soplos AppImage Manager",
          icon: "soplos-appimage-manager.png",
          official: true,
          kind: "apt",
          package: "soplos-appimage-manager",
        },
        {
          name: "Flatseal",
          icon: "flatseal.png",
          kind: "flatpak",
          flatpak: "com.github.tchx84.Flatseal",
        },
        {
          name: "Gear Lever",
          icon: "gear-lever.png",
          kind: "flatpak",
          flatpak: "it.mijorus.gearlever",
        },
        {
          name: "Warehouse",
          icon: "warehouse.png",
          kind: "flatpak",
          flatpak: "io.github.flattool.Warehouse",
        },
        {
          name: "ArtCraft Launcher",
          icon: "artcraft-launcher.png",
          kind: "custom",
          package: "artcraft-launcher",
          customInstall: "install_artcraft_launcher",
          customUninstall: "uninstall_artcraft_launcher",
        },
      ],
    },
    {
      key: "downloads",
      packages: [
        {
          name: "qBittorrent",
          icon: "qbittorrent.png",
          kind: "flatpak",
          flatpak: "org.qbittorrent.qBittorrent",
        },
        {
          name: "Transmission",
          icon: "transmission.png",
          kind: "apt",
          package: "transmission-gtk",
        },
        {
          name: "JDownloader",
          icon: "jdownloader.png",
          kind: "flatpak",
          flatpak: "org.jdownloader.JDownloader",
        },
        {
          name: "Syncthing Tray",
          icon: "syncthing.png",
          kind: "flatpak",
          flatpak: "io.github.martchus.syncthingtray",
        },
      ],
    },
    {
      key: "hardware",
      packages: [
        {
          name: "Resources",
          icon: "resources.png",
          kind: "flatpak",
          flatpak: "net.nokyan.Resources",
        },
        {
          name: "LACT",
          icon: "lact.png",
          kind: "flatpak",
          flatpak: "io.github.ilya_zlobintsev.LACT",
        },
        {
          name: "CPU Power",
          icon: "cpupower.png",
          kind: "custom",
          package: "cpupower-gui",
          customInstall: "install_cpupower_gui",
          customUninstall: "uninstall_cpupower_gui",
        },
        {
          name: "amdgpu_top",
          icon: "amdgpu-top.png",
          kind: "custom",
          package: "amdgpu-top",
          customInstall: "install_amdgpu_top",
        },
        {
          name: "nvtop",
          icon: "nvtop.png",
          official: true,
          kind: "apt",
          package: "nvtop",
        },
        {
          name: "CoolerControl",
          icon: "coolercontrol.png",
          kind: "custom",
          checkPath: "~/AppImages/CoolerControlD-x86_64.AppImage",
          customInstall: "install_coolercontrol",
          customUninstall: "uninstall_coolercontrol",
        },
      ],
    },
    {
      key: "files",
      packages: [
        {
          name: "PeaZip",
          icon: "peazip.png",
          kind: "flatpak",
          flatpak: "io.github.peazip.PeaZip",
        },
        {
          name: "FileZilla",
          icon: "filezilla.png",
          kind: "apt",
          package: "filezilla",
        },
        {
          name: "GNOME Commander",
          icon: "gnome-commander.png",
          kind: "apt",
          package: "gnome-commander",
        },
        {
          name: "Double Commander",
          icon: "doublecmd.png",
          kind: "custom",
          checkPath: "~/AppImages/doublecmd-gtk-latest-x86_64.AppImage",
          customInstall: "install_doublecmd",
          customUninstall: "uninstall_doublecmd",
        },
      ],
    },
    {
      key: "remotes",
      packages: [
        {
          name: "Remmina",
          icon: "remmina.png",
          official: true,
          kind: "flatpak",
          flatpak: "org.remmina.Remmina",
        },
        {
          name: "RustDesk",
          icon: "rustdesk.png",
          kind: "flatpak",
          flatpak: "com.rustdesk.RustDesk",
        },
        {
          name: "AnyDesk",
          icon: "anydesk.png",
          kind: "flatpak",
          flatpak: "com.anydesk.Anydesk",
        },
        {
          name: "NoMachine",
          icon: "nomachine.png",
          kind: "flatpak",
          flatpak: "com.nomachine.nxplayer",
        },
      ],
    },
    {
      key: "virtual",
      packages: [
        {
          name: "VirtualBox",
          icon: "virtualbox.png",
          kind: "custom",
          package: "virtualbox-7.2",
          customInstall: "install_virtualbox",
          customUninstall: "uninstall_virtualbox",
          licenseConfirm: true,
        },
        {
          name: "VirtualBox Extension Pack",
          icon: "virtualbox.png",
          kind: "custom",
          checkPath:
            "/usr/lib/virtualbox/ExtensionPacks/Oracle_VirtualBox_Extension_Pack",
          customInstall: "install_vbox_extpack",
          customUninstall: "uninstall_vbox_extpack",
          licenseConfirm: true,
        },
        {
          name: "VMware Workstation Pro",
          icon: "vmware.png",
          kind: "manual",
          checkPath: "/usr/bin/vmware",
          customInstall: "install_vmware_bundle",
        },
        {
          name: "virt-manager",
          icon: "virtmanager.png",
          official: true,
          kind: "apt",
          package: "virt-manager",
        },
        {
          name: "GNOME Boxes",
          icon: "gnomeboxes.png",
          official: true,
          kind: "apt",
          package: "gnome-boxes",
        },
        {
          name: "Waydroid",
          icon: "waydroid.png",
          kind: "custom",
          package: "waydroid",
          customInstall: "install_waydroid",
          customUninstall: "uninstall_waydroid",
        },
      ],
    },
  ];

  const installedMap: Record<string, boolean> = {};
  let installedVersion = 0;

  let displayProtocol: string | null = null;
  $: waydroidBlocked = displayProtocol === "x11";

  // Descriptions live in the i18n dictionaries under recommended.desc.<slug>,
  // where the slug is the lowercase app name with non-alphanumerics as "-".
  function slug(name: string): string {
    return name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "");
  }

  function pkgKey(catKey: string, pkg: RecPackage): string {
    return `${catKey}:${pkg.name}`;
  }

  async function refreshPkgStatus(catKey: string, pkg: RecPackage) {
    let installed = false;
    if (pkg.checkPath) {
      installed = await api.invoke<boolean>("check_path_exists", {
        path: pkg.checkPath,
      });
    } else if (pkg.package) {
      installed = await api.invoke<boolean>("is_apt_package_installed", {
        package: pkg.package,
      });
    } else if (pkg.flatpak) {
      installed = await api.invoke<boolean>("is_flatpak_installed", {
        appId: pkg.flatpak,
      });
    }
    installedMap[pkgKey(catKey, pkg)] = installed;
    installedVersion++;

    if (pkg.customInstall === "install_antigravity") {
      await refreshAntigravityStatus();
    }
  }

  // Antigravity has no apt/Flatpak tracking (it's a plain tarball in /opt),
  // so checkPath alone can't tell an old copy apart from the current one —
  // this is what let a stale hand-installed 2.1.x sit undetected. Reading
  // its own package.json gives the real installed version to compare
  // against what we currently package.
  let antigravityUpdateAvailable = false;
  let antigravityInstalledVersion: string | null = null;
  let antigravityLatestVersion = "";

  async function refreshAntigravityStatus() {
    const status = await api.invoke<{
      installedVersion: string | null;
      latestVersion: string;
      updateAvailable: boolean;
    }>("get_antigravity_status");
    antigravityInstalledVersion = status.installedVersion;
    antigravityLatestVersion = status.latestVersion;
    antigravityUpdateAvailable = status.updateAvailable;
  }

  async function updateAntigravity(catKey: string, pkg: RecPackage) {
    await runTracked("install_antigravity");
    await refreshPkgStatus(catKey, pkg);
  }

  let infoMessage: {
    key: string;
    name?: string;
    variant?: "info" | "warning";
  } | null = null;
  let flatpakScopeChoice: {
    name: string;
    resolve: (scope: "user" | "system") => void;
  } | null = null;

  function chooseFlatpakScope(name: string): Promise<"user" | "system"> {
    return new Promise((resolve) => {
      flatpakScopeChoice = { name, resolve };
    });
  }

  let licenseConfirmTarget: { catKey: string; pkg: RecPackage } | null = null;

  async function togglePkg(
    catKey: string,
    pkg: RecPackage,
    forcedScope?: "user" | "system",
    append = false,
  ) {
    const key = pkgKey(catKey, pkg);
    const installed = installedMap[key];

    if (
      !installed &&
      pkg.customInstall === "install_waydroid" &&
      waydroidBlocked
    ) {
      infoMessage = { key: "recommended.waydroidNeedsWayland" };
      return;
    }

    if (!installed && pkg.licenseConfirm) {
      licenseConfirmTarget = { catKey, pkg };
      return;
    }

    if (pkg.kind === "manual") {
      const isVmware = pkg.customInstall === "install_vmware_bundle";
      if (installed) {
        if (isVmware) {
          vmwareInstallTarget = { catKey, pkg };
          try {
            await api.invoke("uninstall_vmware");
            infoMessage = { key: "recommended.vmware.uninstallOpened" };
          } catch (error) {
            console.error("Failed to open VMware uninstall terminal", error);
            infoMessage = {
              key: "recommended.vmware.uninstallNoTerminal",
              variant: "warning",
            };
          }
        } else {
          davinciBusy = true;
          try {
            await runTracked("uninstall_davinci", undefined, { append });
          } finally {
            davinciBusy = false;
          }
        }
        await refreshPkgStatus(catKey, pkg);
        return;
      }
      if (isVmware) {
        vmwareInstallTarget = { catKey, pkg };
        vmwareConfirmOpen = true;
      } else {
        davinciConfirmOpen = true;
      }
      return;
    }

    if (pkg.kind === "flatpak" && pkg.flatpak) {
      if (installed) {
        const scope =
          (await api.invoke<string | null>("flatpak_app_scope", {
            appId: pkg.flatpak,
          })) ?? "user";
        await runTracked(
          "uninstall_flatpak_scoped",
          { appId: pkg.flatpak, scope },
          { append },
        );
      } else {
        const state = await api.invoke<string>("flatpak_state");
        if (state === "missing") {
          infoMessage = { key: "softwareTile.flatpakMissing", name: pkg.name };
          return;
        }
        let scope: "user" | "system";
        if (forcedScope) {
          scope = forcedScope;
        } else if (state === "both") {
          scope = await chooseFlatpakScope(pkg.name);
        } else if (state === "system") {
          scope = "system";
          infoMessage = {
            key: "softwareTile.flatpakSystemOnly",
            name: pkg.name,
          };
        } else {
          scope = "user";
        }
        await runTracked(
          "install_flatpak_scoped",
          { appId: pkg.flatpak, scope },
          { append },
        );
        if (pkg.postInstall) {
          await runTracked(pkg.postInstall, undefined, { append: true });
        }
      }
    } else if (pkg.kind === "apt" && pkg.package) {
      await runTracked(
        installed ? "uninstall_apt_packages" : "install_apt_packages",
        { packages: pkg.package },
        { append },
      );
    } else if (pkg.kind === "custom") {
      if (installed) {
        if (pkg.customUninstall) {
          await runTracked(pkg.customUninstall, undefined, { append });
        } else if (pkg.package) {
          await runTracked(
            "uninstall_apt_packages",
            { packages: pkg.package },
            { append },
          );
        }
      } else if (pkg.customInstall) {
        await runTracked(pkg.customInstall, undefined, { append });
      }
    }

    await refreshPkgStatus(catKey, pkg);
  }

  // === DaVinci Resolve (multi-step: deps -> pick installer -> convert to
  // .deb with makeresolvedeb -> install -> optional hardware patches) ===
  let davinciConfirmOpen = false;
  let davinciPatchesOpen = false;
  let davinciBusy = false;
  let davinciApplyMic = false;
  let davinciApplyGpu = false;

  async function startDavinciInstall() {
    davinciConfirmOpen = false;
    const installerPath = await api.invoke<string | null>(
      "pick_davinci_installer",
    );
    if (!installerPath) return;

    davinciBusy = true;
    try {
      if (!(await runTracked("install_davinci_deps"))) return;
      if (!(await runTracked("davinci_extract_and_convert", { installerPath })))
        return;
      if (!(await runTracked("install_davinci_debs", { installerPath })))
        return;

      davinciApplyMic = false;
      davinciApplyGpu = false;
      davinciPatchesOpen = true;
      const applyPatches = await new Promise<boolean>((resolve) => {
        davinciPatchesResolve = resolve;
      });

      if (applyPatches) {
        if (davinciApplyMic) await runTracked("run_davinci_mic_patch");
        if (davinciApplyGpu) await runTracked("run_davinci_gpu_patch");
      }

      await api.invoke("davinci_cleanup", { installerPath });
      infoMessage = { key: "recommended.davinci.installed" };
    } finally {
      davinciBusy = false;
    }
  }

  let davinciPatchesResolve: ((apply: boolean) => void) | null = null;

  function confirmDavinciPatches(apply: boolean) {
    davinciPatchesOpen = false;
    davinciPatchesResolve?.(apply);
    davinciPatchesResolve = null;
  }

  // === VMware Workstation Pro (manual: pick the Broadcom .bundle the user
  // already downloaded -> run its own installer -> install Soplos's
  // vmmon/vmnet module builder on top, since VMware's own
  // vmware-modconfig step is known to fail on systemd-based distros) ===
  let vmwareConfirmOpen = false;
  let vmwareBusy = false;
  let vmwareInstallTarget: { catKey: string; pkg: RecPackage } | null = null;

  // The uninstall terminal (see uninstall_vmware) runs detached from any
  // job the frontend can poll — this is the only signal that it actually
  // closed, so the "installed" badge isn't refreshed (and found stale)
  // right as the window opens, before the user has even answered anything.
  let unlistenVmwareUninstall: UnlistenFn | undefined;
  onMount(async () => {
    // Build the apt icon index in the background so the first Enter in the
    // search box doesn't pay for decompressing the DEP-11 metadata.
    api.invoke("prewarm_search").catch(() => {});
    unlistenVmwareUninstall = await listen(
      "vmware-uninstall-finished",
      async () => {
        if (vmwareInstallTarget)
          await refreshPkgStatus(
            vmwareInstallTarget.catKey,
            vmwareInstallTarget.pkg,
          );
      },
    );
  });
  onDestroy(() => {
    unlistenVmwareUninstall?.();
  });

  async function startVmwareInstall() {
    vmwareConfirmOpen = false;
    const installerPath = await api.invoke<string | null>(
      "pick_vmware_installer",
    );
    if (!installerPath) return;

    vmwareBusy = true;
    try {
      if (!(await runTracked("install_vmware_bundle", { installerPath })))
        return;
      await runTracked("install_vmware_modules", undefined, { append: true });
      infoMessage = { key: "recommended.vmware.installed" };
      if (vmwareInstallTarget)
        await refreshPkgStatus(
          vmwareInstallTarget.catKey,
          vmwareInstallTarget.pkg,
        );
    } finally {
      vmwareBusy = false;
    }
  }

  async function confirmLicenseInstall() {
    const target = licenseConfirmTarget;
    licenseConfirmTarget = null;
    if (!target || !target.pkg.customInstall) return;
    await runTracked(target.pkg.customInstall);
    await refreshPkgStatus(target.catKey, target.pkg);
  }

  // === Search ===
  let searchQuery = "";

  // `searchQuery` has to appear directly in this statement (not just inside a
  // function it calls) or Svelte never re-runs it when you type — same
  // reactivity gotcha fixed earlier in Kernels.svelte.
  $: visibleCategories = (() => {
    const q = searchQuery.trim().toLowerCase();
    return categories
      .map((cat) => ({
        cat,
        packages: q
          ? cat.packages.filter(
              (p) =>
                p.name.toLowerCase().includes(q) ||
                $_(`recommended.desc.${slug(p.name)}`)
                  .toLowerCase()
                  .includes(q),
            )
          : cat.packages,
      }))
      .filter((c) => c.packages.length > 0);
  })();

  // === Extended search: real apt/Flathub/Snap lookup beyond the local catalog ===
  interface SearchResult {
    name: string;
    id: string;
    description: string;
    kind: "apt" | "flatpak" | "snap";
    icon: string | null;
  }

  let extendedResults: SearchResult[] = [];
  let extendedInstalledMap: Record<string, boolean> = {};
  let extendedInstalledVersion = 0;
  let extendedSearching = false;
  let extendedSearchedFor = "";

  let extendedKindFilter: "all" | "apt" | "flatpak" | "snap" = "all";
  function setKindFilter(k: "apt" | "flatpak" | "snap") {
    extendedKindFilter = extendedKindFilter === k ? "all" : k;
  }
  $: visibleExtendedResults =
    extendedKindFilter === "all"
      ? extendedResults
      : extendedResults.filter((r) => r.kind === extendedKindFilter);
  $: aptCount = extendedResults.filter((r) => r.kind === "apt").length;
  $: flatpakCount = extendedResults.filter((r) => r.kind === "flatpak").length;
  $: snapCount = extendedResults.filter((r) => r.kind === "snap").length;

  const MIN_EXTENDED_SEARCH_LEN = 2;
  let extendedSearchToken = 0;

  // Derived value used in the template (avoids repeating .trim() everywhere).
  $: sq = searchQuery.trim();

  // Clears stale extended results whenever the user edits the query so they
  // never linger. Uses beforeUpdate + a previous-value sentinel so only
  // changes to searchQuery trigger the clear — NOT changes to
  // extendedSearchedFor. A $: reactive block reading both variables would
  // re-run when extendedSearchedFor is set inside runExtendedSearch and wipe
  // the results we just received (the root cause of the empty-panel bug).
  let _prevQuery = "";
  beforeUpdate(() => {
    const current = searchQuery.trim();
    if (current !== _prevQuery) {
      _prevQuery = current;
      if (extendedSearchedFor !== "") {
        extendedResults = [];
        extendedSearchedFor = "";
        extendedKindFilter = "all";
      }
    }
  });

  // Unique key for the installed map — prevents snap/flatpak/apt entries with
  // the same id string from overwriting each other (e.g. obs-studio exists in
  // both apt and flatpak with different IDs, but generic names can collide).
  function extKey(r: SearchResult): string {
    return `${r.kind}:${r.id}`;
  }

  function isSecondaryPackage(r: SearchResult): boolean {
    const n = r.name.toLowerCase();
    const id = r.id.toLowerCase();
    // APT/system auxiliary packages: -data, -common, -doc, -dev, -dbg, -help, -l10n, -lang, etc.
    if (/-(data|common|doc|docs|dev|dbg|help(-.*)?|l10n|lang|i18n|locales|translations)$/.test(id)) return true;
    if (/-(data|common|doc|docs|dev|dbg|help(-.*)?|l10n|lang|i18n|locales|translations)$/.test(n)) return true;
    // Shared libraries
    if (/^lib[a-z0-9]/.test(id)) return true;
    // Plugins, extensions, addons
    if (/\b(plugin|plugins|extension|extensions|addon|addons)\b/.test(n)) return true;
    if (/\b(plugin|plugins|extension|extensions|addon|addons)\b/.test(id)) return true;
    return false;
  }

  async function runExtendedSearch(q: string) {
    if (q.length < MIN_EXTENDED_SEARCH_LEN) return;
    const token = ++extendedSearchToken;
    extendedSearching = true;
    progressFraction.set(null);
    isRunning.set(true);

    // Force a browser paint before launching network calls
    await tick();
    await new Promise<void>((r) => requestAnimationFrame(() => r()));
    if (token !== extendedSearchToken) return;

    try {
      const [aptResults, flatpakResults, snapResults] = await Promise.all([
        api.invoke<SearchResult[]>("search_apt", { query: q }).catch(() => []),
        api.invoke<SearchResult[]>("search_flatpak", { query: q }).catch(() => []),
        api.invoke<SearchResult[]>("search_snap", { query: q }).catch(() => []),
      ]);
      if (token !== extendedSearchToken) return;

      const merged = [...flatpakResults, ...snapResults, ...aptResults];
      const ql = q.toLowerCase();

      // Returns true if character at position `pos` in `str` is a word
      // boundary (hyphen, space, dot, underscore, end-of-string).
      const bound = (str: string, pos: number) =>
        pos >= str.length || /[-\s._]/.test(str[pos]);

      const scoreOf = (r: SearchResult) => {
        const n = r.name.toLowerCase();
        const id = r.id.toLowerCase();
        const idSegs = id.split(".");
        const lastSeg = idSegs[idSegs.length - 1] || "";
        const sec = isSecondaryPackage(r);

        // Tier 0: Direct core application match
        // E.g. "gimp" in apt/snap; "org.gimp.GIMP" in flatpak; "obs-studio" or "com.obsproject.Studio" for "obs"
        const isCoreApp =
          id === ql ||
          lastSeg === ql ||
          n === ql ||
          idSegs.includes(ql) ||
          id === `${ql}-studio` ||
          n === `${ql} studio` ||
          (lastSeg === "studio" && idSegs.some((s) => s.includes(ql)));

        if (isCoreApp && !sec) return 0;

        // Plugins and auxiliary packages are pushed below full apps
        if (sec) {
          if (/\b(plugin|plugins|extension|addon)\b/.test(n) || /\b(plugin|plugins|extension|addon)\b/.test(id)) {
            return 3;
          }
          return 4;
        }

        // Tier 1: Strong prefix or exact word match in display name or ID
        if (n.startsWith(ql) && bound(n, ql.length)) return 1;
        if (id.startsWith(ql) && bound(id, ql.length)) return 1;
        if (lastSeg.startsWith(ql) && bound(lastSeg, ql.length)) return 1;

        const words = n.split(/[-\s._]+/);
        if (words.some((w) => w === ql)) return 1;
        if (words.some((w) => w.startsWith(ql) && bound(w, ql.length))) return 2;

        // Tier 2: Substring match anywhere in name or ID
        if (n.includes(ql) || id.includes(ql)) return 2;

        // Tier 5: Description only
        return 5;
      };

      const kindPriority = (kind: string) => {
        if (kind === "flatpak") return 0;
        if (kind === "apt") return 1;
        return 2; // snap
      };

      merged.sort((a, b) => {
        const sa = scoreOf(a);
        const sb = scoreOf(b);
        if (sa !== sb) return sa - sb;
        if (a.name.length !== b.name.length) return a.name.length - b.name.length;
        return kindPriority(a.kind) - kindPriority(b.kind);
      });

      // Display results immediately without waiting for installed checks
      extendedResults = merged;
      extendedSearchedFor = q;
      extendedSearching = false;
      isRunning.set(false);
      progressFraction.set(null);

      // Check installed state via fast batch API instead of 100 individual subprocess spawns
      const aptIds = merged.filter((r) => r.kind === "apt").map((r) => r.id);
      const flatpakIds = merged.filter((r) => r.kind === "flatpak").map((r) => r.id);
      const snapNames = merged.filter((r) => r.kind === "snap").map((r) => r.id);

      const statusPromise =
        aptIds.length > 0 || flatpakIds.length > 0
          ? api
              .invoke<{
                apt: Record<string, boolean>;
                flatpak: Record<string, boolean>;
              }>("get_package_statuses", {
                query: { apt: aptIds, flatpak: flatpakIds, paths: [] },
              })
              .catch(() => null)
          : Promise.resolve(null);

      const snapChecks = snapNames.map((name) =>
        api.invoke<boolean>("is_snap_installed", { name }).catch(() => false),
      );

      const [statuses, ...snapInstalledList] = await Promise.all([
        statusPromise,
        ...snapChecks,
      ]);

      if (token !== extendedSearchToken) return;

      const newMap: Record<string, boolean> = {};
      let snapIdx = 0;
      for (const r of merged) {
        if (r.kind === "apt") {
          newMap[extKey(r)] = statuses?.apt[r.id] ?? false;
        } else if (r.kind === "flatpak") {
          newMap[extKey(r)] = statuses?.flatpak[r.id] ?? false;
        } else {
          newMap[extKey(r)] = snapInstalledList[snapIdx++] ?? false;
        }
      }
      extendedInstalledMap = newMap;
      extendedInstalledVersion++;
    } finally {
      if (token === extendedSearchToken) {
        extendedSearching = false;
        isRunning.set(false);
        progressFraction.set(null);
      }
    }
  }

  async function toggleExtendedResult(result: SearchResult) {
    const key = extKey(result);
    const installed = extendedInstalledMap[key];
    if (result.kind === "apt") {
      await runTracked(
        installed ? "uninstall_apt_packages" : "install_apt_packages",
        { packages: result.id },
      );
      extendedInstalledMap[key] = await api.invoke<boolean>(
        "is_apt_package_installed",
        { package: result.id },
      );
    } else if (result.kind === "snap") {
      await runTracked(installed ? "uninstall_snap" : "install_snap", {
        name: result.id,
      });
      extendedInstalledMap[key] = await api.invoke<boolean>(
        "is_snap_installed",
        { name: result.id },
      );
    } else {
      if (installed) {
        const scope =
          (await api.invoke<string | null>("flatpak_app_scope", {
            appId: result.id,
          })) ?? "user";
        await runTracked("uninstall_flatpak_scoped", {
          appId: result.id,
          scope,
        });
      } else {
        const state = await api.invoke<string>("flatpak_state");
        if (state === "missing") {
          infoMessage = {
            key: "softwareTile.flatpakMissing",
            name: result.name,
          };
          return;
        }
        const scope: "user" | "system" =
          state === "both"
            ? await chooseFlatpakScope(result.name)
            : state === "system"
              ? "system"
              : "user";
        await runTracked("install_flatpak_scoped", { appId: result.id, scope });
      }
      extendedInstalledMap[key] = await api.invoke<boolean>(
        "is_flatpak_installed",
        { appId: result.id },
      );
    }
    extendedInstalledVersion++;
  }

  // === Accordion (single-open normally; all open while batch mode is active) ===
  let expanded: string | null = null;
  function toggleFamily(key: string) {
    expanded = expanded === key ? null : key;
  }

  // === Batch mode ===
  let batchMode = false;
  let selected = new Set<string>();

  function toggleBatchMode() {
    batchMode = !batchMode;
    if (!batchMode) selected = new Set();
  }

  // Selectable in batch mode regardless of installed state — applyToSelected
  // runs togglePkg per item, which installs the not-yet-installed ones and
  // uninstalls the already-installed ones, same as a single click would.
  function isSelectable(catKey: string, pkg: RecPackage): boolean {
    if (pkg.kind === "manual") return false;
    if (
      pkg.customInstall === "install_waydroid" &&
      waydroidBlocked &&
      !installedMap[pkgKey(catKey, pkg)]
    )
      return false;
    return true;
  }

  function toggleSelected(catKey: string, pkg: RecPackage) {
    const key = pkgKey(catKey, pkg);
    if (selected.has(key)) {
      selected.delete(key);
    } else {
      selected.add(key);
    }
    selected = selected;
  }

  function selectAll() {
    const next = new Set(selected);
    for (const { cat, packages } of visibleCategories) {
      for (const pkg of packages) {
        if (isSelectable(cat.key, pkg)) next.add(pkgKey(cat.key, pkg));
      }
    }
    selected = next;
  }

  function deselectAll() {
    selected = new Set();
  }

  function findSelectedPackages(): { catKey: string; pkg: RecPackage }[] {
    const result: { catKey: string; pkg: RecPackage }[] = [];
    for (const cat of categories) {
      for (const pkg of cat.packages) {
        if (selected.has(pkgKey(cat.key, pkg)))
          result.push({ catKey: cat.key, pkg });
      }
    }
    return result;
  }

  let batchBusy = false;

  // Runs togglePkg for every selected item — installs the ones not yet
  // installed and uninstalls the ones that already are, exactly like
  // clicking each one individually would. If several selected items are
  // Flatpaks being installed on a system where both scopes are available,
  // asking once per app would mean confirming the same choice 50 times in
  // a row for a big batch — so that choice is made once upfront and reused
  // for all of them instead.
  async function applyToSelected() {
    batchBusy = true;
    const targets = findSelectedPackages();

    let forcedScope: "user" | "system" | undefined;
    const needsScopeChoice = targets.some(
      ({ catKey, pkg }) =>
        pkg.kind === "flatpak" &&
        pkg.flatpak &&
        !installedMap[pkgKey(catKey, pkg)],
    );
    if (
      needsScopeChoice &&
      (await api.invoke<string>("flatpak_state")) === "both"
    ) {
      forcedScope = await chooseFlatpakScope(
        $_("recommended.selectedCount", { values: { count: targets.length } }),
      );
    }

    for (const [i, { catKey, pkg }] of targets.entries()) {
      await togglePkg(catKey, pkg, forcedScope, i > 0);
    }
    selected = new Set();
    batchBusy = false;
  }

  // One batched call for every app instead of ~100 concurrent per-app calls
  // (each spawning its own subprocess and re-rendering the whole list on
  // arrival). Same precedence as refreshPkgStatus: checkPath, then apt
  // package, then flatpak id; anything else stays "not installed".
  onMount(async () => {
    const apt = new Set<string>();
    const flatpak = new Set<string>();
    const paths = new Set<string>();
    for (const cat of categories) {
      for (const pkg of cat.packages) {
        if (pkg.checkPath) paths.add(pkg.checkPath);
        else if (pkg.package) apt.add(pkg.package);
        else if (pkg.flatpak) flatpak.add(pkg.flatpak);
      }
    }
    try {
      const result = await api.invoke<{
        apt: Record<string, boolean>;
        flatpak: Record<string, boolean>;
        paths: Record<string, boolean>;
      }>("get_package_statuses", {
        query: { apt: [...apt], flatpak: [...flatpak], paths: [...paths] },
      });
      for (const cat of categories) {
        for (const pkg of cat.packages) {
          let installed = false;
          if (pkg.checkPath) installed = result.paths[pkg.checkPath] ?? false;
          else if (pkg.package) installed = result.apt[pkg.package] ?? false;
          else if (pkg.flatpak)
            installed = result.flatpak[pkg.flatpak] ?? false;
          installedMap[pkgKey(cat.key, pkg)] = installed;
        }
      }
      installedVersion++;
    } catch (error) {
      console.error("Failed to read package statuses", error);
    }

    await refreshAntigravityStatus();

    // Waydroid needs a Wayland compositor to work at all (it renders through
    // wlr-layer-shell); Tyron is still X11-only for now, so installing it
    // there would just leave a broken container nobody can launch. Checked
    // here instead of only warning after the fact, same approach as
    // grub-btrfs' is_btrfs_root() guard in soplos-grub-editor: read real
    // system state once and use it to block the action, not just the button.
    try {
      const env = await api.invoke<{ displayProtocol: string }>(
        "get_environment_info",
      );
      displayProtocol = env.displayProtocol;
    } catch (error) {
      console.error("Failed to read environment info", error);
    }
  });
</script>

<div
  class="relative rounded border border-border overflow-hidden mb-5 flex flex-col items-start justify-end text-left p-4 min-h-[200px]"
  style="background: linear-gradient(rgba(20, 20, 22, 0.35), rgba(20, 20, 22, 0.85)), url('/assets/hero/recommended-background.png'); background-size: cover; background-position: center;"
>
  <p class="text-white font-semibold text-[1rem]">
    {$_("recommended.hero.title")}
  </p>
  <p class="text-[0.8rem] text-white/85 mt-1">
    {$_("recommended.hero.description")}
  </p>
</div>

<div
  class="sticky top-[-28px] z-30 bg-dark flex items-center gap-2.5 mb-4 py-2"
>
  <div class="relative flex-1 max-w-[360px]">
    <Search
      size={15}
      class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-muted"
    />
    <input
      type="text"
      placeholder={$_("recommended.searchPlaceholder")}
      bind:value={searchQuery}
      on:keydown={(e) => {
        if (e.key === "Enter") runExtendedSearch(searchQuery.trim());
      }}
      class="w-full pl-8 pr-7 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.82rem] placeholder:text-text-muted focus:outline-none focus:border-primary"
    />
    {#if searchQuery.length > 0}
      <button
        on:click={() => {
          extendedSearchToken++; // cancel any in-flight search
          extendedSearching = false;
          extendedResults = [];
          extendedSearchedFor = "";
          searchQuery = "";
        }}
        title={$_("recommended.clearSearch")}
        class="absolute right-2 top-1/2 -translate-y-1/2 text-text-muted hover:text-text-bright"
      >
        <X size={14} />
      </button>
    {/if}
  </div>
  <button
    class="px-3 py-1.5 rounded text-[0.78rem] font-semibold shrink-0 {batchMode
      ? 'bg-primary hover:bg-primary-hover text-text-bright'
      : 'border border-border text-text hover:bg-surface hover:text-text-bright'}"
    on:click={toggleBatchMode}
  >
    {batchMode ? $_("recommended.modeNormal") : $_("recommended.modeMultiple")}
  </button>
</div>

<div
  class="grid grid-cols-1 md:grid-cols-2 gap-3.5 {batchMode &&
  !($terminalLines.length > 0 && $terminalExpanded)
    ? 'pb-20'
    : ''}"
>
  {#each visibleCategories as { cat, packages }}
    <div
      class="bg-surface border border-border rounded p-4 transition-all {expanded ===
        cat.key ||
      batchMode ||
      searchQuery.trim() !== ''
        ? 'md:col-span-2'
        : ''}"
    >
      <button
        class="w-full flex items-center justify-between gap-3"
        on:click={() => toggleFamily(cat.key)}
      >
        <div class="flex items-center gap-3">
          <div
            class="w-9 h-9 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden"
          >
            <img
              src="/assets/icons/{cat.key}/{cat.packages[0].icon}"
              alt=""
              class="w-full h-full object-contain"
            />
          </div>
          <div class="text-left">
            <div class="text-text-bright font-semibold text-[0.9rem]">
              {$_(`recommended.categories.${cat.key}`)}
            </div>
            <div class="text-[0.75rem] text-text-muted">
              {$_("recommended.appsCount", {
                values: { count: packages.length },
              })}
            </div>
          </div>
        </div>
        <ChevronDown
          size={18}
          class="text-text-muted transition-transform {expanded === cat.key ||
          batchMode ||
          searchQuery.trim() !== ''
            ? 'rotate-180'
            : ''}"
        />
      </button>

      {#if expanded === cat.key || batchMode || searchQuery.trim() !== ""}
        <div
          class="grid grid-cols-1 sm:grid-cols-2 gap-2.5 mt-4 pt-4 border-t border-border"
        >
          {#each packages as pkg}
            {@const key = pkgKey(cat.key, pkg)}
            {@const installed = installedVersion >= 0 && installedMap[key]}
            {@const selectable = isSelectable(cat.key, pkg)}
            <div
              class="border rounded p-3 flex items-start gap-2.5 {batchMode &&
              selectable
                ? 'cursor-pointer ' +
                  (selected.has(key)
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:border-primary/50')
                : 'border-border'}"
              role="checkbox"
              aria-checked={batchMode && selectable ? selected.has(key) : false}
              tabindex={batchMode && selectable ? 0 : -1}
              on:click={() =>
                batchMode && selectable && toggleSelected(cat.key, pkg)}
              on:keydown={(e) => {
                if (
                  batchMode &&
                  selectable &&
                  (e.key === "Enter" || e.key === " ")
                ) {
                  e.preventDefault();
                  toggleSelected(cat.key, pkg);
                }
              }}
            >
              {#if batchMode}
                <input
                  type="checkbox"
                  class="mt-1 shrink-0"
                  disabled={!selectable}
                  checked={selected.has(key)}
                  on:click|stopPropagation={() =>
                    selectable && toggleSelected(cat.key, pkg)}
                  on:change|preventDefault
                />
              {/if}
              <img
                src="/assets/icons/{cat.key}/{pkg.icon}"
                alt=""
                class="w-9 h-9 object-contain shrink-0"
              />
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-1.5 flex-wrap">
                  <span class="text-[0.85rem] font-semibold text-text-bright"
                    >{pkg.name}</span
                  >
                  {#if pkg.kind === "flatpak"}
                    <span
                      class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted"
                      >Flatpak</span
                    >
                  {/if}
                  {#if pkg.badge}
                    <span
                      class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted"
                      >{pkg.badge}</span
                    >
                  {:else if pkg.checkPath?.includes("AppImages")}
                    <span
                      class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted"
                      >AppImage</span
                    >
                  {/if}
                  {#if pkg.official && pkg.kind === "apt"}
                    <span
                      class="text-[0.62rem] px-1.5 py-0.5 rounded bg-success/15 text-success"
                      >{$_("recommended.official")}</span
                    >
                  {/if}
                </div>
                <p class="text-[0.72rem] text-text mt-0.5">
                  {$_(`recommended.desc.${slug(pkg.name)}`)}
                </p>
                {#if !batchMode}
                  {#if pkg.kind === "manual"}
                    {@const isManualBusy =
                      pkg.customInstall === "install_vmware_bundle"
                        ? vmwareBusy
                        : davinciBusy}
                    <button
                      class="mt-2 px-2.5 py-1.5 rounded text-[0.75rem] font-semibold disabled:opacity-60 {installed
                        ? 'border border-danger text-danger hover:bg-danger/10'
                        : 'bg-primary hover:bg-primary-hover text-text-bright'}"
                      disabled={isManualBusy}
                      on:click={() => togglePkg(cat.key, pkg)}
                    >
                      {isManualBusy
                        ? $_(
                            installed
                              ? "recommended.uninstalling"
                              : "recommended.installing",
                          )
                        : $_(installed ? "common.uninstall" : "common.install")}
                    </button>
                  {:else}
                    {@const isAntigravity =
                      pkg.customInstall === "install_antigravity"}
                    {@const isWaydroidBlocked =
                      pkg.customInstall === "install_waydroid" &&
                      waydroidBlocked &&
                      !installed}
                    <div class="mt-2 flex items-center gap-2 flex-wrap">
                      {#if isAntigravity && installed && antigravityUpdateAvailable}
                        <button
                          class="px-2.5 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.75rem] font-semibold"
                          on:click={() => updateAntigravity(cat.key, pkg)}
                        >
                          {$_("recommended.updateTo", {
                            values: { version: antigravityLatestVersion },
                          })}
                        </button>
                      {/if}
                      <button
                        class="px-2.5 py-1.5 rounded text-[0.75rem] font-semibold disabled:opacity-50 disabled:cursor-not-allowed {installed
                          ? 'border border-danger text-danger hover:bg-danger/10'
                          : 'bg-primary hover:bg-primary-hover text-text-bright'}"
                        disabled={isWaydroidBlocked}
                        title={isWaydroidBlocked
                          ? $_("recommended.waydroidNeedsWayland")
                          : undefined}
                        on:click={() => togglePkg(cat.key, pkg)}
                      >
                        {installed
                          ? $_("common.uninstall")
                          : $_("common.install")}
                      </button>
                    </div>
                    {#if isWaydroidBlocked}
                      <p class="text-[0.65rem] text-warning mt-1">
                        {$_("recommended.waydroidNeedsWayland")}
                      </p>
                    {/if}
                    {#if isAntigravity && installed && antigravityInstalledVersion}
                      <p class="text-[0.65rem] text-text-muted mt-1">
                        {antigravityUpdateAvailable
                          ? $_("recommended.installedVersion", {
                              values: { version: antigravityInstalledVersion },
                            })
                          : $_("recommended.version", {
                              values: { version: antigravityInstalledVersion },
                            })}
                      </p>
                    {/if}
                  {/if}
                {:else if installed}
                  <span
                    class="inline-block mt-2 text-[0.68rem] px-2 py-0.5 rounded-full {selected.has(
                      key,
                    )
                      ? 'bg-danger/15 text-danger'
                      : 'bg-success/15 text-success'}"
                  >
                    {selected.has(key)
                      ? $_("recommended.willBeUninstalled")
                      : $_("drivers.badge.installed")}
                  </span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <p class="text-[0.82rem] text-text-muted text-center py-10">
      {$_("recommended.noResults", { values: { query: searchQuery } })}
    </p>
  {/each}
</div>

{#if sq !== "" && !batchMode}
  <div class="bg-surface border border-border rounded p-4 mt-3.5">
    {#if sq.length < MIN_EXTENDED_SEARCH_LEN}
      <p class="text-[0.75rem] text-text-muted">
        Sigue escribiendo y pulsa Intro para buscar en apt, Flathub y Snap…
      </p>
    {:else if extendedSearching}
      <p class="text-[0.75rem] text-text-muted">
        Buscando "{sq}" en apt, Flathub y Snap…
      </p>
    {:else if extendedSearchedFor !== sq}
      <p class="text-[0.75rem] text-text-muted">
        Pulsa Intro para buscar "{sq}" en apt, Flathub y Snap
      </p>
    {:else if extendedResults.length === 0}
      <p class="text-[0.82rem] text-text-muted">
        Sin resultados en apt, Flathub ni Snap para "{extendedSearchedFor}".
      </p>
    {:else}
      <div class="flex items-center justify-between flex-wrap gap-2.5 mb-3">
        <p class="text-[0.75rem] text-text-muted">
          {#if extendedKindFilter === "all"}
            {extendedResults.length} resultados de apt, Flathub y Snap para "{extendedSearchedFor}"
          {:else if extendedKindFilter === "apt"}
            {visibleExtendedResults.length} resultados de APT para "{extendedSearchedFor}"
          {:else if extendedKindFilter === "flatpak"}
            {visibleExtendedResults.length} resultados de Flathub para "{extendedSearchedFor}"
          {:else}
            {visibleExtendedResults.length} resultados de Snap para "{extendedSearchedFor}"
          {/if}
        </p>
        <div class="flex items-center gap-1.5 shrink-0">
          <button
            type="button"
            class="px-2.5 py-1 text-[0.72rem] rounded border transition-colors flex items-center gap-1.5 {extendedKindFilter === 'apt'
              ? 'bg-primary border-primary text-text-bright font-semibold'
              : 'bg-surface-2 border-border text-text hover:text-text-bright hover:border-border-hover'}"
            on:click={() => setKindFilter("apt")}
          >
            <span>APT</span>
            {#if aptCount > 0}
              <span
                class="text-[0.62rem] px-1.5 py-0.2 rounded-full {extendedKindFilter === 'apt'
                  ? 'bg-white/20 text-text-bright'
                  : 'bg-surface text-text-muted'}"
              >
                {aptCount}
              </span>
            {/if}
          </button>
          <button
            type="button"
            class="px-2.5 py-1 text-[0.72rem] rounded border transition-colors flex items-center gap-1.5 {extendedKindFilter === 'flatpak'
              ? 'bg-primary border-primary text-text-bright font-semibold'
              : 'bg-surface-2 border-border text-text hover:text-text-bright hover:border-border-hover'}"
            on:click={() => setKindFilter("flatpak")}
          >
            <span>Flatpak</span>
            {#if flatpakCount > 0}
              <span
                class="text-[0.62rem] px-1.5 py-0.2 rounded-full {extendedKindFilter === 'flatpak'
                  ? 'bg-white/20 text-text-bright'
                  : 'bg-surface text-text-muted'}"
              >
                {flatpakCount}
              </span>
            {/if}
          </button>
          <button
            type="button"
            class="px-2.5 py-1 text-[0.72rem] rounded border transition-colors flex items-center gap-1.5 {extendedKindFilter === 'snap'
              ? 'bg-primary border-primary text-text-bright font-semibold'
              : 'bg-surface-2 border-border text-text hover:text-text-bright hover:border-border-hover'}"
            on:click={() => setKindFilter("snap")}
          >
            <span>Snap</span>
            {#if snapCount > 0}
              <span
                class="text-[0.62rem] px-1.5 py-0.2 rounded-full {extendedKindFilter === 'snap'
                  ? 'bg-white/20 text-text-bright'
                  : 'bg-surface text-text-muted'}"
              >
                {snapCount}
              </span>
            {/if}
          </button>
        </div>
      </div>
      {#if visibleExtendedResults.length === 0}
        <p class="text-[0.78rem] text-text-muted py-6 text-center">
          No hay resultados de {extendedKindFilter === "apt" ? "APT" : extendedKindFilter === "flatpak" ? "Flatpak" : "Snap"} para "{extendedSearchedFor}".
        </p>
      {:else}
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
          {#each visibleExtendedResults as result (result.kind + ":" + result.id)}
          {@const installed =
            extendedInstalledVersion >= 0 && extendedInstalledMap[extKey(result)]}
          <div
            class="border border-border rounded p-3 flex items-start gap-2.5"
          >
            {#if result.icon}
              <img
                src={result.icon}
                alt=""
                class="w-9 h-9 rounded shrink-0 object-contain"
              />
            {:else}
              <div
                class="w-9 h-9 rounded shrink-0 bg-surface-2 flex items-center justify-center"
              >
                <Package size={18} class="text-text-muted" />
              </div>
            {/if}
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5 flex-wrap">
                <span class="text-[0.85rem] font-semibold text-text-bright"
                  >{result.name}</span
                >
                <span
                  class="text-[0.62rem] px-1.5 py-0.5 rounded bg-surface-2 text-text-muted"
                  >{result.kind === "apt"
                    ? "APT"
                    : result.kind === "flatpak"
                      ? "Flatpak"
                      : "Snap"}</span
                >
                {#if installed}
                  <span
                    class="text-[0.62rem] px-1.5 py-0.5 rounded bg-success/15 text-success"
                    >{$_("drivers.badge.installed")}</span
                  >
                {/if}
              </div>
              <p class="text-[0.72rem] text-text mt-0.5">
                {result.description}
              </p>
              <button
                class="mt-2 px-2.5 py-1.5 rounded text-[0.75rem] font-semibold {installed
                  ? 'border border-danger text-danger hover:bg-danger/10'
                  : 'bg-primary hover:bg-primary-hover text-text-bright'}"
                on:click={() => toggleExtendedResult(result)}
              >
                {installed ? $_("common.uninstall") : $_("common.install")}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
    {/if}
  </div>
{/if}

{#if batchMode}
  <div
    class="fixed {$isRunning
      ? 'bottom-[6px]'
      : 'bottom-0'} left-[230px] right-0 bg-surface border-t border-border px-6 py-3 flex items-center gap-3 z-40"
  >
    <button
      class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem]"
      on:click={selectAll}
    >
      {$_("recommended.selectAll")}
    </button>
    <button
      class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 text-[0.78rem]"
      on:click={deselectAll}
    >
      {$_("recommended.deselectAll")}
    </button>
    <span class="flex-1 text-center text-[0.8rem] text-text-muted">
      {$_("recommended.selectedCount", { values: { count: selected.size } })}
    </span>
    <button
      class="px-4 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.82rem] font-semibold disabled:opacity-50"
      disabled={selected.size === 0 || batchBusy}
      on:click={applyToSelected}
    >
      {batchBusy ? $_("gaming.working") : $_("recommended.applySelected")}
    </button>
  </div>
{/if}

{#if $terminalLines.length > 0 && $terminalExpanded}
  <div class="mt-4 {batchMode ? 'pb-20' : ''}" id="job-terminal-anchor">
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
  <Modal
    variant={infoMessage.variant ?? "info"}
    maxWidth={420}
    onClose={() => (infoMessage = null)}
  >
    <p class="whitespace-pre-line">
      {$_(infoMessage.key, { values: { name: infoMessage.name ?? "" } })}
    </p>
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

{#if davinciConfirmOpen}
  <Modal
    variant="confirm"
    title={$_("recommended.davinci.title")}
    confirmLabel={$_("kernels.continue")}
    onClose={() => (davinciConfirmOpen = false)}
    onConfirm={startDavinciInstall}
  >
    <p class="whitespace-pre-line text-text-muted">
      {$_("recommended.davinci.body")}
    </p>
  </Modal>
{/if}

{#if vmwareConfirmOpen}
  <Modal
    variant="confirm"
    title={$_("recommended.vmware.title")}
    confirmLabel={$_("kernels.continue")}
    onClose={() => (vmwareConfirmOpen = false)}
    onConfirm={startVmwareInstall}
  >
    <p class="whitespace-pre-line text-text-muted">
      {$_("recommended.vmware.body")}
    </p>
  </Modal>
{/if}

{#if licenseConfirmTarget}
  <Modal
    variant="warning"
    title={$_(
      `recommended.license.${slug(licenseConfirmTarget.pkg.name)}.title`,
    )}
    confirmLabel={$_("recommended.license.acceptInstall")}
    onClose={() => (licenseConfirmTarget = null)}
    onConfirm={confirmLicenseInstall}
  >
    <p class="text-text-muted">
      {$_(`recommended.license.${slug(licenseConfirmTarget.pkg.name)}.body`)}
    </p>
  </Modal>
{/if}

{#if davinciPatchesOpen}
  <Modal
    variant="confirm"
    maxWidth={480}
    title={$_("recommended.davinci.patchesTitle")}
    confirmLabel={$_("recommended.davinci.apply")}
    cancelLabel={$_("recommended.davinci.skip")}
    onClose={() => confirmDavinciPatches(false)}
    onConfirm={() => confirmDavinciPatches(true)}
  >
    <p class="text-text-muted mb-3">{$_("recommended.davinci.patchesIntro")}</p>
    <div class="flex flex-col gap-2.5">
      <label
        class="flex items-start gap-2.5 px-3 py-2.5 rounded-lg border border-border cursor-pointer hover:border-primary/50 {davinciApplyMic
          ? 'border-primary bg-primary/5'
          : ''}"
      >
        <input type="checkbox" class="mt-1" bind:checked={davinciApplyMic} />
        <span>
          <span class="block text-[0.85rem] font-semibold text-text-bright"
            >{$_("recommended.davinci.micTitle")}</span
          >
          <span class="block text-[0.75rem] text-text-muted"
            >{$_("recommended.davinci.micHint")}</span
          >
        </span>
      </label>
      <label
        class="flex items-start gap-2.5 px-3 py-2.5 rounded-lg border border-border cursor-pointer hover:border-primary/50 {davinciApplyGpu
          ? 'border-primary bg-primary/5'
          : ''}"
      >
        <input type="checkbox" class="mt-1" bind:checked={davinciApplyGpu} />
        <span>
          <span class="block text-[0.85rem] font-semibold text-text-bright"
            >{$_("recommended.davinci.gpuTitle")}</span
          >
          <span class="block text-[0.75rem] text-text-muted"
            >{$_("recommended.davinci.gpuHint")}</span
          >
        </span>
      </label>
    </div>
  </Modal>
{/if}
