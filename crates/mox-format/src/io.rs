use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::FormatError;
use crate::model::{Macro, FORMAT_VERSION};
use crate::validate::validate;
use crate::wire::{WireEvent, WireMacro};

pub fn from_str(s: &str) -> Result<Macro, FormatError> {
    let s = s.strip_prefix('\u{feff}').unwrap_or(s); // 메모장이 붙이는 BOM
    let value: serde_json::Value =
        serde_json::from_str(s).map_err(|e| FormatError::Parse(e.to_string()))?;
    let format = value.get("format").and_then(|v| v.as_str()).unwrap_or("");
    if format != "mox" {
        return Err(FormatError::NotMox(format.to_string()));
    }
    let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
    if version != u64::from(FORMAT_VERSION) {
        return Err(FormatError::UnsupportedVersion(version));
    }
    let wire: WireMacro =
        serde_json::from_value(value).map_err(|e| FormatError::Parse(e.to_string()))?;
    let m = wire.into_macro()?;
    validate(&m)?;
    Ok(m)
}

pub fn to_string(m: &Macro) -> Result<String, FormatError> {
    validate(m)?;
    let mut out = String::from("{\n");
    out.push_str("  \"format\": \"mox\",\n");
    out.push_str(&format!("  \"version\": {FORMAT_VERSION},\n"));
    out.push_str(&format!("  \"meta\": {},\n", compact(&m.meta)?));
    out.push_str(&format!("  \"play\": {},\n", compact(&m.play)?));
    out.push_str("  \"events\": [");
    for (i, e) in m.events.iter().enumerate() {
        out.push_str(if i == 0 { "\n    " } else { ",\n    " });
        out.push_str(&compact(&WireEvent::from(e))?);
    }
    out.push_str(if m.events.is_empty() {
        "]\n"
    } else {
        "\n  ]\n"
    });
    out.push_str("}\n");
    Ok(out)
}

fn compact<T: serde::Serialize>(v: &T) -> Result<String, FormatError> {
    serde_json::to_string(v).map_err(|e| FormatError::Parse(e.to_string()))
}

pub fn load(path: &Path) -> Result<Macro, FormatError> {
    from_str(&fs::read_to_string(path)?)
}

/// 임시 파일에 쓴 뒤 바꿔 넣는다. 저장 중에 멈춰도 기존 파일이 손상되지 않는다.
pub fn save(path: &Path, m: &Macro) -> Result<(), FormatError> {
    let text = to_string(m)?;
    let tmp = tmp_path(path);
    let written = write_then_rename(&tmp, path, &text);
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    Ok(written?)
}

fn write_then_rename(tmp: &Path, path: &Path, text: &str) -> std::io::Result<()> {
    let mut f = fs::File::create(tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    drop(f);
    fs::rename(tmp, path)
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Button, Event, EventKind, Key, KeyCode, Macro, Meta, Os, PlaySettings, Pos, ScreenInfo,
        Unit,
    };
    use proptest::prelude::*;

    fn meta(name: &str) -> Meta {
        Meta {
            name: name.to_string(),
            created: "2026-10-10T14:03:00+09:00".to_string(),
            recorded_on: Os::Macos,
            screen: ScreenInfo {
                width: 1512,
                height: 982,
                scale: 2.0,
                unit: Unit::Point,
            },
        }
    }

    fn sample() -> Macro {
        Macro {
            meta: meta("주간보고 정리"),
            play: PlaySettings::default(),
            events: vec![
                Event::new(
                    0,
                    EventKind::Move {
                        pos: Pos::Abs { x: 512, y: 384 },
                    },
                ),
                Event::new(
                    120,
                    EventKind::Down {
                        button: Button::Left,
                        pos: Pos::Abs { x: 512, y: 384 },
                    },
                ),
                Event::new(
                    300,
                    EventKind::Wheel {
                        dx: 0,
                        dy: -3,
                        pos: Pos::Abs { x: 512, y: 384 },
                    },
                ),
                Event::new(800, EventKind::KeyDown(Key::Code(KeyCode::KeyA))),
                Event::new(
                    60,
                    EventKind::KeyUp(Key::Unknown {
                        raw: 173,
                        os: Os::Windows,
                    }),
                ),
                Event::new(
                    200,
                    EventKind::Up {
                        button: Button::Left,
                        pos: Pos::Ratio {
                            rx: 0.4231,
                            ry: 0.3105,
                        },
                    },
                ),
            ],
        }
    }

    #[test]
    fn writes_one_event_per_line() {
        let text = to_string(&sample()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "{");
        assert_eq!(lines[1], "  \"format\": \"mox\",");
        assert_eq!(lines[2], "  \"version\": 1,");
        assert!(lines.contains(
            &"    {\"d\":120,\"type\":\"down\",\"button\":\"left\",\"x\":512,\"y\":384},"
        ));
        assert!(lines.contains(
            &"    {\"d\":300,\"type\":\"wheel\",\"dx\":0,\"dy\":-3,\"x\":512,\"y\":384},"
        ));
        assert!(lines.contains(&"    {\"d\":60,\"type\":\"key_up\",\"key\":\"Unknown\",\"raw\":173,\"raw_os\":\"windows\"},"));
        assert_eq!(*lines.last().unwrap(), "}");
    }

    #[test]
    fn round_trips_sample() {
        let m = sample();
        assert_eq!(from_str(&to_string(&m).unwrap()).unwrap(), m);
    }

    #[test]
    fn round_trips_empty_macro() {
        let m = Macro {
            meta: meta("빈 매크로"),
            play: PlaySettings::default(),
            events: vec![],
        };
        assert_eq!(from_str(&to_string(&m).unwrap()).unwrap(), m);
    }

    #[test]
    fn round_trips_unicode_name_and_offscreen_ratio() {
        let mut m = sample();
        m.meta.name = "보고서 \"최종\"\n📎 \\ 탭\t".to_string();
        m.events.push(Event::new(
            5,
            EventKind::Move {
                pos: Pos::Ratio {
                    rx: -0.25,
                    ry: 1.75,
                },
            },
        ));
        assert_eq!(from_str(&to_string(&m).unwrap()).unwrap(), m);
    }

    #[test]
    fn floats_round_trip_exactly() {
        // proptest가 찾은 사례: serde_json 기본 파서는 마지막 자리를 바꿨다 (float_roundtrip 필요)
        let mut m = sample();
        let pos = Pos::Ratio {
            rx: 2.2467719385087714,
            ry: 0.1 + 0.2,
        };
        m.events
            .push(Event::new(0, EventKind::Wheel { dx: 0, dy: 0, pos }));
        assert_eq!(from_str(&to_string(&m).unwrap()).unwrap(), m);
    }

    #[test]
    fn accepts_utf8_bom() {
        let text = format!("\u{feff}{}", to_string(&sample()).unwrap());
        assert_eq!(from_str(&text).unwrap(), sample());
    }

    #[test]
    fn missing_play_uses_defaults_and_unknown_fields_are_ignored() {
        let text = r#"{"format":"mox","version":1,"future":true,
            "meta":{"name":"a","created":"c","recorded_on":"windows","screen":{"width":1,"height":1,"scale":1.0,"unit":"pixel"}},
            "events":[{"d":0,"type":"move","x":1,"y":2,"color":"red"}]}"#;
        let m = from_str(text).unwrap();
        assert_eq!(m.play, PlaySettings::default());
        assert_eq!(m.events.len(), 1);
    }

    #[test]
    fn reports_errors_in_korean_without_panicking() {
        let base = r#"{"format":"mox","version":1,"meta":{"name":"a","created":"c","recorded_on":"windows","screen":{"width":1,"height":1,"scale":1.0,"unit":"pixel"}},"events":[EVENT]}"#;
        let cases = [
            (r#"{"d":0,"type":"jump","x":1,"y":1}"#, "알 수 없는 type"),
            (r#"{"d":0,"x":1,"y":1}"#, "type이 없습니다"),
            (r#"{"d":0,"type":"move"}"#, "좌표"),
            (
                r#"{"d":0,"type":"move","x":1,"y":1,"rx":0.1,"ry":0.1}"#,
                "한 쌍만",
            ),
            (r#"{"d":0,"type":"down","x":1,"y":1}"#, "button"),
            (r#"{"d":0,"type":"key_down","key":"Nope"}"#, "알 수 없는 키"),
            (r#"{"d":0,"type":"key_down","key":"Unknown"}"#, "raw"),
            (r#"{"d":-1,"type":"move","x":1,"y":1}"#, ""),
        ];
        for (ev, needle) in cases {
            let err = from_str(&base.replace("EVENT", ev)).unwrap_err();
            match &err {
                FormatError::BadEvent { index, reason } => {
                    assert_eq!(*index, 1, "{ev}");
                    assert!(reason.contains(needle), "{ev}: {reason}");
                }
                other => panic!("{ev}: {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_other_formats_and_versions() {
        assert!(matches!(from_str("not json"), Err(FormatError::Parse(_))));
        assert!(matches!(
            from_str(r#"{"format":"other"}"#),
            Err(FormatError::NotMox(_))
        ));
        assert!(matches!(
            from_str(r#"{"format":"mox","version":2}"#),
            Err(FormatError::UnsupportedVersion(2))
        ));
        assert!(matches!(
            from_str(r#"{"format":"mox","version":1}"#),
            Err(FormatError::Parse(_))
        ));
    }

    #[test]
    fn rejects_out_of_range_play_settings() {
        let mut m = sample();
        m.play.speed = 0.0;
        assert!(matches!(to_string(&m), Err(FormatError::BadPlay(_))));
        m.play.speed = f64::NAN;
        assert!(matches!(to_string(&m), Err(FormatError::BadPlay(_))));
        let mut m = sample();
        m.events[0].set_pos(Pos::Ratio {
            rx: f64::INFINITY,
            ry: 0.0,
        });
        assert!(matches!(
            to_string(&m),
            Err(FormatError::BadEvent { index: 1, .. })
        ));
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("주간보고.mox");
        save(&path, &sample()).unwrap();
        assert_eq!(load(&path).unwrap(), sample());
        assert!(!dir.path().join("주간보고.mox.tmp").exists());
    }

    #[test]
    fn failed_save_keeps_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.mox");
        save(&path, &sample()).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut bad = sample();
        bad.play.speed = 99.0;
        assert!(save(&path, &bad).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(save(&dir.path().join("없는폴더/a.mox"), &sample()).is_err());
    }

    fn arb_pos() -> impl Strategy<Value = Pos> {
        prop_oneof![
            (any::<i32>(), any::<i32>()).prop_map(|(x, y)| Pos::Abs { x, y }),
            (-2.0f64..3.0, -2.0f64..3.0).prop_map(|(rx, ry)| Pos::Ratio { rx, ry }),
        ]
    }

    fn arb_button() -> impl Strategy<Value = Button> {
        prop_oneof![
            Just(Button::Left),
            Just(Button::Right),
            Just(Button::Middle),
            Just(Button::Back),
            Just(Button::Forward)
        ]
    }

    fn arb_key() -> impl Strategy<Value = Key> {
        prop_oneof![
            (0..KeyCode::ALL.len()).prop_map(|i| Key::Code(KeyCode::ALL[i])),
            (
                any::<u32>(),
                prop_oneof![Just(Os::Macos), Just(Os::Windows)]
            )
                .prop_map(|(raw, os)| Key::Unknown { raw, os }),
        ]
    }

    fn arb_event() -> impl Strategy<Value = Event> {
        let kind =
            prop_oneof![
                arb_pos().prop_map(|pos| EventKind::Move { pos }),
                (arb_button(), arb_pos()).prop_map(|(button, pos)| EventKind::Down { button, pos }),
                (arb_button(), arb_pos()).prop_map(|(button, pos)| EventKind::Up { button, pos }),
                (any::<i32>(), any::<i32>(), arb_pos())
                    .prop_map(|(dx, dy, pos)| EventKind::Wheel { dx, dy, pos }),
                arb_key().prop_map(EventKind::KeyDown),
                arb_key().prop_map(EventKind::KeyUp),
            ];
        (any::<u64>(), kind).prop_map(|(d, kind)| Event::new(d, kind))
    }

    proptest! {
        #[test]
        fn any_macro_round_trips(events in proptest::collection::vec(arb_event(), 0..50), name in ".*") {
            let m = Macro { meta: meta(&name), play: PlaySettings::default(), events };
            prop_assert_eq!(from_str(&to_string(&m).unwrap()).unwrap(), m);
        }
    }
}
