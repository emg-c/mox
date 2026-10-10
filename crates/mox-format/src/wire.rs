//! JSON에 쓰는 모양. 모든 필드를 선택적으로 받아서, 무엇이 잘못됐는지 이벤트 번호와 함께 알려 준다.

use serde::{Deserialize, Serialize};

use crate::error::FormatError;
use crate::key::KeyCode;
use crate::model::{Button, Event, EventKind, Key, Macro, Meta, PlaySettings, Pos};
use crate::os::Os;

#[derive(Deserialize)]
pub(crate) struct WireMacro {
    meta: Meta,
    #[serde(default)]
    play: PlaySettings,
    #[serde(default)]
    events: Vec<serde_json::Value>,
}

impl WireMacro {
    pub(crate) fn into_macro(self) -> Result<Macro, FormatError> {
        let events = self
            .events
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                serde_json::from_value::<WireEvent>(v)
                    .map_err(|e| e.to_string())
                    .and_then(WireEvent::into_event)
                    .map_err(|reason| FormatError::BadEvent {
                        index: i + 1,
                        reason,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Macro {
            meta: self.meta,
            play: self.play,
            events,
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct WireEvent {
    #[serde(default)]
    d: u64,
    #[serde(rename = "type", default)]
    ty: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    button: Option<Button>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dx: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dy: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    y: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rx: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ry: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw_os: Option<Os>,
}

impl WireEvent {
    fn pos(&self) -> Result<Pos, String> {
        match (self.x, self.y, self.rx, self.ry) {
            (Some(x), Some(y), None, None) => Ok(Pos::Abs { x, y }),
            (None, None, Some(rx), Some(ry)) => Ok(Pos::Ratio { rx, ry }),
            (None, None, None, None) => Err("좌표(x,y 또는 rx,ry)가 없습니다".into()),
            _ => Err("좌표는 x,y 또는 rx,ry 중 한 쌍만 써야 합니다".into()),
        }
    }

    fn button(&self) -> Result<Button, String> {
        self.button.ok_or_else(|| "button이 없습니다".to_string())
    }

    fn key(&self) -> Result<Key, String> {
        let name = self.key.as_deref().ok_or("key가 없습니다")?;
        if name == "Unknown" {
            return match (self.raw, self.raw_os) {
                (Some(raw), Some(os)) => Ok(Key::Unknown { raw, os }),
                _ => Err("Unknown 키에는 raw와 raw_os가 필요합니다".into()),
            };
        }
        KeyCode::from_name(name)
            .map(Key::Code)
            .ok_or_else(|| format!("알 수 없는 키 이름입니다: {name}"))
    }

    fn into_event(self) -> Result<Event, String> {
        let kind = match self.ty.as_str() {
            "move" => EventKind::Move { pos: self.pos()? },
            "down" => EventKind::Down {
                button: self.button()?,
                pos: self.pos()?,
            },
            "up" => EventKind::Up {
                button: self.button()?,
                pos: self.pos()?,
            },
            "wheel" => EventKind::Wheel {
                dx: self.dx.unwrap_or(0),
                dy: self.dy.unwrap_or(0),
                pos: self.pos()?,
            },
            "key_down" => EventKind::KeyDown(self.key()?),
            "key_up" => EventKind::KeyUp(self.key()?),
            "" => return Err("type이 없습니다".into()),
            other => return Err(format!("알 수 없는 type입니다: {other}")),
        };
        Ok(Event { d: self.d, kind })
    }

    fn set_pos(&mut self, pos: Pos) {
        match pos {
            Pos::Abs { x, y } => {
                self.x = Some(x);
                self.y = Some(y);
            }
            Pos::Ratio { rx, ry } => {
                self.rx = Some(rx);
                self.ry = Some(ry);
            }
        }
    }

    fn set_key(&mut self, key: Key) {
        match key {
            Key::Code(c) => self.key = Some(c.name().to_string()),
            Key::Unknown { raw, os } => {
                self.key = Some("Unknown".to_string());
                self.raw = Some(raw);
                self.raw_os = Some(os);
            }
        }
    }
}

impl From<&Event> for WireEvent {
    fn from(e: &Event) -> Self {
        let mut w = WireEvent {
            d: e.d,
            ..Default::default()
        };
        match e.kind {
            EventKind::Move { pos } => {
                w.ty = "move".into();
                w.set_pos(pos);
            }
            EventKind::Down { button, pos } => {
                w.ty = "down".into();
                w.button = Some(button);
                w.set_pos(pos);
            }
            EventKind::Up { button, pos } => {
                w.ty = "up".into();
                w.button = Some(button);
                w.set_pos(pos);
            }
            EventKind::Wheel { dx, dy, pos } => {
                w.ty = "wheel".into();
                w.dx = Some(dx);
                w.dy = Some(dy);
                w.set_pos(pos);
            }
            EventKind::KeyDown(k) => {
                w.ty = "key_down".into();
                w.set_key(k);
            }
            EventKind::KeyUp(k) => {
                w.ty = "key_up".into();
                w.set_key(k);
            }
        }
        w
    }
}
