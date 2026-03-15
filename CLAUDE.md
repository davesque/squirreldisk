# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SquirrelDisk is a cross-platform disk usage analyzer built with **Tauri 1.x** (Rust backend + React/TypeScript frontend). It scans disks using an external `pdu` (parallel-disk-usage) sidecar binary and renders results as an interactive D3 sunburst chart.

## Build & Development Commands

```bash
npm install                  # Install frontend dependencies
npm run tauri dev            # Start full dev environment (Vite + Rust)
npm run dev                  # Start Vite dev server only (port 1420)
npm run build                # TypeScript check + Vite production build
npm run tauri build          # Bundle desktop app for current platform
```

Rust code is in `src-tauri/` and uses standard `cargo` commands from that directory.

## Architecture

### Frontend ↔ Backend Communication

- **Tauri IPC commands** (`invoke`): `get_disks`, `start_scanning`, `stop_scanning`, `show_in_folder`
- **Tauri events** (`emit_all`/`listen`): `scan_status` (progress updates), `scan_completed` (final tree data)

### Scanning Pipeline

1. Frontend calls `start_scanning(path, ratio)` via Tauri invoke
2. Rust spawns the `pdu` sidecar binary with `--json-output --progress --min-ratio=<value>`
3. `pdu` reports progress on stderr (parsed with regex in `scan.rs`), final JSON tree on stdout
4. Rust emits events back to frontend; the active child process is stored in `MyState` (Mutex) for cancellation

### Key Frontend Modules

- `src/components/DiskList.tsx` — Disk selection screen, polls `get_disks` every 2s
- `src/components/DiskDetail.tsx` — Main scanning view, listens to scan events, renders chart + file list
- `src/d3chart.ts` — D3 sunburst chart creation and update logic
- `src/pruneData.ts` — Transforms scan tree into D3 hierarchy, handles depth cuts and size-based pruning

### Key Rust Modules

- `src-tauri/src/main.rs` — Tauri app setup, window styling, all IPC command handlers
- `src-tauri/src/scan.rs` — Sidecar process spawning, stderr progress parsing, stdout JSON parsing
- `src-tauri/src/window_style.rs` — Platform-specific window decoration (NSWindow on macOS, DWM on Windows)

### Sidecar Binaries

Platform-specific `pdu` binaries live in `src-tauri/bin/`. The aarch64-apple-darwin binary is a clone of the x86_64 one (runs via Rosetta).

## Tech Stack

- **Frontend**: React 18, TypeScript, Vite, Tailwind CSS, D3.js, react-beautiful-dnd
- **Backend**: Rust (edition 2021), Tauri 1.2, sysinfo, parallel-disk-usage sidecar
- **Window styling**: window-vibrancy (macOS), custom DWM attributes (Windows)

## Platform Notes

- Window is borderless and transparent; custom titlebar in `TitleBar.tsx`
- macOS uses private APIs for vibrancy (`macos-private-api` feature flag)
- Linux detection in `App.tsx` adjusts styling (no vibrancy/transparency)
- FS allowlist restricted to `removeFile`/`removeDir` only (for drag-to-delete feature)

## CI/CD

GitHub Actions (`.github/workflows/main.yml`) builds on version tags (`v*`) across macOS, Ubuntu, and Windows. Uses `tauri-action` with signing keys from GitHub secrets.
