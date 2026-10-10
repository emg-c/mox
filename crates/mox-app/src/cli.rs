//! 명령줄 인터페이스. 기존 크레이트 위의 얇은 글루.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use clap::{Parser, Subcommand};
use mox_edit::{
    cap_delays, remove_key_repeats, remove_mouse, remove_moves, scale_delays, set_delay,
    swap_cmd_ctrl, thin_moves_distance, thin_moves_half, thin_moves_interval, to_abs, to_ratio,
    Sel,
};
use mox_engine::{effective_stop_key, play, preflight, PlayOptions, RecordOptions};
use mox_format::{load, save, KeyCombo, Macro, Meta, Os, PlaySettings};
use mox_input::Access;
use mox_payload::{ExportSettings, Payload};

use crate::stubs;
use crate::timeutil::now_iso;

#[derive(Parser)]
#[command(name = "mox", version, about = "마우스·키보드 매크로 녹화·재생기")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 입력을 녹화해 .mox 파일로 저장
    Record {
        #[arg(short, long)]
        out: PathBuf,
        /// 중지 단축키(예: Control+Alt+Escape). 없으면 OS 기본.
        #[arg(long)]
        stop: Option<String>,
    },
    /// .mox 매크로 재생
    Play {
        file: PathBuf,
        #[arg(long)]
        repeat: Option<u32>,
        #[arg(long)]
        speed: Option<f64>,
        #[arg(long)]
        no_timing: bool,
        #[arg(long)]
        no_failsafe: bool,
    },
    /// 매크로 편집(제자리 수정, --out이면 다른 파일)
    Edit {
        file: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[command(subcommand)]
        op: EditOp,
    },
    /// 단독 실행 파일로 추출
    Export {
        file: PathBuf,
        /// windows | macos
        #[arg(long)]
        target: String,
        #[arg(short, long)]
        out: PathBuf,
        #[arg(long)]
        no_confirm: bool,
    },
    /// 입력 계층 자가 진단
    Selftest,
}

#[derive(Subcommand)]
enum EditOp {
    /// 모든 간격에 k를 곱함
    ScaleDelay { k: f64 },
    /// max_ms를 넘는 간격을 max_ms로
    CapDelay { ms: u64 },
    /// 모든 간격을 ms로
    SetDelay { ms: u64 },
    /// 연속 이동을 절반으로 솎음
    ThinHalf,
    /// 이동을 최소 간격(ms)으로 솎음
    ThinInterval { ms: u64 },
    /// 이동을 최소 거리(px)로 솎음
    ThinDistance { px: f64 },
    /// 이동 이벤트 삭제(클릭 유지)
    RmMoves,
    /// 마우스 이벤트 전부 삭제
    RmMouse,
    /// 절대 좌표 → 비율 좌표
    ToRatio,
    /// 비율 좌표 → 절대 좌표
    ToAbs,
    /// 키 자동반복 제거
    RmKeyRepeats,
    /// Cmd ↔ Ctrl 맞바꿈
    SwapCmdCtrl,
}

pub fn run() -> ExitCode {
    match Cli::parse().cmd {
        Cmd::Record { out, stop } => cmd_record(out, stop),
        Cmd::Play {
            file,
            repeat,
            speed,
            no_timing,
            no_failsafe,
        } => cmd_play(file, repeat, speed, no_timing, no_failsafe),
        Cmd::Edit { file, out, op } => cmd_edit(file, out, op),
        Cmd::Export {
            file,
            target,
            out,
            no_confirm,
        } => cmd_export(file, target, out, no_confirm),
        Cmd::Selftest => cmd_selftest(),
    }
}

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("{msg}");
    ExitCode::from(1)
}

fn current_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::Macos
    } else {
        Os::Windows
    }
}

fn cmd_record(out: PathBuf, stop: Option<String>) -> ExitCode {
    let os = current_os();
    let stop_key = match stop {
        Some(s) => match s.parse::<KeyCombo>() {
            Ok(k) => k,
            Err(e) => return fail(format!("중지 키 오류: {e}")),
        },
        None => KeyCombo::default_stop(os),
    };
    if !mox_input::permission_status().capture {
        mox_input::open_settings(Access::Capture);
        return fail(
            "녹화하려면 입력 모니터링 권한이 필요합니다. 설정에서 허용 후 다시 실행하세요.",
        );
    }
    let backend = match mox_input::backend() {
        Ok(b) => b,
        Err(e) => return fail(format!("입력 계층을 시작하지 못했습니다: {e}")),
    };
    println!("녹화 중… 중지하려면 {stop_key} 를 누르세요.");
    let cancel = AtomicBool::new(false);
    let events = match mox_engine::record(&*backend, RecordOptions::new(stop_key), &cancel) {
        Ok(e) => e,
        Err(e) => return fail(format!("녹화 오류: {e}")),
    };
    let name = out
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "매크로".into());
    let m = Macro {
        meta: Meta {
            name,
            created: now_iso(),
            recorded_on: os,
            screen: backend.primary_screen(),
        },
        play: PlaySettings::default(),
        events,
    };
    match save(&out, &m) {
        Ok(()) => {
            println!("저장: {} ({}개 이벤트)", out.display(), m.events.len());
            ExitCode::SUCCESS
        }
        Err(e) => fail(format!("저장 실패: {e}")),
    }
}

fn cmd_play(
    file: PathBuf,
    repeat: Option<u32>,
    speed: Option<f64>,
    no_timing: bool,
    no_failsafe: bool,
) -> ExitCode {
    let mut m = match load(&file) {
        Ok(m) => m,
        Err(e) => return fail(format!("불러오기 실패: {e}")),
    };
    if let Some(r) = repeat {
        m.play.repeat = r;
    }
    if let Some(s) = speed {
        m.play.speed = s;
    }
    if no_timing {
        m.play.timed = false;
    }
    let os = current_os();
    let perm = mox_input::permission_status();
    if !perm.inject {
        mox_input::open_settings(Access::Inject);
        return fail("재생하려면 손쉬운 사용 권한이 필요합니다.");
    }
    if !perm.capture {
        mox_input::open_settings(Access::Capture);
        return fail("중지 키 감지에 입력 모니터링 권한이 필요합니다.");
    }
    let backend = match mox_input::backend() {
        Ok(b) => b,
        Err(e) => return fail(format!("입력 계층 오류: {e}")),
    };
    let screen = backend.primary_screen();
    for w in preflight(&m, os, &screen) {
        eprintln!("⚠ {w:?}");
    }
    let clock = mox_input::clock();
    let stop = AtomicBool::new(false);
    let (stop_key, _) = effective_stop_key(&m.play, os);
    let opts = PlayOptions {
        stop_key,
        failsafe: !no_failsafe,
    };
    match play(&m, &*backend, &*clock, &stop, &opts) {
        Ok(r) => {
            println!(
                "재생 {:?}: 주입 {}개, 건너뜀 {}개",
                r.outcome, r.injected, r.skipped_keys
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(format!("재생 오류: {e}")),
    }
}

fn cmd_edit(file: PathBuf, out: Option<PathBuf>, op: EditOp) -> ExitCode {
    let mut m = match load(&file) {
        Ok(m) => m,
        Err(e) => return fail(format!("불러오기 실패: {e}")),
    };
    let screen = m.meta.screen;
    let n = match op {
        EditOp::ScaleDelay { k } => match scale_delays(&mut m.events, Sel::All, k) {
            Ok(n) => n,
            Err(e) => return fail(e),
        },
        EditOp::CapDelay { ms } => cap_delays(&mut m.events, Sel::All, ms),
        EditOp::SetDelay { ms } => set_delay(&mut m.events, Sel::All, ms),
        EditOp::ThinHalf => thin_moves_half(&mut m.events),
        EditOp::ThinInterval { ms } => thin_moves_interval(&mut m.events, ms),
        EditOp::ThinDistance { px } => thin_moves_distance(&mut m.events, px),
        EditOp::RmMoves => remove_moves(&mut m.events),
        EditOp::RmMouse => remove_mouse(&mut m.events),
        EditOp::ToRatio => match to_ratio(&mut m.events, Sel::All, &screen) {
            Ok(n) => n,
            Err(e) => return fail(e),
        },
        EditOp::ToAbs => match to_abs(&mut m.events, Sel::All, &screen) {
            Ok(n) => n,
            Err(e) => return fail(e),
        },
        EditOp::RmKeyRepeats => remove_key_repeats(&mut m.events),
        EditOp::SwapCmdCtrl => swap_cmd_ctrl(&mut m.events, Sel::All),
    };
    let dest = out.unwrap_or(file);
    match save(&dest, &m) {
        Ok(()) => {
            println!(
                "{}개 적용 → {} ({}개 이벤트)",
                n,
                dest.display(),
                m.events.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(format!("저장 실패: {e}")),
    }
}

fn cmd_export(file: PathBuf, target: String, out: PathBuf, no_confirm: bool) -> ExitCode {
    let mox = match load(&file) {
        Ok(m) => m,
        Err(e) => return fail(format!("불러오기 실패: {e}")),
    };
    let mut settings = ExportSettings::default();
    if no_confirm {
        settings.confirm = false;
    }
    let payload = Payload { settings, mox };
    let result: Result<Vec<u8>, String> = match target.as_str() {
        "windows" => match stubs::win() {
            Some(stub) => mox_export::export_windows(stub, &payload).map_err(|e| e.to_string()),
            None => Err("이 빌드에는 Windows 재생기가 내장되지 않았습니다.".into()),
        },
        "macos" => match stubs::mac() {
            Some(stub) => match tempfile::tempdir() {
                Ok(work) => mox_export::export_macos_zip(stub, &payload, work.path())
                    .map_err(|e| e.to_string()),
                Err(e) => Err(format!("임시 폴더 오류: {e}")),
            },
            None => Err("이 빌드에는 macOS 재생기가 내장되지 않았습니다.".into()),
        },
        other => Err(format!("알 수 없는 대상입니다: {other} (windows|macos)")),
    };
    match result {
        Ok(bytes) => match std::fs::write(&out, bytes) {
            Ok(()) => {
                println!("추출: {}", out.display());
                ExitCode::SUCCESS
            }
            Err(e) => fail(format!("저장 실패: {e}")),
        },
        Err(e) => fail(e),
    }
}

fn cmd_selftest() -> ExitCode {
    match mox_input::selftest() {
        Ok(r) => {
            println!("{r}");
            if r.ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => fail(format!("selftest 실패: {e}")),
    }
}
