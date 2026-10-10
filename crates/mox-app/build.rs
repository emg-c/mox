//! 재생기 바이트를 OUT_DIR에 복사해 include_bytes!로 내장할 수 있게 한다.
//! 환경변수 MOX_WIN_PLAYER / MOX_MAC_PLAYER가 가리키는 파일을 넣는다. 없으면 빈 파일.

use std::{env, fs, path::Path};

fn main() {
    let out = env::var("OUT_DIR").expect("OUT_DIR");
    for (var, name) in [
        ("MOX_WIN_PLAYER", "win_player.bin"),
        ("MOX_MAC_PLAYER", "mac_player.bin"),
    ] {
        let dest = Path::new(&out).join(name);
        match env::var(var) {
            Ok(p) if !p.is_empty() => {
                println!("cargo:rerun-if-changed={p}");
                fs::copy(&p, &dest).unwrap_or_else(|e| panic!("{var}={p} 복사 실패: {e}"));
            }
            _ => {
                let _ = fs::write(&dest, []);
            }
        }
        println!("cargo:rerun-if-env-changed={var}");
    }
}
