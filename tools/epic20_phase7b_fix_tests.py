from pathlib import Path

path = Path("services/cloud/tests/api.rs")
text = path.read_text(encoding="utf-8")
text = text.replace(
    '    let app_a = test_app_for(TOKEN_A, user, "execution-device-a");\n    let app_b = test_app_for(TOKEN_B, user, "execution-device-b");',
    '    let app_a = test_app_for(TOKEN_A, user, "execution-auth-device");\n    let app_b = app_a.clone();',
    1,
)
text = text.replace(
    'execution_pull(app_b.clone(), TOKEN_B, "execution-device-b", None)',
    'execution_pull(app_b.clone(), TOKEN_A, "execution-device-b", None)',
    1,
)
text = text.replace(
    'execution_pull(\n        app_b.clone(),\n        TOKEN_B,',
    'execution_pull(\n        app_b.clone(),\n        TOKEN_A,',
    1,
)
text = text.replace(
    'execution_pull(\n        app_b,\n        TOKEN_B,',
    'execution_pull(\n        app_b,\n        TOKEN_A,',
    1,
)
text = text.replace(
    '    let app_a = test_app_for(TOKEN_A, user, "execution-conflict-a");\n    let app_b = test_app_for(TOKEN_B, user, "execution-conflict-b");',
    '    let app_a = test_app_for(TOKEN_A, user, "execution-auth-device");\n    let app_b = app_a.clone();',
    1,
)
# All conflict-device B requests must hit the same authenticated cloud store.
text = text.replace('Method::POST, "/api/v1/sync/push", TOKEN_B,', 'Method::POST, "/api/v1/sync/push", TOKEN_A,')

if 'test_app_for(TOKEN_B, user, "execution' in text:
    raise SystemExit("a two-device execution test still creates a second cloud store")
path.write_text(text, encoding="utf-8")
