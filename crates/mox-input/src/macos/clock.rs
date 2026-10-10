//! mach 타이머 기반 고해상도 시계. 바쁜 대기 없이 ±1ms를 목표로 한다.

use std::time::Duration;

use mox_engine::Clock;

use super::ffi;

pub struct MacosClock {
    start: u64,
    numer: u64,
    denom: u64,
}

impl MacosClock {
    pub fn new() -> Self {
        let mut tb = ffi::mach_timebase_info_data_t { numer: 0, denom: 0 };
        unsafe { ffi::mach_timebase_info(&mut tb) };
        Self {
            start: unsafe { ffi::mach_absolute_time() },
            numer: tb.numer as u64,
            denom: tb.denom as u64,
        }
    }

    fn ticks_to_nanos(&self, ticks: u64) -> u128 {
        ticks as u128 * self.numer as u128 / self.denom as u128
    }

    fn nanos_to_ticks(&self, nanos: u128) -> u64 {
        (nanos * self.denom as u128 / self.numer as u128).min(u64::MAX as u128) as u64
    }
}

impl Clock for MacosClock {
    fn now(&self) -> Duration {
        let elapsed = unsafe { ffi::mach_absolute_time() }.saturating_sub(self.start);
        let nanos = self.ticks_to_nanos(elapsed);
        Duration::from_nanos(nanos.min(u64::MAX as u128) as u64)
    }

    fn sleep_until(&self, t: Duration) {
        let target_ticks = self.start.saturating_add(self.nanos_to_ticks(t.as_nanos()));
        unsafe { ffi::mach_wait_until(target_ticks) };
    }
}
