const CREDENTIAL_TARGET: &str = "LifeTrace/cloud/lifetrace-desktop/refresh-token";

pub(crate) fn credential_set_internal(refresh_token: &str) -> Result<(), String> {
    if refresh_token.is_empty() || refresh_token.len() > 4096 {
        return Err("invalid refresh token length".to_owned());
    }
    platform::set(refresh_token)
}

pub(crate) fn credential_get_internal() -> Result<Option<String>, String> {
    platform::get()
}

pub(crate) fn credential_clear_internal() -> Result<(), String> {
    platform::clear()
}

#[tauri::command]
pub fn cloud_credential_set(refresh_token: String) -> Result<(), String> {
    if refresh_token.is_empty() || refresh_token.len() > 4096 {
        return Err("invalid refresh token length".to_owned());
    }
    credential_set_internal(&refresh_token)
}

#[tauri::command]
pub fn cloud_credential_get() -> Result<Option<String>, String> {
    credential_get_internal()
}

#[tauri::command]
pub fn cloud_credential_clear() -> Result<(), String> {
    credential_clear_internal()
}

#[cfg(windows)]
mod platform {
    use std::mem::zeroed;
    use std::ptr::{null_mut, slice_from_raw_parts};

    use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    use super::CREDENTIAL_TARGET;

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn set(secret: &str) -> Result<(), String> {
        let target = wide(CREDENTIAL_TARGET);
        let username = wide("LifeTrace Desktop");
        let mut bytes = secret.as_bytes().to_vec();
        let mut credential: CREDENTIALW = unsafe { zeroed() };
        credential.Type = CRED_TYPE_GENERIC;
        credential.TargetName = target.as_ptr() as *mut u16;
        credential.CredentialBlobSize = bytes.len() as u32;
        credential.CredentialBlob = bytes.as_mut_ptr();
        credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
        credential.UserName = username.as_ptr() as *mut u16;
        let success = unsafe { CredWriteW(&credential, 0) };
        bytes.fill(0);
        if success == 0 {
            Err(format!(
                "Windows Credential Manager write failed: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(())
        }
    }

    pub fn get() -> Result<Option<String>, String> {
        let target = wide(CREDENTIAL_TARGET);
        let mut pointer: *mut CREDENTIALW = null_mut();
        let success = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut pointer) };
        if success == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(None);
            }
            return Err(format!("Windows Credential Manager read failed: {error}"));
        }
        if pointer.is_null() {
            return Ok(None);
        }
        let credential = unsafe { &*pointer };
        let blob = unsafe {
            &*slice_from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        };
        let result = String::from_utf8(blob.to_vec())
            .map(Some)
            .map_err(|_| "stored cloud credential is not valid UTF-8".to_owned());
        unsafe { CredFree(pointer.cast()) };
        result
    }

    pub fn clear() -> Result<(), String> {
        let target = wide(CREDENTIAL_TARGET);
        let success = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if success == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                return Ok(());
            }
            Err(format!("Windows Credential Manager delete failed: {error}"))
        } else {
            Ok(())
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn set(_: &str) -> Result<(), String> {
        Err(
            "secure cloud credential storage is available only in the Windows desktop build"
                .to_owned(),
        )
    }

    pub fn get() -> Result<Option<String>, String> {
        Ok(None)
    }

    pub fn clear() -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn credential_target_is_stable_and_contains_no_secret() {
        assert_eq!(
            super::CREDENTIAL_TARGET,
            "LifeTrace/cloud/lifetrace-desktop/refresh-token"
        );
    }
}
