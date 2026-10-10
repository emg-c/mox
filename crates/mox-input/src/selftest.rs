//! 입력 계층 자가 진단. 합성 이벤트를 주입하고 탭으로 되돌아오는지, 시계 오차는 얼마인지 본다.
//! OS에 무관하다(백엔드·시계는 backend()/clock()로 얻는다). 실제 입력 효과가 있으므로
//! 사용자가 의도적으로 실행하는 도구다.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use mox_engine::{InjectEvent, InputError, RawEvent, RawKind};
use mox_format::{Key, KeyCode, Mods};

use crate::{backend, clock, permission_status};

#[derive(Debug)]
pub struct SelftestReport {
    pub capture_perm: bool,
    pub inject_perm: bool,
    pub injected: usize,
    pub captured_injected: usize,
    pub saw_move: bool,
    pub saw_key: bool,
    pub saw_wheel: bool,
    pub clock_error_ms: f64,
}

impl SelftestReport {
    pub fn ok(&self) -> bool {
        self.saw_move && self.saw_key && self.saw_wheel && self.clock_error_ms.abs() < 15.0
    }
}

impl std::fmt::Display for SelftestReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "권한  입력모니터링={} 손쉬운사용={}",
            self.capture_perm, self.inject_perm
        )?;
        writeln!(
            f,
            "주입 {}개 / 캡처로 되돌아온 주입 이벤트 {}개",
            self.injected, self.captured_injected
        )?;
        writeln!(
            f,
            "왕복  이동={} 키={} 휠={}",
            self.saw_move, self.saw_key, self.saw_wheel
        )?;
        writeln!(
            f,
            "시계 오차  {:.2}ms (100ms 대기 기준)",
            self.clock_error_ms
        )?;
        write!(f, "결과  {}", if self.ok() { "통과" } else { "실패" })
    }
}

/// 합성 입력을 주입하고 되돌아오는지 확인한다. F13·구석 이동 등 부작용이 적은 입력만 쓴다.
pub fn selftest() -> Result<SelftestReport, InputError> {
    let perm = permission_status();
    let b = backend()?;
    let (tx, rx) = mpsc::channel::<RawEvent>();
    let _guard = b.start_capture(tx)?;
    std::thread::sleep(Duration::from_millis(100)); // 탭이 자리잡을 시간

    let seq = [
        InjectEvent::MoveTo { x: 120, y: 120 },
        InjectEvent::KeyDown {
            key: KeyCode::F13,
            mods: Mods::default(),
        },
        InjectEvent::KeyUp {
            key: KeyCode::F13,
            mods: Mods::default(),
        },
        InjectEvent::Wheel {
            dx: 0,
            dy: -1,
            x: 120,
            y: 120,
        },
    ];
    for ev in &seq {
        b.inject(ev)?;
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(150));

    let mut captured_injected = 0;
    let (mut saw_move, mut saw_key, mut saw_wheel) = (false, false, false);
    while let Ok(ev) = rx.try_recv() {
        if !ev.injected {
            continue;
        }
        captured_injected += 1;
        match ev.kind {
            RawKind::MouseMove { .. } => saw_move = true,
            RawKind::KeyDown(Key::Code(KeyCode::F13)) => saw_key = true,
            RawKind::Wheel { .. } => saw_wheel = true,
            _ => {}
        }
    }

    // 시계 오차: 100ms를 MacosClock으로 기다리고 실제 경과와 비교.
    let c = clock();
    let wall = Instant::now();
    let t0 = c.now();
    c.sleep_until(t0 + Duration::from_millis(100));
    let actual = wall.elapsed().as_secs_f64() * 1000.0;

    Ok(SelftestReport {
        capture_perm: perm.capture,
        inject_perm: perm.inject,
        injected: seq.len(),
        captured_injected,
        saw_move,
        saw_key,
        saw_wheel,
        clock_error_ms: actual - 100.0,
    })
}
