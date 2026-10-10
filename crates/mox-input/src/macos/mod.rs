//! macOS 입력 계층. 시스템 프레임워크에 raw FFI로 붙는다.

mod capture;
mod clock;
mod ffi;
mod inject;

use std::sync::mpsc::Sender;

use mox_engine::{
    CaptureGuard, Clock, InjectEvent, InputBackend, InputError, RawEvent,
};
use mox_format::{Os, ScreenInfo, Unit};

use crate::{Access, Permission};

pub use clock::MacosClock;

/// 주 디스플레이의 포인트 크기와 배율. 마우스 좌표(CGEvent)와 같은 포인트 단위.
pub fn primary_screen() -> ScreenInfo {
    unsafe {
        let d = ffi::CGMainDisplayID();
        let bounds = ffi::CGDisplayBounds(d);
        let (w_pt, h_pt) = (bounds.size.width, bounds.size.height);
        let w_px = ffi::CGDisplayPixelsWide(d) as f64;
        let scale = if w_pt > 0.0 { w_px / w_pt } else { 1.0 };
        ScreenInfo { width: w_pt as u32, height: h_pt as u32, scale, unit: Unit::Point }
    }
}

struct MacosBackend {
    injector: inject::Injector,
}

impl InputBackend for MacosBackend {
    fn os(&self) -> Os {
        Os::Macos
    }

    fn primary_screen(&self) -> ScreenInfo {
        primary_screen()
    }

    fn start_capture(&self, tx: Sender<RawEvent>) -> Result<CaptureGuard, InputError> {
        capture::start_capture(tx)
    }

    fn inject(&self, ev: &InjectEvent) -> Result<(), InputError> {
        self.injector.inject(ev)
    }
}

pub fn backend() -> Result<Box<dyn InputBackend>, InputError> {
    Ok(Box::new(MacosBackend { injector: inject::Injector::new()? }))
}

pub fn clock() -> Box<dyn Clock> {
    Box::new(MacosClock::new())
}

pub fn permission_status() -> Permission {
    Permission {
        capture: unsafe { ffi::CGPreflightListenEventAccess() },
        inject: unsafe { ffi::AXIsProcessTrusted() },
    }
}

pub fn open_settings(which: Access) {
    let anchor = match which {
        Access::Capture => "Privacy_ListenEvent",
        Access::Inject => "Privacy_Accessibility",
    };
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{anchor}");
    let _ = std::process::Command::new("open").arg(url).status();
}
