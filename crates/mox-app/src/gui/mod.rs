//! egui GUI. 녹화·재생·편집·추출을 창에서 다룬다. 모든 로직은 기존 크레이트 글루.
//! 녹화·재생은 블로킹이므로 백그라운드 스레드에서 돌리고 채널로 결과를 받는다.
#![allow(deprecated)] // egui 0.33의 menu::bar/close_menu는 deprecated지만 동작한다

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;

use mox_edit::{
    cap_delays, remove_key_repeats, remove_mouse, remove_moves, scale_delays, thin_moves_half,
    to_abs, to_ratio, History, Sel,
};
use std::time::Duration;

use mox_engine::{effective_stop_key, play, PlayOptions, RawEvent, RecordOptions, Recorder};
use mox_format::{
    load, save, Button, Event, EventKind, Key, Macro, Meta, Os, Pos, ScreenInfo, Unit,
};

fn current_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::Macos
    } else {
        Os::Windows
    }
}

fn empty_macro() -> Macro {
    Macro {
        meta: Meta {
            name: "새 매크로".into(),
            created: crate::timeutil::now_iso(),
            recorded_on: current_os(),
            screen: ScreenInfo {
                width: 1920,
                height: 1080,
                scale: 1.0,
                unit: Unit::Pixel,
            },
        },
        play: Default::default(),
        events: Vec::new(),
    }
}

/// OS 한글 폰트를 런타임에 불러와 egui에 설치한다. 없으면 기본 폰트(한글은 깨짐).
fn install_korean_font(ctx: &egui::Context) {
    let candidates = [
        "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
        "C:\\Windows\\Fonts\\malgun.ttf",
    ];
    for p in candidates {
        if let Ok(bytes) = std::fs::read(p) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("ko".to_owned(), egui::FontData::from_owned(bytes).into());
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(fam).or_default().push("ko".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// 녹화 스레드가 GUI로 보내는 메시지. 이벤트를 실시간으로 흘려보내고 끝에 완료를 알린다.
enum RecMsg {
    Events(Vec<Event>),
    Done(Result<ScreenInfo, String>),
}

enum Mode {
    Idle,
    Recording {
        cancel: Arc<AtomicBool>,
        rx: Receiver<RecMsg>,
    },
    Playing {
        stop: Arc<AtomicBool>,
        rx: Receiver<Result<String, String>>,
    },
}

struct MoxApp {
    doc: Macro,
    path: Option<PathBuf>,
    history: History,
    selection: BTreeSet<usize>,
    apply_to_selection: bool,
    mode: Mode,
    status: String,
}

impl Default for MoxApp {
    fn default() -> Self {
        Self {
            doc: empty_macro(),
            path: None,
            history: History::new(History::DEFAULT_CAP),
            selection: BTreeSet::new(),
            apply_to_selection: false,
            mode: Mode::Idle,
            status: "준비됨".into(),
        }
    }
}

/// 강조색과 종류별 색. 표·버튼에서 쓴다.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x4d, 0x9b, 0xff);
const STRIPE: egui::Color32 = egui::Color32::from_rgb(0x2a, 0x2d, 0x33);
const C_MOVE: egui::Color32 = egui::Color32::from_rgb(0x9a, 0xa0, 0xa6);
const C_CLICK: egui::Color32 = egui::Color32::from_rgb(0x6c, 0xb6, 0xff);
const C_KEY: egui::Color32 = egui::Color32::from_rgb(0x8c, 0xd9, 0x8c);
const C_WHEEL: egui::Color32 = egui::Color32::from_rgb(0xe0, 0xb0, 0x6a);

/// 다크 테마 + 여백·폰트 등 쉬운 스타일만 적용한다.
fn setup_style(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.style()).clone();
    style.visuals.selection.bg_fill = ACCENT.linear_multiply(0.45);
    style.visuals.hyperlink_color = ACCENT;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(18.0, FontFamily::Proportional),
    );
    ctx.set_style(style);
}

pub fn run() -> ExitCode {
    let opts = eframe::NativeOptions::default();
    let r = eframe::run_native(
        "Mox",
        opts,
        Box::new(|cc| {
            install_korean_font(&cc.egui_ctx);
            setup_style(&cc.egui_ctx);
            Ok(Box::new(MoxApp::default()))
        }),
    );
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("GUI 오류: {e}");
            ExitCode::from(1)
        }
    }
}

fn type_color(e: &Event) -> egui::Color32 {
    match e.kind {
        EventKind::Move { .. } => C_MOVE,
        EventKind::Down { .. } | EventKind::Up { .. } => C_CLICK,
        EventKind::Wheel { .. } => C_WHEEL,
        EventKind::KeyDown(_) | EventKind::KeyUp(_) => C_KEY,
    }
}

fn describe(e: &Event) -> (String, String) {
    let pos = |p: Pos| match p {
        Pos::Abs { x, y } => format!("{x}, {y}"),
        Pos::Ratio { rx, ry } => format!("{rx:.3}, {ry:.3} (비율)"),
    };
    let btn = |b: Button| match b {
        Button::Left => "좌",
        Button::Right => "우",
        Button::Middle => "가운데",
        Button::Back => "뒤로",
        Button::Forward => "앞으로",
    };
    let key = |k: Key| match k {
        Key::Code(c) => c.name().to_string(),
        Key::Unknown { raw, .. } => format!("?{raw}"),
    };
    match e.kind {
        EventKind::Move { pos: p } => ("이동".into(), pos(p)),
        EventKind::Down { button, pos: p } => (format!("{} 누름", btn(button)), pos(p)),
        EventKind::Up { button, pos: p } => (format!("{} 뗌", btn(button)), pos(p)),
        EventKind::Wheel { dx, dy, pos: p } => ("휠".into(), format!("{dx},{dy} @ {}", pos(p))),
        EventKind::KeyDown(k) => ("키 누름".into(), key(k)),
        EventKind::KeyUp(k) => ("키 뗌".into(), key(k)),
    }
}

impl MoxApp {
    /// 편집을 실행하고 이력에 남긴다. 클로저는 (이벤트 목록, 적용 범위)를 받는다.
    /// 범위는 self.selection(불변)에서 만들고, 이력은 self.history/self.doc.events(가변)만 건드려
    /// 서로 다른 필드라 동시에 빌릴 수 있다.
    fn do_edit(&mut self, label: &str, f: impl FnOnce(&mut Vec<Event>, Sel) -> usize) {
        let sel = if self.apply_to_selection && !self.selection.is_empty() {
            Sel::Rows(&self.selection)
        } else {
            Sel::All
        };
        let n = self.history.edit(&mut self.doc.events, |e| f(e, sel));
        self.status = format!("{label}: {n}개 적용");
    }

    fn open_file(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Mox", &["mox"])
            .pick_file()
        {
            match load(&p) {
                Ok(m) => {
                    self.doc = m;
                    self.selection.clear();
                    self.history = History::new(History::DEFAULT_CAP);
                    self.status = format!("열기: {}", p.display());
                    self.path = Some(p);
                }
                Err(e) => self.status = format!("열기 실패: {e}"),
            }
        }
    }

    fn save_file(&mut self, as_new: bool) {
        let dest = if as_new || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("Mox", &["mox"])
                .set_file_name("매크로.mox")
                .save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = dest {
            match save(&p, &self.doc) {
                Ok(()) => {
                    self.status = format!("저장: {}", p.display());
                    self.path = Some(p);
                }
                Err(e) => self.status = format!("저장 실패: {e}"),
            }
        }
    }

    fn export(&mut self, target: &str) {
        let ext = if target == "windows" { "exe" } else { "zip" };
        let Some(out) = rfd::FileDialog::new()
            .set_file_name(format!("매크로.{ext}"))
            .save_file()
        else {
            return;
        };
        let payload = mox_payload::Payload {
            settings: mox_payload::ExportSettings::default(),
            mox: self.doc.clone(),
        };
        let result = match target {
            "windows" => match crate::stubs::win() {
                Some(s) => mox_export::export_windows(s, &payload).map_err(|e| e.to_string()),
                None => Err("이 빌드에 Windows 재생기가 없습니다".into()),
            },
            _ => match crate::stubs::mac() {
                Some(s) => tempfile::tempdir()
                    .map_err(|e| e.to_string())
                    .and_then(|w| {
                        mox_export::export_macos_zip(s, &payload, w.path())
                            .map_err(|e| e.to_string())
                    }),
                None => Err("이 빌드에 macOS 재생기가 없습니다".into()),
            },
        };
        self.status = match result.and_then(|b| std::fs::write(&out, b).map_err(|e| e.to_string()))
        {
            Ok(()) => format!("추출: {}", out.display()),
            Err(e) => format!("추출 실패: {e}"),
        };
    }

    fn start_record(&mut self) {
        let os = current_os();
        if !mox_input::permission_status().capture {
            mox_input::open_settings(mox_input::Access::Capture);
            self.status = "입력 모니터링 권한이 필요합니다".into();
            return;
        }
        // 새 녹화: 표를 비우고 시작한다(이벤트가 실시간으로 쌓인다).
        self.doc.events.clear();
        self.doc.meta.recorded_on = os;
        self.doc.meta.created = crate::timeutil::now_iso();
        self.selection.clear();
        self.history = History::new(History::DEFAULT_CAP);

        let (tx, rx) = std::sync::mpsc::channel::<RecMsg>();
        let cancel = Arc::new(AtomicBool::new(false));
        let c2 = cancel.clone();
        let stop_key = effective_stop_key(&self.doc.play, os).0;
        std::thread::spawn(move || {
            let backend = match mox_input::backend() {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(RecMsg::Done(Err(e.to_string())));
                    return;
                }
            };
            let screen = backend.primary_screen();
            let (raw_tx, raw_rx) = std::sync::mpsc::channel::<RawEvent>();
            let _guard = match backend.start_capture(raw_tx) {
                Ok(g) => g,
                Err(e) => {
                    let _ = tx.send(RecMsg::Done(Err(e.to_string())));
                    return;
                }
            };
            let mut rec = Recorder::new(RecordOptions::new(stop_key));
            let mut sent = 0usize;
            loop {
                if c2.load(Ordering::Relaxed) {
                    break;
                }
                match raw_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(ev) => {
                        let stopped = rec.push(ev);
                        let evs = rec.events();
                        if evs.len() > sent {
                            let _ = tx.send(RecMsg::Events(evs[sent..].to_vec()));
                            sent = evs.len();
                        }
                        if stopped {
                            break;
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            let final_events = rec.finish();
            if final_events.len() > sent {
                let _ = tx.send(RecMsg::Events(final_events[sent..].to_vec()));
            }
            let _ = tx.send(RecMsg::Done(Ok(screen)));
        });
        self.mode = Mode::Recording { cancel, rx };
        self.status = format!("녹화 중… 중지: {stop_key} 또는 정지 버튼");
    }

    fn start_play(&mut self) {
        let os = current_os();
        let perm = mox_input::permission_status();
        if !perm.inject || !perm.capture {
            mox_input::open_settings(if !perm.inject {
                mox_input::Access::Inject
            } else {
                mox_input::Access::Capture
            });
            self.status = "재생 권한(손쉬운 사용·입력 모니터링)이 필요합니다".into();
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = stop.clone();
        let doc = self.doc.clone();
        std::thread::spawn(move || {
            let msg = match mox_input::backend() {
                Ok(b) => {
                    let clock = mox_input::clock();
                    let (stop_key, _) = effective_stop_key(&doc.play, os);
                    let opts = PlayOptions {
                        stop_key,
                        failsafe: true,
                    };
                    play(&doc, &*b, &*clock, &s2, &opts)
                        .map(|r| {
                            format!(
                                "재생 {:?}: 주입 {}개, 건너뜀 {}개",
                                r.outcome, r.injected, r.skipped_keys
                            )
                        })
                        .map_err(|e| e.to_string())
                }
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(msg);
        });
        self.mode = Mode::Playing { stop, rx };
        self.status = "재생 중…".into();
    }

    fn poll_mode(&mut self) {
        match &self.mode {
            Mode::Recording { rx, .. } => {
                // 이번 프레임에 도착한 메시지를 모두 처리한다. 이벤트는 실시간으로 표에 쌓는다.
                let mut done: Option<Result<ScreenInfo, String>> = None;
                loop {
                    match rx.try_recv() {
                        Ok(RecMsg::Events(evs)) => self.doc.events.extend(evs),
                        Ok(RecMsg::Done(r)) => {
                            done = Some(r);
                            break;
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            done = Some(Err("녹화 스레드가 중단되었습니다".into()));
                            break;
                        }
                    }
                }
                if let Some(r) = done {
                    match r {
                        Ok(screen) => {
                            self.doc.meta.screen = screen;
                            self.status = format!("녹화 완료: {}개 이벤트", self.doc.events.len());
                        }
                        Err(e) => self.status = format!("녹화 오류: {e}"),
                    }
                    self.mode = Mode::Idle;
                }
            }
            Mode::Playing { rx, .. } => match rx.try_recv() {
                Ok(msg) => {
                    self.status = msg.unwrap_or_else(|e| format!("재생 오류: {e}"));
                    self.mode = Mode::Idle;
                }
                Err(TryRecvError::Disconnected) => self.mode = Mode::Idle,
                Err(TryRecvError::Empty) => {}
            },
            Mode::Idle => {}
        }
    }

    fn stop_active(&self) {
        match &self.mode {
            Mode::Recording { cancel, .. } => cancel.store(true, Ordering::Relaxed),
            Mode::Playing { stop, .. } => stop.store(true, Ordering::Relaxed),
            Mode::Idle => {}
        }
    }
}

impl eframe::App for MoxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_mode();
        let busy = !matches!(self.mode, Mode::Idle);
        if busy {
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("파일", |ui| {
                    if ui.button("새로 만들기").clicked() {
                        self.doc = empty_macro();
                        self.path = None;
                        self.selection.clear();
                        self.history = History::new(History::DEFAULT_CAP);
                        ui.close_menu();
                    }
                    if ui.button("열기…").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui.button("저장").clicked() {
                        self.save_file(false);
                        ui.close_menu();
                    }
                    if ui.button("다른 이름으로 저장…").clicked() {
                        self.save_file(true);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Windows exe로 추출…").clicked() {
                        self.export("windows");
                        ui.close_menu();
                    }
                    if ui.button("macOS 앱으로 추출…").clicked() {
                        self.export("macos");
                        ui.close_menu();
                    }
                });
                ui.menu_button("편집", |ui| {
                    ui.checkbox(&mut self.apply_to_selection, "선택한 행에만 적용");
                    ui.separator();
                    if ui.button("되돌리기").clicked() {
                        self.history.undo(&mut self.doc.events);
                        ui.close_menu();
                    }
                    if ui.button("다시 하기").clicked() {
                        self.history.redo(&mut self.doc.events);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("딜레이 ×0.5").clicked() {
                        self.do_edit("딜레이 ×0.5", |e, s| {
                            scale_delays(e, s, 0.5).unwrap_or(0)
                        });
                        ui.close_menu();
                    }
                    if ui.button("딜레이 상한 500ms").clicked() {
                        self.do_edit("딜레이 상한", |e, s| cap_delays(e, s, 500));
                        ui.close_menu();
                    }
                    if ui.button("이동 절반으로 솎기").clicked() {
                        self.do_edit("이동 솎기", |e, _| thin_moves_half(e));
                        ui.close_menu();
                    }
                    if ui.button("이동 삭제(클릭 유지)").clicked() {
                        self.do_edit("이동 삭제", |e, _| remove_moves(e));
                        ui.close_menu();
                    }
                    if ui.button("마우스 이벤트 전부 삭제").clicked() {
                        self.do_edit("마우스 삭제", |e, _| remove_mouse(e));
                        ui.close_menu();
                    }
                    if ui.button("키 자동반복 제거").clicked() {
                        self.do_edit("자동반복 제거", |e, _| remove_key_repeats(e));
                        ui.close_menu();
                    }
                    ui.separator();
                    let screen = self.doc.meta.screen;
                    if ui.button("절대 → 비율 좌표").clicked() {
                        self.do_edit("비율 변환", |e, s| to_ratio(e, s, &screen).unwrap_or(0));
                        ui.close_menu();
                    }
                    if ui.button("비율 → 절대 좌표").clicked() {
                        self.do_edit("절대 변환", |e, s| to_abs(e, s, &screen).unwrap_or(0));
                        ui.close_menu();
                    }
                });
            });
        });

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!busy, |ui| {
                    if ui.button("● 녹화").clicked() {
                        self.start_record();
                    }
                    if ui.button("▶ 재생").clicked() {
                        self.start_play();
                    }
                });
                ui.add_enabled_ui(busy, |ui| {
                    if ui.button("■ 정지").clicked() {
                        self.stop_active();
                    }
                });
                ui.separator();
                ui.label("반복");
                ui.add(egui::DragValue::new(&mut self.doc.play.repeat).range(0..=100000));
                ui.label("배속");
                ui.add(
                    egui::DragValue::new(&mut self.doc.play.speed)
                        .speed(0.1)
                        .range(0.1..=10.0),
                );
                ui.checkbox(&mut self.doc.play.timed, "타이밍 유지");
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{}개 이벤트", self.doc.events.len()));
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("#");
                ui.add_space(30.0);
                ui.strong("간격(ms)");
                ui.add_space(20.0);
                ui.strong("누적(ms)");
                ui.add_space(20.0);
                ui.strong("종류");
                ui.add_space(40.0);
                ui.strong("값");
            });
            ui.separator();
            let row_h = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
            let total = self.doc.events.len();
            let mut cumulative: Vec<u64> = Vec::with_capacity(total);
            let mut acc = 0u64;
            for e in &self.doc.events {
                acc = acc.saturating_add(e.d);
                cumulative.push(acc);
            }
            let recording = matches!(self.mode, Mode::Recording { .. });
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(recording)
                .show_rows(ui, row_h, total, |ui, range| {
                    for i in range {
                        let selected = self.selection.contains(&i);
                        let (ty, val) = describe(&self.doc.events[i]);
                        let color = type_color(&self.doc.events[i]);
                        // 줄무늬·선택 배경
                        let bg = if selected {
                            ACCENT.linear_multiply(0.30)
                        } else if i % 2 == 1 {
                            STRIPE
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        if bg != egui::Color32::TRANSPARENT {
                            let rect = egui::Rect::from_min_size(
                                ui.cursor().min,
                                egui::vec2(ui.available_width(), row_h),
                            );
                            ui.painter().rect_filled(rect, 0.0, bg);
                        }
                        ui.horizontal(|ui| {
                            let label = format!("{i:>4}");
                            if ui.selectable_label(selected, label).clicked() {
                                if selected {
                                    self.selection.remove(&i);
                                } else {
                                    self.selection.insert(i);
                                }
                            }
                            ui.add(
                                egui::DragValue::new(&mut self.doc.events[i].d).range(0..=u64::MAX),
                            );
                            ui.add_space(10.0);
                            ui.monospace(format!("{:>8}", cumulative[i]));
                            ui.add_space(10.0);
                            ui.colored_label(color, ty);
                            ui.add_space(10.0);
                            ui.label(val);
                        });
                    }
                });
        });
    }
}
