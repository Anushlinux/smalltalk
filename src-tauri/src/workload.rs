//! Process-wide policy for expensive background work.
//!
//! Cheap status reads and event ingestion deliberately bypass this coordinator.
//! Expensive operations take a typed permit so their overlap and priority are
//! explicit and observable.

use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // The policy catalog is intentionally broader than today's call sites.
pub enum WorkClass {
    CheapStatusRead,
    EventIngest,
    AccessibilitySnapshot,
    ScreenshotCapture,
    Ocr,
    DerivedEvidenceUpdate,
    BackgroundContinue,
    ManualContinue,
    IslandRefresh,
    AuditExport,
    MaintenanceCleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    User = 4,
}

impl WorkClass {
    pub fn priority(self) -> Priority {
        match self {
            Self::ManualContinue | Self::ScreenshotCapture => Priority::User,
            Self::IslandRefresh | Self::AccessibilitySnapshot | Self::Ocr => Priority::High,
            Self::DerivedEvidenceUpdate => Priority::Normal,
            Self::BackgroundContinue | Self::AuditExport | Self::MaintenanceCleanup => {
                Priority::Low
            }
            Self::CheapStatusRead | Self::EventIngest => Priority::High,
        }
    }

    fn group(self) -> Option<Group> {
        match self {
            Self::ManualContinue | Self::BackgroundContinue | Self::IslandRefresh => {
                Some(Group::Continue)
            }
            Self::ScreenshotCapture | Self::AccessibilitySnapshot | Self::Ocr => {
                Some(Group::Capture)
            }
            Self::AuditExport => Some(Group::Audit),
            Self::MaintenanceCleanup => Some(Group::Maintenance),
            Self::DerivedEvidenceUpdate => Some(Group::Derived),
            Self::CheapStatusRead | Self::EventIngest => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    Continue,
    Capture,
    Audit,
    Maintenance,
    Derived,
}

#[derive(Debug)]
struct Waiter {
    id: u64,
    class: WorkClass,
    priority: Priority,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: BTreeMap<Group, WorkClass>,
    queue: VecDeque<Waiter>,
    started: BTreeMap<WorkClass, u64>,
    completed: BTreeMap<WorkClass, u64>,
    failed: BTreeMap<WorkClass, u64>,
    coalesced: u64,
    cancelled_or_superseded: u64,
    background_decisions_avoided: u64,
    durations_ms: BTreeMap<WorkClass, VecDeque<u64>>,
    shutting_down: bool,
}

pub struct WorkloadGovernor {
    state: Mutex<State>,
    changed: Condvar,
}

impl Default for WorkloadGovernor {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
        }
    }
}

static GOVERNOR: OnceLock<WorkloadGovernor> = OnceLock::new();
pub fn governor() -> &'static WorkloadGovernor {
    GOVERNOR.get_or_init(WorkloadGovernor::default)
}

pub struct Permit<'a> {
    governor: &'a WorkloadGovernor,
    class: WorkClass,
    group: Option<Group>,
    started: Instant,
    failed: bool,
}

impl Permit<'_> {
    pub fn mark_failed(&mut self) {
        self.failed = true;
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut state = self
            .governor
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(group) = self.group {
            state.active.remove(&group);
        }
        *state.completed.entry(self.class).or_default() += 1;
        if self.failed {
            *state.failed.entry(self.class).or_default() += 1;
        }
        let samples = state.durations_ms.entry(self.class).or_default();
        samples.push_back(self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
        while samples.len() > 256 {
            samples.pop_front();
        }
        self.governor.changed.notify_all();
    }
}

impl WorkloadGovernor {
    pub fn acquire(&self, class: WorkClass) -> Result<Permit<'_>, String> {
        self.acquire_inner(class, None)
    }

    pub fn acquire_with_timeout(
        &self,
        class: WorkClass,
        timeout: std::time::Duration,
    ) -> Result<Permit<'_>, String> {
        self.acquire_inner(class, Some(timeout))
    }

    fn acquire_inner(
        &self,
        class: WorkClass,
        timeout: Option<std::time::Duration>,
    ) -> Result<Permit<'_>, String> {
        let group = class.group();
        if group.is_none() {
            return Ok(Permit {
                governor: self,
                class,
                group,
                started: Instant::now(),
                failed: false,
            });
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| "workload governor lock poisoned".to_string())?;
        if state.shutting_down {
            return Err("workload governor is shutting down".to_string());
        }
        state.next_id += 1;
        let id = state.next_id;
        state.queue.push_back(Waiter {
            id,
            class,
            priority: class.priority(),
        });
        let deadline = timeout.map(|timeout| Instant::now() + timeout);
        loop {
            if state.shutting_down {
                state.queue.retain(|w| w.id != id);
                state.cancelled_or_superseded += 1;
                return Err("workload governor is shutting down".to_string());
            }
            let can_overlap = allowed_with_active(&state.active, class);
            let is_highest = state
                .queue
                .iter()
                .filter(|w| allowed_with_active(&state.active, w.class))
                .max_by_key(|w| (w.priority, std::cmp::Reverse(w.id)))
                .is_some_and(|w| w.id == id);
            if can_overlap && is_highest {
                state.queue.retain(|w| w.id != id);
                state.active.insert(group.unwrap(), class);
                *state.started.entry(class).or_default() += 1;
                return Ok(Permit {
                    governor: self,
                    class,
                    group,
                    started: Instant::now(),
                    failed: false,
                });
            }
            if let Some(deadline) = deadline {
                let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                    state.queue.retain(|w| w.id != id);
                    state.cancelled_or_superseded += 1;
                    return Err(format!(
                        "workload governor timed out waiting for {}",
                        format!("{class:?}").to_lowercase()
                    ));
                };
                let (next_state, wait_result) = self
                    .changed
                    .wait_timeout(state, remaining)
                    .map_err(|_| "workload governor wait poisoned".to_string())?;
                state = next_state;
                if wait_result.timed_out() {
                    state.queue.retain(|w| w.id != id);
                    state.cancelled_or_superseded += 1;
                    return Err(format!(
                        "workload governor timed out waiting for {}",
                        format!("{class:?}").to_lowercase()
                    ));
                }
            } else {
                state = self
                    .changed
                    .wait(state)
                    .map_err(|_| "workload governor wait poisoned".to_string())?;
            }
        }
    }

    pub fn note_coalesced(&self, background_avoided: bool) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.coalesced += 1;
        if background_avoided {
            state.background_decisions_avoided += 1;
        }
    }

    pub fn diagnostics(&self) -> WorkloadDiagnostics {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let active_operations = state
            .active
            .values()
            .map(|v| format!("{:?}", v).to_lowercase())
            .collect();
        let mut duration_percentiles_ms = BTreeMap::new();
        for (class, samples) in &state.durations_ms {
            let mut sorted: Vec<_> = samples.iter().copied().collect();
            sorted.sort_unstable();
            if !sorted.is_empty() {
                let p50 = sorted[(sorted.len() - 1) * 50 / 100];
                let p95 = sorted[(sorted.len() - 1) * 95 / 100];
                duration_percentiles_ms.insert(
                    format!("{:?}", class).to_lowercase(),
                    DurationPercentiles { p50, p95 },
                );
            }
        }
        WorkloadDiagnostics {
            active_operations,
            queued_operation_count: state.queue.len() as u64,
            coalesced_requests: state.coalesced,
            cancelled_or_superseded_requests: state.cancelled_or_superseded,
            background_decisions_avoided: state.background_decisions_avoided,
            duration_percentiles_ms,
        }
    }

    #[cfg(test)]
    fn shutdown(&self) {
        let mut s = self.state.lock().unwrap();
        s.shutting_down = true;
        s.cancelled_or_superseded += s.queue.len() as u64;
        self.changed.notify_all();
    }
}

fn allowed_with_active(active: &BTreeMap<Group, WorkClass>, class: WorkClass) -> bool {
    let Some(group) = class.group() else {
        return true;
    };
    if active.contains_key(&group) {
        return false;
    }
    match class {
        // Manual Continue establishes its capture boundary before it asks for
        // this permit. If a background capture is still winding down, the
        // decision can safely read the last committed SQLite snapshot while
        // that capture finishes. New capture work remains blocked once Manual
        // Continue owns the Continue group.
        WorkClass::ManualContinue => true,
        WorkClass::IslandRefresh => !active.contains_key(&Group::Capture),
        WorkClass::BackgroundContinue => {
            !active.contains_key(&Group::Capture)
                && !active.contains_key(&Group::Audit)
                && !active.contains_key(&Group::Maintenance)
        }
        WorkClass::ScreenshotCapture | WorkClass::AccessibilitySnapshot | WorkClass::Ocr => {
            !active.contains_key(&Group::Continue)
        }
        WorkClass::AuditExport | WorkClass::MaintenanceCleanup => {
            !active.contains_key(&Group::Continue) && !active.contains_key(&Group::Capture)
        }
        WorkClass::DerivedEvidenceUpdate => !active.contains_key(&Group::Maintenance),
        WorkClass::CheapStatusRead | WorkClass::EventIngest => true,
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DurationPercentiles {
    pub p50: u64,
    pub p95: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct WorkloadDiagnostics {
    pub active_operations: Vec<String>,
    pub queued_operation_count: u64,
    pub coalesced_requests: u64,
    pub cancelled_or_superseded_requests: u64,
    pub background_decisions_avoided: u64,
    pub duration_percentiles_ms: BTreeMap<String, DurationPercentiles>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn policy_classifies_every_required_work_class() {
        let classes = [
            WorkClass::CheapStatusRead,
            WorkClass::EventIngest,
            WorkClass::AccessibilitySnapshot,
            WorkClass::ScreenshotCapture,
            WorkClass::Ocr,
            WorkClass::DerivedEvidenceUpdate,
            WorkClass::BackgroundContinue,
            WorkClass::ManualContinue,
            WorkClass::IslandRefresh,
            WorkClass::AuditExport,
            WorkClass::MaintenanceCleanup,
        ];
        assert!(classes.iter().all(|c| c.priority() >= Priority::Low));
    }

    #[test]
    fn failure_does_not_poison_coordinator() {
        let g = WorkloadGovernor::default();
        {
            let mut p = g.acquire(WorkClass::AuditExport).unwrap();
            p.mark_failed();
        }
        assert!(g.acquire(WorkClass::AuditExport).is_ok());
    }

    #[test]
    fn manual_continue_precedes_queued_maintenance() {
        let g = Arc::new(WorkloadGovernor::default());
        let held = g.acquire(WorkClass::ScreenshotCapture).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let order = Arc::new(Mutex::new(Vec::new()));
        let spawn = |class, label| {
            let g = g.clone();
            let b = barrier.clone();
            let o = order.clone();
            thread::spawn(move || {
                b.wait();
                let _p = g.acquire(class).unwrap();
                o.lock().unwrap().push(label);
            })
        };
        let low = spawn(WorkClass::MaintenanceCleanup, "low");
        let high = spawn(WorkClass::ManualContinue, "manual");
        barrier.wait();
        thread::sleep(Duration::from_millis(20));
        drop(held);
        high.join().unwrap();
        low.join().unwrap();
        assert_eq!(order.lock().unwrap()[0], "manual");
    }

    #[test]
    fn shutdown_rejects_new_and_queued_work() {
        let g = WorkloadGovernor::default();
        g.shutdown();
        assert!(g.acquire(WorkClass::BackgroundContinue).is_err());
    }

    #[test]
    fn timed_acquire_removes_a_waiter_instead_of_waiting_forever() {
        let g = WorkloadGovernor::default();
        let _held = g.acquire(WorkClass::BackgroundContinue).unwrap();

        let error = g
            .acquire_with_timeout(WorkClass::ManualContinue, Duration::from_millis(10))
            .err()
            .expect("manual Continue should time out");

        assert_eq!(
            error,
            "workload governor timed out waiting for manualcontinue"
        );
        assert_eq!(g.diagnostics().queued_operation_count, 0);
        assert_eq!(g.diagnostics().cancelled_or_superseded_requests, 1);
    }

    #[test]
    fn manual_continue_can_use_committed_evidence_while_background_capture_finishes() {
        let g = WorkloadGovernor::default();
        let _capture = g.acquire(WorkClass::ScreenshotCapture).unwrap();

        let manual = g
            .acquire_with_timeout(WorkClass::ManualContinue, Duration::from_millis(10))
            .expect("manual Continue should not wait behind an active capture");

        assert_eq!(
            g.diagnostics().active_operations,
            vec!["manualcontinue", "screenshotcapture"]
        );
        drop(manual);
    }
}
