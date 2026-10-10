#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("파일을 읽지 못했습니다: {0}")]
    Parse(String),
    #[error("mox 파일이 아닙니다 (format = {0:?})")]
    NotMox(String),
    #[error("지원하지 않는 버전입니다: {0} (지원: 1)")]
    UnsupportedVersion(u64),
    #[error("이벤트 #{index}: {reason}")]
    BadEvent { index: usize, reason: String },
    #[error("재생 설정 오류: {0}")]
    BadPlay(String),
    #[error("메타 정보 오류: {0}")]
    BadMeta(String),
    #[error("파일 입출력 오류: {0}")]
    Io(#[from] std::io::Error),
}
