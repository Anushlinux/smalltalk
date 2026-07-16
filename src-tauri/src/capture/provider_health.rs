use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum OperationClass {
    FullDisplayScreenshot,
    ActiveWindowScreenshot,
    AccessibilitySnapshot,
    WindowSnapshot,
    Ocr,
}

impl OperationClass {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::FullDisplayScreenshot => "full_display_screenshot",
            Self::ActiveWindowScreenshot => "active_window_screenshot",
            Self::AccessibilitySnapshot => "accessibility_snapshot",
            Self::WindowSnapshot => "window_snapshot",
            Self::Ocr => "ocr",
        }
    }

    fn policy(self) -> ProviderPolicy {
        match self {
            // Losing all primary images is serious, but a single ordinary
            // helper error may be transient. Two consecutive failures open the
            // display breaker for one minute.
            Self::FullDisplayScreenshot => ProviderPolicy::new(2, Duration::from_secs(60)),
            // A signal/abort from active-window ScreenCaptureKit is the known
            // native hazard. Severe failures open immediately and cool down
            // for fifteen minutes, matching the previous containment policy.
            Self::ActiveWindowScreenshot => ProviderPolicy::new(1, Duration::from_secs(15 * 60)),
            // Semantic helpers have truthful fallbacks. Three consecutive
            // failures avoid retrying them on every event for thirty seconds.
            Self::AccessibilitySnapshot | Self::WindowSnapshot => {
                ProviderPolicy::new(3, Duration::from_secs(30))
            }
            Self::Ocr => ProviderPolicy::new(2, Duration::from_secs(60)),
        }
    }
}

const ALL_OPERATIONS: [OperationClass; 5] = [
    OperationClass::FullDisplayScreenshot,
    OperationClass::ActiveWindowScreenshot,
    OperationClass::AccessibilitySnapshot,
    OperationClass::WindowSnapshot,
    OperationClass::Ocr,
];

#[derive(Debug, Clone, Copy)]
struct ProviderPolicy {
    opening_threshold: u32,
    cooldown: Duration,
}

impl ProviderPolicy {
    const fn new(opening_threshold: u32, cooldown: Duration) -> Self {
        Self {
            opening_threshold,
            cooldown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AttemptDecision {
    Normal,
    RecoveryProbe,
    Skip,
}

#[derive(Debug, Clone, Default)]
struct ProviderState {
    consecutive_failures: u32,
    opened_at: Option<Instant>,
    recovery_probe_in_flight: bool,
    opens: u64,
    probes: u64,
    last_provider: Option<String>,
    fallbacks: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ProviderHealthRegistry {
    states: BTreeMap<OperationClass, ProviderState>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(super) struct ProviderHealthDiagnostics {
    pub states: BTreeMap<String, String>,
    pub providers: BTreeMap<String, String>,
    pub fallback_counts: BTreeMap<String, u64>,
    pub circuit_breaker_opens: u64,
    pub recovery_probes: u64,
}

impl ProviderHealthRegistry {
    pub(super) fn decision(&mut self, operation: OperationClass, now: Instant) -> AttemptDecision {
        let policy = operation.policy();
        let state = self.states.entry(operation).or_default();
        let Some(opened_at) = state.opened_at else {
            return AttemptDecision::Normal;
        };
        if state.recovery_probe_in_flight {
            return AttemptDecision::Skip;
        }
        if now.saturating_duration_since(opened_at) < policy.cooldown {
            return AttemptDecision::Skip;
        }
        state.recovery_probe_in_flight = true;
        state.probes = state.probes.saturating_add(1);
        AttemptDecision::RecoveryProbe
    }

    pub(super) fn record_success(&mut self, operation: OperationClass) {
        let state = self.states.entry(operation).or_default();
        state.consecutive_failures = 0;
        state.opened_at = None;
        state.recovery_probe_in_flight = false;
    }

    pub(super) fn record_provider(
        &mut self,
        operation: OperationClass,
        provider: &str,
        fallback: bool,
    ) {
        let state = self.states.entry(operation).or_default();
        state.last_provider = Some(provider.to_string());
        if fallback {
            state.fallbacks = state.fallbacks.saturating_add(1);
        }
    }

    /// Records a safe provider failure. `severe` means a signal, timeout, or
    /// output-contract failure that should open immediately instead of waiting
    /// for the ordinary consecutive-failure threshold.
    pub(super) fn record_failure(
        &mut self,
        operation: OperationClass,
        now: Instant,
        severe: bool,
    ) -> bool {
        let policy = operation.policy();
        let state = self.states.entry(operation).or_default();
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        let should_open = severe
            || state.recovery_probe_in_flight
            || state.consecutive_failures >= policy.opening_threshold;
        state.recovery_probe_in_flight = false;
        if !should_open {
            return false;
        }
        let newly_opened = state.opened_at.is_none();
        state.opened_at = Some(now);
        if newly_opened {
            state.opens = state.opens.saturating_add(1);
        }
        newly_opened
    }

    pub(super) fn reset_for_new_session(&mut self) {
        for operation in ALL_OPERATIONS {
            self.record_success(operation);
        }
    }

    pub(super) fn state_label(&self, operation: OperationClass) -> &'static str {
        match self.states.get(&operation) {
            Some(state) if state.opened_at.is_some() && state.recovery_probe_in_flight => {
                "recovery_probe"
            }
            Some(state) if state.opened_at.is_some() => "open_cooldown",
            Some(state) if state.consecutive_failures > 0 => "closed_failures_observed",
            _ => "closed",
        }
    }

    pub(super) fn diagnostics(&self) -> ProviderHealthDiagnostics {
        let mut diagnostics = ProviderHealthDiagnostics::default();
        for operation in ALL_OPERATIONS {
            diagnostics.states.insert(
                operation.as_str().to_string(),
                self.state_label(operation).to_string(),
            );
            diagnostics.fallback_counts.insert(
                operation.as_str().to_string(),
                self.states
                    .get(&operation)
                    .map(|state| state.fallbacks)
                    .unwrap_or(0),
            );
            if let Some(state) = self.states.get(&operation) {
                if let Some(provider) = state.last_provider.as_ref() {
                    diagnostics
                        .providers
                        .insert(operation.as_str().to_string(), provider.clone());
                }
                diagnostics.circuit_breaker_opens = diagnostics
                    .circuit_breaker_opens
                    .saturating_add(state.opens);
                diagnostics.recovery_probes =
                    diagnostics.recovery_probes.saturating_add(state.probes);
            }
        }
        diagnostics
    }
}

static PROVIDER_HEALTH: OnceLock<Mutex<ProviderHealthRegistry>> = OnceLock::new();

pub(super) fn registry() -> &'static Mutex<ProviderHealthRegistry> {
    PROVIDER_HEALTH.get_or_init(|| Mutex::new(ProviderHealthRegistry::default()))
}

pub(super) fn with_registry<T>(operation: impl FnOnce(&mut ProviderHealthRegistry) -> T) -> T {
    let mut health = registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&mut health)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_health_is_independent_by_operation() {
        let now = Instant::now();
        let mut health = ProviderHealthRegistry::default();
        assert!(health.record_failure(OperationClass::ActiveWindowScreenshot, now, true));
        assert_eq!(
            health.decision(OperationClass::ActiveWindowScreenshot, now),
            AttemptDecision::Skip
        );
        assert_eq!(
            health.decision(OperationClass::FullDisplayScreenshot, now),
            AttemptDecision::Normal
        );
        assert_eq!(
            health.decision(OperationClass::Ocr, now),
            AttemptDecision::Normal
        );
    }

    #[test]
    fn ordinary_failures_open_only_at_the_documented_threshold() {
        let now = Instant::now();
        let mut health = ProviderHealthRegistry::default();
        assert!(!health.record_failure(OperationClass::AccessibilitySnapshot, now, false));
        assert!(!health.record_failure(OperationClass::AccessibilitySnapshot, now, false));
        assert!(health.record_failure(OperationClass::AccessibilitySnapshot, now, false));
        assert_eq!(
            health.decision(OperationClass::AccessibilitySnapshot, now),
            AttemptDecision::Skip
        );
    }

    #[test]
    fn only_one_recovery_probe_can_be_in_flight() {
        let now = Instant::now();
        let mut health = ProviderHealthRegistry::default();
        health.record_failure(OperationClass::Ocr, now, true);
        let after_cooldown = now + OperationClass::Ocr.policy().cooldown;
        assert_eq!(
            health.decision(OperationClass::Ocr, after_cooldown),
            AttemptDecision::RecoveryProbe
        );
        assert_eq!(
            health.decision(OperationClass::Ocr, after_cooldown),
            AttemptDecision::Skip
        );
        health.record_success(OperationClass::Ocr);
        assert_eq!(
            health.decision(OperationClass::Ocr, after_cooldown),
            AttemptDecision::Normal
        );
    }

    #[test]
    fn new_session_resets_every_provider_without_cross_poisoning() {
        let now = Instant::now();
        let mut health = ProviderHealthRegistry::default();
        for operation in ALL_OPERATIONS {
            health.record_failure(operation, now, true);
        }
        health.reset_for_new_session();
        for operation in ALL_OPERATIONS {
            assert_eq!(health.decision(operation, now), AttemptDecision::Normal);
            assert_eq!(health.state_label(operation), "closed");
        }
    }

    #[test]
    fn diagnostics_keep_actual_provider_and_fallback_counts_by_operation() {
        let mut health = ProviderHealthRegistry::default();
        health.record_provider(
            OperationClass::FullDisplayScreenshot,
            "screen_capture_kit",
            false,
        );
        health.record_provider(
            OperationClass::ActiveWindowScreenshot,
            "screencapture_cli",
            true,
        );
        let diagnostics = health.diagnostics();
        assert_eq!(
            diagnostics
                .providers
                .get("full_display_screenshot")
                .map(String::as_str),
            Some("screen_capture_kit")
        );
        assert_eq!(
            diagnostics
                .providers
                .get("active_window_screenshot")
                .map(String::as_str),
            Some("screencapture_cli")
        );
        assert_eq!(
            diagnostics.fallback_counts.get("active_window_screenshot"),
            Some(&1)
        );
        assert_eq!(diagnostics.fallback_counts.get("ocr"), Some(&0));
    }
}
