use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use mox_format::{
    Button, Event, EventKind, Key, KeyCode, KeyCombo, Macro, Mods, Os, Pos, ScreenInfo,
};

use crate::clock::Clock;
use crate::input::{InjectEvent, InputBackend, InputError, RawEvent, RawKind};

const DOUBLE_CLICK_MS: f64 = 500.0;
const DOUBLE_CLICK_DIST: i64 = 4;
/// 긴 대기 중에도 중지 요청을 이 간격으로 확인한다.
const WAIT_CHUNK: Duration = Duration::from_millis(50);

pub struct PlayOptions {
    pub stop_key: KeyCombo,
    /// 실제 마우스를 주 모니터 모서리에 밀어 붙이면 중지.
    pub failsafe: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayOutcome {
    Finished,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayReport {
    pub outcome: PlayOutcome,
    pub injected: usize,
    /// 다른 OS에서 녹화돼 옮길 수 없었던 키 이벤트 수.
    pub skipped_keys: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error("입력 수집을 시작하지 못했습니다: {0}")]
    Capture(#[source] InputError),
    #[error("{index}번째 이벤트를 주입하지 못했습니다: {source}")]
    Inject { index: usize, source: InputError },
}

/// 재생 중 들어오는 실제 입력을 보며 중지 여부를 판단한다.
struct Watch<'a> {
    rx: Receiver<RawEvent>,
    mods: Mods,
    stop_key: KeyCombo,
    failsafe: bool,
    screen: ScreenInfo,
    stop: &'a AtomicBool,
}

impl Watch<'_> {
    fn poll(&mut self) -> bool {
        while let Ok(raw) = self.rx.try_recv() {
            self.handle(raw);
        }
        self.stop.load(Ordering::Relaxed)
    }

    fn handle(&mut self, raw: RawEvent) {
        if raw.injected {
            return;
        }
        match raw.kind {
            RawKind::KeyDown(Key::Code(c)) => {
                if self.stop_key.matches(self.mods, c) {
                    self.stop.store(true, Ordering::Relaxed);
                }
                if let Some(m) = c.modifier() {
                    self.mods.set(m, true);
                }
            }
            RawKind::KeyUp(Key::Code(c)) => {
                if let Some(m) = c.modifier() {
                    self.mods.set(m, false);
                }
            }
            RawKind::MouseMove { x, y } if self.failsafe && at_corner(&self.screen, x, y) => {
                self.stop.store(true, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

/// 주 모니터의 네 모서리(가장자리 2px) 중 하나인가.
fn at_corner(s: &ScreenInfo, x: i32, y: i32) -> bool {
    let (w, h) = (s.width as i64, s.height as i64);
    let (x, y) = (i64::from(x), i64::from(y));
    let edge = |v: i64, max: i64| (0..=1).contains(&v) || (max - 2..=max - 1).contains(&v);
    edge(x, w) && edge(y, h)
}

#[derive(Default)]
struct State {
    buttons: BTreeSet<Button>,
    keys: BTreeSet<KeyCode>,
    raw_keys: BTreeSet<u32>,
    last_pos: (i32, i32),
    /// (버튼, 누른 시각 ms, 위치, 연속 클릭 수)
    last_down: Option<(Button, f64, (i32, i32), u32)>,
}

fn mods_of(keys: &BTreeSet<KeyCode>) -> Mods {
    let mut m = Mods::default();
    for k in keys {
        if let Some(x) = k.modifier() {
            m.set(x, true);
        }
    }
    m
}

fn resolve(pos: Pos, s: &ScreenInfo) -> (i32, i32) {
    match pos {
        Pos::Abs { x, y } => (x, y),
        Pos::Ratio { rx, ry } => (
            (rx * f64::from(s.width)).round() as i32,
            (ry * f64::from(s.height)).round() as i32,
        ),
    }
}

/// 이벤트를 주입할 형태로 바꾼다. 이 OS에서 낼 수 없는 키면 None.
fn translate(
    st: &mut State,
    ev: &Event,
    at_ms: f64,
    s: &ScreenInfo,
    os: Os,
) -> Option<InjectEvent> {
    Some(match ev.kind {
        EventKind::Move { pos } => {
            let (x, y) = resolve(pos, s);
            st.last_pos = (x, y);
            match st.buttons.iter().next() {
                Some(&button) => InjectEvent::DragTo { button, x, y },
                None => InjectEvent::MoveTo { x, y },
            }
        }
        EventKind::Down { button, pos } => {
            let (x, y) = resolve(pos, s);
            st.last_pos = (x, y);
            let clicks = match st.last_down {
                Some((b, t, (px, py), n))
                    if b == button
                        && at_ms - t <= DOUBLE_CLICK_MS
                        && (i64::from(x) - i64::from(px)).abs() <= DOUBLE_CLICK_DIST
                        && (i64::from(y) - i64::from(py)).abs() <= DOUBLE_CLICK_DIST =>
                {
                    n + 1
                }
                _ => 1,
            };
            st.last_down = Some((button, at_ms, (x, y), clicks));
            st.buttons.insert(button);
            InjectEvent::Down {
                button,
                x,
                y,
                clicks,
            }
        }
        EventKind::Up { button, pos } => {
            let (x, y) = resolve(pos, s);
            st.last_pos = (x, y);
            st.buttons.remove(&button);
            let clicks = match st.last_down {
                Some((b, _, _, n)) if b == button => n,
                _ => 1,
            };
            InjectEvent::Up {
                button,
                x,
                y,
                clicks,
            }
        }
        EventKind::Wheel { dx, dy, pos } => {
            let (x, y) = resolve(pos, s);
            st.last_pos = (x, y);
            InjectEvent::Wheel { dx, dy, x, y }
        }
        EventKind::KeyDown(Key::Code(key)) => {
            st.keys.insert(key);
            InjectEvent::KeyDown {
                key,
                mods: mods_of(&st.keys),
            }
        }
        EventKind::KeyUp(Key::Code(key)) => {
            st.keys.remove(&key);
            InjectEvent::KeyUp {
                key,
                mods: mods_of(&st.keys),
            }
        }
        EventKind::KeyDown(Key::Unknown { raw, os: from }) => {
            if from != os {
                return None;
            }
            st.raw_keys.insert(raw);
            InjectEvent::RawKey { raw, down: true }
        }
        EventKind::KeyUp(Key::Unknown { raw, os: from }) => {
            if from != os {
                return None;
            }
            st.raw_keys.remove(&raw);
            InjectEvent::RawKey { raw, down: false }
        }
    })
}

/// 눌려 있는 버튼과 키를 모두 뗀다. 보조키는 일반 키를 뗀 다음에 뗀다. 실패는 무시한다(최선).
fn release_all(st: &mut State, backend: &dyn InputBackend) {
    let (x, y) = st.last_pos;
    for button in std::mem::take(&mut st.buttons) {
        let _ = backend.inject(&InjectEvent::Up {
            button,
            x,
            y,
            clicks: 1,
        });
    }
    for raw in std::mem::take(&mut st.raw_keys) {
        let _ = backend.inject(&InjectEvent::RawKey { raw, down: false });
    }
    let keys = std::mem::take(&mut st.keys);
    let (mods, normal): (Vec<KeyCode>, Vec<KeyCode>) =
        keys.into_iter().partition(|k| k.modifier().is_some());
    let mut held: BTreeSet<KeyCode> = mods.iter().copied().collect();
    for key in normal {
        let _ = backend.inject(&InjectEvent::KeyUp {
            key,
            mods: mods_of(&held),
        });
    }
    for key in mods {
        held.remove(&key);
        let _ = backend.inject(&InjectEvent::KeyUp {
            key,
            mods: mods_of(&held),
        });
    }
}

fn add_ms(start: Duration, ms: f64) -> Duration {
    Duration::try_from_secs_f64(ms / 1000.0)
        .ok()
        .and_then(|d| start.checked_add(d))
        .unwrap_or(Duration::MAX)
}

/// target까지 기다린다. 기다리는 동안 중지 요청이 오면 true.
fn wait_until(clock: &dyn Clock, target: Duration, watch: &mut Watch) -> bool {
    loop {
        if watch.poll() {
            return true;
        }
        let now = clock.now();
        if now >= target {
            return false;
        }
        clock.sleep_until(target.min(now.saturating_add(WAIT_CHUNK)));
    }
}

/// 매크로를 재생한다. 중지·실패·정상 종료 어느 경우든 눌린 키와 버튼을 남기지 않는다.
pub fn play(
    m: &Macro,
    backend: &dyn InputBackend,
    clock: &dyn Clock,
    stop: &AtomicBool,
    opts: &PlayOptions,
) -> Result<PlayReport, PlayError> {
    let mut report = PlayReport {
        outcome: PlayOutcome::Finished,
        injected: 0,
        skipped_keys: 0,
    };
    if m.events.is_empty() {
        return Ok(report);
    }
    let (tx, rx) = mpsc::channel();
    let _guard = backend.start_capture(tx).map_err(PlayError::Capture)?;
    let screen = backend.primary_screen();
    let os = backend.os();
    let mut watch = Watch {
        rx,
        mods: Mods::default(),
        stop_key: opts.stop_key,
        failsafe: opts.failsafe,
        screen,
        stop,
    };
    let mut st = State::default();
    let p = &m.play;
    let speed = if p.speed.is_finite() && p.speed > 0.0 {
        p.speed
    } else {
        1.0
    };
    let start = clock.now();
    let mut offset_ms = 0.0_f64;
    let mut first = true;
    let mut round: u32 = 0;
    loop {
        for (i, ev) in m.events.iter().enumerate() {
            offset_ms += if p.timed {
                ev.d as f64 / speed
            } else if first {
                0.0
            } else {
                p.min_gap_ms as f64
            };
            first = false;
            if wait_until(clock, add_ms(start, offset_ms), &mut watch) {
                release_all(&mut st, backend);
                report.outcome = PlayOutcome::Stopped;
                return Ok(report);
            }
            match translate(&mut st, ev, offset_ms, &screen, os) {
                Some(inj) => {
                    if let Err(source) = backend.inject(&inj) {
                        release_all(&mut st, backend);
                        return Err(PlayError::Inject {
                            index: i + 1,
                            source,
                        });
                    }
                    report.injected += 1;
                }
                None => report.skipped_keys += 1,
            }
        }
        round = round.saturating_add(1);
        if p.repeat != 0 && round >= p.repeat {
            break;
        }
        offset_ms += p.repeat_gap_ms as f64;
    }
    release_all(&mut st, backend);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{FakeBackend, FakeClock};
    use crate::input::{InjectEvent as I, RawEvent, RawKind};
    use mox_format::KeyCode::*;
    use mox_format::{
        Button, Event, EventKind, Key, KeyCode, KeyCombo, Macro, Meta, Mods, Os, PlaySettings, Pos,
        ScreenInfo, Unit,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;

    pub(super) fn screen() -> ScreenInfo {
        ScreenInfo {
            width: 1000,
            height: 500,
            scale: 1.0,
            unit: Unit::Pixel,
        }
    }
    pub(super) fn mac(events: Vec<Event>, play: PlaySettings) -> Macro {
        Macro {
            meta: Meta {
                name: "t".into(),
                created: "c".into(),
                recorded_on: Os::Windows,
                screen: screen(),
            },
            play,
            events,
        }
    }
    pub(super) fn setup() -> (Arc<FakeClock>, FakeBackend) {
        let clock = Arc::new(FakeClock::new());
        let backend = FakeBackend::new(Os::Windows, screen()).with_clock(clock.clone());
        (clock, backend)
    }
    pub(super) fn opts() -> PlayOptions {
        PlayOptions {
            stop_key: KeyCombo::single(Pause),
            failsafe: true,
        }
    }
    pub(super) fn run(m: &Macro, clock: &FakeClock, backend: &FakeBackend) -> PlayReport {
        play(m, backend, clock, &AtomicBool::new(false), &opts()).unwrap()
    }
    pub(super) fn abs(x: i32, y: i32) -> Pos {
        Pos::Abs { x, y }
    }
    pub(super) fn mv(d: u64, x: i32, y: i32) -> Event {
        Event::new(d, EventKind::Move { pos: abs(x, y) })
    }
    pub(super) fn down(d: u64, x: i32, y: i32) -> Event {
        Event::new(
            d,
            EventKind::Down {
                button: Button::Left,
                pos: abs(x, y),
            },
        )
    }
    pub(super) fn up(d: u64, x: i32, y: i32) -> Event {
        Event::new(
            d,
            EventKind::Up {
                button: Button::Left,
                pos: abs(x, y),
            },
        )
    }
    pub(super) fn kd(d: u64, k: KeyCode) -> Event {
        Event::new(d, EventKind::KeyDown(Key::Code(k)))
    }
    pub(super) fn ku(d: u64, k: KeyCode) -> Event {
        Event::new(d, EventKind::KeyUp(Key::Code(k)))
    }
    pub(super) fn real(kind: RawKind) -> RawEvent {
        RawEvent {
            at: Duration::ZERO,
            kind,
            injected: false,
        }
    }
    pub(super) fn times(backend: &FakeBackend) -> Vec<u64> {
        backend
            .injected_at()
            .iter()
            .map(|(t, _)| t.as_millis() as u64)
            .collect()
    }

    #[test]
    fn follows_delays_scaled_by_speed() {
        let (clock, backend) = setup();
        let m = mac(
            vec![mv(0, 1, 1), mv(100, 2, 2), mv(50, 3, 3)],
            PlaySettings {
                speed: 2.0,
                ..Default::default()
            },
        );
        let r = run(&m, &clock, &backend);
        assert_eq!(
            r,
            PlayReport {
                outcome: PlayOutcome::Finished,
                injected: 3,
                skipped_keys: 0
            }
        );
        assert_eq!(times(&backend), vec![0, 50, 75]);
    }

    #[test]
    fn untimed_uses_min_gap() {
        let (clock, backend) = setup();
        let m = mac(
            vec![mv(0, 1, 1), mv(900, 2, 2), mv(900, 3, 3)],
            PlaySettings {
                timed: false,
                ..Default::default()
            },
        );
        run(&m, &clock, &backend);
        assert_eq!(times(&backend), vec![0, 5, 10]);
    }

    #[test]
    fn repeats_with_gap_without_drift() {
        let (clock, backend) = setup();
        let m = mac(
            vec![mv(0, 1, 1), mv(100, 2, 2)],
            PlaySettings {
                repeat: 2,
                repeat_gap_ms: 1000,
                ..Default::default()
            },
        );
        run(&m, &clock, &backend);
        assert_eq!(times(&backend), vec![0, 100, 1100, 1200]);
    }

    #[test]
    fn move_while_pressed_becomes_drag() {
        let (clock, backend) = setup();
        let m = mac(
            vec![down(0, 1, 1), mv(10, 5, 5), up(10, 5, 5), mv(10, 6, 6)],
            PlaySettings::default(),
        );
        run(&m, &clock, &backend);
        assert_eq!(
            backend.injected(),
            vec![
                I::Down {
                    button: Button::Left,
                    x: 1,
                    y: 1,
                    clicks: 1
                },
                I::DragTo {
                    button: Button::Left,
                    x: 5,
                    y: 5
                },
                I::Up {
                    button: Button::Left,
                    x: 5,
                    y: 5,
                    clicks: 1
                },
                I::MoveTo { x: 6, y: 6 },
            ]
        );
    }

    #[test]
    fn counts_double_clicks_by_time_and_distance() {
        let (clock, backend) = setup();
        let m = mac(
            vec![
                down(0, 10, 10),
                up(50, 10, 10),
                down(50, 12, 11),
                up(50, 12, 11),
                down(600, 12, 11),
                up(10, 12, 11),
            ],
            PlaySettings::default(),
        );
        run(&m, &clock, &backend);
        let clicks: Vec<u32> = backend
            .injected()
            .iter()
            .map(|e| match e {
                I::Down { clicks, .. } | I::Up { clicks, .. } => *clicks,
                _ => 0,
            })
            .collect();
        assert_eq!(clicks, vec![1, 1, 2, 2, 1, 1]);
    }

    #[test]
    fn far_second_click_is_single() {
        let (clock, backend) = setup();
        let m = mac(
            vec![
                down(0, 10, 10),
                up(10, 10, 10),
                down(10, 20, 10),
                up(10, 20, 10),
            ],
            PlaySettings::default(),
        );
        run(&m, &clock, &backend);
        assert!(matches!(backend.injected()[2], I::Down { clicks: 1, .. }));
    }

    #[test]
    fn key_events_carry_modifier_state() {
        let (clock, backend) = setup();
        let m = mac(
            vec![kd(0, MetaLeft), kd(0, KeyC), ku(0, KeyC), ku(0, MetaLeft)],
            PlaySettings::default(),
        );
        run(&m, &clock, &backend);
        let meta = Mods {
            meta: true,
            ..Mods::default()
        };
        assert_eq!(
            backend.injected(),
            vec![
                I::KeyDown {
                    key: MetaLeft,
                    mods: meta
                },
                I::KeyDown {
                    key: KeyC,
                    mods: meta
                },
                I::KeyUp {
                    key: KeyC,
                    mods: meta
                },
                I::KeyUp {
                    key: MetaLeft,
                    mods: Mods::default()
                },
            ]
        );
    }

    #[test]
    fn ratio_positions_use_current_screen_including_other_monitors() {
        let (clock, backend) = setup();
        let ratio = |rx, ry| {
            Event::new(
                0,
                EventKind::Move {
                    pos: Pos::Ratio { rx, ry },
                },
            )
        };
        let m = mac(
            vec![ratio(0.5, 0.5), ratio(1.5, -0.2)],
            PlaySettings::default(),
        );
        run(&m, &clock, &backend);
        assert_eq!(
            backend.injected(),
            vec![I::MoveTo { x: 500, y: 250 }, I::MoveTo { x: 1500, y: -100 }]
        );
    }

    #[test]
    fn unknown_keys_play_only_on_their_os() {
        let (clock, backend) = setup(); // Windows
        let unk = |os| Event::new(0, EventKind::KeyDown(Key::Unknown { raw: 173, os }));
        let unk_up = |os| Event::new(0, EventKind::KeyUp(Key::Unknown { raw: 173, os }));
        let m = mac(
            vec![unk(Os::Windows), unk_up(Os::Windows), unk(Os::Macos)],
            PlaySettings::default(),
        );
        let r = run(&m, &clock, &backend);
        assert_eq!(r.skipped_keys, 1);
        assert_eq!(
            backend.injected(),
            vec![
                I::RawKey {
                    raw: 173,
                    down: true
                },
                I::RawKey {
                    raw: 173,
                    down: false
                }
            ]
        );
    }

    #[test]
    fn wheel_is_passed_through() {
        let (clock, backend) = setup();
        let m = mac(
            vec![Event::new(
                0,
                EventKind::Wheel {
                    dx: 1,
                    dy: -3,
                    pos: abs(7, 8),
                },
            )],
            PlaySettings::default(),
        );
        run(&m, &clock, &backend);
        assert_eq!(
            backend.injected(),
            vec![I::Wheel {
                dx: 1,
                dy: -3,
                x: 7,
                y: 8
            }]
        );
    }
}

#[cfg(test)]
mod safety_tests {
    use super::tests::*;
    use super::*;
    use crate::input::{InjectEvent as I, RawKind};
    use mox_format::KeyCode::*;
    use mox_format::{Button, Key, Mods, PlaySettings};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn empty_macro_finishes_immediately_even_with_infinite_repeat() {
        let (clock, backend) = setup();
        let m = mac(
            vec![],
            PlaySettings {
                repeat: 0,
                ..Default::default()
            },
        );
        let r = run(&m, &clock, &backend);
        assert_eq!(
            r,
            PlayReport {
                outcome: PlayOutcome::Finished,
                injected: 0,
                skipped_keys: 0
            }
        );
        assert_eq!(clock.now(), std::time::Duration::ZERO);
    }

    #[test]
    fn stop_key_during_drag_releases_button() {
        let (clock, backend) = setup();
        let m = mac(
            vec![down(0, 1, 1), mv(10, 2, 2), mv(10, 3, 3), up(10, 3, 3)],
            PlaySettings::default(),
        );
        backend.after_inject(1, real(RawKind::KeyDown(Key::Code(Pause))));
        let r = run(&m, &clock, &backend);
        assert_eq!(r.outcome, PlayOutcome::Stopped);
        assert_eq!(
            backend.injected(),
            vec![
                I::Down {
                    button: Button::Left,
                    x: 1,
                    y: 1,
                    clicks: 1
                },
                I::DragTo {
                    button: Button::Left,
                    x: 2,
                    y: 2
                },
                I::Up {
                    button: Button::Left,
                    x: 2,
                    y: 2,
                    clicks: 1
                },
            ]
        );
    }

    #[test]
    fn injected_stop_key_is_ignored() {
        let (clock, backend) = setup();
        let m = mac(vec![mv(0, 1, 1), mv(10, 2, 2)], PlaySettings::default());
        let mut fake_stop = real(RawKind::KeyDown(Key::Code(Pause)));
        fake_stop.injected = true;
        backend.after_inject(0, fake_stop);
        assert_eq!(run(&m, &clock, &backend).outcome, PlayOutcome::Finished);
    }

    #[test]
    fn failsafe_corner_stops_only_when_enabled() {
        for (failsafe, expected) in [(true, PlayOutcome::Stopped), (false, PlayOutcome::Finished)] {
            let (clock, backend) = setup();
            let m = mac(vec![mv(0, 1, 1), mv(10, 2, 2)], PlaySettings::default());
            backend.after_inject(0, real(RawKind::MouseMove { x: 999, y: 0 }));
            let o = PlayOptions {
                stop_key: KeyCombo::single(Pause),
                failsafe,
            };
            let r = play(&m, &backend, &*clock, &AtomicBool::new(false), &o).unwrap();
            assert_eq!(r.outcome, expected);
        }
    }

    #[test]
    fn mouse_near_but_not_at_corner_does_not_stop() {
        let (clock, backend) = setup();
        let m = mac(vec![mv(0, 1, 1), mv(10, 2, 2)], PlaySettings::default());
        backend.after_inject(0, real(RawKind::MouseMove { x: 500, y: 0 }));
        assert_eq!(run(&m, &clock, &backend).outcome, PlayOutcome::Finished);
    }

    #[test]
    fn external_stop_flag_stops_before_first_event() {
        let (clock, backend) = setup();
        let m = mac(vec![mv(0, 1, 1)], PlaySettings::default());
        let r = play(&m, &backend, &*clock, &AtomicBool::new(true), &opts()).unwrap();
        assert_eq!(r.outcome, PlayOutcome::Stopped);
        assert!(backend.injected().is_empty());
    }

    #[test]
    fn inject_failure_releases_keys_modifiers_last() {
        // KeyA 누름 주입이 실패하면, 실제로 눌렸을 수도 있는 A를 먼저 떼고 Shift를 마지막에 뗀다.
        let (clock, backend) = setup();
        let m = mac(
            vec![kd(0, ShiftLeft), kd(0, KeyA), ku(0, KeyA), ku(0, ShiftLeft)],
            PlaySettings::default(),
        );
        backend.fail_at(1);
        let err = play(&m, &backend, &*clock, &AtomicBool::new(false), &opts()).unwrap_err();
        assert!(matches!(err, PlayError::Inject { index: 2, .. }));
        let shift = Mods {
            shift: true,
            ..Mods::default()
        };
        assert_eq!(
            backend.injected(),
            vec![
                I::KeyDown {
                    key: ShiftLeft,
                    mods: shift
                },
                I::KeyUp {
                    key: KeyA,
                    mods: shift
                },
                I::KeyUp {
                    key: ShiftLeft,
                    mods: Mods::default()
                },
            ]
        );
    }

    #[test]
    fn keys_left_pressed_at_end_are_released() {
        let (clock, backend) = setup();
        let m = mac(vec![kd(0, ShiftLeft)], PlaySettings::default());
        run(&m, &clock, &backend);
        assert_eq!(
            backend.injected().last(),
            Some(&I::KeyUp {
                key: ShiftLeft,
                mods: Mods::default()
            })
        );
    }

    #[test]
    fn huge_delay_does_not_overflow_and_stop_key_escapes() {
        let (clock, backend) = setup();
        let m = mac(
            vec![kd(0, KeyA), kd(u64::MAX, KeyB)],
            PlaySettings {
                speed: 0.1,
                ..Default::default()
            },
        );
        backend.after_inject(0, real(RawKind::KeyDown(Key::Code(Pause))));
        let r = run(&m, &clock, &backend);
        assert_eq!(r.outcome, PlayOutcome::Stopped);
        assert_eq!(
            backend.injected().last(),
            Some(&I::KeyUp {
                key: KeyA,
                mods: Mods::default()
            })
        );
    }
}
