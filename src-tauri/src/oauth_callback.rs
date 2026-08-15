use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::credential_vault::{self, StoredCredential};
use crate::provider_connection;

pub const CALLBACK_SCHEME: &str = "spectra";
pub const CALLBACK_HOST: &str = "oauth";
pub const CALLBACK_PATH: &str = "/callback";
const TRANSACTION_TTL_SECONDS: u64 = 600;

const SUPPORTED_PROVIDERS: &[&str] = &["codex", "claude"];

#[derive(Debug)]
pub struct PendingOAuth {
    pub provider_id: String,
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub token_endpoint: String,
    pub scope: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthPrepareResult {
    pub provider_id: String,
    pub redirect_uri: String,
    pub state: Option<String>,
    pub status: &'static str,
    pub callback_mode: &'static str,
    pub authorize_url: Option<String>,
    pub token_url: Option<String>,
    pub client_id_env: Option<String>,
    pub client_id_configured: bool,
    pub token_exchange_status: &'static str,
    pub scope: String,
    pub quota_endpoint: Option<String>,
    pub quota_endpoint_status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCallbackEvent {
    pub provider_id: String,
    pub status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCallbackRejected {
    pub reason: &'static str,
}

#[derive(Debug)]
pub struct ValidatedOAuthCallback {
    pub code: String,
    pub state: String,
}

#[derive(Debug)]
pub struct OAuthExchangeInput {
    pub provider_id: String,
    pub code: String,
    pub code_verifier: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub token_endpoint: String,
    pub scope: String,
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

#[derive(Debug)]
pub enum OAuthExchangeError {
    Network,
    ProviderRejected,
    InvalidResponse,
    VaultUnavailable,
}

impl OAuthExchangeError {
    pub fn safe_code(&self) -> &'static str {
        match self {
            Self::Network => "token-network-failed",
            Self::ProviderRejected => "token-provider-rejected",
            Self::InvalidResponse => "token-response-invalid",
            Self::VaultUnavailable => "credential-vault-unavailable",
        }
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
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
    // character set. The verifier stays in the native process.
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn prepare(
    pending: &Mutex<Option<PendingOAuth>>,
    loopback_listener: &Mutex<Option<TcpListener>>,
    provider_id: String,
) -> Result<OAuthPrepareResult, String> {
    if !is_supported_provider(&provider_id) {
        return Err("unsupported provider".to_string());
    }

    let config = provider_connection::config(&provider_id)
        .ok_or_else(|| "unsupported provider".to_string())?;
    let redirect_uri = if config.authorize_url.is_some() {
        format!("{CALLBACK_SCHEME}://{CALLBACK_HOST}{CALLBACK_PATH}")
    } else {
        String::new()
    };
    let client_id = provider_connection::client_id(config);
    let metadata = provider_connection::metadata(&provider_id)?;

    let Some(authorize_url) = config.authorize_url else {
        return Ok(OAuthPrepareResult {
            provider_id,
            redirect_uri,
            state: None,
            status: config.connection_status,
            callback_mode: config.callback_mode,
            authorize_url: None,
            token_url: None,
            client_id_env: metadata.client_id_env,
            client_id_configured: false,
            token_exchange_status: metadata.token_exchange_status,
            scope: metadata.scope,
            quota_endpoint: metadata.quota_endpoint,
            quota_endpoint_status: metadata.quota_endpoint_status,
        });
    };

    let Some(client_id) = client_id else {
        return Ok(OAuthPrepareResult {
            provider_id,
            redirect_uri,
            state: None,
            status: "client-id-required",
            callback_mode: config.callback_mode,
            authorize_url: Some(authorize_url.to_string()),
            token_url: config.token_url.map(str::to_string),
            client_id_env: metadata.client_id_env,
            client_id_configured: false,
            token_exchange_status: metadata.token_exchange_status,
            scope: metadata.scope,
            quota_endpoint: metadata.quota_endpoint,
            quota_endpoint_status: metadata.quota_endpoint_status,
        });
    };

    let state = new_state();
    let code_verifier = new_code_verifier();
    let (redirect_uri, listener) = if config.callback_mode == "loopback" {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|_| "unable to reserve loopback callback port".to_string())?;
        let port = listener
            .local_addr()
            .map_err(|_| "unable to read loopback callback port".to_string())?
            .port();
        (
            format!("http://127.0.0.1:{port}{CALLBACK_PATH}"),
            Some(listener),
        )
    } else {
        (
            format!("{CALLBACK_SCHEME}://{CALLBACK_HOST}{CALLBACK_PATH}"),
            None,
        )
    };
    let authorize_url = provider_connection::build_authorize_url(
        config,
        &client_id,
        &redirect_uri,
        &state,
        &code_verifier,
    )?;
    let token_endpoint = config
        .token_url
        .ok_or_else(|| "provider token endpoint is not published".to_string())?;

    let transaction = PendingOAuth {
        provider_id: provider_id.clone(),
        state: state.clone(),
        code_verifier,
        redirect_uri: redirect_uri.clone(),
        client_id,
        client_secret: provider_connection::client_secret(config),
        token_endpoint: token_endpoint.to_string(),
        scope: config.scope.to_string(),
        expires_at: now_seconds() + TRANSACTION_TTL_SECONDS,
    };

    let mut guard = pending
        .lock()
        .map_err(|_| "oauth state unavailable".to_string())?;
    guard.replace(transaction);
    if let Some(listener) = listener {
        let mut loopback_guard = loopback_listener
            .lock()
            .map_err(|_| "oauth loopback state unavailable".to_string())?;
        loopback_guard.replace(listener);
    }

    Ok(OAuthPrepareResult {
        provider_id,
        redirect_uri,
        state: Some(state),
        status: "authorize-ready",
        callback_mode: config.callback_mode,
        authorize_url: Some(authorize_url),
        token_url: Some(token_endpoint.to_string()),
        client_id_env: metadata.client_id_env,
        client_id_configured: true,
        token_exchange_status: metadata.token_exchange_status,
        scope: metadata.scope,
        quota_endpoint: metadata.quota_endpoint,
        quota_endpoint_status: metadata.quota_endpoint_status,
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
    let expected_redirect_uri = format!("{CALLBACK_SCHEME}://{CALLBACK_HOST}{CALLBACK_PATH}");
    parse_callback_for_redirect(url, expected_state, &expected_redirect_uri)
}

fn parse_callback_for_redirect(
    url: &Url,
    expected_state: &str,
    expected_redirect_uri: &str,
) -> Result<ValidatedOAuthCallback, OAuthCallbackError> {
    let expected =
        Url::parse(expected_redirect_uri).map_err(|_| OAuthCallbackError::InvalidHostPath)?;
    if url.scheme() != expected.scheme() {
        return Err(OAuthCallbackError::InvalidScheme);
    }
    if url.host_str() != expected.host_str()
        || url.port_or_known_default() != expected.port_or_known_default()
        || url.path() != expected.path()
    {
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

pub fn take_exchange(
    pending: &Mutex<Option<PendingOAuth>>,
    url: &Url,
) -> Result<OAuthExchangeInput, OAuthCallbackError> {
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

    let ValidatedOAuthCallback { code, state } = if transaction.redirect_uri.starts_with("spectra:")
    {
        parse_callback(url, &transaction.state)?
    } else {
        parse_callback_for_redirect(url, &transaction.state, &transaction.redirect_uri)?
    };
    debug_assert_eq!(state, transaction.state);
    let exchange = OAuthExchangeInput {
        provider_id: transaction.provider_id.clone(),
        code,
        code_verifier: transaction.code_verifier.clone(),
        redirect_uri: transaction.redirect_uri.clone(),
        client_id: transaction.client_id.clone(),
        client_secret: transaction.client_secret.clone(),
        token_endpoint: transaction.token_endpoint.clone(),
        scope: transaction.scope.clone(),
    };
    guard.take();
    Ok(exchange)
}

pub async fn exchange_and_store(
    input: OAuthExchangeInput,
) -> Result<OAuthCallbackEvent, OAuthExchangeError> {
    let client = Client::builder()
        .user_agent("SPECTRA-AI-Usage-Widget/0.1")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| OAuthExchangeError::Network)?;

    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("client_id", input.client_id),
        ("code", input.code),
        ("redirect_uri", input.redirect_uri),
        ("code_verifier", input.code_verifier),
    ];
    if !input.scope.is_empty() {
        form.push(("scope", input.scope));
    }
    if let Some(client_secret) = input.client_secret {
        form.push(("client_secret", client_secret));
    }

    let response = client
        .post(&input.token_endpoint)
        .form(&form)
        .send()
        .await
        .map_err(|_| OAuthExchangeError::Network)?;
    if !response.status().is_success() {
        return Err(OAuthExchangeError::ProviderRejected);
    }

    let token: TokenResponse = response
        .json()
        .await
        .map_err(|_| OAuthExchangeError::InvalidResponse)?;
    if token.access_token.trim().is_empty() {
        return Err(OAuthExchangeError::InvalidResponse);
    }

    let expires_at = token
        .expires_in
        .map(|expires_in| now_seconds().saturating_add(expires_in).to_string());
    let credential = StoredCredential {
        provider_id: input.provider_id.clone(),
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at,
    };
    credential_vault::write(&credential).map_err(|_| OAuthExchangeError::VaultUnavailable)?;

    Ok(OAuthCallbackEvent {
        provider_id: input.provider_id,
        status: "credential-stored",
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

    #[test]
    fn accepts_only_the_reserved_loopback_port() {
        let callback =
            Url::parse("http://127.0.0.1:43127/oauth/callback?code=abc&state=expected").unwrap();
        let result = parse_callback_for_redirect(
            &callback,
            "expected",
            "http://127.0.0.1:43127/oauth/callback",
        )
        .unwrap();
        assert_eq!(result.code, "abc");

        let wrong_port =
            Url::parse("http://127.0.0.1:43128/oauth/callback?code=abc&state=expected").unwrap();
        assert!(matches!(
            parse_callback_for_redirect(
                &wrong_port,
                "expected",
                "http://127.0.0.1:43127/oauth/callback"
            ),
            Err(OAuthCallbackError::InvalidHostPath)
        ));
    }
}
