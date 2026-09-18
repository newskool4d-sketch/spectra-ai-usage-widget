//! Pure scheduling rules for the taskbar strip's periodic provider refresh (spec
//! 2026-09-18 §2-3). No I/O here: the loop in `desktop_shell.rs` feeds clock,
//! preference and credential observations in and runs the lookups it is told to.

use std::time::Duration;

use crate::provider_usage::{ProviderUsageSnapshot, CLAUDE_TOKEN_EXPIRED};

/// Sleep between loop iterations; the strip is redrawn on every tick so age labels advance.
pub const TICK: Duration = Duration::from_secs(15);
/// Regular refresh interval per provider.
pub const INTERVAL_SECS: u64 = 120;
/// Minimum gap before a Claude reset boundary triggers another attempt.
pub const RESET_RETRY_SECS: u64 = 60;
/// Upper bound for the exponential failure backoff.
pub const BACKOFF_MAX_SECS: u64 = 1_800;
/// Safety cap on the expired-token hold when no credential change is ever observed.
pub const TOKEN_HOLD_SECS: u64 = 1_800;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    pub fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    TokenExpired,
    Failed,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::TokenExpired => "token-expired",
            Self::Failed => "failed",
        }
    }
}

/// Classifies a finished lookup from the snapshot's `live_failure` code.
pub fn outcome_for(snapshot: &ProviderUsageSnapshot) -> Outcome {
    match snapshot.live_failure.as_deref() {
        None => Outcome::Ok,
        Some(CLAUDE_TOKEN_EXPIRED) => Outcome::TokenExpired,
        Some(_) => Outcome::Failed,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Inputs {
    pub now: u64,
    pub strip_enabled: bool,
    pub claude_reset_due: bool,
    /// Modification stamp of Claude Code's credentials file, `None` when unreadable.
    pub credentials_modified: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    last_attempt: u64,
    next_allowed: u64,
    failures: u32,
}

impl Slot {
    fn new(now: u64) -> Self {
        Self { last_attempt: now, next_allowed: 0, failures: 0 }
    }

    fn elapsed(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_attempt)
    }

    fn allowed(&self, now: u64) -> bool {
        now >= self.next_allowed
    }

    fn interval_due(&self, now: u64) -> bool {
        self.allowed(now) && self.elapsed(now) >= INTERVAL_SECS
    }

    fn clear(&mut self) {
        self.next_allowed = 0;
        self.failures = 0;
    }
}

#[derive(Debug)]
pub struct Scheduler {
    claude: Slot,
    codex: Slot,
    credentials_seen: Option<u64>,
}

impl Scheduler {
    /// `now` counts as the first attempt: the WebView boots with its own refresh, so the
    /// loop must not repeat it within the first tick.
    pub fn new(now: u64) -> Self {
        Self { claude: Slot::new(now), codex: Slot::new(now), credentials_seen: None }
    }

    /// Providers to refresh on this tick, Claude first. The credentials stamp is tracked
    /// even while the strip is off so a change consumed then never fires later; the first
    /// observation only records the stamp.
    pub fn plan(&mut self, input: Inputs) -> Vec<Provider> {
        let credentials_changed = matches!(
            (self.credentials_seen, input.credentials_modified),
            (Some(seen), Some(current)) if seen != current
        );
        if input.credentials_modified.is_some() {
            self.credentials_seen = input.credentials_modified;
        }
        if !input.strip_enabled {
            return Vec::new();
        }
        let now = input.now;
        let mut due = Vec::with_capacity(2);
        if credentials_changed {
            self.claude.clear();
            due.push(Provider::Claude);
        } else if self.claude.interval_due(now)
            || (input.claude_reset_due && self.claude.allowed(now) && self.claude.elapsed(now) >= RESET_RETRY_SECS)
        {
            due.push(Provider::Claude);
        }
        if self.codex.interval_due(now) {
            due.push(Provider::Codex);
        }
        due
    }

    pub fn record(&mut self, provider: Provider, now: u64, outcome: Outcome) {
        let slot = match provider {
            Provider::Claude => &mut self.claude,
            Provider::Codex => &mut self.codex,
        };
        slot.last_attempt = now;
        match outcome {
            Outcome::Ok => slot.clear(),
            Outcome::TokenExpired => {
                slot.failures = 0;
                slot.next_allowed = now + TOKEN_HOLD_SECS;
            }
            Outcome::Failed => {
                slot.failures = slot.failures.saturating_add(1);
                slot.next_allowed = now + backoff_secs(slot.failures);
            }
        }
    }
}

/// 120·2^(n−1) seconds capped at 30 minutes: 120, 240, 480, 960, 1800, 1800…
pub fn backoff_secs(consecutive_failures: u32) -> u64 {
    let exponent = consecutive_failures.saturating_sub(1).min(16);
    (INTERVAL_SECS << exponent).min(BACKOFF_MAX_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::ProviderUsageSnapshot;

    fn input(now: u64) -> Inputs {
        Inputs { now, strip_enabled: true, claude_reset_due: false, credentials_modified: Some(1) }
    }

    #[test]
    fn first_periodic_refresh_waits_one_full_interval_after_start() {
        let mut scheduler = Scheduler::new(1_000);
        assert!(scheduler.plan(input(1_000 + INTERVAL_SECS - 1)).is_empty());
        assert_eq!(scheduler.plan(input(1_000 + INTERVAL_SECS)), vec![Provider::Claude, Provider::Codex]);
    }

    #[test]
    fn disabled_strip_never_plans_a_lookup() {
        let mut scheduler = Scheduler::new(0);
        let mut off = input(10_000);
        off.strip_enabled = false;
        off.claude_reset_due = true;
        assert!(scheduler.plan(off).is_empty());
        assert_eq!(scheduler.plan(input(10_000)), vec![Provider::Claude, Provider::Codex]);
    }

    #[test]
    fn claude_is_planned_before_codex_and_each_follows_its_own_clock() {
        let mut scheduler = Scheduler::new(0);
        scheduler.record(Provider::Codex, 100, Outcome::Ok);
        assert_eq!(scheduler.plan(input(120)), vec![Provider::Claude]);
        scheduler.record(Provider::Claude, 120, Outcome::Ok);
        assert_eq!(scheduler.plan(input(220)), vec![Provider::Codex]);
    }

    #[test]
    fn reset_boundary_retries_claude_after_the_minimum_gap_only() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.record(Provider::Claude, 1_000, Outcome::Ok);
        let mut due = input(1_000 + RESET_RETRY_SECS - 1);
        due.claude_reset_due = true;
        assert!(scheduler.plan(due).is_empty());
        due.now = 1_000 + RESET_RETRY_SECS;
        assert_eq!(scheduler.plan(due), vec![Provider::Claude]);
    }

    #[test]
    fn expired_token_holds_claude_until_the_safety_cap() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.plan(input(1_000));
        scheduler.record(Provider::Claude, 1_000, Outcome::TokenExpired);
        let mut tick = input(1_000 + INTERVAL_SECS);
        tick.claude_reset_due = true;
        assert_eq!(scheduler.plan(tick), vec![Provider::Codex], "a held Claude is skipped even at a reset boundary");
        scheduler.record(Provider::Codex, tick.now, Outcome::Ok);
        tick.now = 1_000 + TOKEN_HOLD_SECS - 1;
        assert!(!scheduler.plan(tick).contains(&Provider::Claude));
        tick.now = 1_000 + TOKEN_HOLD_SECS;
        assert!(scheduler.plan(tick).contains(&Provider::Claude), "the hold expires after 30 minutes as a safety net");
    }

    #[test]
    fn a_credentials_rewrite_refreshes_claude_immediately_and_clears_the_hold() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.plan(input(1_000));
        scheduler.record(Provider::Claude, 1_000, Outcome::TokenExpired);
        let mut changed = input(1_015);
        changed.credentials_modified = Some(2);
        assert_eq!(scheduler.plan(changed), vec![Provider::Claude]);
        scheduler.record(Provider::Claude, 1_015, Outcome::Ok);
        let mut same = input(1_030);
        same.credentials_modified = Some(2);
        assert!(scheduler.plan(same).is_empty(), "the same stamp does not retrigger");
    }

    #[test]
    fn the_first_credentials_observation_never_triggers_a_refresh() {
        let mut scheduler = Scheduler::new(1_000);
        let mut first = input(1_015);
        first.credentials_modified = Some(42);
        assert!(scheduler.plan(first).is_empty());
    }

    #[test]
    fn credential_changes_seen_while_the_strip_is_off_do_not_fire_later() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.plan(input(1_000));
        let mut off = input(1_015);
        off.strip_enabled = false;
        off.credentials_modified = Some(2);
        assert!(scheduler.plan(off).is_empty());
        let mut on = input(1_030);
        on.credentials_modified = Some(2);
        assert!(scheduler.plan(on).is_empty(), "the change was consumed while off; the regular interval takes over");
    }

    #[test]
    fn failures_back_off_exponentially_up_to_thirty_minutes() {
        assert_eq!([1, 2, 3, 4, 5, 6, 40].map(backoff_secs), [120, 240, 480, 960, 1_800, 1_800, 1_800]);
        let mut scheduler = Scheduler::new(0);
        scheduler.record(Provider::Codex, 1_000, Outcome::Failed);
        scheduler.record(Provider::Codex, 1_120, Outcome::Failed);
        assert!(!scheduler.plan(input(1_120 + 239)).contains(&Provider::Codex));
        assert!(scheduler.plan(input(1_120 + 240)).contains(&Provider::Codex));
        scheduler.record(Provider::Codex, 1_360, Outcome::Ok);
        assert!(!scheduler.plan(input(1_360 + 119)).contains(&Provider::Codex));
        assert!(scheduler.plan(input(1_360 + 120)).contains(&Provider::Codex), "success resets the backoff to the regular interval");
    }

    #[test]
    fn outcome_follows_the_snapshot_failure_code() {
        let mut snapshot = ProviderUsageSnapshot {
            provider_id: "claude".to_string(),
            runtime_available: true,
            auth_state: "signed-in".to_string(),
            connection_state: "connected".to_string(),
            auth_method: None,
            plan_type: None,
            source: Some("claude-usage-api".to_string()),
            last_synced_at: Some(1),
            bridge_installed: false,
            windows: Vec::new(),
            message: String::new(),
            live_failure: None,
        };
        assert_eq!(outcome_for(&snapshot), Outcome::Ok);
        snapshot.live_failure = Some(crate::provider_usage::CLAUDE_TOKEN_EXPIRED.to_string());
        assert_eq!(outcome_for(&snapshot), Outcome::TokenExpired);
        snapshot.live_failure = Some("claude-usage-http-429".to_string());
        assert_eq!(outcome_for(&snapshot), Outcome::Failed);
        assert_eq!([Outcome::Ok.label(), Outcome::TokenExpired.label(), Outcome::Failed.label()], ["ok", "token-expired", "failed"]);
        assert_eq!([Provider::Claude.id(), Provider::Codex.id()], ["claude", "codex"]);
    }
}
