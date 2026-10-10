//! 녹화·재생 엔진. OS 입력 계층은 `InputBackend` 트레이트 뒤에 숨는다.

mod clock;
mod input;
mod play;
mod preflight;
mod record;

#[cfg(any(test, feature = "fake"))]
pub mod fake;

pub use clock::{Clock, SystemClock};
pub use input::{CaptureGuard, InjectEvent, InputBackend, InputError, RawEvent, RawKind};
pub use play::{play, PlayError, PlayOptions, PlayOutcome, PlayReport};
pub use preflight::{effective_stop_key, preflight, Warning};
pub use record::{record, RecordOptions, Recorder};
