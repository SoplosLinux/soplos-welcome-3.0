# Changelog

All notable changes to Soplos Welcome are documented in this file.

## [3.0.1-8-beta] - 2026-10-29

### Changed

- New application icon: an open welcome box with the shield, the chip and the gamepad rising out of it, and the Soplos seal on the ribbon, in place of the S of the logo. It is changed in the sizes of the package (`.ico`, `.icns` and the square logos), the window and the About dialog.

## [3.0.1-7-beta] - 2026-10-28

### Changed

- New application icon, in the sizes of the package, the window and the interface, and the new icons of the Soplos programs on the pages that list them.
- New hero images for the pages and the sections of the program, with the circuit style of the other Soplos 3.0 programs.
- New screenshots of every page, taken with the interface in English.

## [3.0.1-6-beta] - 2026-10-28

### Fixed

- Recommended tab (Development): Google Antigravity version detection now reads `ideVersion` from `product.json` instead of VS Code's base version in `package.json`, accurately identifying installed versions and update availability for 2.6.0.

## [3.0.1-5-beta] - 2026-10-24

### Added

- Gaming tab (launchers): Heronite, a light game launcher written in Rust that gathers Steam, Epic, GOG and Amazon games. It is installed from the `.deb` of its GitHub release, like R2ModMan, and the description is translated in the 8 supported languages.

## [3.0.1-4-beta] - 2026-10-24

### Added

- The hardware scan recognises an Apple computer from its DMI data and, in the Drivers tab, offers what a Mac needs: mbpfan (fan control) and TLP (battery) on MacBooks, both from Debian, and the driver of the FaceTime HD camera on any Mac that has it (Broadcom 1570), which is not in Debian. TLP stops the install instead of removing power-profiles-daemon when the desktop uses it. The camera driver is a community project and experimental: it is built into a local package from sources fetched at a pinned commit, so DKMS rebuilds it for every kernel and it can be removed, and the camera firmware is downloaded from Apple's update servers and checked against pinned checksums. Nothing is shown on a computer that is not a Mac.

### Fixed

- The output of the terminal had a fixed dark background, so in the light theme it stayed dark while its text changed color. It now uses the panel color of the theme.

## [3.0.1-3-beta] - 2026-10-23

### Added

- The window follows the light or dark theme of the desktop. It starts in the theme of the desktop and notices when it changes. The theme is read from the XDG portal and, when that says nothing, from each desktop: the colors of `kdeglobals` on Plasma, the name of the GTK theme on Xfce (a guess from words such as "dark") and the color scheme of GNOME. The button of the sidebar still switches the theme and keeps what it chose; switching to the theme the desktop has goes back to following it.

## [3.0.1-2-beta] - 2026-10-22

### Changed

- Gaming: OpenRGB is now installed from Debian together with its udev rules, instead of as a Flatpak. The Debian package ships no rules, and a Flatpak cannot give the host access to the USB and I2C devices, so the installer generates them with `openrgb --print-udev-rules` into `/etc/udev/rules.d/60-openrgb.rules`, loads the `i2c-dev` module now and at every boot (`/etc/modules-load.d/soplos-openrgb.conf`) and reloads udev. If OpenRGB does not generate its rules the installation fails with a clear message. Removing it also removes the rules and the module file. An OpenRGB already installed as a Flatpak is not detected by the card.

### Fixed

- Gaming: installing the wallpapers did nothing in the three desktops while reporting success. The privileged scripts run under `sh` (dash), which has no brace expansion, so `*.{jpg,png,...}` stayed literal and no file matched. The image extensions are now matched with `case`, the XFCE links and the GNOME list use the same match, and the script fails with an error if the archive holds no image.

## [3.0.1-1-beta] - 2026-10-22

### Added

- Drivers: NVIDIA DKMS patch for the Debian 550 driver on Linux 7.2+ (`NV_550_KERNEL72_COMPAT_PATCH`, Fix 6). NVIDIA 550.163.01 no longer built on kernels 7.2+: `linux/of_gpio.h` and `strncpy()` were removed from the kernel and `struct drm_atomic_state` became `struct drm_atomic_commit`. It is applied only when the source tree still has the 550 text and `patch --dry-run` succeeds.
- Drivers: NVIDIA DKMS patch for the 590 open modules on Linux 7.2+ (`NV_590_KERNEL72_COMPAT_PATCH`, Fix 7). It carries the Fix 6 changes and backports the `nvidia-uvm` changes NVIDIA 610 already has (`page_free` renamed `folio_free`, three-argument `zone_device_page_init()`) through two `conftest.sh` probes. Both patches build all five modules against `7.3.0-soplos-bore-ntsync-v4` and `7.2.11-soplos-bore-ntsync-v4`.
- Gaming launchers: Xodus Nexus, an unofficial Xbox and Microsoft Store (Game Pass) launcher, and the official Epic Games Launcher. Both are AppImages installed in `~/AppImages` with a user `.desktop` entry, like ES-DE, and need no root. Xodus Nexus resolves the latest GitHub release and checks the `.sha512sum` file the project publishes; the Epic Games Launcher is downloaded from Epic's own installer URL, which publishes no checksum, and its entry registers the `com.epicgames.launcher` URL scheme. The icons come from the upstream projects, and the descriptions are in the eight languages.
- Recommended apps: the rest of the ArtCraft suite by Storytold that ships a Linux package. WordCraft, GridCraft and DeckCraft (Word, Excel and PowerPoint alternatives) go in Office, and DesignCraft and FigCraft (InDesign and Figma alternatives) go in Graphics. They install the newest `linux-x86_64.deb` of their GitHub release, like the other Craft apps, with descriptions in the eight languages. CADCraft is left out because there is no CAD section, and AniCraft because it has no published release yet.
- `CHANGELOG.md` is installed as `/usr/share/doc/soplos-welcome/changelog.gz`.
- AppStream metainfo: categories and keywords in the eight languages, the same ones as the desktop entry.
- The Debian copyright now credits the third-party material the package ships: the DaVinci Resolve package creator `makeresolvedeb` by Daniel Tufvesson (distributed by its author without a license statement), the VirtualBox 7.2.10 Guest Additions by Oracle, the application logos in `appimage-icons`, and the `soplos-vmware-modules` package, which holds only Soplos build and kernel-hook scripts and no VMware code. Lucide (ISC), used for the interface icons, is declared too.
- README: version, license, platform, Tauri, Rust and Svelte badges.

### Changed

- Manual page rewritten: the eight sections, the languages, the privilege model, the internal `--run-privileged-script` option, keyboard shortcuts and files. It is installed compressed, as `/usr/share/man/man1/soplos-welcome.1.gz`.

### Fixed

- Drivers: the `NV_DMEM_CGROUP_INIT_PATCH` added in 3.0.1-beta was never applied. Its hunk header declared 7 old and 17 new lines while the body has 6 and 15, so `patch` rejected it as malformed and the install script only printed a warning. The header is corrected (`@@ -2818,6 +2818,15 @@`); it now applies to NVIDIA 610.57.04 and the module builds against the Linux 7.3 headers.
- Drivers: the repository install and the CUDA install applied the DKMS patches after `apt install`, but the driver package builds its DKMS module while dpkg configures it, so that first build ran on unpatched sources and, with `set -e`, a failure there aborted the script before any patch was applied. The install now tolerates that failure, applies the patches, runs `dpkg --configure -a` and repeats the install; in the CUDA install this happens before the repository is removed.
- AppStream metainfo: the screenshots were bare file names instead of web addresses, so software centers could not show them and the file did not validate. They now use their `raw.githubusercontent.com` address.
- Debian copyright: the `Source` field pointed to `soplos-welcome` and now points to the `soplos-welcome-3.0` repository.
- Manual page and README no longer mention `~/.config/soplos-welcome/` (never used by the application) or a system-wide autostart entry in `/etc/xdg/autostart/` (the package only creates the user one, `~/.config/autostart/org.soplos.welcome.desktop`).
- README version badge was still at `3.0.0-9-beta`.

## [3.0.1-beta] - 2026-10-20

### Fixed

- Added NVIDIA DKMS compatibility patch for Linux 7.3+ kernels (`NV_DMEM_CGROUP_INIT_PATCH`), resolving `dmem_cgroup_register_region` API signature changes on 610/Turing+ open modules.

## [3.0.0-9-beta] - 2026-10-19

### Added

- Added Unix manual page documentation (`soplos-welcome.1`) installed to `/usr/share/man/man1/`.
- Full 8-language translations for desktop entry (`soplos-welcome.desktop.hbs`) and AppStream metainfo.
- Standard Debian DEP-5 copyright documentation (`resources/debian/copyright`).
- Building and compilation documentation in `README.md`.

## [3.0.0-8-beta] - 2026-10-18

### Fixed

- Startup crash on Wayland (`Error 71 (Error de protocolo) dispatching to Wayland display`) when running on bare-metal hardware with NVIDIA/Nouveau drivers or following graphics driver changes: forced `WEBKIT_DISABLE_DMABUF_RENDERER=1` by default across all hardware.

## [3.0.0-7-beta] - 2026-10-17

### Changed

- Software tab: "Soplos Software Center" renamed to "Soplos Store" (new name, package `soplos-store` and new icon) in the GNOME, Xfce and Plasma variants.

## [3.0.0-6-beta] - 2026-10-16

### Added

- Software tab: new "Soplos Software Center" entry next to Repo Selector (GNOME, Xfce and Plasma variants), installable/removable like the other store front-ends.

## [3.0.0-5-beta] - 2026-10-14

### Added

- New Recommended applications: ArtCraft Launcher (Storytold's own installer/updater for the whole Craft app suite), under "App management", and SoundCraft (the suite's audio editor), under Multimedia's audio apps.

### Fixed

- Security tab's "Soplos Sys Cleaner" entry launched the advanced program (`soplos-sys-cleaner`) instead of the basic one (`soplos-sys-cleaner-basic`) it's meant to open.
- Recommended's multi-select action bar (select all / install / uninstall) covered the window's progress strip while a batch job was running, so progress was invisible in that mode even though it worked fine for single installs/uninstalls.
- Recommended's extended search (apt / Flathub / Snap) froze the whole window for a long time after pressing Enter, with the global progress bar showing but static. The three search commands ran on the UI's main thread; they now run on a background thread pool, so the window stays responsive and the bar animates while searching.
- The first extended search also paid for building the apt icon index (decompressing the DEP-11 metadata); that index is now built in the background when the Recommended tab opens.
- Flathub and Snap searches now time out after 8 seconds instead of potentially hanging, and the Snap query is URL-encoded so queries with spaces or special characters work.

## [3.0.0-4-beta] - 2026-10-10

### Fixed

- Ralink/MediaTek Wi-Fi firmware install button did nothing: it offered `firmware-ralink`, which has no install candidate in Debian testing/forky. Switched to `firmware-mediatek`, which absorbed the Ralink firmware blobs after MediaTek's acquisition of Ralink.
- Broadcom Wi-Fi chips with no open-source driver (BCM4311, BCM4321, BCM43142 and others) had no working install option — only the open `firmware-b43-installer` was offered, which doesn't cover them. Added a dedicated "Broadcom Wi-Fi (proprietary)" entry installing `broadcom-sta-dkms`.

## [3.0.0-3-beta] - 2026-10-08

### Added

- New Recommended applications: the ArtCraft suite (VectorCraft, FilmCraft, EffectCraft, LightCraft and PrintCraft — clean-room Rust rewrites of Illustrator, Premiere Pro, After Effects, Lightroom and Acrobat), and VMware Workstation Pro.
- VMware Workstation Pro install flow: pick the Broadcom bundle you already downloaded, run its official installer, then automatically install and build Soplos's own vmmon/vmnet kernel modules so VMware keeps working across kernel updates. Full uninstall included.
- DaVinci Resolve: uninstall support (previously install-only).
- New app icon (the "S" mark) for Soplos Welcome itself.
- AppImage-based Recommended entries (Affinity Suite, CoolerControl, Double Commander, Audacity, ES-DE) now write `X-AppImage-Version`/`X-AppImage-UpdateUrl` into the `.desktop` file they generate, so Soplos AppImage Manager's own update check works immediately without the user filling them in by hand under Properties. Double Commander, which has no real upstream version number, only gets the update URL (enabling AppImage Manager's date-based "direct" check instead of a fabricated version).

### Changed

- Patchy now always installs the latest upstream release instead of a version pinned at v1.00, and its icon was updated to the current official logo.
- Audacity switched from the (now discontinued) Debian package to the official AppImage, since Audacity 4 dropped .deb packaging entirely. Icon updated to the real one embedded in the official AppImage.
- Waydroid moved from the Developer category to Virtualization, where it belongs, and is now blocked from installing on an X11 session (it requires Wayland) instead of silently failing.

### Fixed

- VirtualBox Extension Pack install: a `wget` probe for a per-file checksum that VirtualBox's own download server doesn't publish was silently killing the whole script under `set -e`; fixed in the shared download helper.
- VirtualBox Extension Pack install: the downloaded file is now saved under the exact name VBoxManage expects, fixing a "name mismatch" install failure.
- VirtualBox install: now prints a clear completion message instead of leaving the terminal on the last raw apt output line.
- The VMware and DaVinci Resolve file pickers no longer freeze the whole window while the native file dialog is open.
- ES-DE install was pinned to a hardcoded GitLab package file ID that had gone stale (silently stuck on 3.4.0 while upstream was already on 3.5.0); now resolved dynamically from GitLab's release permalink, same as the GitHub-hosted entries.
- 35 install/uninstall actions across the Recommended catalog never printed a completion message, leaving the terminal output on the last raw `apt`/`flatpak` line with no visible confirmation the job had actually finished — every one of them now ends with a clear "X installed."/"X removed." line.

## [3.0.0-1-beta] - 2026-09-30

- New Recommended applications: Concat, Glassy Music, Orchard and Sonora.
- Smaller installed package: removed bundled files nothing in the app referenced, and moved the About-page screenshots out of the embedded webview bundle.

## [3.0.0-beta] - 2026-09-28

First beta of Soplos Welcome 3.0, a full rewrite in Tauri 2, Rust and Svelte.

### Added

- Sections: Home, Software, Drivers, Kernels, Security, Recommended, Customization and Gaming.
- Interface in eight languages: English, Spanish, German, French, Italian, Portuguese, Romanian and Russian. It starts in the system language and can be changed from the sidebar.
- Shared popup component with info, confirm, warning and danger variants, used by every dialog including the Flatpak scope choice and the Flathub dialog.
- Choice between installing Flatpak apps for your user only or for all users.
- System update and upgrade from the sidebar, with per-source selection (APT, Flatpak user, Flatpak system, Snap).
- Recommended applications: about 100 entries in 12 categories, single or multiple selection, and extended search across APT and Flathub.
- New recommended entries: Patchy, Fog Panther, WhatsApp as a Soplos WebApp and the Soplos-packaged Telegram.
- Antigravity update detection by reading the installed version, with an update button.
- DaVinci Resolve guided installation with optional virtual microphone and integrated GPU patches.
- Uninstall support for Brave, Visual Studio Code, Sublime Text and Antigravity.
- Clear disclosure and confirmation before installing VirtualBox (adds Oracle's own repository plus Debian's trixie-security repository) or the VirtualBox Extension Pack (Oracle's PUEL license, separate from VirtualBox's own open-source license).
- Automatic detection of virtual machines (VMware, VirtualBox, QEMU/KVM, Hyper-V) to work around a WebKitGTK rendering bug that otherwise leaves the window blank with no error.
- Launching the app again while it's already open focuses the existing window instead of opening a second one.
- Checksum verification for third-party downloads when the upstream project publishes one.

### Changed

- System cleaning keeps Flatpak runtimes still in use, such as VAAPI, GL and NVIDIA extensions, instead of pruning every unused runtime.
- Drivers, Security and Gaming sections load faster.
- Antigravity installs under /opt instead of the user home.

## [3.0.0-alpha] - 2026-09-17

Full architecture rewrite of Soplos Welcome, from Python 3 + GTK3 to Tauri 2. See the Architecture section of the README for details.

### Added

- Rust backend (`src-tauri/`) replacing the Python 3 + GTK3 codebase: one command module per section, privilege escalation through `pkexec` with the privileged script run from a temporary file instead of a long-lived root process, a job runner that streams long-running command output back to the interface, shell-argument validators for values interpolated into scripts, and checksum verification for third-party downloads.
- Svelte 4 + Tailwind CSS frontend (`src/`) replacing the GTK3 UI, with one view per section and a shared component library.
- Initial scaffolding of every section: Home, Software, Drivers, Kernels, Security, Recommended, Customization and Gaming, ported from the 2.x tabs.
- Soplos's own light/dark theme, independent of the desktop environment's GTK/Qt theme.

