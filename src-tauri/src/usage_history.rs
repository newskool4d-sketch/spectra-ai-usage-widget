//! Numeric quota history only. No prompts, paths, account details or credentials are retained.
use std::{fs, io::{self, Read, Write}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use crate::provider_usage::{app_data_dir, unix_now, ProviderUsageSnapshot};

const RETENTION_SECS: u64 = 31 * 24 * 60 * 60;
const BUCKET_SECS: u64 = 15 * 60;
// Ten shipped quota series (nine plus Claude's per-model weekly limit) × 96 quarter-hour
// samples × 31 days = 29,760. The cap holds twelve series (35,712) so one more per-model
// window and reset transitions fit without truncating the 30-day view.
const MAX_POINTS: usize = 36_000;
const MAX_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageHistoryPoint {
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_group_label: Option<String>,
    pub window_id: String,
    pub label: String,
    pub captured_at: u64,
    pub remaining_percent: f64,
    pub used_percent: f64,
    pub resets_at: Option<u64>,
    pub window_duration_mins: Option<u64>,
}

fn path() -> Option<PathBuf> { app_data_dir().map(|dir| dir.join("usage-history.json")) }

fn valid(point: &UsageHistoryPoint, now: u64) -> bool {
    matches!(point.provider_id.as_str(), "codex" | "claude" | "antigravity")
        && matches!(point.window_id.as_str(), "rolling" | "weekly")
        && point.captured_at > 0 && point.captured_at <= now.saturating_add(60)
        && point.captured_at >= now.saturating_sub(RETENTION_SECS)
        && point.remaining_percent.is_finite() && (0.0..=100.0).contains(&point.remaining_percent)
        && point.used_percent.is_finite() && (0.0..=100.0).contains(&point.used_percent)
        && (point.remaining_percent + point.used_percent - 100.0).abs() < 0.1
        && point.model_group_id.as_ref().map_or(true, |id| id.len() <= 100 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        && point.model_group_label.as_ref().map_or(true, |label| label.chars().count() <= 100 && !label.contains(['@', '\\', '\n', '\r']))
        && point.label.chars().count() <= 40 && !point.label.contains(['@', '\\', '\n', '\r'])
}

pub fn prune(points: &mut Vec<UsageHistoryPoint>, now: u64) {
    points.retain(|point| valid(point, now));
    points.sort_by_key(|point| point.captured_at);
    if points.len() > MAX_POINTS { points.drain(..points.len() - MAX_POINTS); }
}

/// Keep one observation per provider/group/window/15-minute bucket. A reset remains visible
/// because its reset epoch is part of the bucket identity. Failed and stale lookups add no data.
pub fn record(points: &mut Vec<UsageHistoryPoint>, snapshot: &ProviderUsageSnapshot, now: u64) -> bool {
    if snapshot.connection_state != "connected" || snapshot.live_failure.is_some()
        || !matches!(snapshot.source.as_deref(), Some("codex-app-server" | "claude-statusline" | "claude-usage-api" | "antigravity-statusline")) {
        return false;
    }
    let Some(captured_at) = snapshot.last_synced_at else { return false };
    let mut changed = false;
    for window in &snapshot.windows {
        let point = UsageHistoryPoint {
            provider_id: snapshot.provider_id.clone(), model_group_id: window.model_group_id.clone(),
            model_group_label: window.model_group_label.clone(), window_id: window.id.clone(),
            label: window.label.clone(), captured_at, remaining_percent: window.remaining_percent,
            used_percent: window.used_percent, resets_at: window.resets_at, window_duration_mins: window.window_duration_mins,
        };
        if !valid(&point, now) { continue; }
        let existing = points.iter_mut().rev().find(|old| old.provider_id == point.provider_id
            && old.model_group_id == point.model_group_id && old.window_id == point.window_id
            && old.resets_at == point.resets_at && old.captured_at / BUCKET_SECS == point.captured_at / BUCKET_SECS);
        if let Some(old) = existing {
            if point.captured_at >= old.captured_at && *old != point { *old = point; changed = true; }
        } else { points.push(point); changed = true; }
    }
    if changed { prune(points, now); }
    changed
}

fn load_from(path: &Path, now: u64) -> io::Result<Vec<UsageHistoryPoint>> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_BYTES { return Err(io::Error::new(io::ErrorKind::InvalidData, "history exceeds limit")); }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES { return Err(io::Error::new(io::ErrorKind::InvalidData, "history exceeds limit")); }
    let mut points: Vec<UsageHistoryPoint> = serde_json::from_slice(&bytes)?;
    prune(&mut points, now);
    Ok(points)
}

pub fn load() -> Vec<UsageHistoryPoint> {
    path().and_then(|path| load_from(&path, unix_now()).ok()).unwrap_or_default()
}

fn save_to(path: &Path, points: &[UsageHistoryPoint]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "history directory unavailable"))?;
    fs::create_dir_all(parent)?;
    let staging = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = fs::File::create(&staging)?;
        serde_json::to_writer(&mut file, points)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&staging, path)
    })();
    if result.is_err() { let _ = fs::remove_file(&staging); }
    result
}

pub fn save(points: &[UsageHistoryPoint]) -> io::Result<()> {
    let path = path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "history directory unavailable"))?;
    save_to(&path, points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::ProviderQuotaWindow;
    fn sample(at: u64) -> ProviderUsageSnapshot {
        ProviderUsageSnapshot { provider_id: "antigravity".into(), runtime_available: true,
            auth_state: "signed-in".into(), connection_state: "connected".into(), auth_method: None,
            plan_type: Some("pro".into()), source: Some("antigravity-statusline".into()),
            last_synced_at: Some(at), bridge_installed: true, message: "private message excluded".into(), live_failure: None,
            windows: vec![ProviderQuotaWindow { id: "weekly".into(), label: "주간 한도".into(),
                used_percent: 17.0, remaining_percent: 83.0, resets_at: Some(at + 604800), window_duration_mins: Some(10080),
                model_group_id: Some("gemini-argon".into()), model_group_label: Some("Gemini 4 Argon".into()) }] }
    }
    #[test]
    fn duplicate_snapshots_are_not_double_counted() {
        let mut points = vec![]; let sample = sample(1000000);
        assert!(record(&mut points, &sample, 1000000));
        assert!(!record(&mut points, &sample, 1000000)); assert_eq!(points.len(), 1);
    }
    #[test]
    fn groups_are_distinct_and_updates_replace_bucket() {
        let mut points = vec![]; let mut sample = sample(1000000);
        record(&mut points, &sample, 1000000);
        sample.last_synced_at = Some(1000010); sample.windows[0].remaining_percent = 80.0; sample.windows[0].used_percent = 20.0;
        record(&mut points, &sample, 1000010); assert_eq!(points.len(), 1); assert_eq!(points[0].remaining_percent, 80.0);
        sample.windows[0].model_group_id = Some("claude-gpt".into());
        record(&mut points, &sample, 1000010); assert_eq!(points.len(), 2);
    }
    #[test]
    fn thirty_one_days_of_twelve_quota_series_fit_without_truncation() {
        let now = 4_000_000;
        let base = sample(now).windows.remove(0);
        let mut points = Vec::new();
        for series in 0..12 {
            for bucket in 0..31 * 96 {
                points.push(UsageHistoryPoint {
                    provider_id: "antigravity".into(), model_group_id: Some(format!("series-{series}")),
                    model_group_label: Some("Gemini".into()), window_id: base.id.clone(), label: base.label.clone(),
                    captured_at: now - bucket * BUCKET_SECS, remaining_percent: 83.0, used_percent: 17.0,
                    resets_at: None, window_duration_mins: Some(10080),
                });
            }
        }
        let count = points.len(); prune(&mut points, now); assert_eq!(points.len(), count);
        assert_eq!(count, 35_712);
    }

    #[test]
    fn stale_failed_future_and_invalid_data_are_excluded() {
        let mut points = vec![]; let mut sample = sample(1000000);
        sample.connection_state = "stale".into(); assert!(!record(&mut points, &sample, 1000000));
        sample.connection_state = "connected".into(); sample.live_failure = Some("failed".into()); assert!(!record(&mut points, &sample, 1000000));
        sample.live_failure = None; sample.last_synced_at = Some(1001000); assert!(!record(&mut points, &sample, 1000000));
        sample.last_synced_at = Some(1000000); sample.windows[0].remaining_percent = f64::NAN; assert!(!record(&mut points, &sample, 1000000));
    }
    #[test]
    fn retention_and_round_trip_store_only_allowlisted_numbers() {
        let mut points = vec![]; record(&mut points, &sample(1000000), 1000000);
        let serialized = serde_json::to_string(&points).unwrap();
        assert!(!serialized.contains("private")); assert!(!serialized.contains("authState")); assert!(!serialized.contains("planType"));
        let dir = std::env::temp_dir().join(format!("spectra-history-{}", uuid::Uuid::new_v4()));
        let path = dir.join("history.json"); save_to(&path, &points).unwrap();
        assert_eq!(load_from(&path, 1000000).unwrap(), points);
        points[0].remaining_percent = 80.0; points[0].used_percent = 20.0;
        save_to(&path, &points).unwrap(); assert_eq!(load_from(&path, 1000000).unwrap(), points);
        assert!(load_from(&path, 1000000 + RETENTION_SECS + 1).unwrap().is_empty());
        fs::remove_file(path).unwrap(); fs::remove_dir(dir).unwrap();
    }
}
