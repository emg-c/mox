use mox_format::Event;

use crate::{EditError, Sel};

pub fn set_delay(events: &mut [Event], sel: Sel, ms: u64) -> usize {
    let mut n = 0;
    for (i, e) in events.iter_mut().enumerate() {
        if sel.contains(i) {
            e.d = ms;
            n += 1;
        }
    }
    n
}

/// 간격에 k를 곱하고 반올림한다. u64를 넘으면 u64::MAX로 고정한다.
pub fn scale_delays(events: &mut [Event], sel: Sel, k: f64) -> Result<usize, EditError> {
    if !(k.is_finite() && k >= 0.0) {
        return Err(EditError::InvalidFactor(k));
    }
    let mut n = 0;
    for (i, e) in events.iter_mut().enumerate() {
        if sel.contains(i) {
            e.d = (e.d as f64 * k).round() as u64; // f64 → u64 `as`는 포화 변환
            n += 1;
        }
    }
    Ok(n)
}

/// max_ms를 넘는 간격을 max_ms로 줄인다. 바뀐 개수를 돌려준다.
pub fn cap_delays(events: &mut [Event], sel: Sel, max_ms: u64) -> usize {
    let mut n = 0;
    for (i, e) in events.iter_mut().enumerate() {
        if sel.contains(i) && e.d > max_ms {
            e.d = max_ms;
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use crate::Sel;
    use std::collections::BTreeSet;

    #[test]
    fn set_delay_only_on_selection() {
        let mut evs = vec![mv(0, 0, 0), mv(100, 1, 1), mv(200, 2, 2)];
        let rows: BTreeSet<usize> = [1].into();
        assert_eq!(set_delay(&mut evs, Sel::Rows(&rows), 7), 1);
        assert_eq!(delays(&evs), vec![0, 7, 200]);
    }

    #[test]
    fn scale_rounds_and_rejects_bad_factor() {
        let mut evs = vec![mv(0, 0, 0), mv(101, 1, 1), mv(3, 2, 2)];
        scale_delays(&mut evs, Sel::All, 0.5).unwrap();
        assert_eq!(delays(&evs), vec![0, 51, 2]);
        assert_eq!(
            scale_delays(&mut evs, Sel::All, -1.0),
            Err(crate::EditError::InvalidFactor(-1.0))
        );
        assert!(scale_delays(&mut evs, Sel::All, f64::NAN).is_err());
    }

    #[test]
    fn scale_saturates_huge_values() {
        let mut evs = vec![mv(u64::MAX, 0, 0)];
        scale_delays(&mut evs, Sel::All, 10.0).unwrap();
        assert_eq!(delays(&evs), vec![u64::MAX]);
    }

    #[test]
    fn cap_limits_long_waits() {
        let mut evs = vec![mv(0, 0, 0), mv(3000, 1, 1), mv(100, 2, 2)];
        assert_eq!(cap_delays(&mut evs, Sel::All, 500), 1);
        assert_eq!(delays(&evs), vec![0, 500, 100]);
    }
}
