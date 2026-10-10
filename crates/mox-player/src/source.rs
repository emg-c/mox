use std::fs;
use std::path::{Path, PathBuf};

use mox_payload::{read_trailer, ExportSettings, Payload, PayloadError};

#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    /// 실행 파일 끝에 붙은 매크로 (Windows 추출본).
    Embedded,
    /// macOS 번들의 Resources/macro.mox.
    Bundle(PathBuf),
    /// 인자로 받은 .mox 파일.
    File(PathBuf),
}

#[derive(Debug)]
pub struct Loaded {
    pub origin: Origin,
    pub payload: Payload,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("재생할 매크로가 없습니다. 사용법: mox-player [--info] <파일.mox>")]
    Nothing,
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Payload(#[from] PayloadError),
    #[error("{}: {source}", path.display())]
    Format {
        path: PathBuf,
        source: mox_format::FormatError,
    },
}

fn load_mox(path: &Path) -> Result<mox_format::Macro, SourceError> {
    mox_format::load(path).map_err(|source| SourceError::Format {
        path: path.to_path_buf(),
        source,
    })
}

/// exe가 `X.app/Contents/MacOS/<exe>`이면 `X.app/Contents/Resources`.
fn bundle_resources(exe: &Path) -> Option<PathBuf> {
    let macos_dir = exe.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }
    Some(macos_dir.parent()?.join("Resources"))
}

/// 재생할 매크로를 찾는다. 순서: 실행 파일 끝의 내장 데이터 → macOS 번들 Resources → 인자로 받은 .mox.
pub fn locate(exe: &Path, arg: Option<&Path>) -> Result<Loaded, SourceError> {
    let bytes = fs::read(exe).map_err(|source| SourceError::Io {
        path: exe.to_path_buf(),
        source,
    })?;
    if let Some(data) = read_trailer(&bytes)? {
        return Ok(Loaded {
            origin: Origin::Embedded,
            payload: Payload::decode(data)?,
        });
    }
    if let Some(res) = bundle_resources(exe) {
        let mox_path = res.join("macro.mox");
        if mox_path.is_file() {
            let mox = load_mox(&mox_path)?;
            let settings = match fs::read_to_string(res.join("export.json")) {
                Ok(s) => {
                    serde_json::from_str(&s).map_err(|e| PayloadError::Corrupt(e.to_string()))?
                }
                Err(_) => ExportSettings::default(),
            };
            return Ok(Loaded {
                origin: Origin::Bundle(mox_path),
                payload: Payload { settings, mox },
            });
        }
    }
    if let Some(path) = arg {
        // .mox 파일을 직접 열 때는 남에게 받은 파일일 수 있으므로 확인 창을 끌 수 없다
        let payload = Payload {
            settings: ExportSettings::default(),
            mox: load_mox(path)?,
        };
        return Ok(Loaded {
            origin: Origin::File(path.to_path_buf()),
            payload,
        });
    }
    Err(SourceError::Nothing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_payload::append_trailer;
    use std::path::Path;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mox")
    }

    fn payload(confirm: bool) -> Payload {
        Payload {
            settings: ExportSettings {
                confirm,
                countdown_s: 0,
            },
            mox: mox_format::load(&fixture()).unwrap(),
        }
    }

    #[test]
    fn prefers_embedded_payload_over_argument() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("exported");
        fs::write(
            &exe,
            append_trailer(b"stub", &payload(false).encode().unwrap()),
        )
        .unwrap();
        let l = locate(&exe, Some(&fixture())).unwrap();
        assert_eq!(l.origin, Origin::Embedded);
        assert!(!l.payload.settings.confirm);
    }

    #[test]
    fn reads_macos_bundle_resources() {
        let dir = tempfile::tempdir().unwrap();
        let contents = dir.path().join("검증.app/Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        fs::create_dir_all(contents.join("Resources")).unwrap();
        let exe = contents.join("MacOS/mox-player");
        fs::write(&exe, b"stub").unwrap();
        fs::copy(fixture(), contents.join("Resources/macro.mox")).unwrap();
        fs::write(
            contents.join("Resources/export.json"),
            r#"{"confirm":false,"countdown_s":0}"#,
        )
        .unwrap();
        let l = locate(&exe, None).unwrap();
        assert_eq!(
            l.origin,
            Origin::Bundle(contents.join("Resources/macro.mox"))
        );
        assert_eq!(
            l.payload.settings,
            ExportSettings {
                confirm: false,
                countdown_s: 0
            }
        );
    }

    #[test]
    fn bundle_without_export_json_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let contents = dir.path().join("a.app/Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        fs::create_dir_all(contents.join("Resources")).unwrap();
        fs::write(contents.join("MacOS/mox-player"), b"stub").unwrap();
        fs::copy(fixture(), contents.join("Resources/macro.mox")).unwrap();
        let l = locate(&contents.join("MacOS/mox-player"), None).unwrap();
        assert_eq!(l.payload.settings, ExportSettings::default());
    }

    #[test]
    fn opening_mox_file_always_confirms() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("mox-player");
        fs::write(&exe, b"plain player").unwrap();
        let l = locate(&exe, Some(&fixture())).unwrap();
        assert_eq!(l.origin, Origin::File(fixture()));
        assert!(l.payload.settings.confirm);
    }

    #[test]
    fn nothing_to_play_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("mox-player");
        fs::write(&exe, b"plain player").unwrap();
        assert!(matches!(locate(&exe, None), Err(SourceError::Nothing)));
        assert!(matches!(
            locate(&exe, Some(&dir.path().join("없음.mox"))),
            Err(SourceError::Format { .. })
        ));
    }
}
