//! `.mox` 매크로 파일의 데이터 모델과 읽기/쓰기.

mod error;
mod io;
mod key;
mod model;
mod os;
mod validate;
mod wire;

pub use error::FormatError;
pub use io::{from_str, load, save, to_string};
pub use key::{KeyCode, KeyCombo, Modifier, Mods, ParseComboError};
pub use model::{
    Button, Event, EventKind, Key, Macro, Meta, PlaySettings, Pos, ScreenInfo, Unit, FORMAT_VERSION,
};
pub use os::Os;
pub use validate::validate;
