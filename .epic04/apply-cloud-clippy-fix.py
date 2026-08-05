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
audit = "    async fn audit<'e, E>("
audit_replacement = """    // Each argument maps one-to-one to a security-audit column. Keeping the
    // fields explicit makes omissions visible at every call site and avoids
    // accepting partially populated, loosely typed metadata structures.
    #[allow(clippy::too_many_arguments)]
    async fn audit<'e, E>("""
if audit not in text:
    raise SystemExit("audit helper not found")
text = text.replace(audit, audit_replacement, 1)
refresh = "    async fn insert_refresh("
refresh_replacement = """    // Refresh-token lineage and both expiry boundaries must be persisted in
    // the same transaction. Explicit parameters mirror the immutable row and
    // make accidental omission during token rotation a compile-time error.
    #[allow(clippy::too_many_arguments)]
    async fn insert_refresh("""
if refresh not in text:
    raise SystemExit("insert_refresh helper not found")
service.write_text(text.replace(refresh, refresh_replacement, 1), encoding="utf-8")


api_tests = Path("services/lifetrace-cloud/tests/api.rs")
text = api_tests.read_text(encoding="utf-8")
old = """fn test_app_for(token: &str, user: &str, device: &str) -> Router {
    let mut config = Config::default();
    config.dev_auth_token = token.to_owned();
    config.dev_auth_user_id = user.to_owned();
    config.dev_auth_device_id = device.to_owned();
    app(AppState::new(config))
}
"""
new = """fn test_app_for(token: &str, user: &str, device: &str) -> Router {
    let config = Config {
        dev_auth_token: token.to_owned(),
        dev_auth_user_id: user.to_owned(),
        dev_auth_device_id: device.to_owned(),
        ..Config::default()
    };
    app(AppState::new(config))
}
"""
if old not in text:
    raise SystemExit("test_app_for config initializer not found")
text = text.replace(old, new, 1)
old = """async fn expired_cursor_requires_snapshot() {
    let mut config = Config::default();
    config.dev_auth_token = TOKEN_A.to_owned();
    config.retention_entries = 1;
    let app = app(AppState::new(config));
"""
new = """async fn expired_cursor_requires_snapshot() {
    let config = Config {
        dev_auth_token: TOKEN_A.to_owned(),
        retention_entries: 1,
        ..Config::default()
    };
    let app = app(AppState::new(config));
"""
if old not in text:
    raise SystemExit("expired cursor config initializer not found")
api_tests.write_text(text.replace(old, new, 1), encoding="utf-8")
