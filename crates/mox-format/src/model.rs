use serde::{Deserialize, Serialize};

use crate::key::{KeyCode, KeyCombo};
use crate::os::Os;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Pixel,
    Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub unit: Unit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    pub created: String,
    pub recorded_on: Os,
    pub screen: ScreenInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaySettings {
    /// 반복 횟수. 0이면 무제한.
    pub repeat: u32,
    pub repeat_gap_ms: u64,
    /// 배속 0.1~10.
    pub speed: f64,
    /// false면 간격을 무시하고 min_gap_ms 간격으로 연달아 실행한다.
    pub timed: bool,
    pub min_gap_ms: u64,
    /// None이면 재생하는 OS의 기본 중지 키.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_key: Option<KeyCombo>,
}

impl Default for PlaySettings {
    fn default() -> Self {
        Self {
            repeat: 1,
            repeat_gap_ms: 0,
            speed: 1.0,
            timed: true,
            min_gap_ms: 5,
            stop_key: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Button {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pos {
    /// 녹화 OS 단위(Windows 픽셀, macOS 포인트)의 가상 데스크톱 좌표.
    Abs { x: i32, y: i32 },
    /// 주 모니터 크기에 대한 비율. 다른 모니터는 0 미만이나 1 초과.
    Ratio { rx: f64, ry: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Code(KeyCode),
    /// 이름으로 옮길 수 없는 키. 녹화한 OS의 원래 키코드를 보존한다.
    Unknown {
        raw: u32,
        os: Os,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
    Move {
        pos: Pos,
    },
    Down {
        button: Button,
        pos: Pos,
    },
    Up {
        button: Button,
        pos: Pos,
    },
    /// 휠 줄 단위. 양수가 위/오른쪽.
    Wheel {
        dx: i32,
        dy: i32,
        pos: Pos,
    },
    KeyDown(Key),
    KeyUp(Key),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    /// 직전 이벤트와의 간격(ms).
    pub d: u64,
    pub kind: EventKind,
}

impl Event {
    pub fn new(d: u64, kind: EventKind) -> Self {
        Self { d, kind }
    }

    pub fn pos(&self) -> Option<Pos> {
        match self.kind {
            EventKind::Move { pos }
            | EventKind::Down { pos, .. }
            | EventKind::Up { pos, .. }
            | EventKind::Wheel { pos, .. } => Some(pos),
            EventKind::KeyDown(_) | EventKind::KeyUp(_) => None,
        }
    }

    /// 마우스 이벤트의 좌표를 바꾼다. 키 이벤트에는 아무 일도 하지 않는다.
    pub fn set_pos(&mut self, new: Pos) {
        match &mut self.kind {
            EventKind::Move { pos }
            | EventKind::Down { pos, .. }
            | EventKind::Up { pos, .. }
            | EventKind::Wheel { pos, .. } => *pos = new,
            EventKind::KeyDown(_) | EventKind::KeyUp(_) => {}
        }
    }

    pub fn is_move(&self) -> bool {
        matches!(self.kind, EventKind::Move { .. })
    }

    pub fn is_mouse(&self) -> bool {
        self.pos().is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Macro {
    pub meta: Meta,
    pub play: PlaySettings,
    pub events: Vec<Event>,
}
