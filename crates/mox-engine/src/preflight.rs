use mox_format::{EventKind, Key, KeyCombo, Macro, Os, PlaySettings, Pos, ScreenInfo};

/// 재생 전에 사용자에게 확인받을 점.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// 절대 좌표가 있는데 화면 크기나 단위가 녹화 때와 다르다.
    ScreenMismatch {
        recorded: (u32, u32),
        current: (u32, u32),
    },
    /// 다른 OS에서 녹화돼 이 OS에서 낼 수 없는 키가 있다(재생 시 건너뜀).
    UnmappableKeys { count: usize, from: Os },
    /// 지정한 중지 키가 이 OS 키보드에 없어 기본 중지 키를 쓴다.
    StopKeyFallback { requested: KeyCombo, used: KeyCombo },
}

pub fn effective_stop_key(play: &PlaySettings, os: Os) -> (KeyCombo, Option<Warning>) {
    match play.stop_key {
        None => (KeyCombo::default_stop(os), None),
        Some(k) if k.exists_on(os) => (k, None),
        Some(k) => {
            let used = KeyCombo::default_stop(os);
            (used, Some(Warning::StopKeyFallback { requested: k, used }))
        }
    }
}

pub fn preflight(m: &Macro, os: Os, screen: &ScreenInfo) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let rec = &m.meta.screen;
    let has_abs = m
        .events
        .iter()
        .any(|e| matches!(e.pos(), Some(Pos::Abs { .. })));
    if has_abs
        && ((rec.width, rec.height) != (screen.width, screen.height) || rec.unit != screen.unit)
    {
        warnings.push(Warning::ScreenMismatch {
            recorded: (rec.width, rec.height),
            current: (screen.width, screen.height),
        });
    }
    let foreign: Vec<Os> = m
        .events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::KeyDown(Key::Unknown { os: from, .. })
            | EventKind::KeyUp(Key::Unknown { os: from, .. })
                if from != os =>
            {
                Some(from)
            }
            _ => None,
        })
        .collect();
    if let Some(&from) = foreign.first() {
        warnings.push(Warning::UnmappableKeys {
            count: foreign.len(),
            from,
        });
    }
    if let (_, Some(w)) = effective_stop_key(&m.play, os) {
        warnings.push(w);
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_format::{Event, EventKind, Key, KeyCode, Meta, PlaySettings, Pos, Unit};

    fn screen(w: u32, h: u32, unit: Unit) -> ScreenInfo {
        ScreenInfo {
            width: w,
            height: h,
            scale: 1.0,
            unit,
        }
    }
    fn mac(events: Vec<Event>, play: PlaySettings) -> Macro {
        Macro {
            meta: Meta {
                name: "t".into(),
                created: "c".into(),
                recorded_on: Os::Macos,
                screen: screen(1512, 982, Unit::Point),
            },
            play,
            events,
        }
    }
    fn abs_move() -> Event {
        Event::new(
            0,
            EventKind::Move {
                pos: Pos::Abs { x: 1, y: 1 },
            },
        )
    }
    fn ratio_move() -> Event {
        Event::new(
            0,
            EventKind::Move {
                pos: Pos::Ratio { rx: 0.1, ry: 0.1 },
            },
        )
    }

    #[test]
    fn same_screen_and_keys_have_no_warnings() {
        let m = mac(vec![abs_move()], PlaySettings::default());
        assert!(preflight(&m, Os::Macos, &screen(1512, 982, Unit::Point)).is_empty());
    }

    #[test]
    fn warns_on_different_screen_only_with_absolute_positions() {
        let m = mac(vec![abs_move()], PlaySettings::default());
        assert_eq!(
            preflight(&m, Os::Macos, &screen(1920, 1080, Unit::Point)),
            vec![Warning::ScreenMismatch {
                recorded: (1512, 982),
                current: (1920, 1080)
            }]
        );
        assert_eq!(
            preflight(&m, Os::Macos, &screen(1512, 982, Unit::Pixel)).len(),
            1
        );
        let ratio_only = mac(vec![ratio_move()], PlaySettings::default());
        assert!(preflight(&ratio_only, Os::Macos, &screen(1920, 1080, Unit::Point)).is_empty());
    }

    #[test]
    fn warns_on_keys_from_other_os() {
        let unk = Event::new(
            0,
            EventKind::KeyDown(Key::Unknown {
                raw: 1,
                os: Os::Macos,
            }),
        );
        let m = mac(vec![unk, unk], PlaySettings::default());
        assert_eq!(
            preflight(&m, Os::Windows, &screen(1512, 982, Unit::Point)),
            vec![Warning::UnmappableKeys {
                count: 2,
                from: Os::Macos
            }]
        );
    }

    #[test]
    fn falls_back_when_stop_key_missing_on_os() {
        let play = PlaySettings {
            stop_key: Some(KeyCombo::single(KeyCode::Pause)),
            ..Default::default()
        };
        let (used, warn) = effective_stop_key(&play, Os::Macos);
        assert_eq!(used, KeyCombo::default_stop(Os::Macos));
        assert_eq!(
            warn,
            Some(Warning::StopKeyFallback {
                requested: KeyCombo::single(KeyCode::Pause),
                used
            })
        );
        assert_eq!(
            effective_stop_key(&play, Os::Windows),
            (KeyCombo::single(KeyCode::Pause), None)
        );
        assert_eq!(
            effective_stop_key(&PlaySettings::default(), Os::Windows).0,
            KeyCombo::default_stop(Os::Windows)
        );
        let m = mac(vec![], play);
        assert_eq!(
            preflight(&m, Os::Macos, &screen(1512, 982, Unit::Point)).len(),
            1
        );
    }
}
