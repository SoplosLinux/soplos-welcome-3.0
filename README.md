# Soplos Welcome 3.0

Welcome and system setup application for Soplos Linux.

It gives you one place to update the system, install drivers and kernels, harden security, pick applications, tune the desktop and optimize the machine for gaming.

## Architecture

Version 3.0 is a ground-up rewrite of Soplos Welcome. The original app was written in Python 3 with GTK3 (PyGObject), the same skeleton every other Soplos GTK tool follows. This rewrite replaces that entire stack with Tauri 2: a Rust backend and a Svelte 4 + Tailwind CSS frontend, rendered through the system's WebKitGTK.

- **Backend** (`src-tauri/`): one Rust module per section (Software, Drivers, Kernels, Security, Recommended, Customization, Gaming). Actions that need administrator rights run through `pkexec`, never as a long-lived root process; the privileged script itself is written to a temporary file and the same binary re-invokes itself to run it, rather than shelling out arbitrary strings. Long-running commands stream their output back to the interface through a job runner instead of blocking the UI. Inputs that get interpolated into a shell command are validated first, and third-party downloads are checksum-verified when the upstream project publishes one.
- **Frontend** (`src/`): one Svelte view per section, shared components (popups, the flatpak-scope choice, the language switcher), and the 8-language dictionaries under `src/i18n/`.

This is the first Soplos application ported to this architecture. The plan is to move the rest of the Python/GTK3 Soplos tools to the same Tauri 2 + Rust + Svelte pattern over time.

## Installation

Soplos Welcome comes preinstalled on Soplos Linux. If you need to install it again:

```bash
sudo apt install soplos-welcome
```

Open it from the applications menu, or press F1 inside the application to see the About dialog. Tasks that need administrator rights ask for your password through pkexec.

## Sections

- **Home**: system overview and quick links.
- **Software**: APT, Flatpak (user, system or both scopes), Snap and Flathub management, system update and upgrade, and system cleaning that keeps the Flatpak runtimes still in use (VAAPI, GL and NVIDIA extensions).
- **Drivers**: GPU, Wi-Fi and hardware detection with NVIDIA, CUDA, ROCm and oneAPI installers, VirtualBox guest additions and Wi-Fi repair.
- **Kernels**: Soplos kernels repository, XanMod and Liquorix, kernel cleanup and a shortcut to Soplos Kernel Installer.
- **Security**: firewall (UFW), antivirus and rootkit scans, VPN and network tools.
- **Recommended**: about 100 curated applications in 12 categories, single or multiple selection, and extended search across APT and Flathub.
- **Customization**: desktop tools for Xfce, GNOME and KDE, wallpapers and font rendering.
- **Gaming**: launchers, performance mode, sysctl, GPU and disk I/O optimizations, MangoHud, Lutris fixes and one-click revert.

## Languages

The interface is available in English, Spanish, German, French, Italian, Portuguese, Romanian and Russian. It starts in the system language and falls back to English. The language can be changed at any time from the sidebar.

## License

GPL-3.0-or-later. See `LICENSE`.
