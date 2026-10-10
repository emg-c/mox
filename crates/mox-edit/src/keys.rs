use std::collections::BTreeSet;

use mox_format::{Event, EventKind, Key, KeyCode};

use crate::{remove_where, Sel};

/// 이미 눌린 키의 반복 누름(자동반복)을 지운다.
pub fn remove_key_repeats(events: &mut Vec<Event>) -> usize {
    let mut held: BTreeSet<Key> = BTreeSet::new();
    let drop: Vec<bool> = events
        .iter()
        .map(|e| match e.kind {
            EventKind::KeyDown(k) => !held.insert(k),
            EventKind::KeyUp(k) => {
                held.remove(&k);
                false
            }
            _ => false,
        })
        .collect();
    remove_where(events, |i, _| drop[i])
}

fn swapped(k: KeyCode) -> Option<KeyCode> {
    match k {
        KeyCode::MetaLeft => Some(KeyCode::ControlLeft),
        KeyCode::MetaRight => Some(KeyCode::ControlRight),
        KeyCode::ControlLeft => Some(KeyCode::MetaLeft),
        KeyCode::ControlRight => Some(KeyCode::MetaRight),
        _ => None,
    }
}

/// 선택한 행에서 Cmd(Meta)와 Ctrl을 맞바꾼다. 맥↔윈도우 단축키 이식용.
pub fn swap_cmd_ctrl(events: &mut [Event], sel: Sel) -> usize {
    let mut n = 0;
    for (i, e) in events.iter_mut().enumerate() {
        if !sel.contains(i) {
            continue;
        }
        if let EventKind::KeyDown(Key::Code(k)) | EventKind::KeyUp(Key::Code(k)) = &mut e.kind {
            if let Some(s) = swapped(*k) {
                *k = s;
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use crate::Sel;
    use mox_format::KeyCode::*;
    use std::collections::BTreeSet;

    #[test]
    fn removes_auto_repeat_downs() {
        let mut evs = vec![
            kd(0, KeyA),
            kd(500, KeyA),
            kd(30, KeyA),
            ku(30, KeyA),
            kd(10, KeyA),
        ];
        assert_eq!(remove_key_repeats(&mut evs), 2);
        assert_eq!(evs, vec![kd(0, KeyA), ku(560, KeyA), kd(10, KeyA)]);
    }

    #[test]
    fn swaps_meta_and_control_on_selection() {
        let mut evs = vec![
            kd(0, MetaLeft),
            kd(0, KeyC),
            ku(0, KeyC),
            ku(0, MetaRight),
            kd(0, ControlLeft),
        ];
        let rows: BTreeSet<usize> = [0, 1, 2, 3].into();
        assert_eq!(swap_cmd_ctrl(&mut evs, Sel::Rows(&rows)), 2);
        assert_eq!(
            evs,
            vec![
                kd(0, ControlLeft),
                kd(0, KeyC),
                ku(0, KeyC),
                ku(0, ControlRight),
                kd(0, ControlLeft)
            ]
        );
    }
}
