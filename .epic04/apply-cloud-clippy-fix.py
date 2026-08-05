from pathlib import Path


password = Path("services/lifetrace-cloud/src/auth/password.rs")
text = password.read_text(encoding="utf-8")
text = text.replace(
    "if password.as_bytes().len() > self.maximum_bytes {",
    "if password.len() > self.maximum_bytes {",
    1,
)
text = text.replace(
    "hash.params.get_decimal(name).and_then(|value| u32::try_from(value).ok())",
    "hash.params.get_decimal(name)",
    1,
)
password.write_text(text, encoding="utf-8")


security = Path("services/lifetrace-cloud/src/auth/security.rs")
text = security.read_text(encoding="utf-8")
old = """        let mut config = Config::default();
        config.auth_cookie_secure = true;
"""
new = """        let config = Config {
            auth_cookie_secure: true,
            ..Config::default()
        };
"""
if old not in text:
    raise SystemExit("secure-cookie test initializer not found")
security.write_text(text.replace(old, new, 1), encoding="utf-8")


service = Path("services/lifetrace-cloud/src/auth/service.rs")
text = service.read_text(encoding="utf-8")
audit = """    async fn audit<'e, E>(
"""
audit_replacement = """    // Each argument maps one-to-one to a security-audit column. Keeping the
    // fields explicit makes omissions visible at every call site and avoids
    // accepting partially populated, loosely typed metadata structures.
    #[allow(clippy::too_many_arguments)]
    async fn audit<'e, E>(
"""
if audit not in text:
    raise SystemExit("audit helper not found")
text = text.replace(audit, audit_replacement, 1)
refresh = """    async fn insert_refresh(
"""
refresh_replacement = """    // Refresh-token lineage and both expiry boundaries must be persisted in
    // the same transaction. Explicit parameters mirror the immutable row and
    // make accidental omission during token rotation a compile-time error.
    #[allow(clippy::too_many_arguments)]
    async fn insert_refresh(
"""
if refresh not in text:
    raise SystemExit("insert_refresh helper not found")
service.write_text(text.replace(refresh, refresh_replacement, 1), encoding="utf-8")
