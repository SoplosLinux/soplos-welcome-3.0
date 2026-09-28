# Soplos Welcome 3.0

Welcome and system setup application for Soplos Linux.

It gives you one place to update the system, install drivers and kernels, harden security, pick applications, tune the desktop and optimize the machine for gaming.

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
