use flashcore::detect::{self, FlashFile, FlashSettings, Project, Source};
use flashcore::esptool::{self, Conn, Event, Op, Status, Task};
use flashcore::partition::{self, Partition};
use gpui_kit::component::button::*;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;
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

type Choice = Entity<SelectState<Vec<SharedString>>>;

struct Row {
    file: FlashFile,
    offset: Entity<InputState>,
}

pub struct FlashApp {
    port: Choice,
    baud: Choice,
    chip: Choice,
    mode: Choice,
    freq: Choice,
    size: Choice,
    build: Choice,

    projects: Vec<Project>,
    rows: Vec<Row>,
    partitions: Option<Vec<Partition>>,
    merged: Option<PathBuf>,
    use_merged: bool,
    erase_before: bool,

    logs: Vec<(SharedString, bool)>,
    log_scroll: ScrollHandle,
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

impl FlashApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let build = choice(&[], "", window, cx);
        let sub = cx.subscribe_in(&build, window, |this, state, ev: &SelectEvent<_>, window, cx| {
            let SelectEvent::Confirm(Some(_)) = ev else { return };
            if let Some(i) = state.read(cx).selected_index(cx) {
                this.apply_project(i.row, window, cx);
            }
        });
        let mut app = Self {
            port: choice(&[], "", window, cx),
            baud: choice(BAUDS, "460800", window, cx),
            chip: choice(CHIPS, "auto", window, cx),
            mode: choice(MODES, "keep", window, cx),
            freq: choice(FREQS, "keep", window, cx),
            size: choice(SIZES, "keep", window, cx),
            build,
            projects: Vec::new(),
            rows: Vec::new(),
            partitions: None,
            merged: None,
            use_merged: false,
            erase_before: false,
            logs: Vec::new(),
            log_scroll: ScrollHandle::new(),
            progress: 0.,
            status: None,
            task: None,
            started: None,
            elapsed: Duration::ZERO,
            _subs: vec![sub],
        };
        app.refresh_ports(window, cx);
        match esptool::find_esptool() {
            Ok(p) => app.log(format!("esptool: {}", p.display()), false),
            Err(e) => app.log(e, true),
        }
        app.log("Drop a build folder, flasher_args.json, flash_args, partition table or .bin here.", false);
        app
    }

    fn log(&mut self, text: impl Into<SharedString>, error: bool) {
        self.logs.push((text.into(), error));
        if self.logs.len() > MAX_LOG {
            self.logs.drain(..self.logs.len() - MAX_LOG);
        }
        self.log_scroll.scroll_to_bottom();
    }

    fn refresh_ports(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ports = flashcore::list_ports();
        let prev = value(&self.port, cx);
        let names: Vec<SharedString> = ports.iter().map(|p| p.name.clone().into()).collect();
        let pick = prev
            .filter(|p| names.iter().any(|n| n == p))
            .or_else(|| names.first().map(|n| n.to_string()));
        self.port.update(cx, |s, cx| s.set_items(names, window, cx));
        if let Some(p) = pick {
            set(&self.port, &p, window, cx);
        }
        let desc: Vec<String> = ports
            .iter()
            .map(|p| if p.description.is_empty() { p.name.clone() } else { format!("{} ({})", p.name, p.description) })
            .collect();
        self.log(
            if desc.is_empty() { "No serial ports found".into() } else { format!("Ports: {}", desc.join(", ")) },
            false,
        );
        cx.notify();
    }

    // ------------------------------------------------------------ loading

    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        match detect::detect_all(&path) {
            Ok(projects) if !projects.is_empty() => {
                let labels: Vec<SharedString> = projects.iter().map(project_label).collect();
                self.build.update(cx, |s, cx| {
                    s.set_items(labels, window, cx);
                    s.set_selected_index(Some(IndexPath::new(0)), window, cx);
                });
                self.projects = projects;
                self.apply_project(0, window, cx);
            }
            Ok(_) => self.log(format!("No ESP build output found in {}", path.display()), true),
            Err(e) => self.log(e, true),
        }
        cx.notify();
    }

    fn apply_project(&mut self, i: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.projects.get(i).cloned() else { return };
        self.rows = p
            .files
            .iter()
            .map(|f| Row {
                offset: cx.new(|cx| InputState::new(window, cx).default_value(format!("{:#x}", f.offset))),
                file: f.clone(),
            })
            .collect();
        set(&self.chip, p.chip.as_deref().unwrap_or("auto"), window, cx);
        set(&self.mode, p.settings.mode.as_deref().unwrap_or("keep"), window, cx);
        set(&self.freq, p.settings.freq.as_deref().unwrap_or("keep"), window, cx);
        set(&self.size, p.settings.size.as_deref().unwrap_or("keep"), window, cx);
        self.partitions = p.partitions.clone();
        self.merged = p.merged.clone();
        self.use_merged = false;
        self.log(format!("Loaded {}", project_label(&p)), false);
        cx.notify();
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
        self.rows.push(Row {
            offset: cx.new(|cx| InputState::new(window, cx).default_value(format!("{next:#x}"))),
            file: FlashFile { offset: next, path: None, label: "Custom".into(), max_size: None, enabled: true },
        });
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
                    .ok_or_else(|| format!("{}: invalid offset `{text}`", r.file.label))?;
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

    fn program(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let files = match self.plan(cx) {
            Ok(f) => f,
            Err(e) => return self.log(e, true),
        };
        let issues = detect::validate(&files);
        if !issues.is_empty() {
            for i in issues {
                self.log(format!("[blocked] {i}"), true);
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
        self.logs.clear();
        self.progress = 0.;
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
                self.log(if success { "Operation finished successfully." } else { "Operation failed." }, !success);
                return true;
            }
        }
        false
    }

    // ------------------------------------------------------------ view

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.task.is_some();
        let label = |t: &'static str| div().text_sm().text_color(cx.theme().muted_foreground).child(t);
        v_flex()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(label("Port"))
                    .child(Select::new(&self.port).w(px(200.)).small().placeholder("No port"))
                    .child(Button::new("refresh").small().ghost().label("↻").tooltip("Rescan ports").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.refresh_ports(window, cx)),
                    ))
                    .child(label("Baud"))
                    .child(Select::new(&self.baud).w(px(110.)).small())
                    .child(label("Chip"))
                    .child(Select::new(&self.chip).w(px(120.)).small())
                    .child(Button::new("detect").small().label("Detect chip").disabled(busy).on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.start(Op::ChipInfo, window, cx)),
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Button::new("open-dir").small().label("Open build folder…").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.prompt_open(true, window, cx)),
                    ))
                    .child(Button::new("open-file").small().label("Open file…").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.prompt_open(false, window, cx)),
                    ))
                    .when(self.projects.len() > 1, |el| {
                        el.child(label("Build")).child(Select::new(&self.build).w(px(360.)).small())
                    })
                    .when(self.projects.len() == 1, |el| {
                        el.child(label("Build")).child(div().text_sm().child(project_label(&self.projects[0])))
                    }),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(label("Mode"))
                    .child(Select::new(&self.mode).w(px(90.)).small())
                    .child(label("Freq"))
                    .child(Select::new(&self.freq).w(px(90.)).small())
                    .child(label("Size"))
                    .child(Select::new(&self.size).w(px(100.)).small())
                    .child(
                        Checkbox::new("erase-before").label("Erase before").checked(self.erase_before).on_click(
                            cx.listener(|this, v: &bool, _, cx| {
                                this.erase_before = *v;
                                cx.notify();
                            }),
                        ),
                    )
                    .when(self.merged.is_some(), |el| {
                        el.child(
                            Checkbox::new("use-merged")
                                .label("Flash merged image @0x0 instead")
                                .checked(self.use_merged)
                                .on_click(cx.listener(|this, v: &bool, _, cx| {
                                    this.use_merged = *v;
                                    cx.notify();
                                })),
                        )
                    }),
            )
    }

    fn render_files(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let dim = self.use_merged;
        let header = h_flex()
            .px_3()
            .py_1()
            .gap_3()
            .bg(theme.table_head)
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(div().w(px(24.)))
            .child(div().w(px(190.)).child("SEGMENT"))
            .child(div().w(px(110.)).child("ADDRESS"))
            .child(div().flex_1().child("FILE"))
            .child(div().w(px(150.)).child("SIZE / LIMIT"))
            .child(div().w(px(80.)));

        let rows = self.rows.iter().enumerate().map(|(i, r)| {
            let f = &r.file;
            let len = f.path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len());
            let over = matches!((len, f.max_size), (Some(l), Some(m)) if l > m as u64);
            let size_text = match (len, f.max_size) {
                (Some(l), Some(m)) => format!("{} / {}", human(l), human(m as u64)),
                (Some(l), None) => human(l),
                (None, Some(m)) => format!("— / {}", human(m as u64)),
                (None, None) => "—".into(),
            };
            let path_text: SharedString = match &f.path {
                Some(p) if len.is_some() => p.display().to_string().into(),
                Some(p) => format!("missing: {}", p.display()).into(),
                None => "Browse or drop a file…".into(),
            };
            h_flex()
                .id(("row", i))
                .px_3()
                .py_1()
                .gap_3()
                .items_center()
                .border_b_1()
                .border_color(theme.table_row_border)
                .when(i % 2 == 1, |el| el.bg(theme.table_even))
                .when(dim, |el| el.opacity(0.4))
                .child(
                    div().w(px(24.)).child(Checkbox::new(("en", i)).checked(f.enabled).on_click(cx.listener(
                        move |this, v: &bool, _, cx| {
                            this.rows[i].file.enabled = *v;
                            cx.notify();
                        },
                    ))),
                )
                .child(div().w(px(190.)).text_sm().truncate().child(f.label.clone()))
                .child(div().w(px(110.)).child(Input::new(&r.offset).small()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_xs()
                        .truncate()
                        .text_color(if f.path.is_none() {
                            theme.muted_foreground
                        } else if len.is_none() {
                            theme.danger
                        } else {
                            theme.foreground
                        })
                        .child(path_text),
                )
                .child(
                    div()
                        .w(px(150.))
                        .text_xs()
                        .text_color(if over { theme.danger } else { theme.muted_foreground })
                        .child(size_text),
                )
                .child(
                    h_flex()
                        .w(px(80.))
                        .gap_1()
                        .child(Button::new(("browse", i)).small().label("…").tooltip("Choose file").on_click(
                            cx.listener(move |this, _: &ClickEvent, _, cx| this.browse_row(i, cx)),
                        ))
                        .child(Button::new(("del", i)).small().ghost().label("✕").tooltip("Remove").on_click(
                            cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.rows.remove(i);
                                cx.notify();
                            }),
                        )),
                )
        });

        let issues = if self.rows.is_empty() {
            Vec::new()
        } else {
            match self.plan(cx) {
                Ok(f) => detect::validate(&f),
                Err(e) => vec![e],
            }
        };

        v_flex()
            .child(header)
            .children(rows)
            .child(
                h_flex().px_3().py_1().child(Button::new("add").small().ghost().label("+ Add segment").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.add_row(window, cx)),
                )),
            )
            .children(issues.into_iter().map(|i| {
                div().px_3().text_xs().text_color(theme.warning).child(format!("⚠ {i}"))
            }))
            .when_some(self.partitions.as_ref(), |el, parts| el.child(render_partitions(parts, &theme)))
    }

    fn render_actions(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.task.is_some();
        let theme = cx.theme();
        let status = match self.status {
            None => "Idle",
            Some(Status::Connecting) => "Connecting…",
            Some(Status::Erasing) => "Erasing…",
            Some(Status::Writing) => "Writing…",
            Some(Status::Reading) => "Reading…",
            Some(Status::Done) => "Done",
            Some(Status::Error) => "Error",
        };
        let secs = self.elapsed.as_secs();
        h_flex()
            .gap_3()
            .p_3()
            .items_center()
            .border_t_1()
            .border_color(theme.border)
            .when(!busy, |el| {
                el.child(Button::new("program").primary().label("Program").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.program(window, cx)),
                ))
                .child(Button::new("erase").danger().outline().label("Erase chip").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.erase(window, cx)),
                ))
            })
            .when(busy, |el| {
                el.child(Button::new("cancel").danger().label("Cancel").on_click(cx.listener(
                    |this, _: &ClickEvent, _, _| {
                        if let Some(t) = &this.task {
                            t.cancel();
                        }
                    },
                )))
            })
            .child(div().flex_1().child(Progress::new("progress").value(self.progress)))
            .child(div().w(px(40.)).text_sm().child(format!("{:.0}%", self.progress)))
            .child(div().w(px(110.)).text_sm().text_color(match self.status {
                Some(Status::Done) => theme.success,
                Some(Status::Error) => theme.danger,
                _ => theme.foreground,
            }).child(status))
            .child(div().text_sm().text_color(theme.muted_foreground).child(format!(
                "{:02}:{:02}",
                secs / 60,
                secs % 60
            )))
    }

    fn render_log(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id("log")
            .h(px(220.))
            .flex_shrink_0()
            .overflow_y_scroll()
            .track_scroll(&self.log_scroll)
            .p_2()
            .bg(theme.sidebar)
            .border_t_1()
            .border_color(theme.border)
            .font_family("monospace")
            .text_xs()
            .children(self.logs.iter().map(|(t, err)| {
                div().text_color(if *err { theme.danger } else { theme.foreground }).child(t.clone())
            }))
    }
}

fn render_partitions(parts: &[Partition], theme: &Theme) -> impl IntoElement {
    v_flex()
        .mt_3()
        .px_3()
        .gap_1()
        .text_xs()
        .child(div().text_color(theme.muted_foreground).child("PARTITION TABLE"))
        .children(parts.iter().map(|p| {
            h_flex()
                .gap_3()
                .font_family("monospace")
                .child(div().w(px(110.)).child(p.name.clone()))
                .child(div().w(px(50.)).child(p.type_name()))
                .child(div().w(px(80.)).child(p.subtype_name()))
                .child(div().w(px(90.)).child(format!("{:#x}", p.offset)))
                .child(div().w(px(80.)).child(human(p.size as u64)))
        }))
}

fn project_label(p: &Project) -> SharedString {
    let src = match p.source {
        Source::IdfFlasherArgs => "ESP-IDF",
        Source::FlashArgs => "flash_args",
        Source::PartitionTable => "partition table",
        Source::SingleBin => "single image",
    };
    let parent = p.origin.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().into_owned());
    format!(
        "{} · {} · {}",
        p.chip.as_deref().unwrap_or("chip?"),
        src,
        parent.unwrap_or_default()
    )
    .into()
}

impl Render for FlashApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                if let Some(p) = paths.paths().first() {
                    this.open_path(p.clone(), window, cx);
                }
            }))
            .child(self.render_toolbar(cx))
            .child(div().id("files").flex_1().min_h_0().overflow_y_scroll().child(self.render_files(cx)))
            .child(self.render_actions(cx))
            .child(self.render_log(cx))
    }
}
