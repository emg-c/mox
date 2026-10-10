use serde::{Deserialize, Serialize};

/// 매크로를 녹화하거나 재생하는 운영체제.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Macos,
    Windows,
}
