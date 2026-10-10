//! 맥에서 primary_screen()이 올바른 포인트 크기·배율을 반환하는지 확인하는 개발용 도구.
fn main() {
    #[cfg(target_os = "macos")]
    {
        let s = mox_input::macos_probe_screen();
        println!("screen {}x{} scale={} unit={:?}", s.width, s.height, s.scale, s.unit);
    }
    #[cfg(not(target_os = "macos"))]
    println!("macOS 전용 도구입니다");
}
