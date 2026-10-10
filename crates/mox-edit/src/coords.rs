use mox_format::{Event, Pos, ScreenInfo};

use crate::{EditError, Sel};

fn dims(screen: &ScreenInfo) -> Result<(f64, f64), EditError> {
    if screen.width == 0 || screen.height == 0 {
        return Err(EditError::ZeroScreen);
    }
    Ok((f64::from(screen.width), f64::from(screen.height)))
}

/// 선택한 행의 절대 좌표를 주 모니터 기준 비율 좌표로 바꾼다.
pub fn to_ratio(events: &mut [Event], sel: Sel, screen: &ScreenInfo) -> Result<usize, EditError> {
    let (w, h) = dims(screen)?;
    let mut n = 0;
    for (i, e) in events.iter_mut().enumerate() {
        if let (true, Some(Pos::Abs { x, y })) = (sel.contains(i), e.pos()) {
            e.set_pos(Pos::Ratio {
                rx: f64::from(x) / w,
                ry: f64::from(y) / h,
            });
            n += 1;
        }
    }
    Ok(n)
}

/// 선택한 행의 비율 좌표를 절대 좌표로 바꾼다.
pub fn to_abs(events: &mut [Event], sel: Sel, screen: &ScreenInfo) -> Result<usize, EditError> {
    let (w, h) = dims(screen)?;
    let mut n = 0;
    for (i, e) in events.iter_mut().enumerate() {
        if let (true, Some(Pos::Ratio { rx, ry })) = (sel.contains(i), e.pos()) {
            e.set_pos(Pos::Abs {
                x: (rx * w).round() as i32,
                y: (ry * h).round() as i32,
            });
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use crate::{EditError, Sel};
    use mox_format::{EventKind, Pos, ScreenInfo, Unit};
    use std::collections::BTreeSet;

    fn screen(w: u32, h: u32) -> ScreenInfo {
        ScreenInfo {
            width: w,
            height: h,
            scale: 1.0,
            unit: Unit::Pixel,
        }
    }

    #[test]
    fn converts_selected_rows_and_back() {
        let mut evs = vec![mv(0, 500, 250), down(10, -100, 750)];
        let rows: BTreeSet<usize> = [0, 1].into();
        assert_eq!(
            to_ratio(&mut evs, Sel::Rows(&rows), &screen(1000, 500)).unwrap(),
            2
        );
        assert_eq!(evs[0].pos(), Some(Pos::Ratio { rx: 0.5, ry: 0.5 }));
        assert_eq!(evs[1].pos(), Some(Pos::Ratio { rx: -0.1, ry: 1.5 }));
        assert_eq!(to_abs(&mut evs, Sel::All, &screen(1000, 500)).unwrap(), 2);
        assert_eq!(evs, vec![mv(0, 500, 250), down(10, -100, 750)]);
    }

    #[test]
    fn skips_keys_and_already_converted() {
        let mut evs = vec![kd(0, mox_format::KeyCode::KeyA), mv(0, 1, 1)];
        assert_eq!(to_ratio(&mut evs, Sel::All, &screen(10, 10)).unwrap(), 1);
        assert_eq!(to_ratio(&mut evs, Sel::All, &screen(10, 10)).unwrap(), 0);
        assert!(matches!(evs[0].kind, EventKind::KeyDown(_)));
    }

    #[test]
    fn rejects_zero_screen() {
        let mut evs = vec![mv(0, 1, 1)];
        assert_eq!(
            to_ratio(&mut evs, Sel::All, &screen(0, 10)),
            Err(EditError::ZeroScreen)
        );
        assert_eq!(
            to_abs(&mut evs, Sel::All, &screen(10, 0)),
            Err(EditError::ZeroScreen)
        );
    }
}
