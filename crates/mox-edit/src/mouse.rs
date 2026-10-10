use mox_format::{Event, Pos};

use crate::remove_where;

/// 연속된 이동 구간마다 (구간 안 순번, 구간 길이)를 구한다. 이동이 아니면 None.
fn move_runs(events: &[Event]) -> Vec<Option<(usize, usize)>> {
    let mut out = vec![None; events.len()];
    let mut i = 0;
    while i < events.len() {
        if !events[i].is_move() {
            i += 1;
            continue;
        }
        let start = i;
        while i < events.len() && events[i].is_move() {
            i += 1;
        }
        let len = i - start;
        for (k, slot) in out[start..i].iter_mut().enumerate() {
            *slot = Some((k, len));
        }
    }
    out
}

/// 연속 이동 구간에서 하나 걸러 지운다. 구간의 마지막 이동(멈춘 위치)은 남긴다.
pub fn thin_moves_half(events: &mut Vec<Event>) -> usize {
    let runs = move_runs(events);
    remove_where(
        events,
        |i, _| matches!(runs[i], Some((k, len)) if k % 2 == 1 && k + 1 != len),
    )
}

/// 직전에 남긴 이동으로부터 min_ms가 지나지 않은 이동을 지운다. 구간의 마지막 이동은 남긴다.
pub fn thin_moves_interval(events: &mut Vec<Event>, min_ms: u64) -> usize {
    let runs = move_runs(events);
    let mut since_kept: u64 = 0;
    let drop: Vec<bool> = events
        .iter()
        .enumerate()
        .map(|(i, e)| match runs[i] {
            Some((k, len)) => {
                since_kept = since_kept.saturating_add(e.d);
                let keep = k == 0 || k + 1 == len || since_kept >= min_ms;
                if keep {
                    since_kept = 0;
                }
                !keep
            }
            None => false,
        })
        .collect();
    remove_where(events, |i, _| drop[i])
}

/// 직전에 남긴 이동으로부터 min_px보다 가까운 이동을 지운다. 절대 좌표만 대상이고, 구간의 마지막 이동은 남긴다.
pub fn thin_moves_distance(events: &mut Vec<Event>, min_px: f64) -> usize {
    let runs = move_runs(events);
    let mut last: Option<(i32, i32)> = None;
    let drop: Vec<bool> = events
        .iter()
        .enumerate()
        .map(|(i, e)| match (runs[i], e.pos()) {
            (Some((k, len)), Some(Pos::Abs { x, y })) => {
                let far = match last {
                    Some((lx, ly)) if k != 0 => {
                        let (dx, dy) = (f64::from(x) - f64::from(lx), f64::from(y) - f64::from(ly));
                        (dx * dx + dy * dy).sqrt() >= min_px
                    }
                    _ => true,
                };
                let keep = far || k + 1 == len;
                if keep {
                    last = Some((x, y));
                }
                !keep
            }
            _ => false,
        })
        .collect();
    remove_where(events, |i, _| drop[i])
}

pub fn remove_moves(events: &mut Vec<Event>) -> usize {
    remove_where(events, |_, e| e.is_move())
}

/// 마우스 이벤트(이동·클릭·휠)를 모두 지운다. 원본 원본 매크로 도구의 "마우스 이벤트 없애기".
pub fn remove_mouse(events: &mut Vec<Event>) -> usize {
    remove_where(events, |_, e| e.is_mouse())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use mox_format::KeyCode;

    #[test]
    fn half_keeps_every_other_and_last_of_run() {
        // 이동 5개 구간: 0,2,4번째(마지막) 유지 → 1,3번째 삭제
        let mut evs = vec![
            mv(0, 0, 0),
            mv(10, 1, 0),
            mv(10, 2, 0),
            mv(10, 3, 0),
            mv(10, 4, 0),
            down(10, 4, 0),
        ];
        assert_eq!(thin_moves_half(&mut evs), 2);
        assert_eq!(
            evs,
            vec![mv(0, 0, 0), mv(20, 2, 0), mv(20, 4, 0), down(10, 4, 0)]
        );
    }

    #[test]
    fn half_treats_runs_separately() {
        let mut evs = vec![
            mv(0, 0, 0),
            mv(10, 1, 0),
            kd(5, KeyCode::KeyA),
            mv(10, 2, 0),
            mv(10, 3, 0),
        ];
        assert_eq!(thin_moves_half(&mut evs), 0); // 구간 길이 2: 첫째와 마지막만 있음
    }

    #[test]
    fn interval_keeps_moves_spaced_at_least_min() {
        let mut evs = vec![
            mv(0, 0, 0),
            mv(5, 1, 0),
            mv(5, 2, 0),
            mv(5, 3, 0),
            mv(5, 4, 0),
        ];
        assert_eq!(thin_moves_interval(&mut evs, 10), 2);
        assert_eq!(evs, vec![mv(0, 0, 0), mv(10, 2, 0), mv(10, 4, 0)]);
        assert_eq!(total(&evs), 20);
    }

    #[test]
    fn distance_keeps_far_moves_and_last() {
        let mut evs = vec![
            mv(0, 0, 0),
            mv(1, 1, 0),
            mv(1, 2, 0),
            mv(1, 10, 0),
            mv(1, 11, 0),
        ];
        assert_eq!(thin_moves_distance(&mut evs, 5.0), 2);
        assert_eq!(evs, vec![mv(0, 0, 0), mv(3, 10, 0), mv(1, 11, 0)]);
    }

    #[test]
    fn remove_moves_keeps_clicks_and_time() {
        let mut evs = vec![mv(0, 0, 0), down(10, 1, 1), mv(5, 2, 2), up(5, 2, 2)];
        assert_eq!(remove_moves(&mut evs), 2);
        assert_eq!(evs, vec![down(10, 1, 1), up(10, 2, 2)]);
    }

    #[test]
    fn remove_mouse_keeps_only_keys() {
        let mut evs = vec![
            mv(0, 0, 0),
            kd(10, KeyCode::KeyA),
            down(5, 1, 1),
            up(5, 1, 1),
            ku(5, KeyCode::KeyA),
        ];
        assert_eq!(remove_mouse(&mut evs), 3);
        assert_eq!(evs, vec![kd(10, KeyCode::KeyA), ku(15, KeyCode::KeyA)]);
    }
}
