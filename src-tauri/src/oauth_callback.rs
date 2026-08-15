use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use url::Url;
use uuid::Uuid;

pub const CALLBACK_SCHEME: &str = "spectra";
pub const CALLBACK_HOST: &str = "oauth";
pub const CALLBACK_PATH: &str = "/callback";
const TRANSACTION_TTL_SECONDS: u64 = 600;

const SUPPORTED_PROVIDERS: &[&str] = &[
    "openai",
    "claude",
    "gemini",
    "cursor",
    "copilot",
    "perplexity",
];

#[derive(Debug)]
pub struct PendingOAuth {
    pub provider_id: String,
    pub state: String,
    pub code_verifier: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthPrepareResult {
    pub provider_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCallbackEvent {
    pub provider_id: String,
    pub status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct OAuthCallbackRejected {
    pub reason: &'static str,
}

#[derive(Debug)]
pub struct ValidatedOAuthCallback {
    pub code: String,
    pub state: String,
}

#[derive(Debug)]
pub enum OAuthCallbackError {
    InvalidScheme,
    InvalidHostPath,
    FragmentNotAllowed,
    MissingCode,
    DuplicateCode,
    MissingState,
    DuplicateState,
    StateMismatch,
    ProviderRejected,
    Expired,
    NoPendingTransaction,
    StateLockPoisoned,
}

impl OAuthCallbackError {
    pub fn safe_code(&self) -> &'static str {
        match self {
            Self::InvalidScheme => "invalid-scheme",
            Self::InvalidHostPath => "invalid-callback-target",
            Self::FragmentNotAllowed => "fragment-not-allowed",
            Self::MissingCode => "missing-code",
            Self::DuplicateCode => "duplicate-code",
            Self::MissingState => "missing-state",
            Self::DuplicateState => "duplicate-state",
            Self::StateMismatch => "state-mismatch",
            Self::ProviderRejected => "provider-rejected",
            Self::Expired => "transaction-expired",
            Self::NoPendingTransaction => "no-pending-transaction",
            Self::StateLockPoisoned => "state-unavailable",
        }
    }
}

pub fn is_supported_provider(provider_id: &str) -> bool {
    SUPPORTED_PROVIDERS.contains(&provider_id)
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn new_state() -> String {
    Uuid::new_v4().simple().to_string()
}

fn new_code_verifier() -> String {
    // Two UUIDs provide a 64-character verifier using the RFC 7636 unreserved
    // character set. The verifier stays in the native process and is never
    // returned to the webview.
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn prepare(
    pending: &Mutex<Option<PendingOAuth>>,
    provider_id: String,
) -> Result<OAuthPrepareResult, String> {
    if !is_supported_provider(&provider_id) {
        return Err("unsupported provider".to_string());
    }

    let state = new_state();
    let transaction = PendingOAuth {
        provider_id: provider_id.clone(),
        state: state.clone(),
        code_verifier: new_code_verifier(),
        expires_at: now_seconds() + TRANSACTION_TTL_SECONDS,
    };

    let mut guard = pending
        .lock()
        .map_err(|_| "oauth state unavailable".to_string())?;
    guard.replace(transaction);

    Ok(OAuthPrepareResult {
        provider_id,
        redirect_uri: format!("{CALLBACK_SCHEME}://{CALLBACK_HOST}{CALLBACK_PATH}"),
        state,
        status: "native-boundary-ready",
    })
}

fn required_query_value(url: &Url, key: &str) -> Result<String, OAuthCallbackError> {
    let values: Vec<String> = url
        .query_pairs()
        .filter(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
        .collect();

    match (key, values.as_slice()) {
        ("code", []) => Err(OAuthCallbackError::MissingCode),
        ("code", [_]) => Ok(values.into_iter().next().unwrap_or_default()),
        ("code", _) => Err(OAuthCallbackError::DuplicateCode),
        ("state", []) => Err(OAuthCallbackError::MissingState),
        ("state", [_]) => Ok(values.into_iter().next().unwrap_or_default()),
        ("state", _) => Err(OAuthCallbackError::DuplicateState),
        _ => Err(OAuthCallbackError::MissingState),
    }
}

pub fn parse_callback(
    url: &Url,
    expected_state: &str,
) -> Result<ValidatedOAuthCallback, OAuthCallbackError> {
    if url.scheme() != CALLBACK_SCHEME {
        return Err(OAuthCallbackError::InvalidScheme);
    }
    if url.host_str() != Some(CALLBACK_HOST) || url.path() != CALLBACK_PATH {
        return Err(OAuthCallbackError::InvalidHostPath);
    }
    if url.fragment().is_some() {
        return Err(OAuthCallbackError::FragmentNotAllowed);
    }
    if url.query_pairs().any(|(name, _)| name == "error") {
        return Err(OAuthCallbackError::ProviderRejected);
    }

    let code = required_query_value(url, "code")?;
    let state = required_query_value(url, "state")?;
    if code.is_empty() {
        return Err(OAuthCallbackError::MissingCode);
    }
    if state.is_empty() {
        return Err(OAuthCallbackError::MissingState);
    }
    if state != expected_state {
        return Err(OAuthCallbackError::StateMismatch);
    }

    Ok(ValidatedOAuthCallback { code, state })
}

pub fn accept(
    pending: &Mutex<Option<PendingOAuth>>,
    url: &Url,
) -> Result<OAuthCallbackEvent, OAuthCallbackError> {
    let mut guard = pending
        .lock()
        .map_err(|_| OAuthCallbackError::StateLockPoisoned)?;
    let transaction = guard
        .as_ref()
        .ok_or(OAuthCallbackError::NoPendingTransaction)?;

    if now_seconds() > transaction.expires_at {
        guard.take();
        return Err(OAuthCallbackError::Expired);
    }

    let validated = parse_callback(url, &transaction.state)?;
    let provider_id = transaction.provider_id.clone();

    // These values are intentionally kept inside the native boundary. A future
    // provider adapter will exchange them and write the resulting token to the
    // vault without emitting either secret to the webview.
    let exchange_inputs = (
        validated.code,
        validated.state,
        transaction.code_verifier.as_str(),
    );
    drop(exchange_inputs);
    guard.take();

    Ok(OAuthCallbackEvent {
        provider_id,
        status: "callback-accepted",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_expected_state_and_target() {
        let url = Url::parse("spectra://oauth/callback?code=abc&state=state-1").unwrap();
        let result = parse_callback(&url, "state-1").unwrap();
        assert_eq!(result.code, "abc");
        assert_eq!(result.state, "state-1");
    }

    #[test]
    fn rejects_state_replay_and_fragments() {
        let url = Url::parse("spectra://oauth/callback?code=abc&state=wrong").unwrap();
        assert!(matches!(
            parse_callback(&url, "expected"),
            Err(OAuthCallbackError::StateMismatch)
        ));

        let fragment =
            Url::parse("spectra://oauth/callback?code=abc&state=expected#token").unwrap();
        assert!(matches!(
            parse_callback(&fragment, "expected"),
            Err(OAuthCallbackError::FragmentNotAllowed)
        ));
    }
}
