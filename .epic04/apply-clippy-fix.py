from pathlib import Path


payload = Path("crates/lifetrace-contracts/src/domain/payload.rs")
text = payload.read_text(encoding="utf-8")
old = """/// All supported domain payloads, keyed by registered entity type.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityPayload {
"""
new = """/// All supported domain payloads, keyed by registered entity type.
///
/// This is a stable Rust-side protocol dispatch API. Boxing selected variants
/// would impose a source-breaking constructor change across every adapter for
/// no wire-format benefit, so its deliberately heterogeneous size is accepted.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum EntityPayload {
"""
if old not in text:
    raise SystemExit("EntityPayload declaration not found")
payload.write_text(text.replace(old, new, 1), encoding="utf-8")


testkit = Path("crates/lifetrace-contracts/src/sync/testkit.rs")
text = testkit.read_text(encoding="utf-8")
old = """impl SyncServer {
"""
new = """// The reference server intentionally returns the complete structured API
// error used by protocol tests. Boxing it would make this test-only API less
// ergonomic without affecting production request handling.
#[allow(clippy::result_large_err)]
impl SyncServer {
"""
if old not in text:
    raise SystemExit("SyncServer implementation not found")
testkit.write_text(text.replace(old, new, 1), encoding="utf-8")
