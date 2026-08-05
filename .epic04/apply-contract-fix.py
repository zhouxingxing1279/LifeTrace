from pathlib import Path


path = Path("tools/contract-exporter/src/main.rs")
text = path.read_text(encoding="utf-8")
old = '''    let mut output = String::from(
        "// GENERATED FILE - DO NOT EDIT MANUALLY\\n\\
         // LifeTrace authentication protocol v1.\\n\\
         // Rust types in crates/lifetrace-contracts are authoritative.\\n\\n",
    );
'''
new = '''    let mut output = String::from(
        "// GENERATED FILE - DO NOT EDIT MANUALLY\\n\\
         // LifeTrace authentication protocol v1.\\n\\
         // Rust types in crates/lifetrace-contracts are authoritative.\\n\\n\\
         import type { AppId, UserId } from \\\"./lifetrace-contracts.generated\\\";\\n\\n",
    );
'''
if old not in text:
    raise SystemExit("authentication TypeScript header not found in contract exporter")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
