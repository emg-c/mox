//! 매크로를 Windows `.exe` 또는 macOS `.app`(zip)로 추출한다. 컴파일 없이 미리 빌드한 재생기에 매크로를 결합한다.

mod bundle;
mod names;
mod windows;
mod zipdir;

pub use bundle::{bundle_id, export_macos_zip, info_plist, write_bundle, EXECUTABLE};
pub use names::safe_file_name;
pub use windows::export_windows;
pub use zipdir::zip_dir;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("이미 매크로가 들어 있는 실행 파일은 원본으로 쓸 수 없습니다")]
    AlreadyExported,
    #[error(transparent)]
    Payload(#[from] mox_payload::PayloadError),
    #[error("파일 입출력 오류: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip을 만들지 못했습니다: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("macOS 서명에 실패했습니다: {0}")]
    Sign(String),
}

#[cfg(test)]
pub(crate) mod testutil {
    use mox_format::{
        Event, EventKind, Key, KeyCode, Macro, Meta, Os, PlaySettings, ScreenInfo, Unit,
    };
    use mox_payload::{ExportSettings, Payload};

    pub fn payload(name: &str) -> Payload {
        Payload {
            settings: ExportSettings::default(),
            mox: Macro {
                meta: Meta {
                    name: name.into(),
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
                events: vec![Event::new(
                    0,
                    EventKind::KeyDown(Key::Code(KeyCode::ShiftLeft)),
                )],
            },
        }
    }
}
