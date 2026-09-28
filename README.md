# Soplos Welcome 3.0

Rewrite in Tauri 2 + Rust + Svelte, following `Soplos-Welcome-3.0-Informe-Tecnico.md`.

Status: Fase 0 (scaffolding) done. Builds and runs.

## Getting started

```bash
npm install
npm run tauri dev
```

### Blank white window on a VM (VMware/VirtualBox)

WebKitGTK's DMA-BUF renderer can fail silently on virtual GPUs, leaving the window blank with no error. `main.rs` detects virtualization (DMI vendor/product strings, falling back to `systemd-detect-virt`) and sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` automatically before the webview starts, so this should never surface at runtime — Soplos ISOs are used both on bare metal and in VMs. If a new virtualization platform ever slips past the detection, the manual workaround is still:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri dev
```

## Before building

- Install the Rust toolchain and Tauri's system dependencies (WebKitGTK, etc.) if not already present.

## Structure

- `src-tauri/` — Rust backend (commands, services, privilege escalation).
- `src/` — Svelte + Tailwind frontend. Only the Home view has real content; the rest are placeholders for Fases 2-4.
