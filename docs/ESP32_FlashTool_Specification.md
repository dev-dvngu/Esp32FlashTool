# ESP32 Flash Tool — Feature & UI Component Specification

> Desktop application built with **Tauri + Vue 3 + TypeScript**  
> Role: Bridge interface that receives commands from an external interface, forwards them to `esptool.py`, and displays real-time responses.

---

## Table of Contents

1. [Connection Management](#1-connection-management)
2. [Firmware & Partition File Management](#2-firmware--partition-file-management)
3. [Flash Configuration](#3-flash-configuration)
4. [Command Preview Panel](#4-command-preview-panel)
5. [Flash Execution & Progress](#5-flash-execution--progress)
6. [Terminal / Log Console](#6-terminal--log-console)
7. [Notifications & Alerts](#7-notifications--alerts)
8. [Persistent Settings](#8-persistent-settings)
9. [IPC / External Interface Bridge](#9-ipc--external-interface-bridge)
10. [Sidecar / Backend Architecture](#10-sidecar--backend-architecture)
11. [EspTool Distribution Strategy](#11-esptool-distribution-strategy)
12. [UI Component List](#12-ui-component-list)
13. [Change Summary vs. Original Spec](#13-change-summary-vs-original-spec)

---

## 1. Connection Management

### Features

| Feature | Description |
|---|---|
| Serial Port Dropdown | Auto-scan and list all available COM / Serial ports (e.g. `COM3`, `/dev/ttyUSB0`) |
| Refresh Button | Re-scan ports without restarting the application |
| Connection Status Indicator | Visual badge showing `Idle` / `Connected` / `Error` state |
| Connect / Disconnect Toggle | Hold the serial port open for monitoring, separate from the flash operation |
| Baudrate Dropdown | Predefined rates: `9600`, `115200`, `460800`, `921600` — default `115200` |
| Custom Baudrate Input | Manual text input for non-standard baud rates |
| Get Chip Info Button | Calls `esptool.py chip_id` or `flash_id` and populates the Chip Info panel |

### Chip Info Panel (read-only display)

| Field | Example Value |
|---|---|
| Chip Model | `ESP32-S3` |
| MAC Address | `A4:CF:12:6D:3E:F0` |
| Flash Size | `8MB` |
| Hardware Revision | `v0.1` |
| Crystal Frequency | `40MHz` |

### UI Components

- `<PortSelector>` — dropdown + refresh icon button
- `<BaudrateSelector>` — dropdown + optional text input
- `<ConnectionToggle>` — Connect / Disconnect button with status badge
- `<ChipInfoPanel>` — card displaying chip fields, shown after "Get Chip Info"

---

## 2. Firmware & Partition File Management

### Flash Items List

Each row in the table represents one `.bin` file to be written at a specific memory offset.

| Column | Type | Description |
|---|---|---|
| Enable | Checkbox | Include or skip this file in the current flash operation |
| Offset | Hex Input | Memory start address (e.g. `0x1000`, `0x8000`, `0x10000`) with hex format validation |
| File Path | Read-only text | Displays the selected file path |
| File Size | Read-only text | Displays file size in KB/MB for quick sanity check |
| Browse | Button | Opens OS file picker filtered to `.bin` files |
| Delete Row | Icon Button | Removes this row from the list |

**Table constraints:** Minimum 1 row, maximum 5 rows (expandable).

### Table Controls

| Control | Action |
|---|---|
| Add Row Button | Inserts a new blank entry at the bottom |
| Clear All Button | Resets the entire table (with confirmation dialog) |
| Preset Loader Dropdown | Load a predefined offset layout (ESP-IDF Standard, Arduino, Custom) |

### Additional Features

- **Drag-and-drop** — drop a `.bin` file directly onto a row or the table area to populate a new entry
- **Hex validation** — inline error message if offset format is invalid (non-hex characters, out-of-range)
- **File size display** — shown next to file path; warns if file is 0 bytes

### UI Components

- `<FlashItemsTable>` — the full table container
- `<FlashItemRow>` — single row with all columns
- `<HexInput>` — controlled hex address input with validation
- `<FilePicker>` — browse button + path display + file size label
- `<PresetLoader>` — dropdown for framework offset presets

---

## 3. Flash Configuration

### Parameters

| Parameter | Type | Options | Default |
|---|---|---|---|
| Flash Mode | Dropdown | `QIO`, `QOUT`, `DIO`, `DOUT` | `DIO` |
| Flash Frequency | Dropdown | `40m`, `80m` | `40m` |
| Flash Size | Dropdown | `2MB`, `4MB`, `8MB`, `16MB`, `Keep` | `Keep` |
| SPI Pin Config | Hidden (Advanced) | Auto-detect / Manual entry | Auto |

### Erase Options

| Control | Type | Description |
|---|---|---|
| Erase All Flash | Button | Calls `esptool.py erase_flash` immediately — destructive, requires confirmation |
| Erase Before Flashing | Checkbox | Prepends an erase step before writing any files |
| Verify After Write | Checkbox | Appends `esptool.py verify_flash` after writing to confirm data integrity |

### Advanced Section (collapsed by default)

- SPI Pin Config (manual override)
- Custom `esptool.py` arguments (free-text field appended to the command)

### UI Components

- `<FlashModeSelector>` — dropdown
- `<FlashFrequencySelector>` — dropdown
- `<FlashSizeSelector>` — dropdown
- `<EraseButton>` — standalone erase with confirmation modal
- `<FlashOptionsCheckboxGroup>` — erase before / verify after
- `<AdvancedSection>` — collapsible panel with SPI config + custom args

---

## 4. Command Preview Panel

> Critical for the bridge use case — shows exactly what will be sent to `esptool.py`.

### Features

| Feature | Description |
|---|---|
| Live Command Display | Read-only text box showing the assembled CLI command, updated in real time as settings change |
| Copy Command Button | Copies the full command to clipboard |
| Raw Command Input Mode | Toggle that disables the form and accepts a raw CLI string from the external interface directly |

### Example Command Output

```
esptool.py --port COM3 --baud 460800 --flash_mode dio --flash_freq 40m
  write_flash 0x1000 bootloader.bin 0x8000 partition-table.bin 0x10000 app.bin
```

### UI Components

- `<CommandPreviewBox>` — monospace read-only textarea with copy button
- `<RawCommandToggle>` — switch between Form mode and Raw CLI mode
- `<RawCommandInput>` — textarea for injecting a raw command string (visible only in Raw mode)

---

## 5. Flash Execution & Progress

### Flash Button States

| State | Label | Style |
|---|---|---|
| Idle | `Flash` | Primary color, enabled |
| Running | `Flashing…` | Muted, disabled |
| Done | `Flash` | Primary color, re-enabled |
| Error | `Retry` | Warning color |

### Controls During Execution

- **Cancel / Stop Button** — terminates the child process immediately; appears only while flashing is active

### Progress Display

| Component | Description |
|---|---|
| Overall Progress Bar | Percentage parsed from `esptool.py` stdout via regex on `Writing at 0x... (XX%)` |
| Per-file Sub-progress | Individual progress indicator for each `.bin` file when flashing multiple files |
| Elapsed Time Counter | `HH:MM:SS` counter, starts when flash begins |
| Status Badge | Color-coded: `Idle` / `Connecting` / `Erasing` / `Writing` / `Verifying` / `Done` / `Error` |

### UI Components

- `<FlashButton>` — primary action button with state management
- `<CancelButton>` — appears only during active flash
- `<OverallProgressBar>` — full-width bar with percentage label
- `<FileProgressList>` — per-file progress rows
- `<ElapsedTimer>` — formatted time counter
- `<StatusBadge>` — color-coded pill label

---

## 6. Terminal / Log Console

### Features

| Feature | Description |
|---|---|
| Real-time Output | Streams raw stdout + stderr from `esptool.py` line by line |
| Color Coding | White/green for normal output, yellow for warnings, red for errors |
| Auto-scroll | Follows the latest output; toggle to pause auto-scroll |
| Log Level Filter | Toggle to show All / Warnings & Errors only |
| Clear Log | Clears the console display (does not affect the saved log) |
| Copy Log | Copies the full log text to clipboard |
| Save Log | Exports the session log to a `.txt` file with a timestamp filename |

### Styling

- Background: dark (`#0d1117` or similar)
- Font: monospace (e.g. `JetBrains Mono`, `Fira Code`, `Cascadia Code`)
- Line height: comfortable for readability (`1.6`)
- Scrollbar: styled, always visible

### UI Components

- `<LogConsole>` — main terminal container
- `<LogLine>` — single line with type-based color class (`info` / `warn` / `error`)
- `<LogToolbar>` — row with Clear, Copy, Save buttons + auto-scroll toggle + filter toggle

---

## 7. Notifications & Alerts

### Toast Notifications

| Event | Color | Message |
|---|---|---|
| Flash successful | Green ✅ | `"Flash completed in 12.3s"` |
| Flash failed | Red ❌ | `"Flash failed — Invalid head of packet"` |
| Erase successful | Blue ℹ️ | `"Flash erased successfully"` |
| Chip info retrieved | Blue ℹ️ | `"Chip identified: ESP32-S3"` |

### Confirmation Dialogs

| Trigger | Dialog Message |
|---|---|
| Erase All Flash Button | `"This will permanently erase all flash memory. Continue?"` |
| Clear All Rows | `"Remove all firmware entries from the list?"` |

### System Tray Notification (optional)

- Fires when app is minimized and a flash operation completes
- Message: `"ESP32 Flash — Operation complete"` or `"ESP32 Flash — Failed"`

### UI Components

- `<ToastContainer>` — positioned at bottom-right, stacks multiple toasts
- `<Toast>` — individual notification with icon, message, auto-dismiss timer
- `<ConfirmDialog>` — modal with confirm / cancel actions

---

## 8. Persistent Settings

All settings are saved automatically via **Tauri Store** (or LocalStorage fallback) and restored on next launch.

### Saved Values

| Setting | Key |
|---|---|
| Last selected COM port | `settings.port` |
| Last selected baudrate | `settings.baud` |
| Flash items (offsets + file paths) | `settings.flashItems[]` |
| Flash mode / frequency / size | `settings.flashConfig` |
| Window size and position | `settings.windowBounds` |
| Active configuration profile name | `settings.activeProfile` |

### Configuration Profiles

- Save the entire current setup (port + files + offsets + flash config) under a user-defined name
- Load / delete named profiles
- Useful for switching between different projects or board variants

### UI Components

- `<ProfileManager>` — dropdown to select profiles + Save / Delete buttons
- `<SaveProfileDialog>` — input dialog for naming a new profile

---

## 9. IPC / External Interface Bridge

> This section covers how an **external interface** communicates with this application.

### Input — Commands Received

| Format | Example |
|---|---|
| JSON Payload | `{ "port": "COM3", "baud": 460800, "files": [...], "config": {...} }` |
| Raw CLI String | `"--port COM3 --baud 460800 write_flash 0x10000 app.bin"` |

### Output — Events Emitted Back

| Event | Payload |
|---|---|
| `progress` | `{ percent: 42, file: "app.bin" }` |
| `log_line` | `{ text: "Writing at 0x00010000...", level: "info" }` |
| `status_change` | `{ status: "writing" \| "erasing" \| "done" \| "error" }` |
| `completed` | `{ success: true, duration_ms: 12300 }` |
| `error` | `{ code: "INVALID_HEAD", message: "Invalid head of packet" }` |

### IPC Debug Panel

- Separate collapsible panel (independent from the esptool terminal)
- Shows raw messages received from the external interface
- Shows raw events/responses sent back
- Useful for debugging the integration during development

### UI Components

- `<IpcDebugPanel>` — collapsible panel with two columns: Received / Sent
- `<IpcMessageRow>` — single message entry with timestamp, direction, and payload

---

## 10. Sidecar / Backend Architecture

> **Core rule:** The application **must never use any `esptool` or Python binary pre-installed on the user's system.**  
> All execution goes through a self-contained binary that the application either **bundles at build time** or **downloads on first launch**. See [Section 11](#11-esptool-distribution-strategy) for the full distribution strategy.

### Bundled Binaries (Sidecar)

| Platform | Architecture | Binary filename |
|---|---|---|
| Windows | x86_64 | `esptool-x86_64-pc-windows-msvc.exe` |
| Linux | x86_64 | `esptool-x86_64-unknown-linux-gnu` |
| Linux | aarch64 (ARM) | `esptool-aarch64-unknown-linux-gnu` |
| macOS | x86_64 (Intel) | `esptool-x86_64-apple-darwin` |
| macOS | aarch64 (Apple Silicon) | `esptool-aarch64-apple-darwin` |

Tauri sidecar naming follows the pattern: `{binary-name}-{target-triple}` as required by `tauri.conf.json`.

### Hard Requirements

- ❌ Never call system `python`, `python3`, or `pip`
- ❌ Never use `PATH`-resolved `esptool` or `esptool.py` from the OS
- ✅ Always resolve the binary path from the Tauri sidecar directory (`tauri::api::process::current_binary()` sibling path)
- ✅ Binary must be executable immediately after install with no user action required

### Process Management

| Concern | Approach |
|---|---|
| Binary resolution | Resolved at runtime from app's sidecar directory, never from system PATH |
| Start | Spawned on "Flash" button click with assembled argument array |
| Stdout streaming | Line-by-line, piped back to Vue via Tauri events |
| Stderr handling | Captured and displayed in the log console, triggers error state |
| Cancellation | Process killed gracefully (`SIGTERM` / `taskkill`) on Cancel |
| App close during flash | Prompts user; terminates child process before exiting |
| Permission (Linux/macOS) | Binary `chmod +x` applied automatically on first run via Tauri setup hook |

### Version Display

- App queries `esptool version` via sidecar on startup
- Displayed in the About panel and the Status Bar
- Example: `esptool v4.8.1 (bundled)`
- If version query fails, app shows a warning and blocks flash operations

---

## 11. EspTool Distribution Strategy

> This section defines **how the application obtains and manages its own private copy of `esptool`**, completely isolated from the system environment.

### Overview of Two Approaches

| Strategy | Description | Best For |
|---|---|---|
| **A — Bundle at build time** | Pre-compiled `esptool` binaries are included inside the app installer | Offline environments, corporate deployments, simplest UX |
| **B — Download on first launch** | App ships without `esptool`; downloads the correct binary from GitHub Releases on first run | Smaller installer size, always latest version |

The application should support **both strategies** and let the developer choose at build time via a config flag. Strategy A is the **recommended default**.

---

### Strategy A — Bundle at Build Time (Recommended Default)

#### How It Works

1. During the CI/CD build pipeline, pre-compiled `esptool` standalone binaries are downloaded from the [official esptool GitHub Releases](https://github.com/espressif/esptool/releases) for each target platform.
2. The binaries are placed into `src-tauri/binaries/` following Tauri's sidecar naming convention.
3. Tauri packages them into the final installer (`.msi`, `.deb`, `.dmg`, etc.).
4. On install, the binary lands in the app's resource directory alongside the main executable.

#### Directory Layout

```
src-tauri/
└── binaries/
    ├── esptool-x86_64-pc-windows-msvc.exe
    ├── esptool-x86_64-unknown-linux-gnu
    ├── esptool-aarch64-unknown-linux-gnu
    ├── esptool-x86_64-apple-darwin
    └── esptool-aarch64-apple-darwin
```

#### tauri.conf.json Configuration

```json
{
  "tauri": {
    "bundle": {
      "externalBin": [
        "binaries/esptool"
      ]
    }
  }
}
```

#### Build Script (CI Example — download step)

```bash
#!/usr/bin/env bash
# scripts/download-esptool.sh
# Run this during CI before `tauri build`

ESPTOOL_VERSION="v4.8.1"
BASE_URL="https://github.com/espressif/esptool/releases/download/${ESPTOOL_VERSION}"
OUT_DIR="src-tauri/binaries"

mkdir -p "$OUT_DIR"

# Windows
curl -L "${BASE_URL}/esptool-${ESPTOOL_VERSION}-win64.zip" -o /tmp/esptool-win.zip
unzip -p /tmp/esptool-win.zip "esptool.exe" > "${OUT_DIR}/esptool-x86_64-pc-windows-msvc.exe"

# Linux x86_64
curl -L "${BASE_URL}/esptool-${ESPTOOL_VERSION}-linux-amd64.zip" -o /tmp/esptool-linux.zip
unzip -p /tmp/esptool-linux.zip "esptool" > "${OUT_DIR}/esptool-x86_64-unknown-linux-gnu"
chmod +x "${OUT_DIR}/esptool-x86_64-unknown-linux-gnu"

# macOS universal
curl -L "${BASE_URL}/esptool-${ESPTOOL_VERSION}-macos.zip" -o /tmp/esptool-mac.zip
unzip -p /tmp/esptool-mac.zip "esptool" > "${OUT_DIR}/esptool-x86_64-apple-darwin"
cp "${OUT_DIR}/esptool-x86_64-apple-darwin" "${OUT_DIR}/esptool-aarch64-apple-darwin"
chmod +x "${OUT_DIR}/esptool-x86_64-apple-darwin"
chmod +x "${OUT_DIR}/esptool-aarch64-apple-darwin"
```

#### Pros & Cons

| ✅ Pros | ❌ Cons |
|---|---|
| Works fully offline | Larger installer file size (~5–15 MB extra) |
| No network calls at runtime | Must rebuild/release to update esptool version |
| Simplest end-user experience | CI pipeline must download binaries before build |
| No firewall / proxy issues | |

---

### Strategy B — Download on First Launch

#### How It Works

1. App ships **without** the `esptool` binary inside the installer.
2. On first launch, the app detects that `esptool` is missing from its private data directory.
3. App shows a **Setup / First Run screen** and automatically downloads the correct binary for the current platform from the official GitHub Releases API.
4. Binary is saved to the app's **private data directory** (never a system path):

| Platform | Private Data Directory |
|---|---|
| Windows | `%APPDATA%\esp32-flash-tool\bin\` |
| Linux | `~/.local/share/esp32-flash-tool/bin/` |
| macOS | `~/Library/Application Support/esp32-flash-tool/bin/` |

5. `chmod +x` is applied on Linux/macOS.
6. All future runs use this private binary directly.

#### First Run Setup Screen — UI Flow

```
┌─────────────────────────────────────────────┐
│  ESP32 Flash Tool — First Run Setup          │
│                                             │
│  esptool is not yet installed.              │
│  The app will download it now.              │
│                                             │
│  Version:    v4.8.1 (latest)               │
│  Source:     github.com/espressif/esptool   │
│  Size:       ~8 MB                         │
│  Saved to:   %APPDATA%\esp32-flash-tool\bin │
│                                             │
│  [████████████████░░░░░░]  67%             │
│                                             │
│  [ Cancel ]              [ Download Now ]   │
└─────────────────────────────────────────────┘
```

#### GitHub Releases API — Platform Detection Logic

```typescript
// src/api/esptoolManager.ts

const PLATFORM_MAP: Record<string, string> = {
  "win32-x86_64":   "esptool-{version}-win64.zip",
  "linux-x86_64":   "esptool-{version}-linux-amd64.zip",
  "linux-aarch64":  "esptool-{version}-linux-arm64.zip",
  "darwin-x86_64":  "esptool-{version}-macos.zip",
  "darwin-aarch64": "esptool-{version}-macos.zip",  // universal binary
};

async function resolveDownloadUrl(version: string): Promise<string> {
  const platform = await os.platform();   // Tauri os plugin
  const arch     = await os.arch();
  const key      = `${platform}-${arch}`;
  const filename = PLATFORM_MAP[key].replace("{version}", version);
  return `https://github.com/espressif/esptool/releases/download/${version}/${filename}`;
}
```

#### Pros & Cons

| ✅ Pros | ❌ Cons |
|---|---|
| Smaller installer (no binary bundled) | Requires internet on first launch |
| Can pull latest esptool without a full app rebuild | Fails behind strict firewalls without proxy config |
| Easy version update via in-app update check | More complex implementation |

---

### Strategy Comparison

| Criterion | A — Bundle | B — Download |
|---|---|---|
| Installer size | Larger (+8–15 MB) | Smaller |
| Offline install | ✅ Yes | ❌ No |
| Always latest esptool | ❌ Requires app update | ✅ Can pull latest |
| Firewall friendly | ✅ Yes | ⚠️ Depends |
| Implementation complexity | Low | Medium |
| **Recommended default** | ✅ **Yes** | Optional / power users |

---

### In-App EspTool Version Manager (applies to both strategies)

| Feature | Description |
|---|---|
| Current version display | Shows the version of the active `esptool` binary in the About panel |
| Update check button | Queries the GitHub Releases API for the latest tag; compares with current version |
| Update available badge | Shows a dot badge on the Settings icon when a newer version is available |
| Download update | For Strategy B: downloads and replaces the binary in the private data dir |
| For Strategy A | Informs user that an app update is required to get a newer `esptool` |
| Rollback | Keeps the previous binary as `esptool.bak` for one version, allowing rollback if the new binary fails |

### UI Components (EspTool Manager)

- `<EsptoolSetupScreen>` — full-screen first-run download flow (Strategy B only)
- `<EsptoolDownloadProgress>` — progress bar + percentage + cancel button during download
- `<EsptoolVersionBadge>` — displays current version string with source label (`bundled` / `downloaded`)
- `<EsptoolUpdateChecker>` — check now button + last-checked timestamp
- `<EsptoolUpdateBanner>` — dismissible banner shown when an update is available

---

### Security Considerations

| Concern | Mitigation |
|---|---|
| Binary integrity | Verify SHA-256 checksum of downloaded binary against the official release manifest before first use |
| Source authenticity | Download only from `https://github.com/espressif/esptool/releases` — no third-party mirrors |
| Checksum file source | Fetch `SHA256SUMS` from the same GitHub release tag |
| Binary execution scope | The private binary is only granted execute permission; never added to system PATH |
| HTTPS enforcement | All download requests must use HTTPS; plain HTTP must be rejected |

#### Checksum Verification Flow

```
Download esptool binary
        ↓
Download SHA256SUMS file from same release tag
        ↓
Compute SHA-256 of downloaded binary
        ↓
Compare against expected hash from SHA256SUMS
        ↓
  Match? → chmod +x → Ready to use
  No match? → Delete file → Show error: "Download corrupted, please retry"
```

---

## 12. UI Component List

### Layout & Shell

| Component | Description |
|---|---|
| `<AppShell>` | Root layout: sidebar or top nav + main content area |
| `<TitleBar>` | Custom Tauri window title bar with minimize / maximize / close |
| `<SidebarNav>` | Navigation between main sections (Flash, Logs, Settings, About) |
| `<StatusBar>` | Bottom bar showing current port, baud, and global app status |

### Connection Section

| Component | Description |
|---|---|
| `<PortSelector>` | Serial port dropdown + refresh button |
| `<BaudrateSelector>` | Baudrate dropdown + custom input |
| `<ConnectionToggle>` | Connect / Disconnect button + status badge |
| `<ChipInfoPanel>` | Collapsible card with chip details |

### File Management Section

| Component | Description |
|---|---|
| `<FlashItemsTable>` | Full table with drag-and-drop support |
| `<FlashItemRow>` | Single file row with all controls |
| `<HexInput>` | Validated hex address input field |
| `<FilePicker>` | Browse button + file path display + file size |
| `<PresetLoader>` | Offset preset dropdown |

### Configuration Section

| Component | Description |
|---|---|
| `<FlashModeSelector>` | Dropdown: QIO / QOUT / DIO / DOUT |
| `<FlashFrequencySelector>` | Dropdown: 40m / 80m |
| `<FlashSizeSelector>` | Dropdown: 2MB / 4MB / 8MB / 16MB / Keep |
| `<FlashOptionsCheckboxGroup>` | Erase before / Verify after checkboxes |
| `<EraseButton>` | Standalone erase button with confirmation |
| `<AdvancedSection>` | Collapsible panel for SPI config + custom args |

### Command Preview Section

| Component | Description |
|---|---|
| `<CommandPreviewBox>` | Monospace read-only textarea + copy button |
| `<RawCommandToggle>` | Switch between Form mode and Raw CLI mode |
| `<RawCommandInput>` | Raw CLI string input (Raw mode only) |

### Execution Section

| Component | Description |
|---|---|
| `<FlashButton>` | Primary action button with state management |
| `<CancelButton>` | Abort the active flash process |
| `<OverallProgressBar>` | Full-width progress bar with percentage |
| `<FileProgressList>` | Per-file progress indicator list |
| `<ElapsedTimer>` | `HH:MM:SS` elapsed time counter |
| `<StatusBadge>` | Color-coded status pill |

### Log Console Section

| Component | Description |
|---|---|
| `<LogConsole>` | Dark-themed terminal output area |
| `<LogLine>` | Single log line with level-based color |
| `<LogToolbar>` | Clear / Copy / Save buttons + filter + auto-scroll toggle |

### Notifications

| Component | Description |
|---|---|
| `<ToastContainer>` | Fixed-position toast stack (bottom-right) |
| `<Toast>` | Individual notification with icon + message + timer |
| `<ConfirmDialog>` | Modal dialog for destructive action confirmation |

### IPC & Settings

| Component | Description |
|---|---|
| `<IpcDebugPanel>` | Collapsible IPC message log (Received / Sent) |
| `<IpcMessageRow>` | Single IPC message entry |
| `<ProfileManager>` | Profile selector dropdown + Save / Delete |
| `<SaveProfileDialog>` | Prompt dialog for naming a new configuration profile |

### Shared / Utility

| Component | Description |
|---|---|
| `<Dropdown>` | Reusable select dropdown |
| `<IconButton>` | Button with icon only + tooltip |
| `<Tooltip>` | Hover tooltip for any element |
| `<Badge>` | Small label pill (used for status, chip model, etc.) |
| `<Divider>` | Horizontal rule for section separation |
| `<CollapsibleSection>` | Expand/collapse wrapper for Advanced settings |
| `<Modal>` | Generic modal overlay base |
| `<Spinner>` | Loading indicator used during chip info fetch |

---

## 13. Change Summary vs. Original Spec

| Feature | Original Spec | This Document |
|---|---|---|
| Connect / Disconnect toggle | ❌ Not mentioned | ✅ Added |
| Connection status indicator | ❌ Not mentioned | ✅ Added |
| Custom baudrate input | ❌ Not mentioned | ✅ Added |
| Crystal frequency in chip info | ❌ Not mentioned | ✅ Added |
| File size display per row | ❌ Not mentioned | ✅ Added |
| Drag-and-drop `.bin` files | ❌ Not mentioned | ✅ Added |
| Preset offset loader | ❌ Not mentioned | ✅ Added |
| Verify after write checkbox | ❌ Not mentioned | ✅ Added |
| Advanced section (collapsed) | ❌ Not mentioned | ✅ Added |
| Command preview panel | ❌ Not mentioned | ✅ Added (critical for bridge use case) |
| Raw command input mode | ❌ Not mentioned | ✅ Added |
| Cancel button during flash | ❌ Not mentioned | ✅ Added |
| Per-file sub-progress | ❌ Not mentioned | ✅ Added |
| Elapsed time counter | ❌ Not mentioned | ✅ Added |
| Log level filter toggle | ❌ Not mentioned | ✅ Added |
| Save log to `.txt` file | ❌ Not mentioned | ✅ Added |
| System tray notification | ❌ Not mentioned | ✅ Added (optional) |
| Named configuration profiles | ❌ Not mentioned | ✅ Added |
| IPC event API (bridge) | ❌ Not mentioned | ✅ Added |
| IPC debug panel | ❌ Not mentioned | ✅ Added |
| App close during flash prompt | ❌ Not mentioned | ✅ Added |
| Bundled esptool version display | ❌ Not mentioned | ✅ Added |
| Status bar (port/baud/status) | ❌ Not mentioned | ✅ Added |
| Confirmation dialogs (erase/clear) | ❌ Not mentioned | ✅ Added |
| **EspTool distribution strategy** | ❌ Not mentioned | ✅ Added (Section 11) |
| Bundle-at-build-time (Strategy A) | ❌ Not mentioned | ✅ Added — pre-compiled binaries in `src-tauri/binaries/` |
| Download-on-first-launch (Strategy B) | ❌ Not mentioned | ✅ Added — downloads from GitHub Releases to private data dir |
| Per-platform binary targeting | ❌ Not mentioned | ✅ Added — 5 platform/arch combinations covered |
| CI build script for binary download | ❌ Not mentioned | ✅ Added |
| Platform detection logic (TypeScript) | ❌ Not mentioned | ✅ Added |
| SHA-256 checksum verification | ❌ Not mentioned | ✅ Added — security requirement |
| HTTPS-only download enforcement | ❌ Not mentioned | ✅ Added — security requirement |
| In-app version update checker | ❌ Not mentioned | ✅ Added |
| Binary rollback (`esptool.bak`) | ❌ Not mentioned | ✅ Added |
| Never use system PATH esptool | ❌ Not mentioned | ✅ Explicitly documented as hard requirement |
| First-run setup screen (Strategy B) | ❌ Not mentioned | ✅ Added — UI flow and components |

---

*Document version: 1.1 — Generated for ESP32 Flash Tool (Tauri + Vue 3)*
