//! 版本化 Migration 注册表。
//!
//! 所有业务表结构变更必须通过这里的 Migration 完成，禁止在业务模块中
//! 自行创建核心业务表。

mod m0001_framework;
mod m0002_finance;
mod m0003_habits_reviews;
mod m0004_notes;
mod m0005_english;
mod m0006_workouts;
mod m0007_sync_client;
mod m0008_sync_triggers;
mod m0009_execution;
mod m0010_execution_sync;

pub use m0001_framework::M0001Framework;
pub use m0002_finance::M0002Finance;
pub use m0003_habits_reviews::M0003HabitsReviews;
pub use m0004_notes::M0004Notes;
pub use m0005_english::M0005English;
pub use m0006_workouts::M0006Workouts;
pub use m0007_sync_client::M0007SyncClient;
pub use m0008_sync_triggers::M0008SyncTriggers;
pub use m0009_execution::M0009Execution;
pub use m0010_execution_sync::M0010ExecutionSync;

use crate::database::migration_runner::Migration;

/// 全部已注册 Migration（按 version 升序执行）。
pub fn all() -> Vec<Box<dyn Migration>> {
    vec![
        Box::new(M0001Framework),
        Box::new(M0002Finance),
        Box::new(M0003HabitsReviews),
        Box::new(M0004Notes),
        Box::new(M0005English),
        Box::new(M0006Workouts),
        Box::new(M0007SyncClient),
        Box::new(M0008SyncTriggers),
        Box::new(M0009Execution),
        Box::new(M0010ExecutionSync),
    ]
}
