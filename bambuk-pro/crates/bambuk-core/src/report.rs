use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status { Applied, Failed, Unsupported, Skipped }

/// Результат применения одной подсистемы. `Applied` ставится только после проверки (read-back).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub subsystem: String,
    pub status: Status,
    pub detail: String,
}
impl Report {
    pub fn new(subsystem: &str, status: Status, detail: impl Into<String>) -> Self {
        Self { subsystem: subsystem.into(), status, detail: detail.into() }
    }
    pub fn applied(s: &str, d: impl Into<String>) -> Self { Self::new(s, Status::Applied, d) }
    pub fn failed(s: &str, d: impl Into<String>) -> Self { Self::new(s, Status::Failed, d) }
    pub fn unsupported(s: &str, d: impl Into<String>) -> Self { Self::new(s, Status::Unsupported, d) }
    pub fn skipped(s: &str, d: impl Into<String>) -> Self { Self::new(s, Status::Skipped, d) }
}
