//! 旧数据解析与导入。
//!
//! - `json_parser`：把 `(id, data_json, updated_at)` 行安全解析为 JSON 实体。
//! - `d1_import`：旧 D1 / wrangler 数据库导入入口（阶段 7 收敛为唯一实现）。

pub mod d1_import;
pub mod json_parser;
