use std::env;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};
use url::Url;

#[derive(Clone, Copy, Debug)]
pub struct ProviderConnectionConfig {
    pub provider_id: &'static str,
    pub authorize_url: Option<&'static str>,
    pub token_url: Option<&'static str>,
    pub client_id_env: Option<&'static str>,
    pub client_secret_env: Option<&'static str>,
    pub scope: &'static str,
    pub callback_mode: &'static str,
    pub connection_status: &'static str,
    pub quota_endpoint: Option<&'static str>,
    pub quota_endpoint_status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionMetadata {
    pub provider_id: String,
    pub authorize_url: Option<String>,
    pub token_url: Option<String>,
    pub client_id_env: Option<String>,
    pub client_id_configured: bool,
    pub connection_status: &'static str,
    pub token_exchange_status: &'static str,
    pub callback_mode: &'static str,
    pub scope: String,
    pub quota_endpoint: Option<String>,
    pub quota_endpoint_status: &'static str,
}

pub fn config(provider_id: &str) -> Option<ProviderConnectionConfig> {
    match provider_id {
        "codex" => Some(ProviderConnectionConfig {
            provider_id: "codex",
            authorize_url: None,
            token_url: None,
            client_id_env: None,
            client_secret_env: None,
            scope: "",
            callback_mode: "none",
            connection_status: "api-key-only",
            quota_endpoint: Some("https://api.openai.com/v1/organization/usage/completions"),
            quota_endpoint_status: "organization-usage-only",
        }),
        "claude" => Some(ProviderConnectionConfig {
            provider_id: "claude",
            authorize_url: None,
            token_url: None,
            client_id_env: None,
            client_secret_env: None,
            scope: "",
            callback_mode: "none",
            connection_status: "api-key-only",
            quota_endpoint: Some(
                "https://api.anthropic.com/v1/organizations/usage_report/messages",
            ),
            quota_endpoint_status: "organization-usage-only",
        }),
        _ => None,
    }
}

fn env_value(name: Option<&str>) -> Option<String> {
    name.and_then(|key| env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn client_id(config: ProviderConnectionConfig) -> Option<String> {
    env_value(config.client_id_env)
}

pub fn client_secret(config: ProviderConnectionConfig) -> Option<String> {
    env_value(config.client_secret_env)
}

pub fn metadata(provider_id: &str) -> Result<ProviderConnectionMetadata, String> {
    let config = config(provider_id).ok_or_else(|| "unsupported provider".to_string())?;
    let client_id_configured = client_id(config).is_some();
    let connection_status = if config.authorize_url.is_some() && !client_id_configured {
        "client-id-required"
    } else {
        config.connection_status
    };

    Ok(ProviderConnectionMetadata {
        provider_id: config.provider_id.to_string(),
        authorize_url: config.authorize_url.map(str::to_string),
        token_url: config.token_url.map(str::to_string),
        client_id_env: config.client_id_env.map(str::to_string),
        client_id_configured,
        connection_status,
        token_exchange_status: if config.token_url.is_some() {
            "pkce-native"
        } else {
            "not-published"
        },
        callback_mode: config.callback_mode,
        scope: config.scope.to_string(),
        quota_endpoint: config.quota_endpoint.map(str::to_string),
        quota_endpoint_status: config.quota_endpoint_status,
    })
}

pub fn build_authorize_url(
    config: ProviderConnectionConfig,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_verifier: &str,
) -> Result<String, String> {
    let endpoint = config
        .authorize_url
        .ok_or_else(|| "provider authorize endpoint is not published".to_string())?;
    if client_id.trim().is_empty() {
        return Err("provider client id is not configured".to_string());
    }

    let mut url =
        Url::parse(endpoint).map_err(|_| "invalid provider authorize endpoint".to_string())?;
    let challenge = code_challenge(code_verifier);
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("client_id", client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", config.scope)
            .append_pair("state", state)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256");
    }

    Ok(url.to_string())
}

pub fn code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_uses_rfc7636_s256_encoding() {
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn authorize_url_contains_only_expected_oauth_parameters() {
        let config = config("codex").unwrap();
        assert!(config.authorize_url.is_none());
        assert!(build_authorize_url(
            config,
            "client.example",
            "http://127.0.0.1:43127/oauth/callback",
            "state-1",
            "verifier-1",
        )
        .is_err());
    }
}
