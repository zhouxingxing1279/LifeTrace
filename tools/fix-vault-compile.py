from pathlib import Path

path = Path("src-tauri/src/vault.rs")
text = path.read_text(encoding="utf-8")
text = text.replace(
    ')\n        .context("private album password parameters are invalid")?;',
    ')\n        .map_err(|_| anyhow!("private album password parameters are invalid"))?;',
)
text = text.replace(
    '.hash_password_into(password.as_bytes(), &salt, key.as_mut_slice())\n            .context("failed to derive private album key")?;',
    '.hash_password_into(password.as_bytes(), &salt, key.as_mut_slice())\n            .map_err(|_| anyhow!("failed to derive private album key"))?;',
)
text = text.replace(
    'let mut bytes = Zeroizing::new(Vec::new());\n        thumbnail\n            .write_to(&mut Cursor::new(bytes.as_mut()), image::ImageFormat::Png)\n            .context("failed to encode private thumbnail")?;',
    'let mut cursor = Cursor::new(Vec::new());\n        thumbnail\n            .write_to(&mut cursor, image::ImageFormat::Png)\n            .context("failed to encode private thumbnail")?;\n        let bytes = Zeroizing::new(cursor.into_inner());',
)
path.write_text(text, encoding="utf-8")
