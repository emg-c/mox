use std::path::PathBuf;
use std::process::ExitCode;

use mox_player::source::locate;
use mox_player::summary::{current_os, summary};

fn main() -> ExitCode {
    let mut info = false;
    let mut file: Option<PathBuf> = None;
    for arg in std::env::args_os().skip(1) {
        if arg == "--info" {
            info = true;
        } else if file.is_none() {
            file = Some(PathBuf::from(arg));
        }
    }
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("실행 파일 위치를 알 수 없습니다: {e}");
            return ExitCode::from(1);
        }
    };
    let loaded = match locate(&exe, file.as_deref()) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    print!("{}", summary(&loaded, current_os()));
    if info {
        return ExitCode::SUCCESS;
    }
    eprintln!("이 빌드에는 아직 재생 기능이 없습니다. 입력 계층은 다음 단계에서 추가됩니다.");
    ExitCode::from(2)
}
