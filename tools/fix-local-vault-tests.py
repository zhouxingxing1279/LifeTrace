from pathlib import Path

path = Path("src-tauri/src/vault.rs")
text = path.read_text(encoding="utf-8")

old = '''    fn test_state() -> (VaultState, PathBuf) {
        let root = std::env::temp_dir().join(format!("lifetrace-vault-test-{}", Uuid::new_v4()));
        (VaultState::new(root.clone()).expect("state"), root)
    }
'''
new = '''    fn test_state() -> (VaultState, PathBuf) {
        let workspace =
            std::env::temp_dir().join(format!("lifetrace-vault-test-{}", Uuid::new_v4()));
        let vault_root = workspace.join("vault");
        (VaultState::new(vault_root).expect("state"), workspace)
    }
'''
if old not in text:
    raise SystemExit("test_state snippet not found")
text = text.replace(old, new, 1)

old = '''                if let Err(error) = self.save_manifest(master_key, &manifest) {
                    let _ = fs::remove_file(&target);
                    let _ = fs::remove_file(self.thumbnail_path(&asset_id));
                    manifest = previous_manifest;
                    return Err(error);
                }
'''
new = '''                if let Err(error) = self.save_manifest(master_key, &manifest) {
                    let _ = fs::remove_file(&target);
                    let _ = fs::remove_file(self.thumbnail_path(&asset_id));
                    return Err(error);
                }
'''
if old not in text:
    raise SystemExit("unused assignment snippet not found")
text = text.replace(old, new, 1)

marker = '''    #[test]
    fn temporary_files_are_removed_on_startup() {
'''
test = '''    #[test]
    fn importing_from_the_vault_directory_is_rejected() {
        let (state, root) = test_state();
        state.initialize(PASSWORD).expect("initialize");
        let source = sample_file(
            &root.join("vault").join("nested"),
            "already-private.bin",
            b"must not be imported recursively",
        );
        assert!(state
            .import_files(vec![source.to_string_lossy().into_owned()], false, None)
            .is_err());
        let _ = fs::remove_dir_all(root);
    }

'''
if marker not in text:
    raise SystemExit("test insertion marker not found")
text = text.replace(marker, test + marker, 1)

old = '''        state.delete_all(DELETE_CONFIRMATION).expect("delete vault");
        assert!(!root.exists());
'''
new = '''        state.delete_all(DELETE_CONFIRMATION).expect("delete vault");
        assert!(!root.join("vault").exists());
'''
if old not in text:
    raise SystemExit("delete assertion snippet not found")
text = text.replace(old, new, 1)

path.write_text(text, encoding="utf-8")
