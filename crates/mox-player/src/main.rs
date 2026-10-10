use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;

use mox_engine::{effective_stop_key, play, preflight, PlayOptions};
use mox_input::{backend, clock, confirm, open_settings, permission_status, Access};
use mox_player::source::{locate, Loaded};
use mox_player::summary::{current_os, summary};

fn main() -> ExitCode {
    let mut info = false;
    let mut want_selftest = false;
    let mut file: Option<PathBuf> = None;
    for arg in std::env::args_os().skip(1) {
        if arg == "--info" {
            info = true;
        } else if arg == "selftest" {
            want_selftest = true;
        } else if file.is_none() {
            file = Some(PathBuf::from(arg));
        }
    }

    if want_selftest {
        return match mox_input::selftest() {
            Ok(report) => {
                println!("{report}");
                if report.ok() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("selftest 실패: {e}");
                ExitCode::from(1)
            }
        };
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

    if info {
        print!("{}", summary(&loaded, current_os()));
        return ExitCode::SUCCESS;
    }

    play_loaded(loaded)
}

fn play_loaded(loaded: Loaded) -> ExitCode {
    let os = current_os();

    // 재생에는 손쉬운 사용(주입)과 입력 모니터링(중지 키 감지)이 모두 필요하다.
    let perm = permission_status();
    if !perm.inject {
        eprintln!(
            "재생하려면 '손쉬운 사용' 권한이 필요합니다. 설정에서 허용한 뒤 다시 실행하세요."
        );
        open_settings(Access::Inject);
        return ExitCode::from(1);
    }
    if !perm.capture {
        eprintln!("중지 키 감지를 위해 '입력 모니터링' 권한이 필요합니다. 설정에서 허용한 뒤 다시 실행하세요.");
        open_settings(Access::Capture);
        return ExitCode::from(1);
    }

    let b = match backend() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("입력 계층을 시작하지 못했습니다: {e}");
            return ExitCode::from(1);
        }
    };
    let screen = b.primary_screen();
    let warnings = preflight(&loaded.payload.mox, os, &screen);

    if loaded.payload.settings.confirm {
        let mut message = summary(&loaded, os);
        for w in &warnings {
            message.push_str(&format!("\n⚠ {w:?}"));
        }
        if !confirm("Mox 매크로 재생", &message) {
            println!("취소되었습니다.");
            return ExitCode::SUCCESS;
        }
    }

    let countdown = loaded.payload.settings.countdown_s;
    if countdown > 0 {
        for n in (1..=countdown).rev() {
            println!("{n}...");
            thread::sleep(Duration::from_secs(1));
        }
    }

    let clk = clock();
    let stop = AtomicBool::new(false);
    let (stop_key, _) = effective_stop_key(&loaded.payload.mox.play, os);
    let opts = PlayOptions {
        stop_key,
        failsafe: true,
    };
    match play(&loaded.payload.mox, &*b, &*clk, &stop, &opts) {
        Ok(report) => {
            println!(
                "재생 완료: {:?}, 주입 {}개, 건너뜀 {}개",
                report.outcome, report.injected, report.skipped_keys
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("재생 중 오류: {e}");
            ExitCode::from(1)
        }
    }
}
