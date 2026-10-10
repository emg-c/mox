//! 매크로 이벤트 목록을 바꾸는 순수 함수들. 모든 함수는 `Vec<Event>`만 다루고 OS를 모른다.

mod coords;
mod delay;
mod history;
mod keys;
mod mouse;
mod remove;
mod rows;
mod sel;

pub use coords::{to_abs, to_ratio};
pub use delay::{cap_delays, scale_delays, set_delay};
pub use history::History;
pub use keys::{remove_key_repeats, swap_cmd_ctrl};
pub use mouse::{
    remove_mouse, remove_moves, thin_moves_distance, thin_moves_half, thin_moves_interval,
};
pub use remove::remove_where;
pub use rows::{copy, delete, insert, move_down, move_up, replace};
pub use sel::Sel;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EditError {
    #[error("배율은 0 이상의 유한한 수여야 합니다: {0}")]
    InvalidFactor(f64),
    #[error("화면 크기가 0이라 좌표를 변환할 수 없습니다")]
    ZeroScreen,
}

#[cfg(test)]
pub(crate) mod testutil {
    use mox_format::{Button, Event, EventKind, Key, KeyCode, Pos};

    pub fn mv(d: u64, x: i32, y: i32) -> Event {
        Event::new(
            d,
            EventKind::Move {
                pos: Pos::Abs { x, y },
            },
        )
    }
    pub fn down(d: u64, x: i32, y: i32) -> Event {
        Event::new(
            d,
            EventKind::Down {
                button: Button::Left,
                pos: Pos::Abs { x, y },
            },
        )
    }
    pub fn up(d: u64, x: i32, y: i32) -> Event {
        Event::new(
            d,
            EventKind::Up {
                button: Button::Left,
                pos: Pos::Abs { x, y },
            },
        )
    }
    pub fn kd(d: u64, k: KeyCode) -> Event {
        Event::new(d, EventKind::KeyDown(Key::Code(k)))
    }
    pub fn ku(d: u64, k: KeyCode) -> Event {
        Event::new(d, EventKind::KeyUp(Key::Code(k)))
    }
    pub fn delays(evs: &[Event]) -> Vec<u64> {
        evs.iter().map(|e| e.d).collect()
    }
    pub fn total(evs: &[Event]) -> u64 {
        evs.iter().map(|e| e.d).sum()
    }
}
