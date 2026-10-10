//! 실제로 빌드된 mox-player 바이너리를 실행해 본다 (Linux 컨테이너에서도 같은 코드 경로).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mox_payload::{append_trailer, ExportSettings, Payload};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mox")
}

fn player_bytes() -> Vec<u8> {
    fs::read(env!("CARGO_BIN_EXE_mox-player")).unwrap()
}

fn write_exec(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

fn exported(dir: &Path) -> (PathBuf, usize) {
    let stub = player_bytes();
    let payload = Payload {
        settings: ExportSettings::default(),
        mox: mox_format::load(&fixture()).unwrap(),
    };
    let bytes = append_trailer(&stub, &payload.encode().unwrap());
    (write_exec(dir, "exported", &bytes), stub.len())
}

fn text(out: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn embedded_payload_is_found_and_summarized() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, _) = exported(dir.path());
    let out = Command::new(&exe).arg("--info").output().unwrap();
    let (stdout, stderr) = text(&out);
    assert!(out.status.success(), "{stderr}");
    assert!(stdout.contains("이름: 검증용 매크로"), "{stdout}");
    assert!(stdout.contains("이벤트: 2개"), "{stdout}");
    assert!(stdout.contains("예상 시간: 1.5초 (1회)"), "{stdout}");
    assert!(stdout.contains("출처: 내장"), "{stdout}");
}

#[test]
fn plain_player_reads_mox_file_argument() {
    let out = Command::new(env!("CARGO_BIN_EXE_mox-player"))
        .arg("--info")
        .arg(fixture())
        .output()
        .unwrap();
    let (stdout, stderr) = text(&out);
    assert!(out.status.success(), "{stderr}");
    assert!(stdout.contains("이벤트: 2개"), "{stdout}");
    assert!(stdout.contains("sample.mox"), "{stdout}");
}

#[test]
fn plain_player_without_input_explains_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_mox-player"))
        .output()
        .unwrap();
    let (_, stderr) = text(&out);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr.contains("사용법"), "{stderr}");
}

#[test]
fn corrupted_payload_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, stub_len) = exported(dir.path());
    let mut bytes = fs::read(&exe).unwrap();
    bytes[stub_len + 5] ^= 0xff;
    let broken = write_exec(dir.path(), "broken", &bytes);
    let out = Command::new(&broken).arg("--info").output().unwrap();
    let (_, stderr) = text(&out);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr.contains("손상"), "{stderr}");
}

#[test]
fn without_info_attempts_playback_and_fails_gracefully_without_backend() {
    // 이 바이너리는 리눅스 테스트 빌드라 입력 백엔드가 없다(unsupported). 재생을 시도하면
    // 패닉 없이 종료코드 1과 한국어 오류를 내야 한다. (macOS 실기 재생은 Task 7 수동 검증.)
    let dir = tempfile::tempdir().unwrap();
    let (exe, _) = exported(dir.path());
    let out = Command::new(&exe).output().unwrap();
    let (_, stderr) = text(&out);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr.contains("입력 계층"), "{stderr}");
}
