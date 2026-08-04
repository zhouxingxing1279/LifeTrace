//! Repository 层：数据库真实列与前端 DTO 的转换唯一入口。
//!
//! Handler 不直接承担转换逻辑；`state_compat` 负责 `/api/state` 兼容层。

pub mod finance;
pub mod habits;
pub mod state_compat;
