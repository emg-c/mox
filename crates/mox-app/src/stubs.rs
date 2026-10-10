//! build.rs가 OUT_DIR에 넣은 재생기 바이트. 비어 있으면 그 대상 추출은 비활성.

static WIN_PLAYER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/win_player.bin"));
static MAC_PLAYER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mac_player.bin"));

pub fn win() -> Option<&'static [u8]> {
    if WIN_PLAYER.is_empty() {
        None
    } else {
        Some(WIN_PLAYER)
    }
}

pub fn mac() -> Option<&'static [u8]> {
    if MAC_PLAYER.is_empty() {
        None
    } else {
        Some(MAC_PLAYER)
    }
}
