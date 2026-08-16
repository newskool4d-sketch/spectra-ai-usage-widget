use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const CLAUDE_BRIDGE_FLAG: &str = "--claude-statusline-bridge";
const PROVIDER_SNAPSHOT_FLAG: &str = "--provider-snapshot";
const RPC_TIMEOUT: Duration = Duration::from_secs(12);
const STATUSLINE_COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_CODEX_RPC_STREAM_BYTES: u64 = 2 * 1024 * 1024;
const MAX_JSON_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_STATUSLINE_INPUT_BYTES: u64 = 1024 * 1024;
const MAX_STATUSLINE_OUTPUT_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderQuotaWindow {
    pub id: String,
    pub label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub resets_at: Option<u64>,
    pub window_duration_mins: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageSnapshot {
    pub provider_id: String,
    pub runtime_available: bool,
    pub auth_state: String,
    pub connection_state: String,
    pub auth_method: Option<String>,
    pub plan_type: Option<String>,
    pub source: Option<String>,
    pub last_synced_at: Option<u64>,
    pub bridge_installed: bool,
    pub windows: Vec<ProviderQuotaWindow>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderActionResult {
    pub provider_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Clone, Debug)]
enum CommandKind {
    Direct,
    Cmd,
    PowerShell,
}

#[derive(Clone, Debug)]
struct CommandSpec {
    path: PathBuf,
    kind: CommandKind,
}

impl CommandSpec {
    fn command(&self, args: &[&str]) -> Command {
        let mut command = match self.kind {
            CommandKind::Direct => Command::new(&self.path),
            CommandKind::Cmd => {
                let mut command = Command::new("cmd.exe");
                command.args(["/D", "/S", "/C"]).arg(&self.path);
                command
            }
            CommandKind::PowerShell => {
                let mut command = Command::new("powershell.exe");
                command
                    .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
                    .arg(&self.path);
                command
            }
        };
        command.args(args);
        hide_window(&mut command);
        command
    }
}

#[cfg(target_os = "windows")]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_window(_command: &mut Command) {}

fn resolve_command(name: &str) -> Option<CommandSpec> {
    let path = env::var_os("PATH")?;
    let directories: Vec<PathBuf> = env::split_paths(&path).collect();

    #[cfg(target_os = "windows")]
    return resolve_windows_command(name, &directories);

    #[cfg(not(target_os = "windows"))]
    {
        directories.into_iter().find_map(|directory| {
            let path = directory.join(name);
            path.is_file().then_some(CommandSpec {
                path,
                kind: CommandKind::Direct,
            })
        })
    }
}

#[cfg(target_os = "windows")]
fn resolve_windows_command(name: &str, directories: &[PathBuf]) -> Option<CommandSpec> {
    let candidates = [
        (format!("{name}.exe"), CommandKind::Direct),
        (format!("{name}.cmd"), CommandKind::Cmd),
        (format!("{name}.bat"), CommandKind::Cmd),
        (format!("{name}.ps1"), CommandKind::PowerShell),
    ];
    for directory in directories {
        for (candidate, kind) in &candidates {
            let path = directory.join(candidate);
            if path.is_file() {
                return Some(CommandSpec {
                    path,
                    kind: kind.clone(),
                });
            }
        }
    }
    None
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unavailable_snapshot(provider_id: &str, message: &str) -> ProviderUsageSnapshot {
    ProviderUsageSnapshot {
        provider_id: provider_id.to_string(),
        runtime_available: false,
        auth_state: "unknown".to_string(),
        connection_state: "not-installed".to_string(),
        auth_method: None,
        plan_type: None,
        source: None,
        last_synced_at: None,
        bridge_installed: false,
        windows: Vec::new(),
        message: message.to_string(),
    }
}

struct CodexRpc {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
}

impl CodexRpc {
    fn start(spec: &CommandSpec) -> Result<Self, String> {
        let mut child = spec
            .command(&["app-server", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "codex-app-server-start-failed".to_string())?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "codex-app-server-stdin-unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "codex-app-server-stdout-unavailable".to_string())?;
        let (sender, messages) = mpsc::channel();
        std::thread::Builder::new()
            .name("spectra-codex-app-server-reader".to_string())
            .spawn(move || {
                for line in BufReader::new(stdout)
                    .take(MAX_CODEX_RPC_STREAM_BYTES)
                    .lines()
                    .map_while(Result::ok)
                {
                    if let Ok(message) = serde_json::from_str::<Value>(&line) {
                        if sender.send(message).is_err() {
                            break;
                        }
                    }
                }
            })
            .map_err(|_| "codex-app-server-reader-failed".to_string())?;

        let mut rpc = Self {
            child,
            stdin,
            messages,
        };
        rpc.write(&json!({
            "method": "initialize",
            "id": 1,
            "params": {
                "clientInfo": {
                    "name": "spectra_usage_widget",
                    "title": "SPECTRA AI Usage Widget",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }))?;
        rpc.wait_for(1)?;
        rpc.write(&json!({ "method": "initialized", "params": {} }))?;
        Ok(rpc)
    }

    fn write(&mut self, value: &Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.stdin, value)
            .map_err(|_| "codex-app-server-write-failed".to_string())?;
        self.stdin
            .write_all(b"\n")
            .and_then(|_| self.stdin.flush())
            .map_err(|_| "codex-app-server-write-failed".to_string())
    }

    fn request(&mut self, id: u64, method: &str, params: Option<Value>) -> Result<Value, String> {
        let mut request = Map::new();
        request.insert("method".to_string(), Value::String(method.to_string()));
        request.insert("id".to_string(), Value::Number(id.into()));
        if let Some(params) = params {
            request.insert("params".to_string(), params);
        }
        self.write(&Value::Object(request))?;
        self.wait_for(id)
    }

    fn wait_for(&mut self, id: u64) -> Result<Value, String> {
        let deadline = std::time::Instant::now() + RPC_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err("codex-app-server-timeout".to_string());
            }
            let message = self
                .messages
                .recv_timeout(remaining)
                .map_err(|_| "codex-app-server-timeout".to_string())?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if message.get("error").is_some() {
                return Err("codex-app-server-request-failed".to_string());
            }
            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

impl Drop for CodexRpc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn quota_label(duration_mins: Option<u64>, fallback: &str) -> String {
    let Some(minutes) = duration_mins else {
        return fallback.to_string();
    };
    if minutes >= 7 * 24 * 60 && minutes % (7 * 24 * 60) == 0 {
        let weeks = minutes / (7 * 24 * 60);
        return if weeks == 1 {
            "주간 한도".to_string()
        } else {
            format!("{weeks}주 한도")
        };
    }
    if minutes >= 60 && minutes % 60 == 0 {
        return format!("{}시간 한도", minutes / 60);
    }
    format!("{minutes}분 한도")
}

fn quota_window(value: &Value, fallback_id: &str, fallback_label: &str) -> Option<ProviderQuotaWindow> {
    let used_percent = value.get("usedPercent")?.as_f64()?.clamp(0.0, 100.0);
    let duration = value.get("windowDurationMins").and_then(Value::as_u64);
    // Codex's "primary"/"secondary" API fields don't reliably correspond to
    // short/long windows — a plan can expose a single, week-long window as
    // "primary". Derive the id from the actual duration (the same signal
    // quota_label already uses) so a truly weekly window always lands under
    // the "weekly" id, regardless of which field it arrived in.
    let id = match duration {
        Some(minutes) if minutes >= 7 * 24 * 60 => "weekly".to_string(),
        Some(_) => "rolling".to_string(),
        None => fallback_id.to_string(),
    };
    Some(ProviderQuotaWindow {
        id,
        label: quota_label(duration, fallback_label),
        used_percent,
        remaining_percent: (100.0 - used_percent).clamp(0.0, 100.0),
        resets_at: value.get("resetsAt").and_then(Value::as_u64),
        window_duration_mins: duration,
    })
}

fn parse_codex_windows(result: &Value) -> Vec<ProviderQuotaWindow> {
    let limits = result
        .get("rateLimits")
        .filter(|value| value.is_object())
        .or_else(|| {
            result
                .get("rateLimitsByLimitId")
                .and_then(|value| value.get("codex"))
        });
    let Some(limits) = limits else {
        return Vec::new();
    };
    let mut windows = Vec::new();
    if let Some(primary) = limits
        .get("primary")
        .and_then(|value| quota_window(value, "rolling", "단기 한도"))
    {
        windows.push(primary);
    }
    if let Some(secondary) = limits
        .get("secondary")
        .and_then(|value| quota_window(value, "weekly", "주간 한도"))
    {
        windows.push(secondary);
    }
    windows
}

fn codex_snapshot() -> ProviderUsageSnapshot {
    let Some(spec) = resolve_command("codex") else {
        return unavailable_snapshot("codex", "Codex CLI를 찾지 못했습니다.");
    };
    let mut rpc = match CodexRpc::start(&spec) {
        Ok(rpc) => rpc,
        Err(reason) => {
            return ProviderUsageSnapshot {
                provider_id: "codex".to_string(),
                runtime_available: true,
                auth_state: "unknown".to_string(),
                connection_state: "error".to_string(),
                auth_method: None,
                plan_type: None,
                source: None,
                last_synced_at: None,
                bridge_installed: false,
                windows: Vec::new(),
                message: format!("Codex App Server를 시작하지 못했습니다. ({reason})"),
            }
        }
    };
    let account_result =
        match rpc.request(2, "account/read", Some(json!({ "refreshToken": false }))) {
            Ok(result) => result,
            Err(reason) => {
                return ProviderUsageSnapshot {
                    provider_id: "codex".to_string(),
                    runtime_available: true,
                    auth_state: "unknown".to_string(),
                    connection_state: "error".to_string(),
                    auth_method: None,
                    plan_type: None,
                    source: None,
                    last_synced_at: None,
                    bridge_installed: false,
                    windows: Vec::new(),
                    message: format!("Codex 계정 상태를 확인하지 못했습니다. ({reason})"),
                }
            }
        };
    let Some(account) = account_result
        .get("account")
        .filter(|value| !value.is_null())
    else {
        return ProviderUsageSnapshot {
            provider_id: "codex".to_string(),
            runtime_available: true,
            auth_state: "signed-out".to_string(),
            connection_state: "signed-out".to_string(),
            auth_method: None,
            plan_type: None,
            source: None,
            last_synced_at: None,
            bridge_installed: false,
            windows: Vec::new(),
            message: "ChatGPT 계정 로그인이 필요합니다.".to_string(),
        };
    };
    let account_type = account
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let plan_type = account
        .get("planType")
        .and_then(Value::as_str)
        .map(str::to_string);
    let subscription_auth = matches!(
        account_type.as_str(),
        "chatgpt" | "chatgptAuthTokens" | "agentIdentity" | "personalAccessToken"
    );
    if !subscription_auth {
        return ProviderUsageSnapshot {
            provider_id: "codex".to_string(),
            runtime_available: true,
            auth_state: "signed-in".to_string(),
            connection_state: "waiting-for-usage".to_string(),
            auth_method: Some(account_type),
            plan_type,
            source: None,
            last_synced_at: None,
            bridge_installed: false,
            windows: Vec::new(),
            message: "API 키가 아닌 ChatGPT 요금제 로그인이 필요합니다.".to_string(),
        };
    }

    let limits = match rpc.request(3, "account/rateLimits/read", None) {
        Ok(result) => result,
        Err(reason) => {
            return ProviderUsageSnapshot {
                provider_id: "codex".to_string(),
                runtime_available: true,
                auth_state: "signed-in".to_string(),
                connection_state: "error".to_string(),
                auth_method: Some(account_type),
                plan_type,
                source: Some("codex-app-server".to_string()),
                last_synced_at: None,
                bridge_installed: false,
                windows: Vec::new(),
                message: format!("Codex 요금제 한도를 가져오지 못했습니다. ({reason})"),
            }
        }
    };
    let windows = parse_codex_windows(&limits);
    let connected = !windows.is_empty();
    ProviderUsageSnapshot {
        provider_id: "codex".to_string(),
        runtime_available: true,
        auth_state: "signed-in".to_string(),
        connection_state: if connected {
            "connected"
        } else {
            "waiting-for-usage"
        }
        .to_string(),
        auth_method: Some(account_type),
        plan_type,
        source: Some("codex-app-server".to_string()),
        last_synced_at: connected.then(unix_now),
        bridge_installed: false,
        windows,
        message: if connected {
            "Codex 공식 요금제 한도를 동기화했습니다."
        } else {
            "계정은 연결됐지만 표시할 한도 창이 없습니다."
        }
        .to_string(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeAuthStatus {
    logged_in: bool,
    auth_method: Option<String>,
    subscription_type: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ClaudeRateWindow {
    used_percentage: f64,
    resets_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ClaudeUsageCache {
    captured_at: u64,
    five_hour: Option<ClaudeRateWindow>,
    seven_day: Option<ClaudeRateWindow>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeBridgeConfig {
    previous_status_line: Option<Value>,
    previous_command: Option<String>,
}

fn app_data_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os("SPECTRA_DATA_DIR") {
        return Some(PathBuf::from(path));
    }
    #[cfg(target_os = "windows")]
    if let Some(path) = env::var_os("LOCALAPPDATA") {
        return Some(PathBuf::from(path).join("SPECTRA"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join(".local").join("share").join("spectra"))
}

fn claude_settings_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("SPECTRA_CLAUDE_SETTINGS") {
        return Some(PathBuf::from(path));
    }
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .map(|path| path.join(".claude").join("settings.json"))
}

fn claude_cache_path() -> Option<PathBuf> {
    app_data_dir().map(|path| path.join("claude-usage.json"))
}

fn claude_bridge_config_path() -> Option<PathBuf> {
    app_data_dir().map(|path| path.join("claude-statusline-bridge.json"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    if fs::metadata(path)
        .map_err(|_| "json-read-failed".to_string())?
        .len()
        > MAX_JSON_FILE_BYTES
    {
        return Err("json-too-large".to_string());
    }
    let content = fs::read_to_string(path).map_err(|_| "json-read-failed".to_string())?;
    serde_json::from_str(&content).map_err(|_| "json-invalid".to_string())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "json-parent-missing".to_string())?;
    fs::create_dir_all(parent).map_err(|_| "json-parent-create-failed".to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "json-encode-failed".to_string())?;
    fs::write(path, bytes).map_err(|_| "json-write-failed".to_string())
}

fn claude_bridge_installed_at(settings_path: &Path) -> bool {
    read_json(settings_path)
        .ok()
        .and_then(|value| {
            value
                .get("statusLine")
                .and_then(|status| status.get("command"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .is_some_and(|command| command.contains(CLAUDE_BRIDGE_FLAG))
}

fn claude_bridge_installed() -> bool {
    claude_settings_path()
        .as_deref()
        .is_some_and(claude_bridge_installed_at)
}

fn command_output(spec: &CommandSpec, args: &[&str]) -> Result<Output, String> {
    spec.command(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| "provider-command-failed".to_string())
}

fn claude_snapshot() -> ProviderUsageSnapshot {
    let Some(spec) = resolve_command("claude") else {
        return unavailable_snapshot("claude", "Claude Code를 찾지 못했습니다.");
    };
    let output = match command_output(&spec, &["auth", "status"]) {
        Ok(output) => output,
        Err(_) => {
            return ProviderUsageSnapshot {
                provider_id: "claude".to_string(),
                runtime_available: true,
                auth_state: "unknown".to_string(),
                connection_state: "error".to_string(),
                auth_method: None,
                plan_type: None,
                source: None,
                last_synced_at: None,
                bridge_installed: claude_bridge_installed(),
                windows: Vec::new(),
                message: "Claude 계정 상태를 확인하지 못했습니다.".to_string(),
            }
        }
    };
    let status = serde_json::from_slice::<ClaudeAuthStatus>(&output.stdout).ok();
    let Some(status) = status.filter(|status| status.logged_in && output.status.success()) else {
        return ProviderUsageSnapshot {
            provider_id: "claude".to_string(),
            runtime_available: true,
            auth_state: "signed-out".to_string(),
            connection_state: "signed-out".to_string(),
            auth_method: None,
            plan_type: None,
            source: None,
            last_synced_at: None,
            bridge_installed: claude_bridge_installed(),
            windows: Vec::new(),
            message: "Claude Pro/Max 계정 로그인이 필요합니다.".to_string(),
        };
    };

    let bridge_installed = claude_bridge_installed();
    let cache = claude_cache_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str::<ClaudeUsageCache>(&content).ok());
    let mut windows = Vec::new();
    let mut captured_at = None;
    let mut stale = false;
    if let Some(cache) = cache {
        captured_at = Some(cache.captured_at);
        let now = unix_now();
        stale = now.saturating_sub(cache.captured_at) > 24 * 60 * 60;
        if let Some(window) = cache.five_hour {
            stale |= window.resets_at.is_some_and(|reset| reset <= now);
            let used = window.used_percentage.clamp(0.0, 100.0);
            windows.push(ProviderQuotaWindow {
                id: "rolling".to_string(),
                label: "5시간 한도".to_string(),
                used_percent: used,
                remaining_percent: (100.0 - used).clamp(0.0, 100.0),
                resets_at: window.resets_at,
                window_duration_mins: Some(5 * 60),
            });
        }
        if let Some(window) = cache.seven_day {
            stale |= window.resets_at.is_some_and(|reset| reset <= now);
            let used = window.used_percentage.clamp(0.0, 100.0);
            windows.push(ProviderQuotaWindow {
                id: "weekly".to_string(),
                label: "주간 한도".to_string(),
                used_percent: used,
                remaining_percent: (100.0 - used).clamp(0.0, 100.0),
                resets_at: window.resets_at,
                window_duration_mins: Some(7 * 24 * 60),
            });
        }
    }
    let has_usage = !windows.is_empty();
    let connection_state = if has_usage && stale {
        "stale"
    } else if has_usage {
        "connected"
    } else {
        "waiting-for-usage"
    };
    let message = if env::var_os("ANTHROPIC_API_KEY").is_some() {
        "ANTHROPIC_API_KEY가 구독 로그인보다 우선할 수 있습니다."
    } else if !bridge_installed {
        "로그인은 확인됐습니다. 사용량 브리지를 설치하면 5시간·주간 한도를 표시합니다."
    } else if !has_usage {
        "브리지 설치 완료. Claude Code에서 응답을 한 번 받은 뒤 동기화됩니다."
    } else if stale {
        "마지막 Claude Code 활동 이후 한도 창이 갱신되지 않았습니다."
    } else {
        "Claude Code 공식 상태선 한도를 동기화했습니다."
    };
    ProviderUsageSnapshot {
        provider_id: "claude".to_string(),
        runtime_available: true,
        auth_state: "signed-in".to_string(),
        connection_state: connection_state.to_string(),
        auth_method: status.auth_method,
        plan_type: status.subscription_type,
        source: has_usage.then(|| "claude-statusline".to_string()),
        last_synced_at: captured_at,
        bridge_installed,
        windows,
        message: message.to_string(),
    }
}

pub fn snapshot(provider_id: &str) -> ProviderUsageSnapshot {
    match provider_id {
        "codex" => codex_snapshot(),
        "claude" => claude_snapshot(),
        _ => unavailable_snapshot(provider_id, "지원하지 않는 공급자입니다."),
    }
}

pub fn start_login(provider_id: &str) -> ProviderActionResult {
    let (command_name, args): (&str, &[&str]) = match provider_id {
        "codex" => ("codex", &["login"]),
        "claude" => ("claude", &["auth", "login"]),
        _ => {
            return ProviderActionResult {
                provider_id: provider_id.to_string(),
                status: "not-supported".to_string(),
                message: "지원하지 않는 공급자입니다.".to_string(),
            }
        }
    };
    let Some(spec) = resolve_command(command_name) else {
        return ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-installed".to_string(),
            message: format!("{command_name} CLI를 찾지 못했습니다."),
        };
    };
    match spec
        .command(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "login-started".to_string(),
            message: "공식 브라우저 로그인을 시작했습니다. 완료되면 자동으로 다시 확인합니다."
                .to_string(),
        },
        Err(_) => ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-available".to_string(),
            message: "공식 로그인 프로세스를 시작하지 못했습니다.".to_string(),
        },
    }
}

fn install_claude_bridge_at(
    settings_path: &Path,
    bridge_config_path: &Path,
    executable: &Path,
) -> Result<(), String> {
    let mut settings = if settings_path.exists() {
        read_json(settings_path)?
    } else {
        Value::Object(Map::new())
    };
    let settings_object = settings
        .as_object_mut()
        .ok_or_else(|| "claude-settings-invalid".to_string())?;
    let previous = settings_object.get("statusLine").cloned();
    if previous
        .as_ref()
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains(CLAUDE_BRIDGE_FLAG))
    {
        return bridge_config_path
            .is_file()
            .then_some(())
            .ok_or_else(|| "claude-bridge-config-missing".to_string());
    }
    if previous
        .as_ref()
        .is_some_and(|value| value.get("type").and_then(Value::as_str) != Some("command"))
    {
        return Err("claude-statusline-type-unsupported".to_string());
    }
    let previous_command = previous
        .as_ref()
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let bridge_config = ClaudeBridgeConfig {
        previous_status_line: previous.clone(),
        previous_command,
    };
    write_json(
        bridge_config_path,
        &serde_json::to_value(bridge_config).map_err(|_| "bridge-config-invalid".to_string())?,
    )?;

    let command = format!(
        "\"{}\" {CLAUDE_BRIDGE_FLAG}",
        executable.to_string_lossy().replace('"', "")
    );
    let mut status_line = previous
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    status_line.insert("type".to_string(), Value::String("command".to_string()));
    status_line.insert("command".to_string(), Value::String(command));
    settings_object.insert("statusLine".to_string(), Value::Object(status_line));
    write_json(settings_path, &settings)
}

fn remove_claude_bridge_at(settings_path: &Path, bridge_config_path: &Path) -> Result<(), String> {
    if !settings_path.exists() {
        return Ok(());
    }
    let mut settings = read_json(settings_path)?;
    let settings_object = settings
        .as_object_mut()
        .ok_or_else(|| "claude-settings-invalid".to_string())?;
    let active_bridge = settings_object
        .get("statusLine")
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains(CLAUDE_BRIDGE_FLAG));
    if !active_bridge {
        return Err("claude-bridge-not-active".to_string());
    }
    let config = serde_json::from_value::<ClaudeBridgeConfig>(read_json(bridge_config_path)?)
        .map_err(|_| "claude-bridge-config-invalid".to_string())?;
    if let Some(previous) = config.previous_status_line {
        settings_object.insert("statusLine".to_string(), previous);
    } else {
        settings_object.remove("statusLine");
    }
    write_json(settings_path, &settings)
}

pub fn install_bridge(provider_id: &str) -> ProviderActionResult {
    if provider_id != "claude" {
        return ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-supported".to_string(),
            message: "이 공급자는 별도 사용량 브리지가 필요하지 않습니다.".to_string(),
        };
    }
    let Some(settings_path) = claude_settings_path() else {
        return ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-available".to_string(),
            message: "Claude 설정 경로를 찾지 못했습니다.".to_string(),
        };
    };
    let Some(config_path) = claude_bridge_config_path() else {
        return ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-available".to_string(),
            message: "SPECTRA 데이터 경로를 찾지 못했습니다.".to_string(),
        };
    };
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(_) => {
            return ProviderActionResult {
                provider_id: provider_id.to_string(),
                status: "not-available".to_string(),
                message: "SPECTRA 실행 경로를 확인하지 못했습니다.".to_string(),
            }
        }
    };
    match install_claude_bridge_at(&settings_path, &config_path, &executable) {
        Ok(()) => ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "bridge-installed".to_string(),
            message: "Claude 상태선 브리지를 설치했습니다. 기존 상태선 출력은 보존됩니다."
                .to_string(),
        },
        Err(_) => ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-available".to_string(),
            message: "Claude 설정을 보존하며 브리지를 설치하지 못했습니다.".to_string(),
        },
    }
}

pub fn remove_bridge(provider_id: &str) -> ProviderActionResult {
    if provider_id != "claude" {
        return ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-supported".to_string(),
            message: "Codex 자격 증명은 공식 도구가 계속 관리합니다.".to_string(),
        };
    }
    let result = claude_settings_path()
        .zip(claude_bridge_config_path())
        .ok_or_else(|| "bridge-path-unavailable".to_string())
        .and_then(|(settings, config)| remove_claude_bridge_at(&settings, &config));
    match result {
        Ok(()) => ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "bridge-removed".to_string(),
            message: "Claude 브리지를 제거하고 이전 상태선 설정을 복원했습니다.".to_string(),
        },
        Err(_) => ProviderActionResult {
            provider_id: provider_id.to_string(),
            status: "not-available".to_string(),
            message: "Claude 상태선 설정을 복원하지 못했습니다.".to_string(),
        },
    }
}

fn extract_claude_cache(input: &Value) -> Option<ClaudeUsageCache> {
    let limits = input.get("rate_limits")?;
    let parse_window = |name: &str| -> Option<ClaudeRateWindow> {
        let value = limits.get(name)?;
        Some(ClaudeRateWindow {
            used_percentage: value.get("used_percentage")?.as_f64()?.clamp(0.0, 100.0),
            resets_at: value.get("resets_at").and_then(Value::as_u64),
        })
    };
    let five_hour = parse_window("five_hour");
    let seven_day = parse_window("seven_day");
    (five_hour.is_some() || seven_day.is_some()).then(|| ClaudeUsageCache {
        captured_at: unix_now(),
        five_hour,
        seven_day,
    })
}

fn run_previous_statusline(command: &str, input: &[u8]) -> Option<String> {
    if command.contains(CLAUDE_BRIDGE_FLAG) {
        return None;
    }
    #[cfg(target_os = "windows")]
    let mut child = Command::new("cmd.exe")
        .args(["/D", "/S", "/C", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new("sh")
        .args(["-c", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(input).ok()?;
    let deadline = Instant::now() + STATUSLINE_COMMAND_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().ok()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(15));
    };
    if !status.success() {
        return None;
    }
    let mut output = Vec::new();
    child
        .stdout
        .take()?
        .take(MAX_STATUSLINE_OUTPUT_BYTES + 1)
        .read_to_end(&mut output)
        .ok()?;
    if output.len() as u64 > MAX_STATUSLINE_OUTPUT_BYTES {
        return None;
    }
    Some(String::from_utf8_lossy(&output).trim_end().to_string())
}

fn read_statusline_input(reader: impl Read) -> Option<Vec<u8>> {
    let mut input = Vec::new();
    reader
        .take(MAX_STATUSLINE_INPUT_BYTES + 1)
        .read_to_end(&mut input)
        .ok()?;
    (input.len() as u64 <= MAX_STATUSLINE_INPUT_BYTES).then_some(input)
}

pub fn run_statusline_bridge() -> bool {
    if env::args_os().nth(1).as_deref() != Some(&OsString::from(CLAUDE_BRIDGE_FLAG)) {
        return false;
    }
    let Some(input) = read_statusline_input(std::io::stdin()) else {
        println!("Claude");
        return true;
    };
    let parsed = serde_json::from_slice::<Value>(&input).ok();
    if let Some(cache) = parsed.as_ref().and_then(extract_claude_cache) {
        if let Some(path) = claude_cache_path() {
            if let Ok(value) = serde_json::to_value(cache) {
                let _ = write_json(&path, &value);
            }
        }
    }

    let bridge_config = claude_bridge_config_path()
        .and_then(|path| read_json(&path).ok())
        .and_then(|value| serde_json::from_value::<ClaudeBridgeConfig>(value).ok())
        .unwrap_or_default();
    if let Some(previous) = bridge_config
        .previous_command
        .as_deref()
        .and_then(|command| run_previous_statusline(command, &input))
        .filter(|output| !output.is_empty())
    {
        println!("{previous}");
        return true;
    }

    let cache = parsed.as_ref().and_then(extract_claude_cache);
    let mut parts = Vec::new();
    if let Some(window) = cache.as_ref().and_then(|cache| cache.five_hour.as_ref()) {
        parts.push(format!("5h {:.0}%", window.used_percentage));
    }
    if let Some(window) = cache.as_ref().and_then(|cache| cache.seven_day.as_ref()) {
        parts.push(format!("7d {:.0}%", window.used_percentage));
    }
    if parts.is_empty() {
        println!("Claude");
    } else {
        println!("Claude · {}", parts.join(" · "));
    }
    true
}

pub fn run_provider_snapshot_cli() -> bool {
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() != Some(&OsString::from(PROVIDER_SNAPSHOT_FLAG)) {
        return false;
    }
    let provider_id = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());
    let result = snapshot(&provider_id);
    if serde_json::to_writer_pretty(std::io::stdout(), &result).is_ok() {
        println!();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("spectra-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn parses_codex_primary_and_secondary_windows() {
        let result = json!({
            "rateLimits": {
                "primary": { "usedPercent": 25.0, "windowDurationMins": 300, "resetsAt": 1_800_000_000 },
                "secondary": { "usedPercent": 42.0, "windowDurationMins": 10_080, "resetsAt": 1_800_100_000 }
            }
        });
        let windows = parse_codex_windows(&result);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].label, "5시간 한도");
        assert_eq!(windows[0].remaining_percent, 75.0);
        assert_eq!(windows[1].label, "주간 한도");
        assert_eq!(windows[1].remaining_percent, 58.0);
    }

    #[test]
    fn codex_window_id_follows_actual_duration_not_api_field_name() {
        // Some Codex plans expose only one window, under the "primary" field,
        // whose real cadence is weekly — not the 5-hour-ish window "rolling"
        // implies. The id must reflect the real duration so the frontend's
        // by-id lookup (rolling = short window, weekly = long window) stays
        // correct regardless of which API field the window arrived in.
        let result = json!({
            "rateLimits": {
                "primary": { "usedPercent": 49.0, "windowDurationMins": 10_080, "resetsAt": 1_787_224_998 }
            }
        });
        let windows = parse_codex_windows(&result);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].id, "weekly");
        assert_eq!(windows[0].label, "주간 한도");
    }

    #[test]
    fn extracts_only_claude_rate_limit_fields() {
        let input = json!({
            "session_id": "private-session",
            "rate_limits": {
                "five_hour": { "used_percentage": 23.5, "resets_at": 1_800_000_000 },
                "seven_day": { "used_percentage": 41.2, "resets_at": 1_800_100_000 }
            }
        });
        let cache = extract_claude_cache(&input).unwrap();
        assert_eq!(cache.five_hour.unwrap().used_percentage, 23.5);
        assert_eq!(cache.seven_day.unwrap().used_percentage, 41.2);
    }

    #[test]
    fn bridge_install_and_remove_restore_existing_statusline() {
        let root = temp_dir("bridge-restore");
        let settings = root.join("settings.json");
        let bridge = root.join("bridge.json");
        let executable = root.join("spectra-native.exe");
        let original = json!({
            "statusLine": {
                "type": "command",
                "command": "existing-statusline",
                "padding": 2
            },
            "theme": "dark"
        });
        write_json(&settings, &original).unwrap();
        install_claude_bridge_at(&settings, &bridge, &executable).unwrap();
        let installed = read_json(&settings).unwrap();
        assert!(installed["statusLine"]["command"]
            .as_str()
            .unwrap()
            .contains(CLAUDE_BRIDGE_FLAG));
        assert_eq!(installed["statusLine"]["padding"], 2);

        remove_claude_bridge_at(&settings, &bridge).unwrap();
        assert_eq!(read_json(&settings).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bridge_remove_does_not_overwrite_a_newer_user_statusline() {
        let root = temp_dir("bridge-user-change");
        let settings = root.join("settings.json");
        let bridge = root.join("bridge.json");
        let executable = root.join("spectra-native.exe");
        write_json(
            &settings,
            &json!({ "statusLine": { "type": "command", "command": "original" } }),
        )
        .unwrap();
        install_claude_bridge_at(&settings, &bridge, &executable).unwrap();
        write_json(
            &settings,
            &json!({ "statusLine": { "type": "command", "command": "new-user-command" } }),
        )
        .unwrap();

        assert!(remove_claude_bridge_at(&settings, &bridge).is_err());
        assert_eq!(
            read_json(&settings).unwrap()["statusLine"]["command"],
            "new-user-command"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn command_resolution_respects_path_directory_order() {
        let root = temp_dir("command-resolution");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("codex.cmd"), b"").unwrap();
        fs::write(second.join("codex.exe"), b"").unwrap();

        let resolved = resolve_windows_command("codex", &[first.clone(), second]).unwrap();
        assert_eq!(resolved.path, first.join("codex.cmd"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn statusline_input_is_bounded() {
        let accepted = vec![b'x'; MAX_STATUSLINE_INPUT_BYTES as usize];
        let rejected = vec![b'x'; MAX_STATUSLINE_INPUT_BYTES as usize + 1];
        assert!(read_statusline_input(std::io::Cursor::new(accepted)).is_some());
        assert!(read_statusline_input(std::io::Cursor::new(rejected)).is_none());
    }
}
