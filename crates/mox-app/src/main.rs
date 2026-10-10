mod cli;
mod stubs;
mod timeutil;

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args_os().count() > 1 {
        cli::run()
    } else {
        eprintln!("사용법: mox <명령>  (자세히: mox --help)");
        eprintln!("GUI는 다음 빌드 단계에서 추가됩니다.");
        ExitCode::from(1)
    }
}
