//! macOS 가상 키코드(CGKeyCode, Carbon의 kVK_* 상수) ↔ OS 중립 `KeyCode` 변환.
//!
//! 표는 macOS SDK의 `HIToolbox.framework/Headers/Events.h`를 기준으로 한다.
//! 순수 데이터라 리눅스에서도 테스트한다.

use mox_format::KeyCode;

/// (macOS 가상 키코드, KeyCode) 쌍. 1:1 매핑만 넣는다.
const TABLE: &[(u16, KeyCode)] = &[
    (0x00, KeyCode::KeyA),
    (0x01, KeyCode::KeyS),
    (0x02, KeyCode::KeyD),
    (0x03, KeyCode::KeyF),
    (0x04, KeyCode::KeyH),
    (0x05, KeyCode::KeyG),
    (0x06, KeyCode::KeyZ),
    (0x07, KeyCode::KeyX),
    (0x08, KeyCode::KeyC),
    (0x09, KeyCode::KeyV),
    (0x0A, KeyCode::IntlBackslash), // kVK_ISO_Section
    (0x0B, KeyCode::KeyB),
    (0x0C, KeyCode::KeyQ),
    (0x0D, KeyCode::KeyW),
    (0x0E, KeyCode::KeyE),
    (0x0F, KeyCode::KeyR),
    (0x10, KeyCode::KeyY),
    (0x11, KeyCode::KeyT),
    (0x12, KeyCode::Digit1),
    (0x13, KeyCode::Digit2),
    (0x14, KeyCode::Digit3),
    (0x15, KeyCode::Digit4),
    (0x16, KeyCode::Digit6),
    (0x17, KeyCode::Digit5),
    (0x18, KeyCode::Equal),
    (0x19, KeyCode::Digit9),
    (0x1A, KeyCode::Digit7),
    (0x1B, KeyCode::Minus),
    (0x1C, KeyCode::Digit8),
    (0x1D, KeyCode::Digit0),
    (0x1E, KeyCode::BracketRight),
    (0x1F, KeyCode::KeyO),
    (0x20, KeyCode::KeyU),
    (0x21, KeyCode::BracketLeft),
    (0x22, KeyCode::KeyI),
    (0x23, KeyCode::KeyP),
    (0x24, KeyCode::Enter), // kVK_Return
    (0x25, KeyCode::KeyL),
    (0x26, KeyCode::KeyJ),
    (0x27, KeyCode::Quote),
    (0x28, KeyCode::KeyK),
    (0x29, KeyCode::Semicolon),
    (0x2A, KeyCode::Backslash),
    (0x2B, KeyCode::Comma),
    (0x2C, KeyCode::Slash),
    (0x2D, KeyCode::KeyN),
    (0x2E, KeyCode::KeyM),
    (0x2F, KeyCode::Period),
    (0x30, KeyCode::Tab),
    (0x31, KeyCode::Space),
    (0x32, KeyCode::Backquote), // kVK_ANSI_Grave
    (0x33, KeyCode::Backspace), // kVK_Delete (맥의 Delete = 백스페이스)
    (0x35, KeyCode::Escape),
    (0x36, KeyCode::MetaRight), // kVK_RightCommand
    (0x37, KeyCode::MetaLeft),  // kVK_Command
    (0x38, KeyCode::ShiftLeft),
    (0x39, KeyCode::CapsLock),
    (0x3A, KeyCode::AltLeft), // kVK_Option
    (0x3B, KeyCode::ControlLeft),
    (0x3C, KeyCode::ShiftRight),
    (0x3D, KeyCode::AltRight), // kVK_RightOption
    (0x3E, KeyCode::ControlRight),
    (0x40, KeyCode::F17),
    (0x41, KeyCode::NumpadDecimal),
    (0x43, KeyCode::NumpadMultiply),
    (0x45, KeyCode::NumpadAdd),
    (0x47, KeyCode::NumLock), // kVK_ANSI_KeypadClear
    (0x4B, KeyCode::NumpadDivide),
    (0x4C, KeyCode::NumpadEnter),
    (0x4E, KeyCode::NumpadSubtract),
    (0x4F, KeyCode::F18),
    (0x50, KeyCode::F19),
    (0x52, KeyCode::Numpad0),
    (0x53, KeyCode::Numpad1),
    (0x54, KeyCode::Numpad2),
    (0x55, KeyCode::Numpad3),
    (0x56, KeyCode::Numpad4),
    (0x57, KeyCode::Numpad5),
    (0x58, KeyCode::Numpad6),
    (0x59, KeyCode::Numpad7),
    (0x5A, KeyCode::F20),
    (0x5B, KeyCode::Numpad8),
    (0x5C, KeyCode::Numpad9),
    (0x60, KeyCode::F5),
    (0x61, KeyCode::F6),
    (0x62, KeyCode::F7),
    (0x63, KeyCode::F3),
    (0x64, KeyCode::F8),
    (0x65, KeyCode::F9),
    (0x67, KeyCode::F11),
    (0x69, KeyCode::F13),
    (0x6A, KeyCode::F16),
    (0x6B, KeyCode::F14),
    (0x6D, KeyCode::F10),
    (0x6E, KeyCode::ContextMenu), // kVK_ContextualMenu
    (0x6F, KeyCode::F12),
    (0x71, KeyCode::F15),
    (0x72, KeyCode::Insert), // kVK_Help (맥 Help → 웹 Insert 관례)
    (0x73, KeyCode::Home),
    (0x74, KeyCode::PageUp),
    (0x75, KeyCode::Delete), // kVK_ForwardDelete
    (0x76, KeyCode::F4),
    (0x77, KeyCode::End),
    (0x78, KeyCode::F2),
    (0x79, KeyCode::PageDown),
    (0x7A, KeyCode::F1),
    (0x7B, KeyCode::ArrowLeft),
    (0x7C, KeyCode::ArrowRight),
    (0x7D, KeyCode::ArrowDown),
    (0x7E, KeyCode::ArrowUp),
];

/// macOS 가상 키코드 → KeyCode. 표에 없으면 None.
pub fn keycode_to_code(vk: u16) -> Option<KeyCode> {
    TABLE.iter().find(|&&(k, _)| k == vk).map(|&(_, c)| c)
}

/// KeyCode → macOS 가상 키코드. 표에 없으면 None(맥에서 낼 수 없는 키).
pub fn code_to_keycode(code: KeyCode) -> Option<u16> {
    TABLE.iter().find(|&&(_, c)| c == code).map(|&(k, _)| k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_no_duplicate_keys_or_codes() {
        let mut vks: Vec<u16> = TABLE.iter().map(|&(k, _)| k).collect();
        vks.sort_unstable();
        vks.dedup();
        assert_eq!(vks.len(), TABLE.len(), "중복된 가상 키코드가 있다");
        let mut codes: Vec<&str> = TABLE.iter().map(|&(_, c)| c.name()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), TABLE.len(), "중복된 KeyCode가 있다");
    }

    #[test]
    fn round_trips_both_directions() {
        for &(vk, code) in TABLE {
            assert_eq!(keycode_to_code(vk), Some(code));
            assert_eq!(code_to_keycode(code), Some(vk));
        }
    }

    #[test]
    fn well_known_keys_are_correct() {
        assert_eq!(keycode_to_code(0x00), Some(KeyCode::KeyA));
        assert_eq!(keycode_to_code(0x24), Some(KeyCode::Enter));
        assert_eq!(keycode_to_code(0x31), Some(KeyCode::Space));
        assert_eq!(keycode_to_code(0x37), Some(KeyCode::MetaLeft));
        assert_eq!(keycode_to_code(0x38), Some(KeyCode::ShiftLeft));
        assert_eq!(keycode_to_code(0x33), Some(KeyCode::Backspace));
        assert_eq!(keycode_to_code(0x75), Some(KeyCode::Delete));
        assert_eq!(code_to_keycode(KeyCode::ControlLeft), Some(0x3B));
        assert_eq!(code_to_keycode(KeyCode::MetaRight), Some(0x36));
    }

    #[test]
    fn unknown_keycodes_return_none() {
        assert_eq!(keycode_to_code(0x3F), None); // kVK_Function (대응 없음)
        assert_eq!(keycode_to_code(0xFF), None);
        assert_eq!(code_to_keycode(KeyCode::F24), None); // 맥 키보드에 없음
        assert_eq!(code_to_keycode(KeyCode::Pause), None);
    }
}
