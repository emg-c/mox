//! Windows 가상 키코드(VK) ↔ OS 중립 `KeyCode` 변환. 순수 데이터, 리눅스에서 테스트.
//! VK 상수는 Win32 `winuser.h` 기준.

use mox_format::KeyCode;

const TABLE: &[(u16, KeyCode)] = &[
    (0x08, KeyCode::Backspace),
    (0x09, KeyCode::Tab),
    (0x0D, KeyCode::Enter),
    (0x13, KeyCode::Pause),
    (0x14, KeyCode::CapsLock),
    (0x1B, KeyCode::Escape),
    (0x20, KeyCode::Space),
    (0x21, KeyCode::PageUp),
    (0x22, KeyCode::PageDown),
    (0x23, KeyCode::End),
    (0x24, KeyCode::Home),
    (0x25, KeyCode::ArrowLeft),
    (0x26, KeyCode::ArrowUp),
    (0x27, KeyCode::ArrowRight),
    (0x28, KeyCode::ArrowDown),
    (0x2C, KeyCode::PrintScreen),
    (0x2D, KeyCode::Insert),
    (0x2E, KeyCode::Delete),
    (0x30, KeyCode::Digit0),
    (0x31, KeyCode::Digit1),
    (0x32, KeyCode::Digit2),
    (0x33, KeyCode::Digit3),
    (0x34, KeyCode::Digit4),
    (0x35, KeyCode::Digit5),
    (0x36, KeyCode::Digit6),
    (0x37, KeyCode::Digit7),
    (0x38, KeyCode::Digit8),
    (0x39, KeyCode::Digit9),
    (0x41, KeyCode::KeyA),
    (0x42, KeyCode::KeyB),
    (0x43, KeyCode::KeyC),
    (0x44, KeyCode::KeyD),
    (0x45, KeyCode::KeyE),
    (0x46, KeyCode::KeyF),
    (0x47, KeyCode::KeyG),
    (0x48, KeyCode::KeyH),
    (0x49, KeyCode::KeyI),
    (0x4A, KeyCode::KeyJ),
    (0x4B, KeyCode::KeyK),
    (0x4C, KeyCode::KeyL),
    (0x4D, KeyCode::KeyM),
    (0x4E, KeyCode::KeyN),
    (0x4F, KeyCode::KeyO),
    (0x50, KeyCode::KeyP),
    (0x51, KeyCode::KeyQ),
    (0x52, KeyCode::KeyR),
    (0x53, KeyCode::KeyS),
    (0x54, KeyCode::KeyT),
    (0x55, KeyCode::KeyU),
    (0x56, KeyCode::KeyV),
    (0x57, KeyCode::KeyW),
    (0x58, KeyCode::KeyX),
    (0x59, KeyCode::KeyY),
    (0x5A, KeyCode::KeyZ),
    (0x5B, KeyCode::MetaLeft),
    (0x5C, KeyCode::MetaRight),
    (0x5D, KeyCode::ContextMenu),
    (0x60, KeyCode::Numpad0),
    (0x61, KeyCode::Numpad1),
    (0x62, KeyCode::Numpad2),
    (0x63, KeyCode::Numpad3),
    (0x64, KeyCode::Numpad4),
    (0x65, KeyCode::Numpad5),
    (0x66, KeyCode::Numpad6),
    (0x67, KeyCode::Numpad7),
    (0x68, KeyCode::Numpad8),
    (0x69, KeyCode::Numpad9),
    (0x6A, KeyCode::NumpadMultiply),
    (0x6B, KeyCode::NumpadAdd),
    (0x6D, KeyCode::NumpadSubtract),
    (0x6E, KeyCode::NumpadDecimal),
    (0x6F, KeyCode::NumpadDivide),
    (0x70, KeyCode::F1),
    (0x71, KeyCode::F2),
    (0x72, KeyCode::F3),
    (0x73, KeyCode::F4),
    (0x74, KeyCode::F5),
    (0x75, KeyCode::F6),
    (0x76, KeyCode::F7),
    (0x77, KeyCode::F8),
    (0x78, KeyCode::F9),
    (0x79, KeyCode::F10),
    (0x7A, KeyCode::F11),
    (0x7B, KeyCode::F12),
    (0x7C, KeyCode::F13),
    (0x7D, KeyCode::F14),
    (0x7E, KeyCode::F15),
    (0x7F, KeyCode::F16),
    (0x80, KeyCode::F17),
    (0x81, KeyCode::F18),
    (0x82, KeyCode::F19),
    (0x83, KeyCode::F20),
    (0x84, KeyCode::F21),
    (0x85, KeyCode::F22),
    (0x86, KeyCode::F23),
    (0x87, KeyCode::F24),
    (0x90, KeyCode::NumLock),
    (0x91, KeyCode::ScrollLock),
    (0xA0, KeyCode::ShiftLeft),
    (0xA1, KeyCode::ShiftRight),
    (0xA2, KeyCode::ControlLeft),
    (0xA3, KeyCode::ControlRight),
    (0xA4, KeyCode::AltLeft),
    (0xA5, KeyCode::AltRight),
    (0xBA, KeyCode::Semicolon),
    (0xBB, KeyCode::Equal),
    (0xBC, KeyCode::Comma),
    (0xBD, KeyCode::Minus),
    (0xBE, KeyCode::Period),
    (0xBF, KeyCode::Slash),
    (0xC0, KeyCode::Backquote),
    (0xDB, KeyCode::BracketLeft),
    (0xDC, KeyCode::Backslash),
    (0xDD, KeyCode::BracketRight),
    (0xDE, KeyCode::Quote),
    (0xE2, KeyCode::IntlBackslash),
];

/// Windows VK → KeyCode.
pub fn vk_to_code(vk: u16) -> Option<KeyCode> {
    TABLE.iter().find(|&&(v, _)| v == vk).map(|&(_, c)| c)
}

/// KeyCode → Windows VK.
pub fn code_to_vk(code: KeyCode) -> Option<u16> {
    TABLE.iter().find(|&&(_, c)| c == code).map(|&(v, _)| v)
}

/// SendInput에서 KEYEVENTF_EXTENDEDKEY가 필요한 키(확장 키).
pub fn is_extended(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ArrowLeft
            | KeyCode::ArrowUp
            | KeyCode::ArrowRight
            | KeyCode::ArrowDown
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Insert
            | KeyCode::Delete
            | KeyCode::NumLock
            | KeyCode::NumpadDivide
            | KeyCode::PrintScreen
            | KeyCode::ControlRight
            | KeyCode::AltRight
            | KeyCode::MetaLeft
            | KeyCode::MetaRight
            | KeyCode::ContextMenu
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicates_and_round_trips() {
        let mut vks: Vec<u16> = TABLE.iter().map(|&(v, _)| v).collect();
        vks.sort_unstable();
        vks.dedup();
        assert_eq!(vks.len(), TABLE.len());
        for &(vk, code) in TABLE {
            assert_eq!(vk_to_code(vk), Some(code));
            assert_eq!(code_to_vk(code), Some(vk));
        }
    }

    #[test]
    fn well_known() {
        assert_eq!(vk_to_code(0x41), Some(KeyCode::KeyA));
        assert_eq!(vk_to_code(0x0D), Some(KeyCode::Enter));
        assert_eq!(vk_to_code(0x13), Some(KeyCode::Pause));
        assert_eq!(code_to_vk(KeyCode::ControlLeft), Some(0xA2));
        assert!(is_extended(KeyCode::ArrowUp));
        assert!(!is_extended(KeyCode::KeyA));
    }

    #[test]
    fn unknown_none() {
        assert_eq!(vk_to_code(0x07), None);
        assert_eq!(code_to_vk(KeyCode::IntlBackslash), Some(0xE2));
    }
}
