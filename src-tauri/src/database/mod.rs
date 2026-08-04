//! 统一数据库层：连接、备份、版本化 Migration、校验、旧数据解析与 Repository。
//!
//! EPIC-01 的目标是把核心业务从 `(id, data_json, updated_at)` 迁移到真实列，
//! 本模块是迁移与备份的唯一入口。

pub mod backup;
pub mod connection;
pub mod legacy;
pub mod migration_runner;
pub mod migrations;
pub mod repositories;
pub mod validation;
