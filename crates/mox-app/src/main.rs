mod cli;
mod stubs;
mod timeutil;

#[cfg(feature = "gui")]
mod gui;

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args_os().count() > 1 {
        cli::run()
    } else {
        #[cfg(feature = "gui")]
        {
            gui::run()
        }
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("사용법: mox <명령>  (자세히: mox --help)");
            eprintln!("이 빌드에는 GUI가 포함되지 않았습니다.");
            ExitCode::from(1)
        }
    }
}
