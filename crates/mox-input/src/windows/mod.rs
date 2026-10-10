//! Windows 입력 계층. Microsoft `windows-sys`로 Win32 API에 붙는다.

mod capture;
mod clock;
mod inject;

use std::sync::mpsc::Sender;

use mox_engine::{CaptureGuard, Clock, InjectEvent, InputBackend, InputError, RawEvent};
use mox_format::{Os, ScreenInfo, Unit};
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, MessageBoxW, IDOK, MB_ICONINFORMATION, MB_OKCANCEL, SM_CXSCREEN, SM_CYSCREEN,
};

use crate::{Access, Permission};

pub use clock::WinClock;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn primary_screen() -> ScreenInfo {
    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    ScreenInfo {
        width: w.max(0) as u32,
        height: h.max(0) as u32,
        scale: 1.0,
        unit: Unit::Pixel,
    }
}

struct WindowsBackend;

impl InputBackend for WindowsBackend {
    fn os(&self) -> Os {
        Os::Windows
    }

    fn primary_screen(&self) -> ScreenInfo {
        primary_screen()
    }

    fn start_capture(&self, tx: Sender<RawEvent>) -> Result<CaptureGuard, InputError> {
        capture::start_capture(tx)
    }

    fn inject(&self, ev: &InjectEvent) -> Result<(), InputError> {
        inject::inject(ev)
    }
}

pub fn backend() -> Result<Box<dyn InputBackend>, InputError> {
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    Ok(Box::new(WindowsBackend))
}

pub fn clock() -> Box<dyn Clock> {
    Box::new(WinClock::new())
}

pub fn permission_status() -> Permission {
    Permission::GRANTED
}

pub fn open_settings(_which: Access) {}

pub fn confirm(header: &str, message: &str) -> bool {
    let title = to_wide(header);
    let body = to_wide(message);
    let rc = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OKCANCEL | MB_ICONINFORMATION,
        )
    };
    rc == IDOK
}
