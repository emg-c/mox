use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use mox_format::{Button, Event, EventKind, Key, KeyCombo, Mods, Pos};

use crate::input::{InputBackend, InputError, RawEvent, RawKind};

pub struct RecordOptions {
    /// 직전에 기록한 이동으로부터 이 시간이 지나야 다음 이동을 기록한다.
    pub move_sample_ms: u64,
    pub stop_key: KeyCombo,
}

impl RecordOptions {
    pub fn new(stop_key: KeyCombo) -> Self {
        Self {
            move_sample_ms: 10,
            stop_key,
        }
    }
}

/// 원시 입력을 받아 `.mox` 이벤트로 정리한다. OS와 무관한 순수 로직.
pub struct Recorder {
    opts: RecordOptions,
    events: Vec<Event>,
    last_emit: Option<Duration>,
    last_move: Option<(Duration, i32, i32)>,
    pending_move: Option<(Duration, i32, i32)>,
    phys_mods: Mods,
    down_keys: BTreeSet<Key>,
    down_buttons: BTreeSet<Button>,
    stopped: bool,
}

impl Recorder {
    pub fn new(opts: RecordOptions) -> Self {
        Self {
            opts,
            events: Vec::new(),
            last_emit: None,
            last_move: None,
            pending_move: None,
            phys_mods: Mods::default(),
            down_keys: BTreeSet::new(),
            down_buttons: BTreeSet::new(),
            stopped: false,
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// 원시 입력 하나를 처리한다. 중지 키가 눌렸으면(이미 멈췄으면) true.
    pub fn push(&mut self, ev: RawEvent) -> bool {
        if self.stopped || ev.injected {
            return self.stopped;
        }
        let at = ev.at;
        match ev.kind {
            RawKind::MouseMove { x, y } => self.on_move(at, x, y),
            RawKind::MouseDown { button, x, y } => {
                self.down_buttons.insert(button);
                self.emit(
                    at,
                    EventKind::Down {
                        button,
                        pos: Pos::Abs { x, y },
                    },
                );
            }
            RawKind::MouseUp { button, x, y } => {
                // 녹화 전에 눌린 버튼(GUI 녹화 버튼 등)의 뗌은 버린다
                if self.down_buttons.remove(&button) {
                    self.emit(
                        at,
                        EventKind::Up {
                            button,
                            pos: Pos::Abs { x, y },
                        },
                    );
                }
            }
            RawKind::Wheel { dx, dy, x, y } => {
                self.emit(
                    at,
                    EventKind::Wheel {
                        dx,
                        dy,
                        pos: Pos::Abs { x, y },
                    },
                );
            }
            RawKind::KeyDown(key) => {
                if let Key::Code(code) = key {
                    if self.opts.stop_key.matches(self.phys_mods, code) {
                        self.stopped = true;
                        self.trim_stop_modifiers();
                        return true;
                    }
                    if let Some(m) = code.modifier() {
                        self.phys_mods.set(m, true);
                    }
                }
                self.down_keys.insert(key);
                self.emit(at, EventKind::KeyDown(key));
            }
            RawKind::KeyUp(key) => {
                if let Key::Code(code) = key {
                    if let Some(m) = code.modifier() {
                        self.phys_mods.set(m, false);
                    }
                }
                // 녹화 전에 눌린 키(시작 단축키 등)의 뗌은 버린다
                if self.down_keys.remove(&key) {
                    self.emit(at, EventKind::KeyUp(key));
                }
            }
        }
        false
    }

    pub fn finish(mut self) -> Vec<Event> {
        self.flush_pending();
        self.events
    }

    fn on_move(&mut self, at: Duration, x: i32, y: i32) {
        let latest = self
            .pending_move
            .or(self.last_move)
            .map(|(_, px, py)| (px, py));
        if latest == Some((x, y)) {
            return;
        }
        let sample = Duration::from_millis(self.opts.move_sample_ms);
        match self.last_move {
            Some((t, _, _)) if at.saturating_sub(t) < sample => {
                self.pending_move = Some((at, x, y))
            }
            _ => {
                self.pending_move = None;
                self.push_event(
                    at,
                    EventKind::Move {
                        pos: Pos::Abs { x, y },
                    },
                );
                self.last_move = Some((at, x, y));
            }
        }
    }

    fn flush_pending(&mut self) {
        if let Some((t, x, y)) = self.pending_move.take() {
            self.push_event(
                t,
                EventKind::Move {
                    pos: Pos::Abs { x, y },
                },
            );
            self.last_move = Some((t, x, y));
        }
    }

    fn emit(&mut self, at: Duration, kind: EventKind) {
        self.flush_pending();
        self.push_event(at, kind);
    }

    fn push_event(&mut self, at: Duration, kind: EventKind) {
        let d = match self.last_emit {
            None => 0,
            Some(prev) => at.saturating_sub(prev).as_millis() as u64,
        };
        self.last_emit = Some(at);
        self.events.push(Event::new(d, kind));
    }

    /// 중지 조합을 누르느라 기록된 보조키 누름을 끝에서 지운다.
    fn trim_stop_modifiers(&mut self) {
        let combo = self.opts.stop_key.mods;
        while let Some(last) = self.events.last() {
            match last.kind {
                EventKind::KeyDown(Key::Code(c)) if c.modifier().is_some_and(|m| combo.has(m)) => {
                    self.down_keys.remove(&Key::Code(c));
                    self.events.pop();
                }
                _ => break,
            }
        }
    }
}

/// 중지 키나 cancel 플래그가 올 때까지 녹화한다. 수집이 끊기면 그때까지의 이벤트를 돌려준다.
pub fn record(
    backend: &dyn InputBackend,
    opts: RecordOptions,
    cancel: &AtomicBool,
) -> Result<Vec<Event>, InputError> {
    let (tx, rx) = mpsc::channel();
    let _guard = backend.start_capture(tx)?;
    let mut rec = Recorder::new(opts);
    while !cancel.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(ev) => {
                if rec.push(ev) {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(rec.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeBackend;
    use mox_format::{Button, Event, EventKind, Key, KeyCode, KeyCombo, Os, Pos, ScreenInfo, Unit};
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    fn raw(ms: u64, kind: RawKind) -> RawEvent {
        RawEvent {
            at: Duration::from_millis(ms),
            kind,
            injected: false,
        }
    }
    fn mv(ms: u64, x: i32, y: i32) -> RawEvent {
        raw(ms, RawKind::MouseMove { x, y })
    }
    fn kd(ms: u64, k: KeyCode) -> RawEvent {
        raw(ms, RawKind::KeyDown(Key::Code(k)))
    }
    fn ku(ms: u64, k: KeyCode) -> RawEvent {
        raw(ms, RawKind::KeyUp(Key::Code(k)))
    }
    fn ev_move(d: u64, x: i32, y: i32) -> Event {
        Event::new(
            d,
            EventKind::Move {
                pos: Pos::Abs { x, y },
            },
        )
    }
    fn ev_kd(d: u64, k: KeyCode) -> Event {
        Event::new(d, EventKind::KeyDown(Key::Code(k)))
    }
    fn ev_ku(d: u64, k: KeyCode) -> Event {
        Event::new(d, EventKind::KeyUp(Key::Code(k)))
    }
    fn pause() -> RecordOptions {
        RecordOptions::new(KeyCombo::single(KeyCode::Pause))
    }
    fn run(opts: RecordOptions, raws: Vec<RawEvent>) -> Vec<Event> {
        let mut r = Recorder::new(opts);
        for e in raws {
            r.push(e);
        }
        r.finish()
    }

    #[test]
    fn delays_are_gaps_between_recorded_events() {
        let evs = run(
            pause(),
            vec![
                kd(0, KeyCode::KeyA),
                ku(100, KeyCode::KeyA),
                kd(250, KeyCode::KeyB),
            ],
        );
        assert_eq!(
            evs,
            vec![
                ev_kd(0, KeyCode::KeyA),
                ev_ku(100, KeyCode::KeyA),
                ev_kd(150, KeyCode::KeyB)
            ]
        );
    }

    #[test]
    fn samples_moves_but_keeps_last_position_before_other_events() {
        let evs = run(
            pause(),
            vec![
                mv(0, 0, 0),
                mv(3, 1, 0),
                mv(6, 2, 0),
                mv(12, 3, 0),
                mv(14, 4, 0),
                kd(20, KeyCode::KeyA),
            ],
        );
        // 3,6ms는 10ms 안이라 보류됐다가 12ms 이동이 기록되며 버려지고,
        // 14ms 이동은 보류 중이다가 키 입력 직전에 기록된다.
        assert_eq!(
            evs,
            vec![
                ev_move(0, 0, 0),
                ev_move(12, 3, 0),
                ev_move(2, 4, 0),
                ev_kd(6, KeyCode::KeyA)
            ]
        );
    }

    #[test]
    fn pending_move_is_flushed_at_finish() {
        let evs = run(pause(), vec![mv(0, 0, 0), mv(5, 9, 9)]);
        assert_eq!(evs, vec![ev_move(0, 0, 0), ev_move(5, 9, 9)]);
    }

    #[test]
    fn drops_same_position_moves_and_injected_events() {
        let mut injected = kd(5, KeyCode::KeyZ);
        injected.injected = true;
        let evs = run(
            pause(),
            vec![mv(0, 5, 5), mv(20, 5, 5), injected, mv(40, 6, 6)],
        );
        assert_eq!(evs, vec![ev_move(0, 5, 5), ev_move(40, 6, 6)]);
    }

    #[test]
    fn drops_releases_of_inputs_pressed_before_recording() {
        // 시작 단축키(Ctrl+Alt+R)의 뗌과 GUI 녹화 버튼의 마우스 뗌
        let evs = run(
            pause(),
            vec![
                ku(0, KeyCode::KeyR),
                ku(1, KeyCode::AltLeft),
                ku(2, KeyCode::ControlLeft),
                raw(
                    3,
                    RawKind::MouseUp {
                        button: Button::Left,
                        x: 1,
                        y: 1,
                    },
                ),
                kd(10, KeyCode::KeyA),
            ],
        );
        assert_eq!(evs, vec![ev_kd(0, KeyCode::KeyA)]);
    }

    #[test]
    fn stop_combo_ends_recording_and_is_trimmed() {
        let stop: KeyCombo = "Control+Alt+Escape".parse().unwrap();
        let mut r = Recorder::new(RecordOptions::new(stop));
        assert!(!r.push(kd(0, KeyCode::KeyA)));
        assert!(!r.push(ku(10, KeyCode::KeyA)));
        assert!(!r.push(kd(20, KeyCode::ControlLeft)));
        assert!(!r.push(kd(30, KeyCode::AltLeft)));
        assert!(r.push(kd(40, KeyCode::Escape)));
        assert!(r.is_stopped());
        assert!(r.push(kd(50, KeyCode::KeyB))); // 중지 뒤 입력은 무시
        assert_eq!(
            r.finish(),
            vec![ev_kd(0, KeyCode::KeyA), ev_ku(10, KeyCode::KeyA)]
        );
    }

    #[test]
    fn stop_key_with_extra_modifier_does_not_stop() {
        let mut r = Recorder::new(pause());
        r.push(kd(0, KeyCode::ShiftLeft));
        assert!(!r.push(kd(5, KeyCode::Pause)));
    }

    #[test]
    fn record_returns_events_when_stop_key_arrives() {
        let screen = ScreenInfo {
            width: 100,
            height: 100,
            scale: 1.0,
            unit: Unit::Pixel,
        };
        let backend = FakeBackend::new(Os::Windows, screen);
        backend.on_capture(vec![
            kd(0, KeyCode::KeyA),
            ku(5, KeyCode::KeyA),
            kd(9, KeyCode::Pause),
        ]);
        let evs = record(&backend, pause(), &AtomicBool::new(false)).unwrap();
        assert_eq!(evs, vec![ev_kd(0, KeyCode::KeyA), ev_ku(5, KeyCode::KeyA)]);
    }

    #[test]
    fn record_stops_on_cancel_flag() {
        let screen = ScreenInfo {
            width: 100,
            height: 100,
            scale: 1.0,
            unit: Unit::Pixel,
        };
        let backend = FakeBackend::new(Os::Windows, screen);
        backend.on_capture(vec![kd(0, KeyCode::KeyA)]);
        let evs = record(&backend, pause(), &AtomicBool::new(true)).unwrap();
        assert!(evs.is_empty());
    }
}
