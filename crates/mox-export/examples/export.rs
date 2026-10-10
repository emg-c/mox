//! 개발용: 미리 빌드한 재생기에 .mox를 결합해 추출 파일을 만든다.
//! cargo run -p mox-export --example export -- <재생기> <windows|macos> <입력.mox> <출력>

use std::path::Path;

use mox_payload::{ExportSettings, Payload};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [stub, target, input, output] = args.as_slice() else {
        return Err("사용법: export <재생기> <windows|macos> <입력.mox> <출력>".into());
    };
    let stub = std::fs::read(stub)?;
    let payload = Payload {
        settings: ExportSettings::default(),
        mox: mox_format::load(Path::new(input))?,
    };
    let bytes = match target.as_str() {
        "windows" => mox_export::export_windows(&stub, &payload)?,
        "macos" => {
            let work = tempfile::tempdir()?;
            mox_export::export_macos_zip(&stub, &payload, work.path())?
        }
        other => return Err(format!("알 수 없는 대상입니다: {other}").into()),
    };
    std::fs::write(output, bytes)?;
    println!("{output}");
    Ok(())
}
