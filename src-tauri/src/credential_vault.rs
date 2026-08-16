use serde::{Deserialize, Serialize};

pub const SERVICE_NAME: &str = "com.spectra.ai-usage-widget";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCredential {
    pub provider_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum VaultError {
    UnsupportedPlatform,
    InvalidProvider,
    Unavailable,
    InvalidCredential,
    CorruptCredential,
    DeleteFailed,
}

fn valid_provider(provider_id: &str) -> bool {
    matches!(provider_id, "codex" | "claude")
}

fn account_name(provider_id: &str) -> Result<String, VaultError> {
    if valid_provider(provider_id) {
        Ok(format!("provider:{provider_id}"))
    } else {
        Err(VaultError::InvalidProvider)
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn entry(provider_id: &str) -> Result<keyring::Entry, VaultError> {
    let account = account_name(provider_id)?;
    keyring::Entry::new(SERVICE_NAME, &account).map_err(|_| VaultError::Unavailable)
}

pub fn read(provider_id: &str) -> Result<Option<StoredCredential>, VaultError> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let entry = entry(provider_id)?;
        match entry.get_password() {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|_| VaultError::CorruptCredential),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(VaultError::Unavailable),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = provider_id;
        Err(VaultError::UnsupportedPlatform)
    }
}

pub fn exists(provider_id: &str) -> Result<bool, VaultError> {
    Ok(read(provider_id)?.is_some())
}

/// Called by the provider adapter after a native token exchange is added.
#[allow(dead_code)]
pub fn write(credential: &StoredCredential) -> Result<(), VaultError> {
    if credential.access_token.is_empty() || credential.provider_id.is_empty() {
        return Err(VaultError::InvalidCredential);
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let entry = entry(&credential.provider_id)?;
        let serialized =
            serde_json::to_string(credential).map_err(|_| VaultError::CorruptCredential)?;
        entry
            .set_password(&serialized)
            .map_err(|_| VaultError::Unavailable)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(VaultError::UnsupportedPlatform)
    }
}

pub fn remove(provider_id: &str) -> Result<(), VaultError> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let entry = entry(provider_id)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(VaultError::DeleteFailed),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = provider_id;
        Err(VaultError::UnsupportedPlatform)
    }
}
