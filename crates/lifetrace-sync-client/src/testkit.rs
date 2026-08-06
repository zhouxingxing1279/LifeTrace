use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;
use lifetrace_contracts::sync::v1::*;

use crate::{SyncError, SyncTransport};

#[derive(Default)]
pub struct ScriptedTransport {
    pub capabilities: Mutex<VecDeque<Result<CapabilitiesResponseV1, SyncError>>>,
    pub pushes: Mutex<VecDeque<Result<PushResponseV1, SyncError>>>,
    pub pulls: Mutex<VecDeque<Result<PullResponseV1, SyncError>>>,
    pub snapshots: Mutex<VecDeque<Result<SnapshotResponseV1, SyncError>>>,
}

#[async_trait]
impl SyncTransport for ScriptedTransport {
    async fn capabilities(&self) -> Result<CapabilitiesResponseV1, SyncError> {
        self.capabilities.lock().unwrap().pop_front().unwrap()
    }
    async fn push(&self, _request: PushRequestV1) -> Result<PushResponseV1, SyncError> {
        self.pushes.lock().unwrap().pop_front().unwrap()
    }
    async fn pull(&self, _request: PullRequestV1) -> Result<PullResponseV1, SyncError> {
        self.pulls.lock().unwrap().pop_front().unwrap()
    }
    async fn snapshot(&self, _request: SnapshotRequestV1) -> Result<SnapshotResponseV1, SyncError> {
        self.snapshots.lock().unwrap().pop_front().unwrap()
    }
}
