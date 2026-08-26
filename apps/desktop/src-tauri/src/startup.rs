use std::sync::{Arc, RwLock};

use serde::Serialize;
use tauri::State;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalServiceStatus {
    pub phase: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LocalServiceState {
    inner: Arc<RwLock<LocalServiceStatus>>,
}

impl Default for LocalServiceState {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(LocalServiceStatus {
                phase: "starting".to_string(),
                error: None,
            })),
        }
    }
}

impl LocalServiceState {
    pub fn fail(&self, error: impl Into<String>) {
        if let Ok(mut status) = self.inner.write() {
            status.phase = "failed".to_string();
            status.error = Some(error.into());
        }
    }

    pub fn snapshot(&self) -> LocalServiceStatus {
        self.inner
            .read()
            .map(|status| status.clone())
            .unwrap_or_else(|_| LocalServiceStatus {
                phase: "failed".to_string(),
                error: Some("本地服务启动状态锁已损坏".to_string()),
            })
    }
}

#[tauri::command]
pub fn local_service_status(state: State<'_, LocalServiceState>) -> LocalServiceStatus {
    state.snapshot()
}

#[cfg(test)]
mod tests {
    use super::LocalServiceState;

    #[test]
    fn startup_state_records_service_failure() {
        let state = LocalServiceState::default();
        assert_eq!(state.snapshot().phase, "starting");
        assert!(state.snapshot().error.is_none());

        state.fail("address already in use");
        let status = state.snapshot();
        assert_eq!(status.phase, "failed");
        assert_eq!(status.error.as_deref(), Some("address already in use"));
    }
}
