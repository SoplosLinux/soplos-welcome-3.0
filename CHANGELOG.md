# Changelog

All notable changes to Soplos Welcome are documented in this file.

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

