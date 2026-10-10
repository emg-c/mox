//! macOS 입력 계층. 시스템 프레임워크(CoreGraphics, CoreFoundation)에 raw FFI로 붙는다.
//!
//! 이 파일은 계획 3을 태스크 단위로 채운다: FFI 선언(Task 2) → 캡처(Task 3) → 주입(Task 4)
//! → 권한·시계(Task 5). 아래는 아직 채워지지 않은 부분의 임시 구현이다.

use mox_engine::{Clock, InputBackend, InputError, SystemClock};

use crate::{Access, Permission};

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
