//! Keyboard shortcuts. `TABLE` drives both the bindings and the F1 cheat-sheet.

use gpui_kit::{actions, App, KeyBinding};

actions!(
    flashtool,
    [Help, DetectChip, OpenFolder, OpenFile, Reload, Program, Cancel, RescanPorts, ToggleErase, EraseChip,
     ToggleConsole, ToggleMonitor, Fullscreen, ClearLog]
);

pub const CONTEXT: &str = "FlashApp";

/// (keystrokes as shown, description). Keep in sync with `bind`.
pub const TABLE: &[(&str, &str)] = &[
    ("F1", "Show / hide this list"),
    ("F2", "Detect chip"),
    ("F3 · Ctrl+Shift+O", "Open project folder"),
    ("Shift+F3 · Ctrl+O", "Open file"),
    ("F4", "Reload project from disk"),
    ("F5 · Ctrl+Enter", "Program"),
    ("Shift+F5 · Esc", "Cancel"),
    ("F6", "Rescan serial ports"),
    ("F7", "Toggle “Erase first”"),
    ("F8", "Erase whole chip (asks first)"),
    ("F9", "Show / hide console"),
    ("F10", "Start / stop serial monitor"),
    ("F11", "Fullscreen"),
    ("Ctrl+L", "Clear console"),
];

pub fn bind(cx: &mut App) {
    let c = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("f1", Help, c),
        KeyBinding::new("f2", DetectChip, c),
        KeyBinding::new("f3", OpenFolder, c),
        KeyBinding::new("ctrl-shift-o", OpenFolder, c),
        KeyBinding::new("shift-f3", OpenFile, c),
        KeyBinding::new("ctrl-o", OpenFile, c),
        KeyBinding::new("f4", Reload, c),
        KeyBinding::new("f5", Program, c),
        KeyBinding::new("ctrl-enter", Program, c),
        KeyBinding::new("shift-f5", Cancel, c),
        KeyBinding::new("escape", Cancel, c),
        KeyBinding::new("f6", RescanPorts, c),
        KeyBinding::new("f7", ToggleErase, c),
        KeyBinding::new("f8", EraseChip, c),
        KeyBinding::new("f9", ToggleConsole, c),
        KeyBinding::new("f10", ToggleMonitor, c),
        KeyBinding::new("f11", Fullscreen, c),
        KeyBinding::new("ctrl-l", ClearLog, c),
    ]);
}
