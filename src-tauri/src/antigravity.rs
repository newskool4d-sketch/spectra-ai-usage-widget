//! Official Antigravity CLI status-line integration.
//! https://antigravity.google/docs/cli/statusline
//! No credentials, process inspection, private RPC, conversation data or raw payloads
//! are read or retained. Only supported quota fractions, reset times and plan tier
//! cross the status-line boundary. The CLI must run to deliver new snapshots.

use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::provider_usage::{self, ProviderActionResult, ProviderQuotaWindow, ProviderUsageSnapshot};

pub const BRIDGE_FLAG: &str = "--antigravity-statusline-bridge";
const FRESH_SECS: u64 = 300;
#[cfg(windows)]
const WINDOWS_COMMAND_PREFIX: &str = "powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand ";
#[cfg(windows)]
const WINDOWS_SCRIPT_PREFIX: &str = "$ProgressPreference = 'SilentlyContinue'; $OutputEncoding = [Text.UTF8Encoding]::new($false); [Console]::InputEncoding = $OutputEncoding; [Console]::OutputEncoding = $OutputEncoding; $payload = [Console]::In.ReadToEnd(); $payload | & '";

#[cfg(windows)]
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 { ALPHABET[((value >> 6) & 63) as usize] as char } else { '=' });
        output.push(if chunk.len() > 2 { ALPHABET[(value & 63) as usize] as char } else { '=' });
    }
    output
}

#[cfg(windows)]
fn decoded_command_script(command: &str) -> Option<String> {
    let encoded = command.strip_prefix(WINDOWS_COMMAND_PREFIX)?;
    if encoded.len() % 4 != 0 || encoded.len() > 32768 { return None; }
    let mut bytes = Vec::with_capacity(encoded.len() / 4 * 3);
    let digit = |ch: u8| match ch {
        b'A'..=b'Z' => Some(ch - b'A'), b'a'..=b'z' => Some(ch - b'a' + 26),
        b'0'..=b'9' => Some(ch - b'0' + 52), b'+' => Some(62), b'/' => Some(63), _ => None,
    };
    for (index, chunk) in encoded.as_bytes().chunks_exact(4).enumerate() {
        let a = digit(chunk[0])?; let b = digit(chunk[1])?;
        let last = (index + 1) * 4 == encoded.len();
        if chunk[2] == b'=' && (chunk[3] != b'=' || !last) { return None; }
        if chunk[3] == b'=' && !last { return None; }
        let c = if chunk[2] == b'=' { 0 } else { digit(chunk[2])? };
        let d = if chunk[3] == b'=' { 0 } else { digit(chunk[3])? };
        bytes.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' { bytes.push((b << 4) | (c >> 2)); }
        if chunk[3] != b'=' { bytes.push((c << 6) | d); }
    }
    if bytes.len() % 2 != 0 { return None; }
    String::from_utf16(&bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect::<Vec<_>>()).ok()
}

fn bridge_command(executable: &Path) -> Result<String, String> {
    let executable = executable.to_str().filter(|path| !path.contains(['\0', '\n', '\r']))
        .ok_or("antigravity-executable-path-invalid")?;
    #[cfg(windows)]
    {
        // Go's os/exec quoting of cmd /c cannot preserve a leading quoted executable.
        // Keep the outer command free of quotes; the UTF-16 PowerShell script owns
        // path quoting and explicitly forwards UTF-8 stdin, never credentials.
        let script = format!("{WINDOWS_SCRIPT_PREFIX}{}' {BRIDGE_FLAG}; exit $LASTEXITCODE", executable.replace('\'', "''"));
        let utf16 = script.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
        Ok(format!("{WINDOWS_COMMAND_PREFIX}{}", base64_encode(&utf16)))
    }
    #[cfg(not(windows))]
    { Ok(format!("'{}' {BRIDGE_FLAG}", executable.replace('\'', "'\"'\"'"))) }
}

fn is_bridge_command(command: &str) -> bool {
    if command.contains(BRIDGE_FLAG) { return true; }
    #[cfg(windows)]
    { return decoded_command_script(command).is_some_and(|script| script.starts_with(WINDOWS_SCRIPT_PREFIX)
        && script.ends_with(&format!("' {BRIDGE_FLAG}; exit $LASTEXITCODE"))); }
    #[cfg(not(windows))]
    { false }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Quota {
    group: String,
    period: String,
    remaining: f64,
    resets_at: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Cache {
    captured_at: u64,
    plan: Option<String>,
    quotas: Vec<Quota>,
}

#[derive(Default, Deserialize, Serialize)]
struct BridgeConfig {
    previous_status_line: Option<Value>,
}

fn settings_path() -> Option<PathBuf> {
    env::var_os("SPECTRA_ANTIGRAVITY_SETTINGS").map(PathBuf::from).or_else(|| {
        provider_usage::provider_home_dir()
            .map(|home| home.join(".gemini").join("antigravity-cli").join("settings.json"))
    })
}

fn cache_path() -> Option<PathBuf> {
    provider_usage::app_data_dir().map(|dir| dir.join("antigravity-usage.json"))
}

fn config_path() -> Option<PathBuf> {
    provider_usage::app_data_dir().map(|dir| dir.join("antigravity-statusline-bridge.json"))
}

fn runtime_available() -> bool {
    #[cfg(windows)]
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        if PathBuf::from(local).join("agy").join("bin").join("agy.exe").is_file() { return true; }
    }
    provider_usage::command_available("agy")
}

fn our_statusline(settings: &Value) -> bool {
    settings.get("statusLine").and_then(|line| line.get("command")).and_then(Value::as_str)
        .is_some_and(is_bridge_command)
}

fn enabled_statusline(settings: &Value) -> bool {
    our_statusline(settings)
        && settings.pointer("/statusLine/enabled").and_then(Value::as_bool) != Some(false)
}

fn previous_command(line: Option<&Value>) -> Option<&str> {
    let line = line?;
    if line.get("enabled").and_then(Value::as_bool) == Some(false) { return None; }
    line.get("command").and_then(Value::as_str).filter(|command| !command.trim().is_empty() && !is_bridge_command(command))
}

fn expand_command_home(command: &str, home: &str) -> Option<String> {
    let (prefix, quote) = match command.as_bytes().first().copied() {
        Some(b'"') => (1, Some('"')),
        Some(b'\'') if !cfg!(windows) => (1, Some('\'')),
        _ => (0, None),
    };
    let candidate = &command[prefix..];
    if !candidate.starts_with('~') || !candidate[1..].chars().next().map_or(true, |ch| ch == '/' || ch == '\\' || Some(ch) == quote) {
        return Some(command.to_string());
    }
    let end = if let Some(quote) = quote { candidate.find(quote)? }
        else { candidate.find(|ch: char| ch.is_whitespace() || "|&;<>".contains(ch)).unwrap_or(candidate.len()) };
    let expanded = format!("{}{}", home.trim_end_matches(['/', '\\']), &candidate[1..end]);
    if expanded.contains(['\n', '\r', '\0']) { return None; }
    #[cfg(windows)]
    let executable = {
        // cmd expands percent variables even inside quotes. Do not reinterpret an
        // unusual home directory as shell syntax; fail closed instead.
        if expanded.contains(['"', '%', '!']) { return None; }
        format!("\"{expanded}\"")
    };
    #[cfg(not(windows))]
    let executable = format!("'{}'", expanded.replace('\'', "'\"'\"'"));
    let tail = prefix + end + usize::from(quote.is_some());
    Some(format!("{executable}{}", &command[tail..]))
}

fn group_label(group: &str) -> Option<&'static str> {
    match group {
        "gemini" => Some("Gemini"),
        "claude-gpt" => Some("Claude · GPT"),
        "gemini-4-argon" => Some("Gemini 4 Argon"),
        _ => None,
    }
}

// Bucket identifiers are public provider metadata, not model display names. Unknown
// durations/groups stay unavailable; never invent a percentage for remaining_amount.
fn bucket_parts(bucket: &str) -> Option<(&'static str, &'static str)> {
    let normalized = bucket.to_ascii_lowercase().replace('_', "-");
    let (prefix, period) = [
        ("-weekly", "weekly"), ("-7d", "weekly"), ("-seven-day", "weekly"),
        ("-five-hour", "rolling"), ("-five-hours", "rolling"), ("-5-hour", "rolling"),
        ("-5-hours", "rolling"), ("-5h", "rolling"), ("-rolling", "rolling"),
    ].into_iter().find_map(|(suffix, period)| normalized.strip_suffix(suffix).map(|prefix| (prefix, period)))?;
    let group = match prefix {
        "gemini" | "gemini-models" => "gemini",
        "claude" | "gpt" | "claude-gpt" | "claude-and-gpt" | "claude-gpt-models" | "3p" => "claude-gpt",
        "gemini-4-argon" | "gemini-argon" | "argon" | "gemini-argon-wtu" => "gemini-4-argon",
        _ => return None,
    };
    Some((group, period))
}

fn extract_cache(input: &Value, now: u64) -> Option<Cache> {
    if input.get("product").and_then(Value::as_str) != Some("antigravity") { return None; }
    let buckets = input.get("quota")?.as_object()?;
    let mut quotas = Vec::<Quota>::new();
    for (bucket, value) in buckets {
        let Some((group, period)) = bucket_parts(bucket) else { continue; };
        if value.get("disabled").and_then(Value::as_bool) == Some(true) { continue; }
        let Some(fraction) = value.get("remaining_fraction").and_then(Value::as_f64)
            .filter(|fraction| fraction.is_finite() && (0.0..=1.0).contains(fraction)) else { continue; };
        let resets_at = value.get("reset_time").and_then(Value::as_str)
            .and_then(provider_usage::parse_rfc3339_seconds)
            .or_else(|| value.get("reset_in_seconds").and_then(Value::as_u64)
                .filter(|seconds| *seconds <= 8 * 24 * 60 * 60).and_then(|seconds| now.checked_add(seconds)));
        if let Some(existing) = quotas.iter_mut().find(|quota| quota.group == group && quota.period == period) {
            // If an upstream alias is emitted alongside the main bucket, retain the
            // most restrictive value rather than silently overstating the remaining quota.
            if fraction * 100.0 < existing.remaining {
                existing.remaining = fraction * 100.0;
                existing.resets_at = resets_at;
            }
        } else {
            quotas.push(Quota { group: group.into(), period: period.into(), remaining: fraction * 100.0, resets_at });
        }
    }
    if quotas.is_empty() { return None; }
    let plan = input.get("plan_tier").and_then(Value::as_str).and_then(|tier| match tier.to_ascii_lowercase().as_str() {
        "pro" | "google ai pro" => Some("Google AI Pro"),
        "ultra" | "google ai ultra" => Some("Google AI Ultra"),
        "free" => Some("Antigravity Free"),
        _ => None,
    }).map(str::to_string);
    Some(Cache { captured_at: now, plan, quotas })
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path.parent().ok_or("antigravity-path-invalid")?;
    fs::create_dir_all(parent).map_err(|_| "antigravity-write-failed")?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|_| "antigravity-write-failed")?;
        let bytes = serde_json::to_vec_pretty(value).map_err(|_| "antigravity-json-failed")?;
        file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|_| "antigravity-write-failed")?;
        drop(file);
        fs::rename(&temporary, path).map_err(|_| "antigravity-write-failed")?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result.map_err(str::to_string)
}

fn install_at(settings_path: &Path, backup_path: &Path, executable: &Path) -> Result<(), String> {
    let mut settings = if settings_path.exists() { provider_usage::read_json(settings_path)? } else { json!({}) };
    let command = bridge_command(executable)?;
    if our_statusline(&settings) {
        // Never overwrite the original backup on reinstall. A disabled bridge is
        // explicitly re-enabled only when the user requests installation again.
        provider_usage::read_json(backup_path).and_then(|value| serde_json::from_value::<BridgeConfig>(value)
            .map_err(|_| "antigravity-backup-invalid".to_string()))?;
        settings["statusLine"]["enabled"] = Value::Bool(true);
        settings["statusLine"]["command"] = json!(command);
        return write_json_atomic(settings_path, &settings);
    }
    let object = settings.as_object_mut().ok_or("antigravity-settings-invalid")?;
    let previous = object.get("statusLine").cloned();
    if previous.as_ref().is_some_and(|value| !value.is_object()) { return Err("antigravity-statusline-invalid".into()); }
    if previous.as_ref().and_then(|value| value.get("type")).and_then(Value::as_str).is_some_and(|kind| kind != "command") {
        return Err("antigravity-statusline-type-unsupported".into());
    }
    write_json_atomic(backup_path, &serde_json::to_value(BridgeConfig { previous_status_line: previous.clone() })
        .map_err(|_| "antigravity-backup-invalid")?)?;
    let has_custom = previous_command(previous.as_ref()).is_some();
    let mut line = previous.unwrap_or_else(|| json!({}));
    line["type"] = json!("command");
    line["command"] = json!(command);
    line["enabled"] = Value::Bool(true);
    // A disabled custom command falls back to the CLI's default status line.
    // Keep that default visible while the quota-only bridge is enabled.
    if !has_custom { line["stack_with_default"] = Value::Bool(true); }
    object.insert("statusLine".into(), line);
    write_json_atomic(settings_path, &settings)
}

fn remove_at(settings_path: &Path, backup_path: &Path) -> Result<(), String> {
    let mut settings = provider_usage::read_json(settings_path)?;
    if !our_statusline(&settings) { return Err("antigravity-bridge-not-active".into()); }
    let config: BridgeConfig = serde_json::from_value(provider_usage::read_json(backup_path)?).map_err(|_| "antigravity-backup-invalid")?;
    let object = settings.as_object_mut().ok_or("antigravity-settings-invalid")?;
    if let Some(previous) = config.previous_status_line { object.insert("statusLine".into(), previous); }
    else { object.remove("statusLine"); }
    write_json_atomic(settings_path, &settings)
}

fn action(success: bool, installed: bool) -> ProviderActionResult {
    ProviderActionResult {
        provider_id: "antigravity".into(),
        status: if !success { "not-available" } else if installed { "bridge-installed" } else { "bridge-removed" }.into(),
        message: if !success { "Antigravity CLI의 기존 설정을 보존하며 사용량 연결을 변경하지 못했습니다." }
            else if installed { "Antigravity CLI 사용량 연결을 설정했습니다. agy를 실행하고 /usage를 열면 수치가 갱신됩니다. 기존 상태선 출력은 보존됩니다." }
            else { "Antigravity 사용량 연결을 해제하고 이전 상태선 설정을 복원했습니다." }.into(),
    }
}

pub fn install_bridge() -> ProviderActionResult {
    let result = settings_path().zip(config_path()).zip(env::current_exe().ok())
        .ok_or_else(|| "antigravity-path-unavailable".to_string())
        .and_then(|((settings, backup), executable)| install_at(&settings, &backup, &executable));
    action(result.is_ok(), true)
}

pub fn remove_bridge() -> ProviderActionResult {
    let result = settings_path().zip(config_path()).ok_or_else(|| "antigravity-path-unavailable".to_string())
        .and_then(|(settings, backup)| remove_at(&settings, &backup));
    action(result.is_ok(), false)
}

fn snapshot_at(cache: Option<Cache>, runtime: bool, bridge: bool, now: u64) -> ProviderUsageSnapshot {
    let mut result = ProviderUsageSnapshot {
        provider_id: "antigravity".into(), runtime_available: runtime,
        auth_state: "unknown".into(), connection_state: if runtime { "waiting-for-usage" } else { "not-installed" }.into(),
        auth_method: Some("provider-delegated".into()), plan_type: None, source: None,
        last_synced_at: None, bridge_installed: bridge, windows: Vec::new(), live_failure: None,
        message: if !runtime { "Antigravity CLI(agy)를 찾지 못했습니다." }
            else if !bridge { "사용량 연결을 설정한 뒤 Antigravity CLI(agy)를 실행해 주세요." }
            else { "Antigravity CLI에서 /usage를 열면 모델군별 한도가 전달됩니다. 아직 수신한 데이터가 없습니다." }.into(),
    };
    if !runtime || !bridge { return result; }
    let Some(cache) = cache.filter(|cache| cache.captured_at <= now && cache.captured_at > 0) else { return result; };
    for quota in &cache.quotas {
        let Some(group_label) = group_label(&quota.group) else { continue; };
        if !quota.remaining.is_finite() || !(0.0..=100.0).contains(&quota.remaining) { continue; }
        let (label, duration) = match quota.period.as_str() { "rolling" => ("5시간 한도", 300), "weekly" => ("주간 한도", 10080), _ => continue };
        result.windows.push(ProviderQuotaWindow {
            id: quota.period.clone(), model_group_id: Some(quota.group.clone()), model_group_label: Some(group_label.into()),
            label: label.into(), used_percent: 100.0 - quota.remaining, remaining_percent: quota.remaining,
            resets_at: quota.resets_at, window_duration_mins: Some(duration),
        });
    }
    if result.windows.is_empty() { return result; }
    let stale = now.saturating_sub(cache.captured_at) > FRESH_SECS
        || result.windows.iter().any(|window| window.resets_at.is_some_and(|reset| reset <= now));
    result.auth_state = "signed-in".into();
    result.connection_state = if stale { "stale" } else { "connected" }.into();
    result.plan_type = cache.plan;
    result.source = Some("antigravity-statusline".into());
    result.last_synced_at = Some(cache.captured_at);
    result.message = if stale { "Antigravity CLI의 마지막 수신값입니다. agy에서 /usage를 열어 갱신해 주세요." }
        else { "Antigravity CLI 공식 상태선에서 수신했습니다. GUI 앱만 사용하면 자동 갱신되지 않습니다." }.into();
    result
}

pub fn snapshot() -> ProviderUsageSnapshot {
    let cache = cache_path().and_then(|path| provider_usage::read_json(&path).ok())
        .and_then(|value| serde_json::from_value(value).ok());
    let bridge = settings_path().and_then(|path| provider_usage::read_json(&path).ok()).is_some_and(|settings| enabled_statusline(&settings));
    snapshot_at(cache, runtime_available(), bridge, provider_usage::unix_now())
}

pub fn run_statusline_bridge() -> bool {
    if env::args_os().nth(1).as_deref() != Some(&OsString::from(BRIDGE_FLAG)) { return false; }
    let Some(input) = provider_usage::read_statusline_input(std::io::stdin()) else { return true; };
    if let Some(cache) = serde_json::from_slice::<Value>(&input).ok().as_ref()
        .and_then(|value| extract_cache(value, provider_usage::unix_now())) {
        if let Some(path) = cache_path() {
            if let Ok(value) = serde_json::to_value(cache) { let _ = write_json_atomic(&path, &value); }
        }
    }
    // Forward only to the exact command the user already configured; nothing from
    // the payload can become a shell command. Do not retain the raw stdin anywhere.
    if let Some(config) = config_path().and_then(|path| provider_usage::read_json(&path).ok())
        .and_then(|value| serde_json::from_value::<BridgeConfig>(value).ok()) {
        if let Some(command) = previous_command(config.previous_status_line.as_ref()) {
            let home = provider_usage::provider_home_dir();
            let expanded = match home.as_deref().and_then(Path::to_str) { Some(home) => expand_command_home(command, home), None => Some(command.to_string()) };
            if let Some(output) = expanded.as_deref().and_then(|command| provider_usage::run_previous_statusline(command, &input)) {
                println!("{output}");
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> Value {
        json!({"product":"antigravity", "plan_tier":"Pro", "email":"PRIVATE_EMAIL", "conversation_title":"PRIVATE_TITLE", "cwd":"PRIVATE_PATH", "quota": {
            "gemini-weekly":{"remaining_fraction":0.9,"reset_in_seconds":600},
            "gemini-five-hour":{"remaining_fraction":0.5,"reset_in_seconds":300},
            "claude-gpt-weekly":{"remaining_fraction":1.0},
            "gemini-4-argon-weekly":{"remaining_fraction":0.83}
        }})
    }

    #[test]
    fn only_allowlisted_quota_and_plan_fields_are_persisted() {
        let cache = extract_cache(&payload(), 1000).unwrap();
        assert_eq!(cache.quotas.len(), 4);
        assert_eq!(cache.plan.as_deref(), Some("Google AI Pro"));
        let encoded = serde_json::to_string(&cache).unwrap();
        for forbidden in ["PRIVATE", "email", "title", "cwd", "token", "conversation"] { assert!(!encoded.contains(forbidden)); }
        assert_eq!(cache.quotas.iter().find(|q| q.group == "gemini" && q.period == "rolling").unwrap().resets_at, Some(1300));
    }

    #[test]
    fn documented_fields_and_observed_cli_bucket_ids_produce_all_five_windows() {
        // Bucket identifiers were checked against the CLI /usage model-group labels.
        // Values below are synthetic; no live statusline payload is retained.
        let input = json!({"product":"antigravity","plan_tier":"Pro","quota": {
            "3p-5h":{"remaining_fraction":0.65,"reset_time":"2026-11-03T00:00:00Z","reset_in_seconds":600},
            "3p-weekly":{"remaining_fraction":0.72,"reset_time":"2026-11-05T00:00:00Z","reset_in_seconds":3600},
            "gemini-5h":{"remaining_fraction":0.8,"reset_time":"2026-11-03T00:00:00Z","reset_in_seconds":600},
            "gemini-argon-wtu-weekly":{"remaining_fraction":0.7,"reset_time":"2026-11-05T00:00:00Z","reset_in_seconds":3600},
            "gemini-weekly":{"remaining_fraction":0.9,"reset_time":"2026-11-05T00:00:00Z","reset_in_seconds":3600}
        }});
        let cache = extract_cache(&input, 1000).unwrap();
        assert_eq!(cache.quotas.len(), 5);
        assert_eq!(cache.quotas.iter().filter(|quota| quota.group == "gemini").count(), 2);
        assert_eq!(cache.quotas.iter().filter(|quota| quota.group == "claude-gpt").count(), 2);
        assert_eq!(cache.quotas.iter().filter(|quota| quota.group == "gemini-4-argon" && quota.period == "weekly").count(), 1);
        assert!(cache.quotas.iter().all(|quota| quota.resets_at.is_some()));
    }

    #[test]
    fn invalid_disabled_unknown_and_amount_only_buckets_do_not_become_percentages() {
        let p = json!({"product":"antigravity","quota": {
            "gemini-weekly":{"remaining_fraction":1.1}, "gemini-five-hour":{"remaining_fraction":0.8,"disabled":true},
            "gemini-4-argon-weekly":{"remaining_amount":83}, "private-title-weekly":{"remaining_fraction":1.0}
        }});
        assert!(extract_cache(&p, 1000).is_none());
        let mut wrong = payload(); wrong["product"] = json!("other");
        assert!(extract_cache(&wrong, 1000).is_none());
    }

    #[test]
    fn capture_age_and_reset_boundaries_preserve_stale_semantics() {
        let cache = extract_cache(&payload(), 1000).unwrap();
        let fresh = snapshot_at(Some(cache.clone()), true, true, 1100);
        assert_eq!(fresh.connection_state, "connected");
        assert_eq!(fresh.windows.len(), 4);
        assert_eq!(fresh.source.as_deref(), Some("antigravity-statusline"));
        assert_eq!(snapshot_at(Some(cache.clone()), true, true, 1300).connection_state, "stale");
        assert_eq!(snapshot_at(Some(cache.clone()), true, true, 900).connection_state, "waiting-for-usage");
        assert!(snapshot_at(Some(cache), true, false, 1100).windows.is_empty());
        assert_eq!(snapshot_at(None, false, false, 1000).connection_state, "not-installed");
    }

    #[test]
    fn existing_command_and_settings_are_restored_after_repeated_install() {
        let dir = env::temp_dir().join(format!("spectra-antigravity-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let settings = dir.join("settings.json"); let backup = dir.join("backup.json");
        let original = json!({"unrelated":17,"statusLine":{"command":"echo original","padding":2,"stack_with_default":false,"enabled":false}});
        write_json_atomic(&settings, &original).unwrap();
        install_at(&settings, &backup, Path::new("C:/Apps/SPECTRA.exe")).unwrap();
        install_at(&settings, &backup, Path::new("C:/Apps/SPECTRA.exe")).unwrap();
        let current = provider_usage::read_json(&settings).unwrap();
        assert!(enabled_statusline(&current));
        assert_eq!(current["statusLine"]["stack_with_default"], true);
        assert_eq!(current["statusLine"]["padding"], 2);
        remove_at(&settings, &backup).unwrap();
        assert_eq!(provider_usage::read_json(&settings).unwrap(), original);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn removal_does_not_overwrite_a_new_user_command() {
        let dir = env::temp_dir().join(format!("spectra-antigravity-{}", uuid::Uuid::new_v4()));
        let settings = dir.join("settings.json"); let backup = dir.join("backup.json");
        install_at(&settings, &backup, Path::new("C:/Apps/SPECTRA.exe")).unwrap();
        let replacement = json!({"statusLine":{"command":"echo new"}});
        write_json_atomic(&settings, &replacement).unwrap();
        assert!(remove_at(&settings, &backup).is_err());
        assert_eq!(provider_usage::read_json(&settings).unwrap(), replacement);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn default_statusline_is_kept_and_malformed_settings_are_not_replaced() {
        let dir = env::temp_dir().join(format!("spectra-antigravity-{}", uuid::Uuid::new_v4()));
        let settings = dir.join("settings.json"); let backup = dir.join("backup.json");
        install_at(&settings, &backup, Path::new("C:/Apps/SPECTRA.exe")).unwrap();
        assert_eq!(provider_usage::read_json(&settings).unwrap()["statusLine"]["stack_with_default"], true);
        remove_at(&settings, &backup).unwrap();
        assert_eq!(provider_usage::read_json(&settings).unwrap(), json!({}));
        fs::write(&settings, "{broken").unwrap();
        assert!(install_at(&settings, &backup, Path::new("C:/Apps/SPECTRA.exe")).is_err());
        assert_eq!(fs::read_to_string(&settings).unwrap(), "{broken");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn disabled_previous_command_never_reaches_the_execution_branch() {
        let disabled = json!({"command":"echo must-not-run","enabled":false,"stack_with_default":false});
        let mut executions = 0;
        if previous_command(Some(&disabled)).is_some() { executions += 1; }
        assert_eq!(executions, 0);
        assert_eq!(previous_command(Some(&json!({"command":"echo enabled","enabled":true}))), Some("echo enabled"));
        assert_eq!(previous_command(Some(&json!({"command":"echo implicit"}))), Some("echo implicit"));
        assert_eq!(previous_command(Some(&json!({"command":"  "}))), None);
    }

    #[test]
    fn enabled_custom_command_retains_its_default_stacking_preference() {
        let dir = env::temp_dir().join(format!("spectra-antigravity-{}", uuid::Uuid::new_v4()));
        let settings = dir.join("settings.json"); let backup = dir.join("backup.json");
        let original = json!({"statusLine":{"command":"echo active","enabled":true,"stack_with_default":false}});
        write_json_atomic(&settings, &original).unwrap();
        install_at(&settings, &backup, Path::new("C:/Apps/SPECTRA.exe")).unwrap();
        assert_eq!(provider_usage::read_json(&settings).unwrap()["statusLine"]["stack_with_default"], false);
        let config: BridgeConfig = serde_json::from_value(provider_usage::read_json(&backup).unwrap()).unwrap();
        assert_eq!(previous_command(config.previous_status_line.as_ref()), Some("echo active"));
        remove_at(&settings, &backup).unwrap();
        assert_eq!(provider_usage::read_json(&settings).unwrap(), original);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn leading_tilde_expands_only_the_executable_and_preserves_arguments() {
        let home = "C:/Users/테스트 사용자";
        #[cfg(windows)]
        let executable = "\"C:/Users/테스트 사용자/.gemini/status.cmd\"";
        #[cfg(not(windows))]
        let executable = "'C:/Users/테스트 사용자/.gemini/status.cmd'";
        assert_eq!(expand_command_home("~/.gemini/status.cmd --mode slim", home), Some(format!("{executable} --mode slim")));
        assert_eq!(expand_command_home("\"~/.gemini/status.cmd\" --mode slim", home), Some(format!("{executable} --mode slim")));
        assert_eq!(expand_command_home("echo ~/unchanged", home), Some("echo ~/unchanged".into()));
        assert_eq!(expand_command_home("~another-user/status.cmd", home), Some("~another-user/status.cmd".into()));
        assert_eq!(expand_command_home("\"~/unterminated", home), None);
    }

    #[cfg(windows)]
    #[test]
    fn encoded_bridge_is_recognized_and_preserves_an_escaped_unicode_path() {
        let command = bridge_command(Path::new("C:/Users/한글 사용자's App/SPECTRA.exe")).unwrap();
        assert!(!command.contains('"'));
        assert!(command.starts_with(WINDOWS_COMMAND_PREFIX));
        let script = decoded_command_script(&command).unwrap();
        assert!(script.contains("'C:/Users/한글 사용자''s App/SPECTRA.exe'"));
        assert!(is_bridge_command(&command));
        assert_eq!(previous_command(Some(&json!({"command":command}))), None, "a saved bridge cannot chain recursively");
        assert!(!is_bridge_command("powershell.exe -EncodedCommand unrelated"));
    }

    #[cfg(unix)]
    #[test]
    fn unix_bridge_quotes_a_macos_bundle_path_and_preserves_utf8_stdin() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};
        let dir = env::temp_dir().join(format!("spectra 한글 user's $(printf BAD) {}", uuid::Uuid::new_v4()));
        let binary_dir = dir.join("SPECTRA.app/Contents/MacOS");
        fs::create_dir_all(&binary_dir).unwrap();
        let executable = binary_dir.join("spectra-native");
        fs::write(&executable, b"#!/bin/sh\ncat\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let command = bridge_command(&executable).unwrap();
        assert!(is_bridge_command(&command));
        let mut child = Command::new("/bin/sh").args(["-c", &command])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let input = json!({"product":"antigravity", "fixture":"한글 보존", "quota":{"gemini-weekly":{"remaining_fraction":0.73}}});
        child.stdin.take().unwrap().write_all(&serde_json::to_vec(&input).unwrap()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(serde_json::from_slice::<Value>(&output.stdout).unwrap(), input);
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn cmd_encoded_wrapper_preserves_utf8_stdin_with_spaces_and_korean_path() {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        let dir = env::temp_dir().join(format!("spectra 한글 wrapper {}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let executable = dir.join("한글 user's input.cmd");
        fs::write(&executable, b"@echo off\r\nfindstr /R .\r\n").unwrap();
        let command = bridge_command(&executable).unwrap();
        let mut child = Command::new("cmd.exe").args(["/D", "/S", "/C", &command])
            .creation_flags(0x0800_0000).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().unwrap();
        let input = json!({"product":"antigravity","plan_tier":"Pro","fixture":"한글 보존","quota":{"gemini-weekly":{"remaining_fraction":0.73}}});
        child.stdin.take().unwrap().write_all(&serde_json::to_vec(&input).unwrap()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "wrapper failed: {}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty(), "wrapper must not emit module-loading progress or errors");
        assert_eq!(serde_json::from_slice::<Value>(&output.stdout).unwrap(), input);
        fs::remove_dir_all(&dir).unwrap();
    }
}
