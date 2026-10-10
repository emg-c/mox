use crate::error::FormatError;
use crate::model::{Macro, Pos};

/// 파일에 쓰기 전과 읽은 직후에 값이 허용 범위인지 확인한다.
pub fn validate(m: &Macro) -> Result<(), FormatError> {
    let speed = m.play.speed;
    if !(speed.is_finite() && (0.1..=10.0).contains(&speed)) {
        return Err(FormatError::BadPlay(format!(
            "speed는 0.1~10 사이여야 합니다: {speed}"
        )));
    }
    let scale = m.meta.screen.scale;
    if !(scale.is_finite() && scale > 0.0) {
        return Err(FormatError::BadMeta(format!(
            "screen.scale은 0보다 커야 합니다: {scale}"
        )));
    }
    for (i, e) in m.events.iter().enumerate() {
        if let Some(Pos::Ratio { rx, ry }) = e.pos() {
            if !rx.is_finite() || !ry.is_finite() {
                return Err(FormatError::BadEvent {
                    index: i + 1,
                    reason: "비율 좌표가 유한한 수가 아닙니다".into(),
                });
            }
        }
    }
    Ok(())
}
