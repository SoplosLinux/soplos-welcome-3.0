# Changelog

All notable changes to Soplos Welcome are documented in this file.

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

### Changed

- System cleaning keeps Flatpak runtimes still in use, such as VAAPI, GL and NVIDIA extensions, instead of pruning every unused runtime.
- Drivers, Security and Gaming sections load faster.
- Antigravity installs under /opt instead of the user home.

