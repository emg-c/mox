//! 추출한 실행 파일에 붙는 매크로 데이터: 내보내기 설정 + `.mox` 본문, 그리고 실행 파일 끝 트레일러.

mod envelope;
mod trailer;

pub use envelope::{ExportSettings, Payload};
pub use trailer::{append_trailer, read_trailer, MAGIC, PAYLOAD_VERSION, TRAILER_LEN};

#[derive(Debug, thiserror::Error)]
pub enum PayloadError {
    #[error("내장 매크로가 손상됐습니다: {0}")]
    Corrupt(String),
    #[error("지원하지 않는 내장 데이터 버전입니다: {0} (지원: 1)")]
    UnsupportedVersion(u32),
    #[error(transparent)]
    Format(#[from] mox_format::FormatError),
}

#[cfg(test)]
pub(crate) mod testutil {
    use mox_format::{
        Event, EventKind, Key, KeyCode, Macro, Meta, Os, PlaySettings, ScreenInfo, Unit,
    };

    pub fn sample() -> Macro {
        Macro {
            meta: Meta {
                name: "주간보고 정리".into(),
                created: "2026-10-10T14:03:00+09:00".into(),
                recorded_on: Os::Macos,
                screen: ScreenInfo {
                    width: 1512,
                    height: 982,
                    scale: 2.0,
                    unit: Unit::Point,
                },
            },
            play: PlaySettings::default(),
            events: vec![
                Event::new(0, EventKind::KeyDown(Key::Code(KeyCode::ShiftLeft))),
                Event::new(1500, EventKind::KeyUp(Key::Code(KeyCode::ShiftLeft))),
            ],
        }
    }
}
