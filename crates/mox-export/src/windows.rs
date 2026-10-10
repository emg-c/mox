use mox_payload::{append_trailer, read_trailer, Payload};

use crate::ExportError;

/// Windows 재생기 실행 파일 끝에 매크로를 붙인다.
pub fn export_windows(stub: &[u8], payload: &Payload) -> Result<Vec<u8>, ExportError> {
    if read_trailer(stub)?.is_some() {
        return Err(ExportError::AlreadyExported);
    }
    Ok(append_trailer(stub, &payload.encode()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::payload;
    use mox_payload::{read_trailer, Payload};

    #[test]
    fn appends_payload_readable_by_player() {
        let p = payload("주간보고");
        let out = export_windows(b"MZ-fake-stub", &p).unwrap();
        assert!(out.starts_with(b"MZ-fake-stub"));
        let data = read_trailer(&out).unwrap().unwrap();
        assert_eq!(Payload::decode(data).unwrap(), p);
    }

    #[test]
    fn refuses_already_exported_stub() {
        let once = export_windows(b"MZ", &payload("a")).unwrap();
        assert!(matches!(
            export_windows(&once, &payload("b")),
            Err(ExportError::AlreadyExported)
        ));
    }
}
