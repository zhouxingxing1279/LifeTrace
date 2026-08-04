//! Habit domain DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::domain::enums::{
    ActivityCheckinMethod, ActivityLogStatus, ActivityScheduleType, ActivitySyncSource,
    ActivityType,
};
use crate::ids::EntityId;
use crate::json_value::JsonValue;
use crate::time::LocalDate;

/// `habit.activity`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Activity {
    pub meta: EntityMeta,
    pub name: String,
    pub activity_type: ActivityType,
    pub unit: String,
    pub minimum_target: Option<f64>,
    pub normal_target: Option<f64>,
    pub target_period: String,
    pub target_days: Vec<u8>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub schedule_type: Option<ActivityScheduleType>,
    pub start_date: Option<LocalDate>,
    pub checkin_method: Option<ActivityCheckinMethod>,
    pub sync_source: Option<ActivitySyncSource>,
    pub description: Option<String>,
    pub is_archived: bool,
}

/// `habit.log`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActivityLog {
    pub meta: EntityMeta,
    pub activity_id: Option<EntityId>,
    pub log_date: LocalDate,
    pub value: Option<f64>,
    pub status: Option<ActivityLogStatus>,
    pub note: Option<String>,
    pub metadata: Option<JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::UserId;
    use crate::time::UtcTimestamp;

    #[test]
    fn activity_wire_is_camel_case() {
        let stamp: UtcTimestamp = "2026-08-04T15:30:00Z".parse().unwrap();
        let activity = Activity {
            meta: EntityMeta {
                id: EntityId::new("piano"),
                user_id: UserId::new("local-user"),
                created_at: stamp,
                updated_at: stamp,
                deleted_at: None,
                local_version: 2,
                server_version: None,
                modified_by_device: None,
            },
            name: "piano practice".to_owned(),
            activity_type: ActivityType::new(ActivityType::DURATION),
            unit: "minutes".to_owned(),
            minimum_target: Some(10.0),
            normal_target: Some(30.0),
            target_period: "daily".to_owned(),
            target_days: vec![],
            icon: Some("music".to_owned()),
            color: None,
            schedule_type: None,
            start_date: None,
            checkin_method: None,
            sync_source: None,
            description: None,
            is_archived: false,
        };
        let value = serde_json::to_value(&activity).unwrap();
        assert_eq!(value["activityType"], "duration");
        assert_eq!(value["normalTarget"], 30.0);
        assert_eq!(value["targetDays"], serde_json::json!([]));
        let back: Activity = serde_json::from_value(value).unwrap();
        assert_eq!(back, activity);
    }
}
