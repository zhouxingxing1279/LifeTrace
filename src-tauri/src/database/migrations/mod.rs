//! 版本化 Migration 注册表。
//!
//! 所有业务表结构变更必须通过这里的 Migration 完成，禁止在业务模块中
//! 自行创建核心业务表。

mod m0001_framework;
mod m0002_finance;

pub use m0001_framework::M0001Framework;
pub use m0002_finance::M0002Finance;

use crate::database::migration_runner::Migration;

/// 全部已注册 Migration（按 version 升序执行）。
pub fn all() -> Vec<Box<dyn Migration>> {
    vec![Box::new(M0001Framework), Box::new(M0002Finance)]
}
