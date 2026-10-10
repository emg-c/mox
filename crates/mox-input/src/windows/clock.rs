//! QueryPerformanceCounter + 고해상도 대기 타이머 기반 시계.

use std::time::Duration;

use mox_engine::Clock;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows_sys::Win32::System::Threading::{
    CreateWaitableTimerExW, SetWaitableTimer, WaitForSingleObject,
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, INFINITE, TIMER_ALL_ACCESS,
};

pub struct WinClock {
    freq: u64,
    start: u64,
    timer: HANDLE,
}

// 타이머 핸들은 재생 스레드에서만 쓴다.
unsafe impl Send for WinClock {}
unsafe impl Sync for WinClock {}

fn qpc() -> u64 {
    let mut v = 0i64;
    unsafe { QueryPerformanceCounter(&mut v) };
    v as u64
}

impl WinClock {
    pub fn new() -> Self {
        let mut f = 0i64;
        unsafe { QueryPerformanceFrequency(&mut f) };
        let timer = unsafe {
            CreateWaitableTimerExW(
                std::ptr::null(),
                std::ptr::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS,
            )
        };
        Self {
            freq: f as u64,
            start: qpc(),
            timer,
        }
    }
}

impl Clock for WinClock {
    fn now(&self) -> Duration {
        let ticks = qpc().saturating_sub(self.start);
        Duration::from_nanos(
            (ticks as u128 * 1_000_000_000 / self.freq as u128).min(u64::MAX as u128) as u64,
        )
    }

    fn sleep_until(&self, t: Duration) {
        let target = self.start + (t.as_nanos() * self.freq as u128 / 1_000_000_000) as u64;
        let now = qpc();
        if target <= now {
            return;
        }
        let remaining_ticks = target - now;
        // 100ns 단위, 음수 = 상대 시간.
        let due_100ns = -((remaining_ticks as i128 * 10_000_000 / self.freq as i128) as i64);
        if self.timer.is_null()
            || unsafe { SetWaitableTimer(self.timer, &due_100ns, 0, None, std::ptr::null(), 0) }
                == 0
        {
            std::thread::sleep(self.now().saturating_sub(t).max(Duration::from_millis(0)));
            return;
        }
        unsafe { WaitForSingleObject(self.timer, INFINITE) };
    }
}

impl Drop for WinClock {
    fn drop(&mut self) {
        if !self.timer.is_null() {
            unsafe { CloseHandle(self.timer) };
        }
    }
}
