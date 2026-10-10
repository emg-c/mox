use std::time::{Duration, Instant};

pub trait Clock: Send + Sync {
    /// 시계 생성 기준 경과 시간.
    fn now(&self) -> Duration;
    /// t까지 잠든다. 이미 지났으면 바로 돌아온다.
    fn sleep_until(&self, t: Duration);
}

/// 표준 라이브러리 시계. OS별 고해상도 대기는 계획 3·4에서 입력 계층과 함께 바꾼다.
pub struct SystemClock {
    start: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }

    fn sleep_until(&self, t: Duration) {
        let now = self.now();
        if t > now {
            std::thread::sleep(t - now);
        }
    }
}
