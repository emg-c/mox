use mox_format::Macro;
use serde::{Deserialize, Serialize};

use crate::{PayloadError, PAYLOAD_VERSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    /// 재생 전에 확인 창을 띄운다.
    pub confirm: bool,
    /// 재생 전 카운트다운(초). 0이면 바로 시작.
    pub countdown_s: u32,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            confirm: true,
            countdown_s: 3,
        }
    }
}

impl ExportSettings {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("ExportSettings 직렬화는 실패하지 않는다")
    }
}

/// 추출한 실행 파일 하나에 담기는 것.
#[derive(Debug, Clone, PartialEq)]
pub struct Payload {
    pub settings: ExportSettings,
    pub mox: Macro,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    export_version: u32,
    #[serde(default)]
    settings: ExportSettings,
    /// `.mox` 파일 본문 그대로.
    #[serde(rename = "macro")]
    macro_text: String,
}

impl Payload {
    pub fn encode(&self) -> Result<Vec<u8>, PayloadError> {
        let env = Envelope {
            export_version: PAYLOAD_VERSION,
            settings: self.settings,
            macro_text: mox_format::to_string(&self.mox)?,
        };
        serde_json::to_vec(&env).map_err(|e| PayloadError::Corrupt(e.to_string()))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, PayloadError> {
        let env: Envelope =
            serde_json::from_slice(bytes).map_err(|e| PayloadError::Corrupt(e.to_string()))?;
        if env.export_version != PAYLOAD_VERSION {
            return Err(PayloadError::UnsupportedVersion(env.export_version));
        }
        Ok(Self {
            settings: env.settings,
            mox: mox_format::from_str(&env.macro_text)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::sample;

    #[test]
    fn payload_round_trips() {
        let p = Payload {
            settings: ExportSettings {
                confirm: false,
                countdown_s: 0,
            },
            mox: sample(),
        };
        assert_eq!(Payload::decode(&p.encode().unwrap()).unwrap(), p);
    }

    #[test]
    fn default_settings_confirm_with_countdown() {
        assert_eq!(
            ExportSettings::default(),
            ExportSettings {
                confirm: true,
                countdown_s: 3
            }
        );
        let s: ExportSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, ExportSettings::default());
        assert_eq!(
            serde_json::from_str::<ExportSettings>(&s.to_json()).unwrap(),
            s
        );
    }

    #[test]
    fn decode_reports_errors_without_panicking() {
        assert!(matches!(
            Payload::decode(b"not json"),
            Err(PayloadError::Corrupt(_))
        ));
        let wrong = br#"{"export_version":9,"settings":{},"macro":""}"#;
        assert!(matches!(
            Payload::decode(wrong),
            Err(PayloadError::UnsupportedVersion(9))
        ));
        let bad_mox = br#"{"export_version":1,"settings":{},"macro":"{}"}"#;
        assert!(matches!(
            Payload::decode(bad_mox),
            Err(PayloadError::Format(_))
        ));
    }
}
