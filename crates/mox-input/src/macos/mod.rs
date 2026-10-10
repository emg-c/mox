//! macOS 입력 계층. 시스템 프레임워크(CoreGraphics, CoreFoundation)에 raw FFI로 붙는다.
//!
//! 이 파일은 계획 3을 태스크 단위로 채운다: FFI 선언(Task 2) → 캡처(Task 3) → 주입(Task 4)
//! → 권한·시계(Task 5). 아직 채워지지 않은 부분은 임시 구현이다.

mod ffi;

use mox_engine::{Clock, InputBackend, InputError, SystemClock};
use mox_format::{ScreenInfo, Unit};

use crate::{Access, Permission};

/// 주 디스플레이의 포인트 크기와 배율. 마우스 좌표(CGEvent)와 같은 포인트 단위.
pub fn primary_screen() -> ScreenInfo {
    unsafe {
        let d = ffi::CGMainDisplayID();
        let bounds = ffi::CGDisplayBounds(d);
        let (w_pt, h_pt) = (bounds.size.width, bounds.size.height);
        let w_px = ffi::CGDisplayPixelsWide(d) as f64;
        let scale = if w_pt > 0.0 { w_px / w_pt } else { 1.0 };
        ScreenInfo {
            width: w_pt as u32,
            height: h_pt as u32,
            scale,
            unit: Unit::Point,
        }
    }
}

pub fn backend() -> Result<Box<dyn InputBackend>, InputError> {
    Err(InputError::Os("macOS 백엔드는 아직 구현 중입니다".into()))
}

pub fn clock() -> Box<dyn Clock> {
    Box::new(SystemClock::new())
}

pub fn permission_status() -> Permission {
    Permission::GRANTED
}

pub fn open_settings(_which: Access) {}
