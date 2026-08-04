//! 版本化 Migration 注册表。
//!
//! 所有业务表结构变更必须通过这里的 Migration 完成，禁止在业务模块中
//! 自行创建核心业务表。

mod m0001_framework;
mod m0002_finance;
mod m0003_habits_reviews;
mod m0004_notes;
mod m0005_english;

pub use m0001_framework::M0001Framework;
pub use m0002_finance::M0002Finance;
pub use m0003_habits_reviews::M0003HabitsReviews;
pub use m0004_notes::M0004Notes;
pub use m0005_english::M0005English;

use crate::database::migration_runner::Migration;

/// 全部已注册 Migration（按 version 升序执行）。
pub fn all() -> Vec<Box<dyn Migration>> {
    vec![
        Box::new(M0001Framework),
        Box::new(M0002Finance),
        Box::new(M0003HabitsReviews),
        Box::new(M0004Notes),
        Box::new(M0005English),
    ]
}
