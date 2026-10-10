use crate::PayloadError;

pub const MAGIC: &[u8; 8] = b"MOXPAYLD";
pub const TRAILER_LEN: usize = 24;
pub const PAYLOAD_VERSION: u32 = 1;

/// 재생기 뒤에 데이터와 트레일러를 붙인다.
/// `[stub][data][MAGIC | 길이 u64 LE | 버전 u32 LE | CRC32 u32 LE]`
pub fn append_trailer(stub: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(stub.len() + data.len() + TRAILER_LEN);
    out.extend_from_slice(stub);
    out.extend_from_slice(data);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend_from_slice(&PAYLOAD_VERSION.to_le_bytes());
    out.extend_from_slice(&crc32fast::hash(data).to_le_bytes());
    out
}

fn le_u64(b: &[u8]) -> u64 {
    u64::from_le_bytes(b.try_into().expect("8바이트"))
}

fn le_u32(b: &[u8]) -> u32 {
    u32::from_le_bytes(b.try_into().expect("4바이트"))
}

/// 파일 끝의 트레일러를 읽어 데이터를 돌려준다. 트레일러가 없으면 `Ok(None)`.
pub fn read_trailer(file: &[u8]) -> Result<Option<&[u8]>, PayloadError> {
    let Some(body_end) = file.len().checked_sub(TRAILER_LEN) else {
        return Ok(None);
    };
    let t = &file[body_end..];
    if &t[0..8] != MAGIC {
        return Ok(None);
    }
    let version = le_u32(&t[16..20]);
    if version != PAYLOAD_VERSION {
        return Err(PayloadError::UnsupportedVersion(version));
    }
    let len = usize::try_from(le_u64(&t[8..16]))
        .ok()
        .filter(|&l| l <= body_end)
        .ok_or_else(|| PayloadError::Corrupt("데이터 길이가 파일보다 깁니다".into()))?;
    let data = &file[body_end - len..body_end];
    if crc32fast::hash(data) != le_u32(&t[20..24]) {
        return Err(PayloadError::Corrupt("CRC가 맞지 않습니다".into()));
    }
    Ok(Some(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_data_after_stub() {
        let out = append_trailer(b"STUB-BYTES", b"{\"hello\":1}");
        assert_eq!(out.len(), 10 + 11 + TRAILER_LEN);
        assert_eq!(&out[out.len() - TRAILER_LEN..out.len() - 16], MAGIC);
        assert_eq!(read_trailer(&out).unwrap(), Some(&b"{\"hello\":1}"[..]));
    }

    #[test]
    fn empty_data_round_trips() {
        assert_eq!(
            read_trailer(&append_trailer(b"x", b"")).unwrap(),
            Some(&b""[..])
        );
    }

    #[test]
    fn plain_files_have_no_trailer() {
        assert_eq!(read_trailer(b"").unwrap(), None);
        assert_eq!(
            read_trailer(b"just a normal executable with no payload").unwrap(),
            None
        );
    }

    #[test]
    fn detects_flipped_byte() {
        let mut out = append_trailer(b"stub", b"payload-data");
        out[6] ^= 0xff; // 데이터 영역
        assert!(matches!(read_trailer(&out), Err(PayloadError::Corrupt(_))));
    }

    #[test]
    fn detects_length_longer_than_file() {
        let mut out = append_trailer(b"", b"abc");
        let n = out.len();
        out[n - 16..n - 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(matches!(read_trailer(&out), Err(PayloadError::Corrupt(_))));
    }

    #[test]
    fn rejects_other_versions() {
        let mut out = append_trailer(b"", b"abc");
        let n = out.len();
        out[n - 8..n - 4].copy_from_slice(&2u32.to_le_bytes());
        assert!(matches!(
            read_trailer(&out),
            Err(PayloadError::UnsupportedVersion(2))
        ));
    }
}
