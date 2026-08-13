//! In-process BeeCount WebSocket fan-out for a single LifeTrace cloud node.

use chrono::{DateTime, Utc};
use lifetrace_contracts::UserId;
use serde_json::{json, Value};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct BeeCountRealtimeEvent {
    pub user_id: String,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub struct BeeCountRealtimeHub {
    sender: broadcast::Sender<BeeCountRealtimeEvent>,
}

impl Default for BeeCountRealtimeHub {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(512);
        Self { sender }
    }
}

impl BeeCountRealtimeHub {
    pub fn subscribe(&self) -> broadcast::Receiver<BeeCountRealtimeEvent> {
        self.sender.subscribe()
    }

    pub fn publish_sync_change(
        &self,
        user_id: &UserId,
        ledger_id: &str,
        server_cursor: i64,
        server_timestamp: DateTime<Utc>,
    ) {
        self.publish(
            user_id.as_str(),
            json!({
                "type": "sync_change",
                "ledgerId": ledger_id,
                "serverCursor": server_cursor,
                "serverTimestamp": server_timestamp,
            }),
        );
    }

    pub fn publish(&self, user_id: &str, payload: Value) {
        let _ = self.sender.send(BeeCountRealtimeEvent {
            user_id: user_id.to_owned(),
            payload,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sync_change_uses_stock_beecount_field_names() {
        let hub = BeeCountRealtimeHub::default();
        let mut receiver = hub.subscribe();
        hub.publish_sync_change(&UserId::new("user-a"), "ledger-a", 42, Utc::now());
        let event = receiver.recv().await.unwrap();
        assert_eq!(event.user_id, "user-a");
        assert_eq!(event.payload["type"], "sync_change");
        assert_eq!(event.payload["ledgerId"], "ledger-a");
        assert_eq!(event.payload["serverCursor"], 42);
        assert!(event.payload.get("server_cursor").is_none());
    }
}
