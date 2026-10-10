use std::sync::mpsc::Sender;
use std::time::Duration;

use mox_format::{Button, Key, KeyCode, Mods, Os, ScreenInfo};

/// OS 훅이 넘겨주는 실제 입력 하나.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawKind {
    MouseMove { x: i32, y: i32 },
    MouseDown { button: Button, x: i32, y: i32 },
    MouseUp { button: Button, x: i32, y: i32 },
    Wheel { dx: i32, dy: i32, x: i32, y: i32 },
    KeyDown(Key),
    KeyUp(Key),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawEvent {
    /// 수집 시작 기준 단조 시각.
    pub at: Duration,
    pub kind: RawKind,
    /// 프로그램이 주입한 입력이면 true (재생 입력의 재녹화 방지).
    pub injected: bool,
}

/// OS에 주입할 입력 하나. 엔진이 드래그·더블클릭·보조키 상태를 이미 계산해서 넘긴다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InjectEvent {
    MoveTo {
        x: i32,
        y: i32,
    },
    /// 버튼을 누른 채 이동 (macOS는 Dragged 이벤트가 필요하다).
    DragTo {
        button: Button,
        x: i32,
        y: i32,
    },
    Down {
        button: Button,
        x: i32,
        y: i32,
        clicks: u32,
    },
    Up {
        button: Button,
        x: i32,
        y: i32,
        clicks: u32,
    },
    Wheel {
        dx: i32,
        dy: i32,
        x: i32,
        y: i32,
    },
    /// mods는 이 이벤트가 일어난 뒤의 보조키 상태 (macOS 플래그용).
    KeyDown {
        key: KeyCode,
        mods: Mods,
    },
    KeyUp {
        key: KeyCode,
        mods: Mods,
    },
    /// 이름으로 옮기지 못한 키를 같은 OS에서 원래 키코드로 주입.
    RawKey {
        raw: u32,
        down: bool,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("권한이 없습니다: {0}")]
    Permission(String),
    #[error("입력 계층 오류: {0}")]
    Os(String),
}

/// drop하면 입력 수집이 끝난다.
pub struct CaptureGuard {
    _inner: Box<dyn Send>,
}

impl CaptureGuard {
    pub fn new(inner: impl Send + 'static) -> Self {
        Self {
            _inner: Box::new(inner),
        }
    }
}

pub trait InputBackend {
    fn os(&self) -> Os;
    fn primary_screen(&self) -> ScreenInfo;
    /// 실제 입력 수집을 시작한다. 이벤트는 tx로 흘러나온다.
    fn start_capture(&self, tx: Sender<RawEvent>) -> Result<CaptureGuard, InputError>;
    fn inject(&self, ev: &InjectEvent) -> Result<(), InputError>;
}
