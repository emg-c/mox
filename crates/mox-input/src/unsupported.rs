//! macOS가 아닌 타깃(리눅스 등)용 스텁. 워크스페이스가 컴파일·테스트되게 하되, 실제 입력은 지원하지 않는다.

use mox_engine::{Clock, InputBackend, InputError, SystemClock};

use crate::{Access, Permission};

pub fn backend() -> Result<Box<dyn InputBackend>, InputError> {
    Err(InputError::Os(
        "이 운영체제의 입력 계층은 지원하지 않습니다".into(),
    ))
}

pub fn clock() -> Box<dyn Clock> {
    Box::new(SystemClock::new())
}

pub fn permission_status() -> Permission {
    Permission::GRANTED
}

pub fn open_settings(_which: Access) {}

pub fn confirm(_header: &str, _message: &str) -> bool {
    true
}
