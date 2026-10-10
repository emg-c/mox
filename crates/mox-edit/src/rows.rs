use std::collections::BTreeSet;

use mox_format::Event;

use crate::{remove_where, Sel};

/// at 위치에 끼워 넣는다. at이 길이보다 크면 끝에 붙인다.
pub fn insert(events: &mut Vec<Event>, at: usize, new: &[Event]) {
    let at = at.min(events.len());
    events.splice(at..at, new.iter().copied());
}

/// 선택한 행을 지운다. 지운 행의 간격은 다음 행에 더해진다.
pub fn delete(events: &mut Vec<Event>, sel: Sel) -> usize {
    remove_where(events, |i, _| sel.contains(i))
}

pub fn replace(events: &mut [Event], index: usize, e: Event) -> bool {
    match events.get_mut(index) {
        Some(slot) => {
            *slot = e;
            true
        }
        None => false,
    }
}

pub fn copy(events: &[Event], rows: &BTreeSet<usize>) -> Vec<Event> {
    rows.iter()
        .filter_map(|&i| events.get(i).copied())
        .collect()
}

/// 선택한 행을 한 칸 위로 옮긴다(간격 d는 행과 함께 움직인다).
/// 맨 위 행이 선택에 있거나 범위를 벗어나면 아무것도 하지 않는다. 옮긴 뒤의 선택을 돌려준다.
pub fn move_up(events: &mut [Event], rows: &BTreeSet<usize>) -> BTreeSet<usize> {
    let (Some(&first), Some(&last)) = (rows.first(), rows.last()) else {
        return rows.clone();
    };
    if first == 0 || last >= events.len() {
        return rows.clone();
    }
    for &i in rows {
        events.swap(i - 1, i);
    }
    rows.iter().map(|i| i - 1).collect()
}

pub fn move_down(events: &mut [Event], rows: &BTreeSet<usize>) -> BTreeSet<usize> {
    let Some(&last) = rows.last() else {
        return rows.clone();
    };
    if last + 1 >= events.len() {
        return rows.clone();
    }
    for &i in rows.iter().rev() {
        events.swap(i, i + 1);
    }
    rows.iter().map(|i| i + 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use crate::Sel;
    use mox_format::KeyCode;
    use std::collections::BTreeSet;

    fn abc() -> Vec<mox_format::Event> {
        vec![
            kd(0, KeyCode::KeyA),
            kd(10, KeyCode::KeyB),
            kd(20, KeyCode::KeyC),
        ]
    }

    #[test]
    fn insert_clamps_index() {
        let mut evs = abc();
        insert(&mut evs, 99, &[kd(5, KeyCode::KeyZ)]);
        assert_eq!(evs.last().unwrap(), &kd(5, KeyCode::KeyZ));
        insert(&mut evs, 0, &[kd(1, KeyCode::KeyY)]);
        assert_eq!(evs[0], kd(1, KeyCode::KeyY));
    }

    #[test]
    fn delete_keeps_total_time() {
        let mut evs = abc();
        let rows: BTreeSet<usize> = [1].into();
        assert_eq!(delete(&mut evs, Sel::Rows(&rows)), 1);
        assert_eq!(evs, vec![kd(0, KeyCode::KeyA), kd(30, KeyCode::KeyC)]);
    }

    #[test]
    fn replace_checks_bounds() {
        let mut evs = abc();
        assert!(replace(&mut evs, 2, kd(1, KeyCode::KeyZ)));
        assert!(!replace(&mut evs, 3, kd(1, KeyCode::KeyZ)));
        assert_eq!(evs[2], kd(1, KeyCode::KeyZ));
    }

    #[test]
    fn copy_selected_rows_in_order() {
        let rows: BTreeSet<usize> = [2, 0].into();
        assert_eq!(
            copy(&abc(), &rows),
            vec![kd(0, KeyCode::KeyA), kd(20, KeyCode::KeyC)]
        );
    }

    #[test]
    fn move_up_and_down_shift_selection() {
        let mut evs = abc();
        let rows: BTreeSet<usize> = [1, 2].into();
        let moved = move_up(&mut evs, &rows);
        assert_eq!(moved, [0, 1].into());
        assert_eq!(evs[2], kd(0, KeyCode::KeyA));
        assert_eq!(move_up(&mut evs, &moved), moved); // 맨 위에선 그대로
        let back = move_down(&mut evs, &moved);
        assert_eq!(back, [1, 2].into());
        assert_eq!(evs, abc());
        assert_eq!(move_down(&mut evs, &back), back); // 맨 아래에선 그대로
        assert_eq!(move_down(&mut evs, &BTreeSet::new()), BTreeSet::new());
    }
}
