use std::time::Instant;

#[derive(Debug, Clone)]
pub enum NotificationKind {
    Info,
    Progress,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: NotificationKind,
    pub message: String,
    pub progress: Option<u8>,
    pub created_at: Instant,
    pub dismiss_after: Option<std::time::Duration>,
}

impl Notification {
    pub fn new(kind: NotificationKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            progress: None,
            created_at: Instant::now(),
            dismiss_after: Some(std::time::Duration::from_secs(2)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NotificationState {
    pub current: Option<Notification>,
}
