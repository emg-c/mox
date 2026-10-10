use mox_format::Event;

/// 조건에 맞는 이벤트를 지운다. 지운 이벤트의 간격(d)은 다음에 남는 이벤트에 더해서
/// 나머지 이벤트의 재생 시각이 바뀌지 않게 한다. 지운 개수를 돌려준다.
pub fn remove_where(events: &mut Vec<Event>, mut pred: impl FnMut(usize, &Event) -> bool) -> usize {
    let mut carry: u64 = 0;
    let mut removed = 0;
    let mut kept = Vec::with_capacity(events.len());
    for (i, mut e) in events.drain(..).enumerate() {
        if pred(i, &e) {
            carry = carry.saturating_add(e.d);
            removed += 1;
        } else {
            e.d = e.d.saturating_add(carry);
            carry = 0;
            kept.push(e);
        }
    }
    *events = kept;
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use mox_format::KeyCode;

    #[test]
    fn carries_removed_delay_to_next_kept_event() {
        let mut evs = vec![
            kd(0, KeyCode::KeyA),
            mv(100, 1, 1),
            mv(50, 2, 2),
            ku(30, KeyCode::KeyA),
        ];
        let n = remove_where(&mut evs, |_, e| e.is_move());
        assert_eq!(n, 2);
        assert_eq!(delays(&evs), vec![0, 180]);
    }

    #[test]
    fn trailing_removed_delays_are_dropped() {
        let mut evs = vec![kd(0, KeyCode::KeyA), mv(100, 1, 1)];
        remove_where(&mut evs, |_, e| e.is_move());
        assert_eq!(delays(&evs), vec![0]);
    }

    #[test]
    fn carry_saturates_instead_of_overflowing() {
        let mut evs = vec![mv(u64::MAX, 1, 1), kd(u64::MAX, KeyCode::KeyA)];
        remove_where(&mut evs, |_, e| e.is_move());
        assert_eq!(delays(&evs), vec![u64::MAX]);
    }
}
