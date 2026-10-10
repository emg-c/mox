//! OS별 입력 계층. `mox-engine`의 `InputBackend`/`Clock` 트레이트를 각 OS의 시스템 API로 구현한다.
//!
//! OS 의존 코드는 이 크레이트의 `#[cfg(target_os=...)]` 모듈에만 있다.
//! 리눅스 등 지원하지 않는 OS에서는 `unsupported` 모듈이 컴파일을 통과시키되 실행 시 오류를 낸다.

pub mod keymap;

use mox_engine::{Clock, InputBackend, InputError};

/// 녹화·재생에 필요한 권한. macOS에서만 의미가 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permission {
    /// 녹화(입력 모니터링) 허용 여부.
    pub capture: bool,
    /// 재생(손쉬운 사용) 허용 여부.
    pub inject: bool,
}

impl Permission {
    /// 모든 권한이 있는 상태(권한 개념이 없는 OS의 기본값).
    pub const GRANTED: Permission = Permission { capture: true, inject: true };
}

/// 열어 줄 시스템 설정 창.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Capture,
    Inject,
}

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod os_impl;

#[cfg(not(target_os = "macos"))]
#[path = "unsupported.rs"]
mod os_impl;

/// 이 OS의 입력 백엔드를 만든다.
pub fn backend() -> Result<Box<dyn InputBackend>, InputError> {
    os_impl::backend()
}

/// 이 OS의 고해상도 시계.
pub fn clock() -> Box<dyn Clock> {
    os_impl::clock()
}

/// 현재 권한 상태.
pub fn permission_status() -> Permission {
    os_impl::permission_status()
}

/// 해당 권한의 시스템 설정 창을 연다.
pub fn open_settings(which: Access) {
    os_impl::open_settings(which)
}

/// 개발용: 주 디스플레이 정보. probe 예제에서 쓴다.
#[cfg(target_os = "macos")]
pub fn macos_probe_screen() -> mox_format::ScreenInfo {
    os_impl::primary_screen()
}
