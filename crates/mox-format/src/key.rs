//! OS 중립 키 이름(웹 표준 KeyboardEvent.code)과 단축키 조합.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::os::Os;

macro_rules! key_codes {
    ($($name:ident),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum KeyCode {
            $($name),*
        }

        impl KeyCode {
            pub const ALL: &'static [KeyCode] = &[$(KeyCode::$name),*];

            pub fn name(self) -> &'static str {
                match self {
                    $(KeyCode::$name => stringify!($name)),*
                }
            }

            pub fn from_name(s: &str) -> Option<KeyCode> {
                match s {
                    $(stringify!($name) => Some(KeyCode::$name),)*
                    _ => None,
                }
            }
        }
    };
}

key_codes!(
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,
    IntlBackslash,
    CapsLock,
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    NumLock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadDivide,
    NumpadMultiply,
    NumpadSubtract,
    NumpadAdd,
    NumpadEnter,
    NumpadDecimal,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    MetaLeft,
    MetaRight,
    ContextMenu,
    Lang1,
    Lang2,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modifier {
    Control,
    Alt,
    Shift,
    Meta,
}

impl KeyCode {
    pub fn modifier(self) -> Option<Modifier> {
        match self {
            KeyCode::ControlLeft | KeyCode::ControlRight => Some(Modifier::Control),
            KeyCode::AltLeft | KeyCode::AltRight => Some(Modifier::Alt),
            KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(Modifier::Shift),
            KeyCode::MetaLeft | KeyCode::MetaRight => Some(Modifier::Meta),
            _ => None,
        }
    }

    /// 이 OS의 일반 키보드에 있는 키인가. 맥 키보드에는 Pause 같은 키가 없다.
    pub fn exists_on(self, os: Os) -> bool {
        match os {
            Os::Windows => true,
            Os::Macos => !matches!(
                self,
                KeyCode::Pause
                    | KeyCode::ScrollLock
                    | KeyCode::PrintScreen
                    | KeyCode::Insert
                    | KeyCode::ContextMenu
                    | KeyCode::NumLock
            ),
        }
    }
}

/// 눌려 있는 보조키 상태.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Mods {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Mods {
    pub fn set(&mut self, m: Modifier, on: bool) {
        match m {
            Modifier::Control => self.control = on,
            Modifier::Alt => self.alt = on,
            Modifier::Shift => self.shift = on,
            Modifier::Meta => self.meta = on,
        }
    }

    pub fn has(&self, m: Modifier) -> bool {
        match m {
            Modifier::Control => self.control,
            Modifier::Alt => self.alt,
            Modifier::Shift => self.shift,
            Modifier::Meta => self.meta,
        }
    }
}

/// 단축키 하나: 보조키 조합 + 키 하나. 문자열로는 "Control+Alt+Escape".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub mods: Mods,
    pub key: KeyCode,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("잘못된 단축키입니다: {0:?}")]
pub struct ParseComboError(pub String);

impl KeyCombo {
    pub fn new(mods: Mods, key: KeyCode) -> Self {
        Self { mods, key }
    }

    pub fn single(key: KeyCode) -> Self {
        Self {
            mods: Mods::default(),
            key,
        }
    }

    /// 키가 같고 보조키 상태가 정확히 같을 때만 일치한다.
    pub fn matches(&self, mods: Mods, key: KeyCode) -> bool {
        self.key == key && self.mods == mods
    }

    pub fn exists_on(&self, os: Os) -> bool {
        self.key.exists_on(os)
    }

    pub fn default_stop(os: Os) -> Self {
        match os {
            Os::Windows => Self::single(KeyCode::Pause),
            Os::Macos => Self::new(
                Mods {
                    control: true,
                    alt: true,
                    ..Mods::default()
                },
                KeyCode::Escape,
            ),
        }
    }

    pub fn default_record_start(_os: Os) -> Self {
        Self::new(
            Mods {
                control: true,
                alt: true,
                ..Mods::default()
            },
            KeyCode::KeyR,
        )
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [
            (self.mods.control, "Control"),
            (self.mods.alt, "Alt"),
            (self.mods.shift, "Shift"),
            (self.mods.meta, "Meta"),
        ] {
            if on {
                write!(f, "{name}+")?;
            }
        }
        f.write_str(self.key.name())
    }
}

impl FromStr for KeyCombo {
    type Err = ParseComboError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseComboError(s.to_string());
        let parts: Vec<&str> = s.split('+').map(str::trim).collect();
        let (last, mods_part) = parts.split_last().ok_or_else(err)?;
        let key = KeyCode::from_name(last).ok_or_else(err)?;
        let mut mods = Mods::default();
        for p in mods_part {
            let m = match p.to_ascii_lowercase().as_str() {
                "control" | "ctrl" => Modifier::Control,
                "alt" | "option" => Modifier::Alt,
                "shift" => Modifier::Shift,
                "meta" | "cmd" | "command" | "win" | "super" => Modifier::Meta,
                _ => return Err(err()),
            };
            mods.set(m, true);
        }
        Ok(Self { mods, key })
    }
}

impl Serialize for KeyCombo {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for KeyCombo {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_name_round_trips() {
        for &k in KeyCode::ALL {
            assert_eq!(KeyCode::from_name(k.name()), Some(k), "{k:?}");
        }
    }

    #[test]
    fn parses_and_displays_combo() {
        let c: KeyCombo = "Control+Alt+Escape".parse().unwrap();
        assert!(c.mods.control && c.mods.alt && !c.mods.shift && !c.mods.meta);
        assert_eq!(c.key, KeyCode::Escape);
        assert_eq!(c.to_string(), "Control+Alt+Escape");
    }

    #[test]
    fn accepts_modifier_aliases_and_normalizes_order() {
        let c: KeyCombo = "cmd+shift+KeyS".parse().unwrap();
        assert!(c.mods.meta && c.mods.shift);
        assert_eq!(c.to_string(), "Shift+Meta+KeyS");
        let c: KeyCombo = "ctrl+option+KeyR".parse().unwrap();
        assert_eq!(c.to_string(), "Control+Alt+KeyR");
    }

    #[test]
    fn parses_single_key() {
        assert_eq!(
            "Pause".parse::<KeyCombo>().unwrap(),
            KeyCombo::single(KeyCode::Pause)
        );
    }

    #[test]
    fn rejects_bad_combos() {
        for s in ["", "Control+", "Control+Nope", "Hyper+KeyA", "+KeyA"] {
            assert!(s.parse::<KeyCombo>().is_err(), "{s:?}");
        }
    }

    #[test]
    fn matches_requires_exact_modifiers() {
        let c: KeyCombo = "Control+Alt+Escape".parse().unwrap();
        let mut m = Mods::default();
        m.set(Modifier::Control, true);
        assert!(!c.matches(m, KeyCode::Escape));
        m.set(Modifier::Alt, true);
        assert!(c.matches(m, KeyCode::Escape));
        m.set(Modifier::Shift, true);
        assert!(!c.matches(m, KeyCode::Escape));
    }

    #[test]
    fn defaults_exist_on_their_os() {
        for os in [Os::Macos, Os::Windows] {
            assert!(KeyCombo::default_stop(os).exists_on(os));
            assert!(KeyCombo::default_record_start(os).exists_on(os));
        }
        assert!(!KeyCombo::default_stop(Os::Windows).exists_on(Os::Macos)); // Pause
    }

    #[test]
    fn combo_serializes_as_string() {
        let c: KeyCombo = "Control+Alt+KeyR".parse().unwrap();
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"Control+Alt+KeyR\"");
        assert_eq!(serde_json::from_str::<KeyCombo>(&json).unwrap(), c);
        assert!(serde_json::from_str::<KeyCombo>("\"Bad+Key\"").is_err());
    }
}
