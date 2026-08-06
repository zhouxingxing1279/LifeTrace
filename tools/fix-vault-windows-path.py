from pathlib import Path

path = Path("src-tauri/src/vault.rs")
text = path.read_text(encoding="utf-8")
old = '''                if source
                    .canonicalize()
                    .ok()
                    .is_some_and(|path| path.starts_with(&self.root))
                {
                    bail!("不能从私密相册内部目录重复导入文件");
                }
'''
new = '''                let canonical_source = source
                    .canonicalize()
                    .with_context(|| format!("无法解析所选文件路径：{source_path}"))?;
                let canonical_root = self
                    .root
                    .canonicalize()
                    .context("failed to resolve private album directory")?;
                if canonical_source.starts_with(&canonical_root) {
                    bail!("不能从私密相册内部目录重复导入文件");
                }
'''
if old not in text:
    raise SystemExit("vault path boundary snippet not found")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
