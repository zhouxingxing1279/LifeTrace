from pathlib import Path

path = Path("services/cloud/tests/api.rs")
text = path.read_text(encoding="utf-8")
marker = "fn execution_client(device_id: &str) -> Value"
if marker not in text:
    raise SystemExit("execution E2E marker not found")

prefix, execution = text.split(marker, 1)
execution = execution.replace(
    '    let app_a = test_app_for(TOKEN_A, user, "execution-device-a");\n    let app_b = test_app_for(TOKEN_B, user, "execution-device-b");',
    '    let app_a = test_app_for(TOKEN_A, user, "execution-auth-device");\n    let app_b = app_a.clone();',
    1,
)
execution = execution.replace(
    '    let app_a = test_app_for(TOKEN_A, user, "execution-conflict-a");\n    let app_b = test_app_for(TOKEN_B, user, "execution-conflict-b");',
    '    let app_a = test_app_for(TOKEN_A, user, "execution-auth-device");\n    let app_b = app_a.clone();',
    1,
)
# One development-auth Router owns one MemoryRepository. Device identity for
# protocol semantics still differs through execution_client(device_id), while
# Bearer authentication must use the token accepted by that shared Router.
execution = execution.replace("TOKEN_B", "TOKEN_A")

if 'test_app_for(TOKEN_A, user, "execution-device-b")' in execution:
    raise SystemExit("device B still creates an independent cloud store")
if 'test_app_for(TOKEN_A, user, "execution-conflict-b")' in execution:
    raise SystemExit("conflict device B still creates an independent cloud store")

path.write_text(prefix + marker + execution, encoding="utf-8")
