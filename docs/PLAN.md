# ESP32 Flash Tool — Project Assessment & Improvement Plan

> Assessment date: March 19, 2026  
> Current version: 0.1.0  
> Stack: Tauri v2 + Vue 3 + TypeScript + Pinia + TailwindCSS

---

## Table of Contents

1. [Current State Overview](#1-current-state-overview)
2. [Fundamental Architecture Mismatch](#2-fundamental-architecture-mismatch)
3. [Layout Assessment](#3-layout-assessment)
4. [Functionality Assessment](#4-functionality-assessment)
5. [Proposed UX Model](#5-proposed-ux-model)
6. [Layout Redesign Plan](#6-layout-redesign-plan)
7. [Backend Improvements](#7-backend-improvements)
8. [Implementation Roadmap](#8-implementation-roadmap)
9. [Esptool CLI Parity & Wrapper Scope](#9-esptool-cli-parity--wrapper-scope)
10. [Extended Roadmap — CLI Parity](#10-extended-roadmap--cli-parity)

---

## 1. Current State Overview

### What works today

| Feature | Status | Notes |
|---------|--------|-------|
| Multi-binary flashing | **Working** | Tested on real ESP32 hardware |
| Serial port scanning | **Working** | Auto-scan on startup |
| Progress parsing | **Working** | Regex on esptool stdout |
| Real-time log streaming | **Working** | stdout + stderr piped to UI |
| Cancel running flash | **Working** | Kills child process |
| Erase flash (standalone) | **Working** | Separate erase_flash command |
| Memory/hex viewer | **Working** | read_flash + local file viewer |
| Chip identification | **Working** | via esptool chip_id |
| Persistent settings | **Working** | localStorage save/load |
| Windows build (CI) | **Working** | GitHub Actions → MSI/NSIS |
| Sidecar binary resolution | **Working** | Linux + Windows (.exe suffix fix) |

### File structure

```
src/
├── App.vue                         — Root: tabs between workspaces
├── stores/main.ts                  — Pinia store (connection, flash, logs)
├── api/commands.ts                 — IPC invoke wrappers
├── layouts/
│   ├── MainLayout.vue              — Shell: header + sidebar + content + log + progress
│   ├── TopBar.vue                  — (unused — replaced by header in MainLayout)
│   ├── LeftSidebar.vue             — (unused — replaced by icon sidebar in MainLayout)
│   ├── RightSidebar.vue            — Connection panel + UART config + chip info
│   └── BottomPanel.vue             — Log console with verbosity filter
├── features/
│   ├── flashing/FlashingWorkspace.vue — Flash files table + config + actions
│   └── memory/MemoryWorkspace.vue     — Hex viewer with tabs
├── components/shared/
│   ├── Card.vue, Badge.vue, Button.vue, HexInput.vue
└── bridge/commands/index.ts        — (legacy, superseded by api/commands.ts)
```

---

## 2. Fundamental Architecture Mismatch

### The core problem

The current UI is modeled after **STM32CubeProgrammer**, which uses a **persistent connection** architecture:

```
STM32CubeProgrammer flow:
  Connect → [hold connection open] → Read / Write / Erase → Disconnect
```

However, **esptool** uses a **command-per-invocation** architecture:

```
esptool flow:
  Each command (write_flash, erase_flash, chip_id, read_flash) is an
  independent process that:
    1. Opens serial port
    2. Resets chip into bootloader mode
    3. Performs operation
    4. Closes serial port
    5. Optionally hard-resets chip back to normal mode
```

### Why this matters

| Issue | Impact |
|-------|--------|
| **Fake "Connected" state** | After `chip_id`, the serial port is immediately released. The green "Connected" indicator is misleading — there is no live connection. |
| **Forced Connect-before-Flash** | The UI requires clicking "Connect" before "Program", but esptool doesn't need a prior connection — it connects on its own per command. This adds an unnecessary step. |
| **Stale chip info** | After flashing, the chip hard-resets. The "Connected" status and chip info become stale. The user sees "Connected" but the device is running user firmware, not in bootloader mode. |
| **Port locking conflicts** | If we ever try to hold the port open (for a real "connection"), it would conflict with esptool spawning its own process that needs exclusive port access. |
| **Mental model confusion** | Users familiar with esptool expect "select port → flash". The Connect/Disconnect toggle adds confusion without functional benefit. |

### Conclusion

> **The Connect/Disconnect paradigm does not fit esptool's architecture.**  
> We should adopt a **command-centric** UX model instead.

---

## 3. Layout Assessment

### What works well

- **Right sidebar for UART config** — Compact, always visible, good for quick settings changes.
- **Bottom log panel** — Standard location for terminal output, familiar to IDE users.
- **Progress bar below log** — Unobtrusive, clear feedback.
- **Icon sidebar for navigation** — Clean, minimal footprint.
- **Flash items table** — Clear columns (checkbox, description, address, file path).
- **STM32CubeProgrammer color scheme** — Professional look with dark blue header/sidebar.

### What needs improvement

| Area | Problem | Recommendation |
|------|---------|----------------|
| **RightSidebar is too complex** | Combines connection UI, UART config, AND chip info in 280px. Too cramped. | Move UART config (port + baud) to the workspace header area. Use right sidebar only for device info or remove it entirely. |
| **Connect/Disconnect button** | Misleading (see Section 2). | Replace with "Get Device Info" button that runs `chip_id` on demand without implying persistent connection. |
| **Header duplicates info** | Header shows "Connected/Not connected" status that doesn't reflect reality. | Show esptool version + last operation status instead. |
| **TopBar.vue and LeftSidebar.vue** | These files exist but are not used (dead code). | Delete them. |
| **Card.vue and Badge.vue** | Dark theme variants that don't match the current light/blue theme. | Update to match the current design system or remove if unused. |
| **bridge/commands/index.ts** | Legacy file from early prototype. Superseded by `api/commands.ts`. | Delete it. |
| **No command preview** | Spec calls for a command preview panel showing the assembled CLI command. This is critical for debugging. | Add a collapsible command preview area in FlashingWorkspace. |
| **Log timestamps are wrong** | `new Date().toLocaleTimeString()` is called at render time, not at event time. If the log re-renders, all timestamps become the current time. | Capture timestamp when the log event arrives, store it with the log entry. |
| **No elapsed timer** | Spec calls for HH:MM:SS elapsed timer during flash. | Add timer that starts/stops with flash operation. |

---

## 4. Functionality Assessment

### Implemented vs. Specification

| Spec Feature | Status | Priority |
|--------------|--------|----------|
| Serial port scan + refresh | Done | — |
| Baudrate selection | Done | — |
| Flash items table (multi-file) | Done | — |
| Hex offset validation | Done | — |
| Flash mode / freq / size config | Done | — |
| Erase before flash (checkbox) | Done | — |
| Verify after write (checkbox) | Done | — |
| Standalone erase flash | Done | — |
| Real-time log streaming | Done | — |
| Progress bar | Done | — |
| Cancel operation | Done | — |
| Chip info (chip_id) | Done | — |
| Memory/hex viewer | Done | — |
| Persistent settings | Done | — |
| Windows installer (CI) | Done | — |
| **Command preview panel** | **Missing** | High |
| **Elapsed timer** | **Missing** | Medium |
| **Per-file sub-progress** | **Missing** | Low |
| **Drag-and-drop .bin files** | **Missing** | Low |
| **File size display** | **Missing** | Low |
| **Preset loader (ESP-IDF/Arduino)** | **Missing** | Medium |
| **Toast notifications** | **Missing** | Medium |
| **Confirmation dialog (erase)** | Partial | Low (uses browser confirm()) |
| **Log save to file** | **Missing** | Medium |
| **Log copy to clipboard** | **Missing** | Low |
| **Log color coding** | Done | — |
| **Configuration profiles** | **Missing** | Low |
| **Custom extra args field** | Done (store) | UI not exposed |
| **IPC debug panel** | **Missing** | Low |
| **App close during flash prompt** | **Missing** | Medium |

---

## 5. Proposed UX Model

### Replace "Connect/Disconnect" with "Command-Centric" approach

Instead of pretending to hold a connection, the UI should treat each operation as a standalone action that automatically handles its own connection.

#### New flow

```
1. User selects port + baud (always accessible at top)
2. User optionally clicks "Device Info" → runs chip_id, shows result
3. User configures flash items + options
4. User clicks "Program" → esptool handles connection internally
5. Progress + logs stream in real-time
6. Operation completes → status shown → ready for next command
```

#### Status model change

**Current (connection-based):**
```
idle → connecting → [connected] → writing/erasing → done/error
```

**Proposed (command-based):**
```
ready → running(command_type) → done/error → ready
```

| Old Status | New Status | Meaning |
|------------|-----------|---------|
| idle | ready | App is ready, no operation in progress |
| connecting | running | esptool is connecting to the device (part of the command) |
| writing | running | esptool is writing flash |
| erasing | running | esptool is erasing flash |
| done | ready | Last operation succeeded (show success indicator) |
| error | ready | Last operation failed (show error indicator) |

The key difference: **"ready" does not imply a connection exists**. It simply means the app can accept a new command.

### Remove forced "Connect before Flash" gate

Currently `FlashingWorkspace.vue` checks `store.status !== 'done'` and blocks with an alert. This should be removed — the user should be able to flash directly after selecting port + files.

---

## 6. Layout Redesign Plan

### Option A: Simplified single-panel layout (Recommended)

Remove the right sidebar entirely. Move port/baud selection to a compact toolbar at the top of the workspace area.

```
┌──────────────────────────────────────────────────────┐
│ [icon] ESP32 Flash Tool    [esptool v4.8.1]  [ready] │  ← Header
├──┬───────────────────────────────────────────────────┤
│  │ Port: [/dev/ttyUSB0 ▼] [⟳]  Baud: [460800 ▼]    │  ← Toolbar
│  │ [Device Info]                                      │
│  ├───────────────────────────────────────────────────┤
│▣ │                                                    │
│  │  ☐ Erase before programming   ☐ Verify after      │  ← Options
│▣ │  Flash Mode: [DIO]  Freq: [40m]  Size: [Keep]     │
│  ├───────────────────────────────────────────────────┤
│  │  ☑ Bootloader    │ 0x1000  │ /path/boot.bin  [Browse] │
│▣ │  ☑ Partition     │ 0x8000  │ /path/part.bin  [Browse] │  ← Flash table
│  │  ☑ Application   │ 0x10000 │ /path/app.bin   [Browse] │
│  ├───────────────────────────────────────────────────┤
│  │  [Full Chip Erase]  [Program]  [Cancel]            │  ← Actions
│  │  ████████████████████░░░░░░░░  72%  00:12          │  ← Progress
│  ├───────────────────────────────────────────────────┤
│  │  $ esptool --port /dev/ttyUSB0 --baud 460800 ...  │  ← Command preview
│  ├───────────────────────────────────────────────────┤
│  │  12:34:56 Writing at 0x00010000... (72 %)          │
│  │  12:34:55 Writing at 0x0000be93... (70 %)          │  ← Log
│  │  12:34:54 Connecting....                           │
└──┴───────────────────────────────────────────────────┘
```

**Advantages:**
- More horizontal space for file paths (critical on smaller screens)
- No wasted space on a "connection panel" that doesn't reflect reality
- Simpler navigation
- Closer to how tools like Arduino IDE or PlatformIO handle flashing

### Option B: Keep right sidebar but repurpose it

If the STM32CubeProgrammer aesthetic is strongly preferred, keep the right sidebar but change its content:

**Instead of "Connection + UART config + Chip info", show:**
- Device info (from last chip_id query, read-only)
- Quick flash presets (ESP-IDF, Arduino, MicroPython layouts)
- Operation history (last 5 flash operations with timestamps)

Port + Baud selection would move to the workspace toolbar regardless.

---

## 7. Backend Improvements

### 7.1 Remove artificial "Connected" state dependency

The `get_chip_info` command currently causes the store to set `status = 'done'` (meaning "connected"). Flash/erase/read operations check this status as a gate.

**Change:** Flash and erase operations should only require:
- A port is selected (`port !== ''`)
- No other operation is in progress (`!isBusy`)

### 7.2 Add command preview generation

Add a Rust command or a frontend utility that assembles the full esptool CLI command string from the current configuration, for display in the command preview panel.

This can be done entirely in the frontend since it's just string concatenation of the current settings.

### 7.3 Fix log timestamps

```typescript
// Current (broken):
{{ new Date().toLocaleTimeString() }} : {{ log.text }}

// Fixed: capture timestamp when event arrives
addLog(text: string, level: string) {
  this.logs.push({ text, level, timestamp: new Date().toISOString() });
}
```

### 7.4 Add elapsed timer

Track `flashStartTime` in the store. Display `Date.now() - flashStartTime` formatted as `MM:SS` during operations.

### 7.5 Cleanup dead code

| File | Action |
|------|--------|
| `src/layouts/TopBar.vue` | Delete (unused, replaced by MainLayout header) |
| `src/layouts/LeftSidebar.vue` | Delete (unused, replaced by icon sidebar in MainLayout) |
| `src/bridge/commands/index.ts` | Delete (legacy, replaced by `api/commands.ts`) |
| `src/bridge/sidecar/index.ts` | Delete (legacy) |
| `src/api/sidecars.ts` | Delete if unused |

---

## 8. Implementation Roadmap

### Phase 1 — Critical fixes (UX correctness)

| # | Task | Effort |
|---|------|--------|
| 1.1 | Remove "Connect before Flash" requirement — allow direct flash when port is selected | Small |
| 1.2 | Replace "Connect/Disconnect" with "Device Info" button (still runs chip_id, but no fake connection state) | Medium |
| 1.3 | Fix log timestamps (capture at event time, not render time) | Small |
| 1.4 | Clean up dead code (TopBar.vue, LeftSidebar.vue, bridge/) | Small |
| 1.5 | Move port + baud selection to workspace toolbar (always accessible, no right sidebar dependency) | Medium |

### Phase 2 — Missing high-value features

| # | Task | Effort |
|---|------|--------|
| 2.1 | Command preview panel (show assembled CLI command in real-time) | Medium |
| 2.2 | Elapsed timer during operations | Small |
| 2.3 | Toast notifications for operation completion | Medium |
| 2.4 | Log save to file | Small |
| 2.5 | Preset loader (ESP-IDF / Arduino / Custom offset layouts) | Medium |
| 2.6 | Expose extra args field in UI | Small |

### Phase 3 — Polish & advanced features

| # | Task | Effort |
|---|------|--------|
| 3.1 | Drag-and-drop .bin file support | Medium |
| 3.2 | File size display per row | Small |
| 3.3 | Per-file sub-progress indicator | Medium |
| 3.4 | Configuration profiles (save/load named setups) | Medium |
| 3.5 | App close during flash prompt | Small |
| 3.6 | Confirmation dialog component (replace browser confirm) | Medium |

### Phase 4 — Layout decision

Choose between:
- **Option A** (remove right sidebar, simplified layout) — recommended for esptool workflow
- **Option B** (repurpose right sidebar) — if STM32CubeProgrammer aesthetic is preferred

This decision should be made before starting Phase 2, as it affects where new UI elements are placed.

---

## 9. Esptool CLI Parity & Wrapper Scope

A **complete esptool wrapper** should eventually expose (or safely forward) the same surface area as the official CLI, so power users are not blocked and the GUI remains honest about what it can do. The following matrices are based on **esptool v5.1.0** help output (`esptool --help`). The project currently bundles a specific release (e.g. v4.x); **subcommand names and flags may differ slightly** between major versions — treat the tables below as the **target contract**, and validate against `esptool --help` from the **bundled** sidecar at build time.

### 9.1 Design principles

| Principle | Rationale |
|-----------|-----------|
| **No silent subset** | If the app cannot run a command, it should say so (or offer “Raw CLI” / passthrough), not pretend the feature exists. |
| **Global options first** | Chip, port, baud, before/after reset, stub, verbosity, and connect attempts affect *every* operation — they belong in a shared “Session / Transport” or “Advanced” panel and in command preview. |
| **First-class for daily workflows** | Prioritize UI for: `write-flash`, `read-flash`, `erase-flash`, `erase-region`, `chip-id`, `flash-id`, `read-mac`, `verify-flash`, `version`. |
| **Developer / CI commands** | `elf2image`, `image-info`, `merge-bin` are often used without a device — expose as separate tools or a “Offline tools” tab. |
| **Dangerous / expert** | `load-ram`, `write-mem`, `dump-mem`, `read-mem`, flash status/SFDP, security info — require confirmations, warnings, and optional “expert mode” toggle. |

### 9.2 Global options (CLI) vs project today

| Option | Purpose | Current app | Target |
|--------|-----------|-------------|--------|
| `--chip` / `-c` | Force chip family | Not exposed (auto-detect) | Dropdown + “auto” default; pass through to CLI |
| `--port` / `-p` | Serial device | **Yes** | Keep |
| `--baud` / `-b` | Baud for flash/read | **Yes** | Keep |
| `--port-filter` | Filter by VID/PID/name/serial | **No** | Add optional field(s) or raw append |
| `--before` | Reset before connect | **No** | Advanced: map to presets (`default-reset`, `no-reset`, …) |
| `--after` / `-a` | Reset after operation | **No** | Advanced: critical for “stay in bootloader” vs run app |
| `--no-stub` | ROM-only (limited features) | **No** | Checkbox in advanced |
| `--trace` / `-t` | Trace esptool protocol | **No** | Toggle + log verbosity |
| `--verbose` / `-v` | Verbose output | **No** | Toggle |
| `--silent` / `-s` | Errors only | **No** | Toggle |
| `--override-vddsdio` | ESP32 VDDSDIO override | **No** | Expert-only dropdown |
| `--connect-attempts` | Retry count | **No** | Numeric input |

### 9.3 Basic commands — coverage matrix

| Command | Role | Current app | Planned UI / backend |
|---------|------|-------------|----------------------|
| `write-flash` | Program flash at offsets | **Yes** (`write_flash` in v4.x) | Keep table + presets; align name with bundled esptool |
| `read-flash` | Dump flash to file | **Partial** (dedicated flow) | Unify with Memory tab; expose size/offset clearly |
| `erase-flash` | Full chip erase | **Yes** | Keep + confirmation |
| `erase-region` | Erase address range | **No** | New: offset + length |
| `read-mac` | Print MAC | **No** | Button → log panel |
| `flash-id` | Flash manufacturer / device ID | **No** | Button or merge into “Device info” |
| `elf2image` | ELF → flashable image | **No** | Offline tool tab or external wizard |
| `image-info` | Inspect firmware image | **No** | File picker → output in log |
| `merge-bin` | Merge binaries | **No** | Offline: inputs + output path |
| `version` | Esptool version | **Yes** (startup check) | Keep in About / status bar |

### 9.4 Advanced commands — coverage matrix

| Command | Role | Current app | Planned |
|---------|------|-------------|---------|
| `verify-flash` | Verify region vs file | **Partial** (`--verify` on write) | Optional standalone verify action |
| `load-ram` | Load to RAM & run | **No** | Expert + strong warning |
| `dump-mem` | Dump memory range | **No** | Expert |
| `read-mem` | Read word from address | **No** | Optional: single-shot or small range |
| `write-mem` | Write word | **No** | Expert + confirmation |
| `read-flash-status` | Read status register | **No** | Diagnostics submenu |
| `write-flash-status` | Write status register | **No** | Expert + warning |
| `read-flash-sfdp` | Read SFDP | **No** | Diagnostics |
| `get-security-info` | Security report | **No** | Diagnostics (chip-dependent) |
| `chip-id` | Chip identification | **Yes** (as `chip_id` / get chip info) | Align naming with CLI |
| `run` | Run application in flash | **No** | Optional button after flash |

### 9.5 “Escape hatch”: raw CLI / saved profiles

To reach **full parity** without building 30 screens immediately:

1. **`extra_args` (global)** — append to every spawned esptool invocation (already in payload type; ensure UI + preview).
2. **“Raw command” mode** — single multiline field validated as `esptool [options] command …`, execute with same streaming + cancel pipeline (security: warn on destructive commands).
3. **Command presets** — save named argument strings per workspace (flash / read / erase).

This trio lets advanced users mirror **any** `esptool --help` workflow while the GUI catches up.

---

## 10. Extended Roadmap — CLI Parity

Work packages layered on top of [Section 8](#8-implementation-roadmap). Order by dependency.

### Phase 5 — Global options (transport layer)

| # | Task | Effort |
|---|------|--------|
| 5.1 | Expose `--chip` (auto + explicit family) in settings + command preview | Medium |
| 5.2 | Expose `--before` / `--after` reset behavior (defaults documented in UI) | Medium |
| 5.3 | Expose `--connect-attempts`, `--no-stub`, `--verbose` / `--trace` / `--silent` | Small–Medium |
| 5.4 | Optional `--port-filter` (text field or structured VID/PID) | Medium |

### Phase 6 — Basic commands not yet in UI

| # | Task | Effort |
|---|------|--------|
| 6.1 | `erase-region` (start + length) with confirmation | Medium |
| 6.2 | `read-mac`, `flash-id` as one-click actions (output to log) | Small |
| 6.3 | `verify-flash` standalone (file + address + length) | Medium |
| 6.4 | `run` (post-flash run) where applicable | Small |
| 6.5 | Offline tools: `image-info`, `merge-bin`, `elf2image` (no device) — new sub-view | Large |

### Parity wave 7 — Advanced / diagnostics

| # | Task | Effort |
|---|------|--------|
| PW7.1 | Flash status / SFDP read (read-only diagnostics) | Medium |
| PW7.2 | `get-security-info` (when chip supports) | Medium |
| PW7.3 | `read-mem` / `write-mem` / `dump-mem` / `load-ram` behind “Expert mode” + confirmations | Large |

### Parity wave 8 — Maintenance & escape hatch

| # | Task | Effort |
|---|------|--------|
| PW8.1 | CI step: run `bundled-esptool --help` and diff against documented command list (fail on unexpected removals) | Medium |
| PW8.2 | Document upgrade path when bumping esptool major (4 → 5): subcommand renames (`write_flash` vs `write-flash`) | Small |
| PW8.3 | Raw CLI mode + saved raw profiles (escape hatch) | Large |

---

## Summary

The application's **core flash functionality is solid** and proven on real hardware. The main improvement needed is **aligning the UX model with esptool's command-per-invocation architecture** rather than forcing a persistent-connection paradigm. This means removing the Connect/Disconnect gate, fixing the status model, and making port selection a simple toolbar element rather than a connection ceremony.

The STM32CubeProgrammer-inspired layout works well aesthetically, but the right sidebar's "UART Configuration + Connection" purpose should be reconsidered since esptool doesn't maintain a persistent serial connection. Repurposing or simplifying this area will improve both usability and code clarity.

**Wrapper completeness:** A serious esptool GUI should aim for **CLI parity** over time: global options (`--chip`, `--before`/`--after`, verbosity, connect tuning), the full set of **basic** commands (erase-region, read-mac, flash-id, merge-bin, image-info, …), and **advanced/diagnostics** behind expert gates — plus a **raw CLI / passthrough** escape hatch so users are never blocked while specific forms are still missing. Sections [9](#9-esptool-cli-parity--wrapper-scope) and [10](#10-extended-roadmap--cli-parity) define the target matrices and phased roadmap.
