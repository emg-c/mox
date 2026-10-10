const INVALID: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];
const FALLBACK: &str = "Mox 매크로";
const MAX_CHARS: usize = 100;

/// 파일·앱 이름으로 쓸 수 없는 문자를 `_`로 바꾸고, 앞뒤 점·공백을 지운다.
/// 비면 기본 이름, Windows 예약어면 앞에 `_`를 붙인다. 최대 100자.
pub fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || INVALID.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned
        .trim()
        .trim_end_matches(['.', ' '])
        .trim_start_matches(['.', ' ']);
    if trimmed.is_empty() {
        return FALLBACK.to_string();
    }
    let limited: String = trimmed.chars().take(MAX_CHARS).collect();
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&limited)) {
        format!("_{limited}")
    } else {
        limited
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_characters_invalid_on_windows_or_macos() {
        assert_eq!(safe_file_name("주간/보고:최종?"), "주간_보고_최종_");
        assert_eq!(safe_file_name("a*b\"c<d>e|f\\g"), "a_b_c_d_e_f_g");
        assert_eq!(safe_file_name("\u{0}탭\t"), "_탭_");
    }

    #[test]
    fn trims_dots_and_spaces() {
        assert_eq!(safe_file_name("  .숨김. "), "숨김");
    }

    #[test]
    fn falls_back_when_empty() {
        assert_eq!(safe_file_name(""), "Mox 매크로");
        assert_eq!(safe_file_name(" .. "), "Mox 매크로");
    }

    #[test]
    fn avoids_windows_reserved_names() {
        assert_eq!(safe_file_name("con"), "_con");
        assert_eq!(safe_file_name("LPT1"), "_LPT1");
        assert_eq!(safe_file_name("console"), "console");
    }

    #[test]
    fn limits_length_by_characters() {
        let long = "가".repeat(150);
        assert_eq!(safe_file_name(&long).chars().count(), 100);
    }
}
