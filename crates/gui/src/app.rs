use flashcore::detect::{self, FlashFile, FlashSettings, Project, Source};
use flashcore::esptool::{self, Conn, Event, Op, Status, Task};
use flashcore::monitor::{self, Level, MonEvent, Monitor};
use gpui_kit::component::input::InputEvent;
use flashcore::partition::{self, Partition};
use gpui_kit::component::button::*;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use crate::keys;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const CHIPS: &[&str] = &[
    "auto", "esp32", "esp32s2", "esp32s3", "esp32c2", "esp32c3", "esp32c5", "esp32c6", "esp32h2",
    "esp32p4", "esp8266",
];
const BAUDS: &[&str] = &["115200", "230400", "460800", "921600", "1500000"];
const MODES: &[&str] = &["keep", "dio", "dout", "qio", "qout"];
const FREQS: &[&str] = &["keep", "80m", "40m", "26m", "20m"];
const SIZES: &[&str] = &["keep", "detect", "1MB", "2MB", "4MB", "8MB", "16MB", "32MB"];
const MAX_LOG: usize = 3000;
const MON_BAUDS: &[&str] = &["9600", "57600", "74880", "115200", "230400", "460800", "921600"];
const ENDINGS: &[&str] = &["LF", "CRLF", "CR", "None"];

type Choice = Entity<SelectState<Vec<SharedString>>>;

struct Row {
    file: FlashFile,
    offset: Entity<InputState>,
}

pub struct FlashApp {
    focus: FocusHandle,
    show_help: bool,
    /// Last path given to `open_path`, for F4 reload.
    opened: Option<PathBuf>,
    port: Choice,
    ports: Vec<String>,
    baud: Choice,
    chip: Choice,
    mode: Choice,
    freq: Choice,
    size: Choice,
    build: Choice,

    projects: Vec<Project>,
    current: usize,
    rows: Vec<Row>,
    partitions: Option<Vec<Partition>>,
    merged: Option<PathBuf>,
    use_merged: bool,
    erase_before: bool,

    monitor: Option<Monitor>,
    /// Bumped on every open/close so a stale reader's `Closed` can't clear a newer monitor.
    mon_gen: u64,
    mon_lines: Vec<(SharedString, Level)>,
    mon_pending: String,
    mon_baud: Choice,
    mon_ending: Choice,
    mon_input: Entity<InputState>,
    mon_scroll: ScrollHandle,
    mon_after: bool,
    mon_paused: bool,
    tab_monitor: bool,
    /// Monitor was closed to free the port for a job; reopen when it ends.
    resume_monitor: bool,
    job_write: bool,

    logs: Vec<(SharedString, bool)>,
    log_scroll: ScrollHandle,
    log_open: bool,
    progress: f32,
    status: Option<Status>,
    task: Option<Task>,
    started: Option<Instant>,
    elapsed: Duration,
    _subs: Vec<Subscription>,
}

fn choice(items: &[&str], default: &str, window: &mut Window, cx: &mut App) -> Choice {
    let items: Vec<SharedString> = items.iter().map(|s| SharedString::from(*s)).collect();
    let idx = items.iter().position(|s| s == default).map(IndexPath::new);
    cx.new(|cx| SelectState::new(items, idx, window, cx))
}

fn value(c: &Choice, cx: &App) -> Option<String> {
    c.read(cx).selected_value().map(|s| s.to_string())
}

fn set(c: &Choice, v: &str, window: &mut Window, cx: &mut App) {
    c.update(cx, |s, cx| s.set_selected_value(&SharedString::from(v.to_owned()), window, cx));
}

fn human(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 20 => format!("{:.2} MB", b as f64 / (1 << 20) as f64),
        b if b >= 1 << 10 => format!("{:.1} KB", b as f64 / 1024.0),
        b => format!("{b} B"),
    }
}

/// Stable per-segment colour, shared by the row dot and the memory map.
fn seg_color(i: usize) -> Hsla {
    hsla((0.58 + i as f32 * 0.137) % 1.0, 0.55, 0.62, 1.0)
}

fn file_len(f: &FlashFile) -> Option<u64> {
    f.path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len())
}

impl FlashApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let build = choice(&[], "", window, cx);
        let sub = cx.subscribe_in(&build, window, |this, state, ev: &SelectEvent<_>, window, cx| {
            let SelectEvent::Confirm(Some(_)) = ev else { return };
            if let Some(i) = state.read(cx).selected_index(cx) {
                this.apply_project(i.row, window, cx);
            }
        });
        let mon_input = cx.new(|cx| InputState::new(window, cx).placeholder("Send to device — Enter"));
        let send_sub = cx.subscribe_in(&mon_input, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.send_input(window, cx);
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let mut app = Self {
            focus,
            show_help: false,
            opened: None,
            port: choice(&[], "", window, cx),
            ports: Vec::new(),
            baud: choice(BAUDS, "460800", window, cx),
            chip: choice(CHIPS, "auto", window, cx),
            mode: choice(MODES, "keep", window, cx),
            freq: choice(FREQS, "keep", window, cx),
            size: choice(SIZES, "keep", window, cx),
            build,
            projects: Vec::new(),
            current: 0,
            rows: Vec::new(),
            partitions: None,
            merged: None,
            use_merged: false,
            erase_before: false,
            monitor: None,
            mon_gen: 0,
            mon_lines: Vec::new(),
            mon_pending: String::new(),
            mon_baud: choice(MON_BAUDS, "115200", window, cx),
            mon_ending: choice(ENDINGS, "LF", window, cx),
            mon_input,
            mon_scroll: ScrollHandle::new(),
            mon_after: false,
            mon_paused: false,
            tab_monitor: false,
            resume_monitor: false,
            job_write: false,
            logs: Vec::new(),
            log_scroll: ScrollHandle::new(),
            log_open: true,
            progress: 0.,
            status: None,
            task: None,
            started: None,
            elapsed: Duration::ZERO,
            _subs: vec![sub, send_sub],
        };
        app.refresh_ports(window, cx);
        // Hotplug: rescan when the set of ports changes. ponytail: 1.5 s poll, udev monitor if it ever matters.
        cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(1500)).await;
            let names: Vec<String> = flashcore::list_ports().into_iter().map(|p| p.name).collect();
            let alive = this.update_in(cx, |this, window, cx| {
                if this.task.is_none() && names != this.ports {
                    this.refresh_ports(window, cx);
                }
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();
        match esptool::find_esptool() {
            Ok(p) => app.log(format!("esptool: {}", p.display()), false),
            Err(e) => app.log(e, true),
        }
        app
    }

    fn log(&mut self, text: impl Into<SharedString>, error: bool) {
        self.logs.push((text.into(), error));
        if self.logs.len() > MAX_LOG {
            self.logs.drain(..self.logs.len() - MAX_LOG);
        }
        if error {
            self.log_open = true;
        }
        self.log_scroll.scroll_to_bottom();
    }

    fn refresh_ports(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ports = flashcore::list_ports();
        let prev = value(&self.port, cx);
        self.ports = ports.iter().map(|p| p.name.clone()).collect();
        let names: Vec<SharedString> = ports.iter().map(|p| p.name.clone().into()).collect();
        let pick = prev
            .filter(|p| names.iter().any(|n| n == p))
            .or_else(|| names.first().map(|n| n.to_string()));
        self.port.update(cx, |s, cx| s.set_items(names, window, cx));
        if let Some(p) = pick {
            set(&self.port, &p, window, cx);
        }
        for p in &ports {
            let d = if p.description.is_empty() { String::new() } else { format!(" — {}", p.description) };
            self.log(format!("Port {}{d}", p.name), false);
        }
        if ports.is_empty() {
            self.log("No USB serial port found. Plug in the board and press ↻.", false);
        }
        cx.notify();
    }

    // ------------------------------------------------------------ loading

    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        match detect::detect_all(&path) {
            Ok(projects) if !projects.is_empty() => {
                // Reloading the same path keeps the selected build.
                let keep = if self.opened.as_ref() == Some(&path) { self.current.min(projects.len() - 1) } else { 0 };
                self.opened = Some(path.clone());
                let labels: Vec<SharedString> =
                    projects.iter().map(|p| format!("{} · {}", project_name(p), subtitle(p)).into()).collect();
                self.build.update(cx, |s, cx| {
                    s.set_items(labels, window, cx);
                    s.set_selected_index(Some(IndexPath::new(keep)), window, cx);
                });
                self.projects = projects;
                self.apply_project(keep, window, cx);
            }
            Ok(_) => self.log(format!("No ESP build output found in {}", path.display()), true),
            Err(e) => self.log(e, true),
        }
        cx.notify();
    }

    fn apply_project(&mut self, i: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.projects.get(i).cloned() else { return };
        self.current = i;
        self.rows = p.files.iter().map(|f| self.make_row(f.clone(), window, cx)).collect();
        set(&self.chip, p.chip.as_deref().unwrap_or("auto"), window, cx);
        set(&self.mode, p.settings.mode.as_deref().unwrap_or("keep"), window, cx);
        set(&self.freq, p.settings.freq.as_deref().unwrap_or("keep"), window, cx);
        set(&self.size, p.settings.size.as_deref().unwrap_or("keep"), window, cx);
        self.partitions = p.partitions.clone();
        self.merged = p.merged.clone();
        self.use_merged = false;
        self.log(format!("Loaded {} ({})", project_name(&p), p.origin.display()), false);
        cx.notify();
    }

    fn make_row(&self, file: FlashFile, window: &mut Window, cx: &mut Context<Self>) -> Row {
        let text = format!("{:#x}", file.offset);
        Row { offset: cx.new(|cx| InputState::new(window, cx).default_value(text)), file }
    }

    fn prompt_open(&mut self, directories: bool, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: !directories,
            directories,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = rx.await {
                if let Some(p) = paths.pop() {
                    let _ = this.update_in(cx, |this, window, cx| this.open_path(p, window, cx));
                }
            }
        })
        .detach();
    }

    fn browse_row(&mut self, row: usize, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = rx.await {
                if let Some(p) = paths.pop() {
                    let _ = this.update(cx, |this, cx| {
                        if let Some(r) = this.rows.get_mut(row) {
                            r.file.path = Some(p);
                            r.file.enabled = true;
                        }
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let next = self.rows.last().map_or(0, |r| r.file.offset + 0x1000);
        let file = FlashFile { offset: next, path: None, label: "Custom".into(), max_size: None, enabled: true };
        let row = self.make_row(file, window, cx);
        self.rows.push(row);
        cx.notify();
    }

    /// Files as currently shown, with offsets read back from the inputs.
    fn plan(&self, cx: &App) -> Result<Vec<FlashFile>, String> {
        if self.use_merged {
            if let Some(m) = &self.merged {
                return Ok(vec![FlashFile {
                    offset: 0,
                    path: Some(m.clone()),
                    label: "Merged image".into(),
                    max_size: None,
                    enabled: true,
                }]);
            }
        }
        self.rows
            .iter()
            .map(|r| {
                let text = r.offset.read(cx).value();
                let offset = partition::parse_num(&text)
                    .ok_or_else(|| format!("{}: invalid address `{text}`", r.file.label))?;
                Ok(FlashFile { offset, ..r.file.clone() })
            })
            .collect()
    }

    // ------------------------------------------------------------ operations

    fn conn(&self, cx: &App) -> Result<Conn, String> {
        let port = value(&self.port, cx).ok_or("Select a serial port first")?;
        Ok(Conn {
            port,
            baud: value(&self.baud, cx).and_then(|b| b.parse().ok()).unwrap_or(460800),
            chip: value(&self.chip, cx).filter(|c| c != "auto"),
        })
    }

    // ------------------------------------------------------------ monitor

    fn start_monitor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.monitor.is_some() {
            return;
        }
        let Some(port) = value(&self.port, cx) else { return self.log("Select a serial port first", true) };
        let baud = value(&self.mon_baud, cx).and_then(|b| b.parse().ok()).unwrap_or(115200);
        let (tx, rx) = mpsc::channel();
        match monitor::open(&port, baud, Arc::new(move |e| _ = tx.send(e))) {
            Ok(m) => self.monitor = Some(m),
            Err(e) => return self.log(format!("Monitor: {e}"), true),
        }
        self.mon_gen += 1;
        let gen = self.mon_gen;
        self.push_mon_line(format!("── monitor {port} @ {baud} ──"), Level::Other);
        self.log_open = true;
        cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(30)).await;
            let done = this.update(cx, |this, cx| {
                let mut done = false;
                loop {
                    match rx.try_recv() {
                        Ok(MonEvent::Data(b)) => this.push_mon_bytes(&b),
                        Ok(MonEvent::Closed(err)) => {
                            if this.mon_gen == gen {
                                this.monitor = None;
                            }
                            if let Some(e) = err {
                                this.push_mon_line(format!("── monitor stopped: {e} ──"), Level::Error);
                            }
                            done = true;
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            done = true;
                            break;
                        }
                    }
                }
                cx.notify();
                done
            });
            if done.unwrap_or(true) {
                break;
            }
        })
        .detach();
        cx.notify();
    }

    /// Blocks until the port is released (≤ ~50 ms).
    fn stop_monitor(&mut self) {
        if self.monitor.take().is_some() {
            self.mon_gen += 1;
            self.push_mon_line("── monitor closed ──".into(), Level::Other);
        }
    }

    fn toggle_monitor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.task.is_some() {
            return;
        }
        if self.monitor.is_some() {
            self.stop_monitor();
        } else {
            self.tab_monitor = true;
            self.start_monitor(window, cx);
        }
        cx.notify();
    }

    fn reset_board(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.task.is_some() {
            return;
        }
        self.tab_monitor = true;
        self.start_monitor(window, cx);
        if let Some(m) = &self.monitor {
            m.reset();
        }
    }

    fn send_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(m) = &self.monitor else { return };
        let ending = match value(&self.mon_ending, cx).as_deref() {
            Some("CRLF") => "\r\n",
            Some("CR") => "\r",
            Some("None") => "",
            _ => "\n",
        };
        let text = self.mon_input.read(cx).value().to_string();
        m.send(format!("{text}{ending}").into_bytes());
        self.mon_input.update(cx, |s, cx| s.set_value("", window, cx));
    }

    // ponytail: lossy UTF-8 per chunk; a multibyte char split across reads shows as �.
    fn push_mon_bytes(&mut self, bytes: &[u8]) {
        self.mon_pending.push_str(&String::from_utf8_lossy(bytes));
        while let Some(i) = self.mon_pending.find('\n') {
            let line: String = self.mon_pending.drain(..=i).collect();
            let text = esptool::strip_ansi(line.trim_end_matches(['\r', '\n']));
            let lvl = monitor::level(&text);
            self.push_mon_line(text, lvl);
        }
    }

    fn push_mon_line(&mut self, text: String, lvl: Level) {
        self.mon_lines.push((text.into(), lvl));
        if self.mon_lines.len() > MAX_LOG {
            self.mon_lines.drain(..self.mon_lines.len() - MAX_LOG);
        }
        if !self.mon_paused {
            self.mon_scroll.scroll_to_bottom();
        }
    }

    fn cancel(&mut self) {
        if let Some(t) = &self.task {
            t.cancel();
        }
    }

    /// Board-talking shortcuts only fire when idle with a port, same as their buttons.
    fn can_talk(&self) -> bool {
        self.task.is_none() && !self.ports.is_empty()
    }

    fn program(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let files = match self.plan(cx) {
            Ok(f) => f,
            Err(e) => return self.log(e, true),
        };
        let issues = detect::validate(&files);
        if !issues.is_empty() {
            for i in issues {
                self.log(format!("Blocked: {i}"), true);
            }
            return cx.notify();
        }
        let keep = |c: &Choice| value(c, cx).filter(|v| v != "keep");
        let op = Op::Write {
            settings: FlashSettings { mode: keep(&self.mode), freq: keep(&self.freq), size: keep(&self.size) },
            erase_before: self.erase_before,
            files: files.into_iter().filter(|f| f.enabled).filter_map(|f| Some((f.offset, f.path?))).collect(),
            extra_args: Vec::new(),
        };
        self.start(op, window, cx);
    }

    fn erase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let answer = window.prompt(
            PromptLevel::Warning,
            "Erase the entire flash?",
            Some("Everything on the chip, including NVS data, will be lost."),
            &["Erase", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                let _ = this.update_in(cx, |this, window, cx| this.start(Op::Erase, window, cx));
            }
        })
        .detach();
    }

    fn start(&mut self, op: Op, window: &mut Window, cx: &mut Context<Self>) {
        if self.task.is_some() {
            return;
        }
        let conn = match self.conn(cx) {
            Ok(c) => c,
            Err(e) => return self.log(e, true),
        };
        let tool = match esptool::find_esptool() {
            Ok(t) => t,
            Err(e) => return self.log(e, true),
        };
        if self.monitor.is_some() {
            self.stop_monitor();
            self.resume_monitor = true;
        }
        self.job_write = matches!(op, Op::Write { .. });
        self.tab_monitor = false;
        self.logs.clear();
        self.log_open = true;
        self.progress = 0.;
        self.status = Some(Status::Connecting);
        self.started = Some(Instant::now());
        let (tx, rx) = mpsc::channel();
        self.task = Some(esptool::spawn(tool, conn, op, Arc::new(move |e| _ = tx.send(e))));

        // ponytail: 50 ms polling of a std channel; fine for log-rate events.
        cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(50)).await;
            let done = this.update_in(cx, |this, window, cx| {
                let mut done = false;
                while let Ok(ev) = rx.try_recv() {
                    done |= this.on_event(ev, window, cx);
                }
                if let Some(t) = this.started {
                    this.elapsed = t.elapsed();
                }
                cx.notify();
                done
            });
            if done.unwrap_or(true) {
                break;
            }
        })
        .detach();
        cx.notify();
    }

    /// Returns true on the final event.
    fn on_event(&mut self, ev: Event, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match ev {
            Event::Log { text, error } => {
                if let Some(chip) = esptool::chip_from_output(&text) {
                    if value(&self.chip, cx).as_deref() == Some("auto") {
                        set(&self.chip, chip, window, cx);
                    }
                }
                self.log(text, error);
            }
            Event::Progress(p) => self.progress = p as f32,
            Event::Status(s) => self.status = Some(s),
            Event::Finished { success } => {
                self.task = None;
                self.started = None;
                if success {
                    self.progress = 100.;
                }
                self.log(if success { "Finished successfully." } else { "Operation failed." }, !success);
                // esptool's default `--after hard-reset` already rebooted the board.
                let after_flash = success && self.job_write && self.mon_after;
                if after_flash || std::mem::take(&mut self.resume_monitor) {
                    self.start_monitor(window, cx);
                    self.tab_monitor |= after_flash && self.monitor.is_some();
                }
                return true;
            }
        }
        false
    }

    /// Flash size for the memory map: the chosen size, else enough to hold everything.
    fn flash_total(&self, cx: &App) -> u64 {
        if let Some(n) = value(&self.size, cx).and_then(|s| partition::parse_num(s.trim_end_matches('B'))) {
            return n as u64;
        }
        let end = self
            .rows
            .iter()
            .map(|r| r.file.offset as u64 + file_len(&r.file).unwrap_or(0))
            .chain(self.partitions.iter().flatten().map(|p| p.end() as u64))
            .max()
            .unwrap_or(0);
        end.max(1 << 20).next_power_of_two()
    }
}

// ---------------------------------------------------------------- view helpers

fn field(label: &'static str, el: impl IntoElement, cx: &App) -> Div {
    v_flex()
        .gap_1()
        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(label))
        .child(el)
}

fn section_title(text: impl Into<SharedString>, cx: &App) -> Div {
    div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(cx.theme().muted_foreground).child(text.into())
}

/// Human name of the firmware project: the first ancestor that isn't a build
/// directory (`build`, `partition_table`, Arduino's `vendor.arch.board`, or an
/// `--output-dir build/esp32`). A loose image is named after itself.
fn project_name(p: &Project) -> String {
    if p.source == Source::SingleBin {
        if let Some(stem) = p.origin.file_name().and_then(|n| n.to_str()?.split('.').next()) {
            return stem.to_owned();
        }
    }
    p.origin
        .ancestors()
        .skip(1)
        .filter_map(|d| d.file_name()?.to_str())
        .find(|n| *n != "build" && *n != "partition_table" && n.matches('.').count() < 2 && !CHIPS.contains(n))
        .unwrap_or("firmware")
        .to_owned()
}

fn subtitle(p: &Project) -> String {
    let src = match p.source {
        Source::IdfFlasherArgs => "ESP-IDF",
        Source::FlashArgs => "Arduino",
        Source::PartitionTable => "Partition table",
        Source::SingleBin => "Single image",
    };
    format!("{} · {src}", p.chip.as_deref().unwrap_or("unknown chip"))
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Closing the window mid-flash must not leave an orphaned esptool on the port.
impl Drop for FlashApp {
    fn drop(&mut self) {
        if let Some(t) = &self.task {
            t.cancel();
        }
    }
}

impl FlashApp {
    /// F1 cheat-sheet, drawn over the window; click anywhere or Esc/F1 closes it.
    fn render_help(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        div()
            .id("help")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background.opacity(0.7))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.show_help = false;
                cx.notify();
            }))
            .child(
                v_flex()
                    .w(px(420.))
                    .p_5()
                    .gap_2()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .child(div().pb_2().font_weight(FontWeight::SEMIBOLD).child("Keyboard shortcuts"))
                    .children(keys::TABLE.iter().map(|(k, d)| {
                        h_flex()
                            .gap_4()
                            .text_sm()
                            .child(div().w(px(150.)).flex_shrink_0().font_family("monospace").text_color(theme.muted_foreground).child(*k))
                            .child(*d)
                    })),
            )
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.task.is_some();
        let theme = cx.theme();
        h_flex()
            .px_4()
            .py_3()
            .gap_4()
            .items_end()
            .justify_between()
            .bg(theme.title_bar)
            .border_b_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .gap_3()
                    .items_end()
                    .child(field(
                        "Serial port",
                        h_flex()
                            .gap_1()
                            .child(Select::new(&self.port).w(px(200.)).small().placeholder("No USB port"))
                            .child(
                                Button::new("refresh")
                                    .small()
                                    .ghost()
                                    .icon(IconName::RefreshCw)
                                    .tooltip("Rescan ports (F6)")
                                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                        this.refresh_ports(window, cx)
                                    })),
                            ),
                        cx,
                    ))
                    .child(field("Baud", Select::new(&self.baud).w(px(110.)).small(), cx))
                    .child(field("Chip", Select::new(&self.chip).w(px(120.)).small(), cx))
                    .child(
                        Button::new("detect")
                            .small()
                            .icon(IconName::Cpu)
                            .label("Detect")
                            .tooltip("Connect and identify the chip (F2)")
                            .disabled(busy || self.ports.is_empty())
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.start(Op::ChipInfo, window, cx)
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("open-dir")
                            .small()
                            .icon(IconName::FolderOpen)
                            .label("Open folder")
                            .tooltip("Sketch, ESP-IDF project or build directory (F3)")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.prompt_open(true, window, cx)
                            })),
                    )
                    .child(
                        Button::new("open-file")
                            .small()
                            .ghost()
                            .icon(IconName::File)
                            .label("Open file")
                            .tooltip("flasher_args.json, flash_args, partitions.csv/.bin or a firmware .bin (Shift+F3)")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.prompt_open(false, window, cx)
                            })),
                    ),
            )
    }

    fn render_project(&self, p: &Project, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .px_4()
            .pt_4()
            .pb_3()
            .gap_4()
            .items_end()
            .justify_between()
            .child(
                v_flex()
                    .gap_1()
                    .min_w_0()
                    .when(self.projects.len() > 1, |el| el.child(Select::new(&self.build).w(px(320.)).small()))
                    .when(self.projects.len() == 1, |el| {
                        el.child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(project_name(p)))
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .truncate()
                            .child(format!("{} · {}", subtitle(p), p.origin.display())),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_end()
                    .flex_shrink_0()
                    .child(field("Flash mode", Select::new(&self.mode).w(px(90.)).small(), cx))
                    .child(field("Frequency", Select::new(&self.freq).w(px(90.)).small(), cx))
                    .child(field("Size", Select::new(&self.size).w(px(100.)).small(), cx))
                    .child(
                        v_flex()
                            .gap_2()
                            .pb_1()
                            .child(
                                Switch::new("erase-before")
                                    .label("Erase first")
                                    .tooltip("Erase the whole chip before writing (F7)")
                                    .checked(self.erase_before)
                                    .on_click(cx.listener(|this, v: &bool, _, cx| {
                                        this.erase_before = *v;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Switch::new("mon-after")
                                    .label("Monitor after")
                                    .tooltip("Open the serial monitor when flashing succeeds")
                                    .checked(self.mon_after)
                                    .on_click(cx.listener(|this, v: &bool, _, cx| {
                                        this.mon_after = *v;
                                        cx.notify();
                                    })),
                            )
                            .when(self.merged.is_some(), |el| {
                                el.child(
                                    Switch::new("use-merged")
                                        .label("Merged image")
                                        .tooltip("Write the single *.merged.bin at 0x0 instead of the segments")
                                        .checked(self.use_merged)
                                        .on_click(cx.listener(|this, v: &bool, _, cx| {
                                            this.use_merged = *v;
                                            cx.notify();
                                        })),
                                )
                            }),
                    ),
            )
    }

    /// One bar = the whole flash. Partitions are outlined, segments filled with
    /// their row colour, so overlaps and free space are visible at a glance.
    fn render_memory_map(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let total = self.flash_total(cx) as f32;
        let frac = |v: u64| (v as f32 / total).clamp(0., 1.);
        let blocks: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.file.enabled && !self.use_merged)
            .filter_map(|(i, r)| {
                let len = file_len(&r.file)?;
                let off = partition::parse_num(&r.offset.read(cx).value()).unwrap_or(r.file.offset) as u64;
                let tip = format!("{} · {:#x}–{:#x} · {}", r.file.label, off, off + len, human(len));
                Some(
                    div()
                        .id(("map", i))
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(frac(off)))
                        .w(relative(frac(len).max(0.004)))
                        .bg(seg_color(i))
                        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx)),
                )
            })
            .collect();
        // Drawn over the segments so every partition keeps its divider and name;
        // names only where they fit (~5% of the bar), the rest are in the tooltips.
        let parts = self.partitions.iter().flatten().map(|p| {
            let w = frac(p.size as u64);
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(frac(p.offset as u64)))
                .w(relative(w))
                .border_l_1()
                .border_color(theme.background)
                .flex()
                .items_center()
                .px_1()
                .text_xs()
                .text_color(theme.foreground)
                .overflow_hidden()
                .when(w >= 0.05, |el| el.child(div().truncate().child(p.name.clone())))
        });
        v_flex()
            .px_4()
            .pb_3()
            .gap_1()
            .child(
                div()
                    .relative()
                    .h(px(26.))
                    .w_full()
                    .rounded_md()
                    .overflow_hidden()
                    .bg(theme.muted)
                    .children(blocks)
                    .children(parts),
            )
            .child(
                h_flex()
                    .justify_between()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("0x0")
                    .child(format!("{} flash", human(total as u64))),
            )
    }

    fn render_segments(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let dim = self.use_merged;
        let rows: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let f = &r.file;
                let len = file_len(f);
                let over = matches!((len, f.max_size), (Some(l), Some(m)) if l > m as u64);
                let (name, dir, name_color) = match &f.path {
                    Some(p) if len.is_some() => (
                        file_name(p),
                        p.parent().map(|d| d.display().to_string()).unwrap_or_default(),
                        theme.foreground,
                    ),
                    Some(p) => (file_name(p), "File not found".into(), theme.danger),
                    None => (
                        "No file".into(),
                        "Choose a file or drop one on the window".into(),
                        theme.muted_foreground,
                    ),
                };
                let usage = match (len, f.max_size) {
                    (Some(l), Some(m)) => Some((l as f32 / m as f32).min(1.)),
                    _ => None,
                };
                let size_text = match (len, f.max_size) {
                    (Some(l), Some(m)) => format!("{} / {}", human(l), human(m as u64)),
                    (Some(l), None) => human(l),
                    (None, Some(m)) => format!("{} slot", human(m as u64)),
                    (None, None) => String::new(),
                };
                h_flex()
                    .id(("row", i))
                    .px_4()
                    .py_2()
                    .gap_3()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.table_row_border)
                    .hover(|s| s.bg(theme.table_even))
                    .when(dim || !f.enabled, |el| el.opacity(0.45))
                    .child(Checkbox::new(("en", i)).checked(f.enabled).on_click(cx.listener(
                        move |this, v: &bool, _, cx| {
                            this.rows[i].file.enabled = *v;
                            cx.notify();
                        },
                    )))
                    .child(div().size(px(8.)).rounded_full().flex_shrink_0().bg(seg_color(i)))
                    .child(
                        div()
                            .w(px(150.))
                            .flex_shrink_0()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(f.label.clone()),
                    )
                    .child(div().w(px(104.)).flex_shrink_0().child(Input::new(&r.offset).small()))
                    .child(
                        v_flex()
                            .id(("file", i))
                            .flex_1()
                            .min_w_0()
                            .child(div().text_sm().text_color(name_color).truncate().child(name))
                            .child(div().text_xs().text_color(theme.muted_foreground).truncate().child(dir))
                            .when_some(f.path.clone(), |el, p| {
                                el.tooltip(move |window, cx| Tooltip::new(p.display().to_string()).build(window, cx))
                            }),
                    )
                    .child(
                        v_flex()
                            .w(px(140.))
                            .flex_shrink_0()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if over { theme.danger } else { theme.muted_foreground })
                                    .child(size_text),
                            )
                            .when_some(usage, |el, u| {
                                el.child(
                                    div().h(px(3.)).w_full().rounded_full().bg(theme.muted).child(
                                        div()
                                            .h_full()
                                            .rounded_full()
                                            .w(relative(u))
                                            .bg(if over { theme.danger } else { theme.muted_foreground }),
                                    ),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .child(
                                Button::new(("browse", i))
                                    .small()
                                    .ghost()
                                    .icon(IconName::FolderOpen)
                                    .tooltip("Choose file")
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.browse_row(i, cx))),
                            )
                            .child(
                                Button::new(("del", i))
                                    .small()
                                    .ghost()
                                    .icon(IconName::Close)
                                    .tooltip("Remove segment")
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.rows.remove(i);
                                        cx.notify();
                                    })),
                            ),
                    )
            })
            .collect();

        v_flex()
            .child(
                h_flex()
                    .px_4()
                    .py_1()
                    .justify_between()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(section_title(format!("SEGMENTS · {}", self.rows.len()), cx))
                    .child(
                        Button::new("add")
                            .xsmall()
                            .ghost()
                            .icon(IconName::Plus)
                            .label("Add")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.add_row(window, cx))),
                    ),
            )
            .children(rows)
    }

    fn render_issues(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let issues = match self.plan(cx) {
            Ok(f) => detect::validate(&f),
            Err(e) => vec![e],
        };
        let theme = cx.theme();
        v_flex().px_4().py_2().gap_1().children(issues.into_iter().map(|i| {
            h_flex()
                .gap_2()
                .items_center()
                .text_xs()
                .text_color(theme.warning)
                .child(Icon::new(IconName::TriangleAlert).size_3p5())
                .child(i)
        }))
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_3()
            .child(Icon::new(IconName::FolderOpen).size_10().text_color(theme.muted_foreground))
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Drop an Arduino or ESP-IDF build here"))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Project or build folder, flasher_args.json, flash_args, partitions.csv / .bin, or a firmware .bin"),
            )
            .child(
                h_flex()
                    .gap_2()
                    .pt_2()
                    .child(Button::new("empty-dir").primary().icon(IconName::FolderOpen).label("Open folder").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.prompt_open(true, window, cx)),
                    ))
                    .child(Button::new("empty-file").icon(IconName::File).label("Open file").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.prompt_open(false, window, cx)),
                    )),
            )
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.task.is_some();
        let theme = cx.theme().clone();
        let (icon, text, color) = match self.status {
            None => (None, "Ready", theme.muted_foreground),
            Some(Status::Connecting) => (Some(IconName::LoaderCircle), "Connecting…", theme.foreground),
            Some(Status::Erasing) => (Some(IconName::LoaderCircle), "Erasing…", theme.foreground),
            Some(Status::Writing) => (Some(IconName::LoaderCircle), "Writing…", theme.foreground),
            Some(Status::Reading) => (Some(IconName::LoaderCircle), "Reading…", theme.foreground),
            Some(Status::Done) => (Some(IconName::CircleCheck), "Done", theme.success),
            Some(Status::Error) => (Some(IconName::CircleX), "Failed", theme.danger),
        };
        let secs = self.elapsed.as_secs();
        let has_port = !self.ports.is_empty();
        let can_program = has_port && (!self.rows.is_empty() || self.use_merged);
        h_flex()
            .px_4()
            .py_3()
            .gap_4()
            .items_center()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.title_bar)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .w(px(130.))
                    .text_sm()
                    .text_color(color)
                    .when_some(icon, |el, i| el.child(Icon::new(i).size_4()))
                    .child(text),
            )
            .child(div().flex_1().child(Progress::new("progress").value(self.progress)))
            .child(
                div()
                    .w(px(96.))
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!("{:.0}%  {:02}:{:02}", self.progress, secs / 60, secs % 60)),
            )
            .when(!busy, |el| {
                el.child(
                    Button::new("erase")
                        .ghost()
                        .danger()
                        .icon(IconName::Delete)
                        .label("Erase chip")
                        .tooltip("Erase the whole flash (F8)")
                        .disabled(!has_port)
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.erase(window, cx))),
                )
                .child(
                    Button::new("program")
                        .primary()
                        .icon(IconName::Play)
                        .label("Program")
                        .tooltip("Write the enabled segments (F5)")
                        .disabled(!can_program)
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.program(window, cx))),
                )
            })
            .when(busy, |el| {
                el.child(Button::new("cancel").danger().icon(IconName::Square).label("Cancel").tooltip("Stop (Esc)").on_click(
                    cx.listener(|this, _: &ClickEvent, _, _| this.cancel()),
                ))
            })
    }

    fn clear_console(&mut self) {
        if self.tab_monitor {
            self.mon_lines.clear();
            self.mon_pending.clear();
        } else {
            self.logs.clear();
        }
    }

    fn render_console(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let running = self.monitor.is_some();
        let busy = self.task.is_some();
        let tab = |id: &'static str, label: &'static str, active: bool, monitor: bool| {
            Button::new(id)
                .xsmall()
                .ghost()
                .selected(active)
                .label(if monitor && running { format!("{label} ●") } else { label.to_owned() })
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.tab_monitor = monitor;
                    this.log_open = true;
                    cx.notify();
                }))
        };
        let head = h_flex()
            .px_4()
            .py_1()
            .gap_2()
            .items_center()
            .child(
                h_flex()
                    .id("console-head")
                    .gap_2()
                    .items_center()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.log_open = !this.log_open;
                        cx.notify();
                    }))
                    .child(
                        Icon::new(if self.log_open { IconName::ChevronDown } else { IconName::ChevronRight })
                            .size_3p5()
                            .text_color(theme.muted_foreground),
                    )
                    .child(section_title("CONSOLE", cx)),
            )
            .child(tab("tab-log", "Log", !self.tab_monitor, false))
            .child(tab("tab-mon", "Monitor", self.tab_monitor, true))
            .child(div().flex_1())
            .when(self.tab_monitor, |el| {
                el.child(Select::new(&self.mon_baud).w(px(100.)).xsmall().disabled(running))
                    .child(Select::new(&self.mon_ending).w(px(80.)).xsmall())
                    .child(
                        Button::new("mon-reset")
                            .xsmall()
                            .ghost()
                            .icon(IconName::RefreshCw)
                            .label("Reset board")
                            .tooltip("Pulse EN via RTS to reboot without flashing")
                            .disabled(busy || self.ports.is_empty())
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.reset_board(window, cx))),
                    )
                    .child(
                        Button::new("mon-pause")
                            .xsmall()
                            .ghost()
                            .selected(self.mon_paused)
                            .label(if self.mon_paused { "Resume" } else { "Pause" })
                            .tooltip("Stop following new output")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.mon_paused = !this.mon_paused;
                                if !this.mon_paused {
                                    this.mon_scroll.scroll_to_bottom();
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("mon-toggle")
                            .xsmall()
                            .when(running, |b| b.danger())
                            .when(!running, |b| b.primary())
                            .icon(if running { IconName::Square } else { IconName::Play })
                            .label(if running { "Stop" } else { "Start" })
                            .tooltip("Start / stop the serial monitor (F10)")
                            .disabled(busy || (!running && self.ports.is_empty()))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_monitor(window, cx))),
                    )
            })
            .child(Button::new("clear-log").xsmall().ghost().label("Clear").tooltip("Clear (Ctrl+L)").on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.clear_console();
                    cx.notify();
                }),
            ));

        let lines = div()
            .h(px(if self.tab_monitor { 200. } else { 170. }))
            .px_4()
            .pb_2()
            .bg(theme.background)
            .font_family("monospace")
            .text_xs()
            .line_height(rems(1.35));
        let body = if self.tab_monitor {
            let color = |l: Level| match l {
                Level::Error => theme.danger,
                Level::Warn => theme.warning,
                Level::Info => theme.success,
                Level::Other => theme.foreground,
            };
            v_flex()
                .child(
                    lines
                        .id("mon")
                        .overflow_y_scroll()
                        .track_scroll(&self.mon_scroll)
                        .children(self.mon_lines.iter().map(|(t, l)| div().text_color(color(*l)).child(t.clone())))
                        .when(!self.mon_pending.is_empty(), |el| {
                            el.child(div().child(esptool::strip_ansi(&self.mon_pending)))
                        }),
                )
                .child(
                    h_flex()
                        .px_4()
                        .pb_2()
                        .gap_2()
                        .child(div().flex_1().child(Input::new(&self.mon_input).small().disabled(!running)))
                        .child(
                            Button::new("mon-send")
                                .small()
                                .label("Send")
                                .disabled(!running)
                                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.send_input(window, cx))),
                        ),
                )
                .into_any_element()
        } else {
            lines
                .id("log")
                .overflow_y_scroll()
                .track_scroll(&self.log_scroll)
                .children(self.logs.iter().map(|(t, err)| {
                    div().text_color(if *err { theme.danger } else { theme.muted_foreground }).child(t.clone())
                }))
                .into_any_element()
        };
        v_flex()
            .flex_shrink_0()
            .border_t_1()
            .border_color(theme.border)
            .child(head)
            .when(self.log_open, |el| el.child(body))
    }
}

impl Render for FlashApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let project = self.projects.get(self.current).cloned();
        let body = match &project {
            None => self.render_empty(cx).into_any_element(),
            Some(p) => v_flex()
                .flex_1()
                .min_h_0()
                .child(self.render_project(p, cx))
                .child(self.render_memory_map(cx))
                .child(
                    div()
                        .id("segments")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(self.render_segments(cx))
                        .child(self.render_issues(cx)),
                )
                .into_any_element(),
        };
        v_flex()
            .size_full()
            .relative()
            .key_context(keys::CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &keys::Help, _, cx| {
                this.show_help = !this.show_help;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &keys::DetectChip, window, cx| {
                if this.can_talk() {
                    this.start(Op::ChipInfo, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &keys::OpenFolder, window, cx| {
                if this.task.is_none() {
                    this.prompt_open(true, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &keys::OpenFile, window, cx| {
                if this.task.is_none() {
                    this.prompt_open(false, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &keys::Reload, window, cx| {
                if let (None, Some(p)) = (&this.task, this.opened.clone()) {
                    this.open_path(p, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &keys::Program, window, cx| {
                if this.can_talk() {
                    this.program(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &keys::Cancel, _, cx| {
                if this.show_help {
                    this.show_help = false;
                    cx.notify();
                } else {
                    this.cancel();
                }
            }))
            .on_action(cx.listener(|this, _: &keys::RescanPorts, window, cx| {
                if this.task.is_none() {
                    this.refresh_ports(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &keys::ToggleErase, _, cx| {
                if this.task.is_none() {
                    this.erase_before = !this.erase_before;
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &keys::EraseChip, window, cx| {
                if this.can_talk() {
                    this.erase(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &keys::ToggleConsole, _, cx| {
                this.log_open = !this.log_open;
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &keys::Fullscreen, window, _| window.toggle_fullscreen()))
            .on_action(cx.listener(|this, _: &keys::ClearLog, _, cx| {
                this.clear_console();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &keys::ToggleMonitor, window, cx| this.toggle_monitor(window, cx)))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                if let Some(p) = paths.paths().first() {
                    this.open_path(p.clone(), window, cx);
                }
            }))
            // Wayland/GNOME gives client-decorated windows no frame: draw our own.
            .child(TitleBar::new().child(div().text_sm().child("EspFlash Tool")))
            .child(self.render_header(cx))
            .child(body)
            .child(self.render_footer(cx))
            .child(self.render_console(cx))
            .when(self.show_help, |el| el.child(self.render_help(cx)))
    }
}
