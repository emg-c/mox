use std::time::Duration;

use mox_engine::effective_stop_key;
use mox_format::{Macro, Os};

use crate::source::{Loaded, Origin};

/// 재생기가 도는 OS. macOS가 아니면 Windows 규칙을 따른다(리눅스 테스트 빌드 포함).
pub fn current_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::Macos
    } else {
        Os::Windows
    }
}

/// 한 번 재생하는 데 걸리는 예상 시간.
pub fn one_round(m: &Macro) -> Duration {
    let p = &m.play;
    let ms = if p.timed {
        let speed = if p.speed.is_finite() && p.speed > 0.0 {
            p.speed
        } else {
            1.0
        };
        m.events.iter().map(|e| e.d as f64).sum::<f64>() / speed
    } else {
        m.events.len().saturating_sub(1) as f64 * p.min_gap_ms as f64
    };
    Duration::try_from_secs_f64(ms / 1000.0).unwrap_or(Duration::MAX)
}

pub fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, sec) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 {
        format!("{h}시간 {m}분 {sec}초")
    } else if m > 0 {
        format!("{m}분 {sec}초")
    } else {
        format!("{sec}.{}초", d.subsec_millis() / 100)
    }
}

pub fn summary(l: &Loaded, os: Os) -> String {
    let m = &l.payload.mox;
    let (stop, _) = effective_stop_key(&m.play, os);
    let repeat = match m.play.repeat {
        0 => "무제한 반복".to_string(),
        1 => "1회".to_string(),
        n => format!("{n}회 반복"),
    };
    let origin = match &l.origin {
        Origin::Embedded => "내장".to_string(),
        Origin::Bundle(p) | Origin::File(p) => p.display().to_string(),
    };
    format!(
        "이름: {}\n이벤트: {}개\n예상 시간: {} ({})\n중지 키: {}\n출처: {}\n",
        m.meta.name,
        m.events.len(),
        format_duration(one_round(m)),
        repeat,
        stop,
        origin
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{Loaded, Origin};
    use mox_format::{Event, EventKind, Key, KeyCode, Meta, PlaySettings, ScreenInfo, Unit};
    use mox_payload::{ExportSettings, Payload};
    use std::time::Duration;

    fn mac(ds: &[u64], play: PlaySettings) -> Macro {
        Macro {
            meta: Meta {
                name: "요약".into(),
                created: "c".into(),
                recorded_on: Os::Windows,
                screen: ScreenInfo {
                    width: 1,
                    height: 1,
                    scale: 1.0,
                    unit: Unit::Pixel,
                },
            },
            play,
            events: ds
                .iter()
                .map(|&d| Event::new(d, EventKind::KeyDown(Key::Code(KeyCode::KeyA))))
                .collect(),
        }
    }

    #[test]
    fn one_round_respects_speed_and_untimed() {
        assert_eq!(
            one_round(&mac(&[0, 1000, 500], PlaySettings::default())),
            Duration::from_millis(1500)
        );
        let fast = PlaySettings {
            speed: 2.0,
            ..Default::default()
        };
        assert_eq!(
            one_round(&mac(&[0, 1000, 500], fast)),
            Duration::from_millis(750)
        );
        let untimed = PlaySettings {
            timed: false,
            min_gap_ms: 5,
            ..Default::default()
        };
        assert_eq!(
            one_round(&mac(&[0, 1000, 500], untimed)),
            Duration::from_millis(10)
        );
        assert_eq!(
            one_round(&mac(&[], PlaySettings::default())),
            Duration::ZERO
        );
        let huge = one_round(&mac(
            &[u64::MAX, u64::MAX],
            PlaySettings {
                speed: 0.1,
                ..Default::default()
            },
        ));
        assert!(huge > Duration::from_secs(1_000_000_000_000), "{huge:?}"); // 패닉·오버플로 없이 아주 큰 값
    }

    #[test]
    fn formats_durations() {
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.5초");
        assert_eq!(format_duration(Duration::from_secs(125)), "2분 5초");
        assert_eq!(
            format_duration(Duration::from_secs(3 * 3600 + 61)),
            "3시간 1분 1초"
        );
    }

    #[test]
    fn summary_lists_name_count_time_repeat_stop_key_and_origin() {
        let l = Loaded {
            origin: Origin::Embedded,
            payload: Payload {
                settings: ExportSettings::default(),
                mox: mac(
                    &[0, 1500],
                    PlaySettings {
                        repeat: 0,
                        ..Default::default()
                    },
                ),
            },
        };
        let s = summary(&l, Os::Windows);
        assert_eq!(
            s,
            "이름: 요약\n이벤트: 2개\n예상 시간: 1.5초 (무제한 반복)\n중지 키: Pause\n출처: 내장\n"
        );
        let mac_s = summary(&l, Os::Macos);
        assert!(mac_s.contains("중지 키: Control+Alt+Escape"), "{mac_s}");
    }
}
