//! 테스트용 가짜 입력 장치와 가짜 시계. 실제 입력 없이 엔진 동작을 결정적으로 검증한다.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mox_format::{Os, ScreenInfo};

use crate::clock::Clock;
use crate::input::{CaptureGuard, InjectEvent, InputBackend, InputError, RawEvent};

#[derive(Default)]
pub struct FakeClock {
    now: Mutex<Duration>,
}

impl FakeClock {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }

    /// 실제로 기다리지 않고 시각만 앞으로 옮긴다.
    fn sleep_until(&self, t: Duration) {
        let mut now = self.now.lock().unwrap();
        if t > *now {
            *now = t;
        }
    }
}

pub struct FakeBackend {
    os: Os,
    screen: ScreenInfo,
    clock: Option<Arc<FakeClock>>,
    injected: Mutex<Vec<(Duration, InjectEvent)>>,
    capture: Mutex<Option<Sender<RawEvent>>>,
    on_capture: Mutex<Vec<RawEvent>>,
    script: Mutex<HashMap<usize, Vec<RawEvent>>>,
    fail_at: Mutex<Option<usize>>,
}

impl FakeBackend {
    pub fn new(os: Os, screen: ScreenInfo) -> Self {
        Self {
            os,
            screen,
            clock: None,
            injected: Mutex::new(Vec::new()),
            capture: Mutex::new(None),
            on_capture: Mutex::new(Vec::new()),
            script: Mutex::new(HashMap::new()),
            fail_at: Mutex::new(None),
        }
    }

    /// 주입 시각을 기록할 시계를 붙인다.
    pub fn with_clock(mut self, clock: Arc<FakeClock>) -> Self {
        self.clock = Some(clock);
        self
    }

    /// 수집이 시작되자마자 보낼 실제 입력.
    pub fn on_capture(&self, raws: Vec<RawEvent>) {
        self.on_capture.lock().unwrap().extend(raws);
    }

    /// n번째(0부터) 주입이 끝난 직후 보낼 실제 입력 (재생 중 중지 키 누르기 흉내).
    pub fn after_inject(&self, n: usize, raw: RawEvent) {
        self.script.lock().unwrap().entry(n).or_default().push(raw);
    }

    /// n번째(0부터) 주입을 한 번 실패시킨다.
    pub fn fail_at(&self, n: usize) {
        *self.fail_at.lock().unwrap() = Some(n);
    }

    pub fn injected(&self) -> Vec<InjectEvent> {
        self.injected
            .lock()
            .unwrap()
            .iter()
            .map(|(_, e)| *e)
            .collect()
    }

    pub fn injected_at(&self) -> Vec<(Duration, InjectEvent)> {
        self.injected.lock().unwrap().clone()
    }

    fn send(&self, raws: Vec<RawEvent>) {
        if let Some(tx) = &*self.capture.lock().unwrap() {
            for r in raws {
                let _ = tx.send(r);
            }
        }
    }
}

impl InputBackend for FakeBackend {
    fn os(&self) -> Os {
        self.os
    }

    fn primary_screen(&self) -> ScreenInfo {
        self.screen
    }

    fn start_capture(&self, tx: Sender<RawEvent>) -> Result<CaptureGuard, InputError> {
        *self.capture.lock().unwrap() = Some(tx);
        let queued = std::mem::take(&mut *self.on_capture.lock().unwrap());
        self.send(queued);
        Ok(CaptureGuard::new(()))
    }

    fn inject(&self, ev: &InjectEvent) -> Result<(), InputError> {
        let n = {
            let mut injected = self.injected.lock().unwrap();
            let n = injected.len();
            let mut fail = self.fail_at.lock().unwrap();
            if *fail == Some(n) {
                *fail = None;
                return Err(InputError::Os("가짜 주입 실패".into()));
            }
            let at = self.clock.as_ref().map(|c| c.now()).unwrap_or_default();
            injected.push((at, *ev));
            n
        };
        let raws = self.script.lock().unwrap().remove(&n).unwrap_or_default();
        self.send(raws);
        Ok(())
    }
}
