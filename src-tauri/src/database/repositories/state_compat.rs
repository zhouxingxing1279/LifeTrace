//! `/api/state` 兼容层。
//!
//! 读取：数据库真实列 → Rust Model → 旧 camelCase DTO。
//! 写入：旧 DTO → Validation → Repository → 新表。
//!
//! 阶段 1 尚无规范化业务表，此模块为空壳；财务（阶段 2）与习惯/复盘
//! （阶段 3）落地后在此实现转换，禁止把整个 DTO 写回 `data_json`。
