//! Finite process-wide policy for expensive runtime work.
//!
//! Cheap status reads and event ingestion deliberately bypass admission. Every
//! queued operation has a capacity, deadline, cancellation token, priority,
//! and optional coalescing identity. No caller should hold a SQLite connection
//! or transaction while waiting here.

use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const TOTAL_QUEUE_CAPACITY: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
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

    fn queue_capacity(self) -> usize {
        match self.group() {
            Some(Group::Continue) => 8,
            Some(Group::Capture) => 20,
            Some(Group::Audit) => 2,
            Some(Group::Maintenance) => 1,
            Some(Group::Derived) => 8,
            None => 0,
        }
    }

    fn default_deadline(self) -> Duration {
        match self {
            Self::ManualContinue => Duration::from_secs(8),
            Self::ScreenshotCapture | Self::AccessibilitySnapshot | Self::Ocr => {
                Duration::from_secs(3)
            }
            Self::IslandRefresh | Self::BackgroundContinue => Duration::from_secs(5),
            Self::AuditExport | Self::MaintenanceCleanup | Self::DerivedEvidenceUpdate => {
                Duration::from_secs(2)
            }
            Self::CheapStatusRead | Self::EventIngest => Duration::ZERO,
        }
    }

    fn as_key(self) -> String {
        format!("{self:?}").to_lowercase()
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

#[derive(Debug, Clone, Default)]
pub struct WorkCancellationToken(Arc<AtomicBool>);

impl WorkCancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone)]
pub struct WorkRequest {
    class: WorkClass,
    deadline: Duration,
    identity: Option<String>,
    cancellation: WorkCancellationToken,
}

impl WorkRequest {
    pub fn new(class: WorkClass) -> Self {
        Self {
            class,
            deadline: class.default_deadline(),
            identity: None,
            cancellation: WorkCancellationToken::default(),
        }
    }

    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    pub fn identity(mut self, identity: impl Into<String>) -> Self {
        self.identity = Some(identity.into());
        self
    }

    pub fn cancellation(mut self, cancellation: WorkCancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }
}

#[derive(Debug)]
struct Waiter {
    id: u64,
    class: WorkClass,
    priority: Priority,
    identity: Option<String>,
    deadline: Instant,
    cancellation: WorkCancellationToken,
}

#[derive(Debug)]
struct ActiveWork {
    id: u64,
    class: WorkClass,
    cancellation: WorkCancellationToken,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: BTreeMap<Group, ActiveWork>,
    queue: VecDeque<Waiter>,
    started: BTreeMap<WorkClass, u64>,
    completed: BTreeMap<WorkClass, u64>,
    failed: BTreeMap<WorkClass, u64>,
    rejected: BTreeMap<WorkClass, u64>,
    coalesced: u64,
    cancelled_or_superseded: u64,
    background_decisions_avoided: u64,
    queue_high_water_mark: usize,
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

pub fn shutdown() {
    governor().shutdown_inner();
}

pub struct Permit<'a> {
    governor: &'a WorkloadGovernor,
    id: u64,
    class: WorkClass,
    group: Option<Group>,
    cancellation: WorkCancellationToken,
    started: Instant,
    failed: bool,
}

impl Permit<'_> {
    pub fn mark_failed(&mut self) {
        self.failed = true;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
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
            if state
                .active
                .get(&group)
                .is_some_and(|active| active.id == self.id)
            {
                state.active.remove(&group);
            }
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
        self.acquire_request(WorkRequest::new(class))
    }

    pub fn acquire_with_timeout(
        &self,
        class: WorkClass,
        timeout: Duration,
    ) -> Result<Permit<'_>, String> {
        self.acquire_request(WorkRequest::new(class).deadline(timeout))
    }

    pub fn try_acquire(&self, class: WorkClass) -> Result<Permit<'_>, String> {
        let group = class.group();
        if group.is_none() {
            return self.acquire(class);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| "workload governor lock poisoned".to_string())?;
        if state.shutting_down {
            return Err("workload governor is shutting down".to_string());
        }
        if !allowed_with_active(&state.active, class)
            || state
                .queue
                .iter()
                .any(|waiter| waiter.priority > class.priority())
        {
            return Err(format!("{} is not immediately available", class.as_key()));
        }
        state.next_id = state.next_id.saturating_add(1);
        let id = state.next_id;
        let cancellation = WorkCancellationToken::default();
        state.active.insert(
            group.unwrap(),
            ActiveWork {
                id,
                class,
                cancellation: cancellation.clone(),
            },
        );
        *state.started.entry(class).or_default() += 1;
        Ok(Permit {
            governor: self,
            id,
            class,
            group,
            cancellation,
            started: Instant::now(),
            failed: false,
        })
    }

    pub fn acquire_request(&self, request: WorkRequest) -> Result<Permit<'_>, String> {
        let group = request.class.group();
        if group.is_none() {
            return Ok(Permit {
                governor: self,
                id: 0,
                class: request.class,
                group,
                cancellation: request.cancellation,
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

        if request.class == WorkClass::ManualContinue {
            supersede_background_continue(&mut state);
        }
        if let Some(identity) = request.identity.as_deref() {
            if state.queue.iter().any(|waiter| {
                waiter.class == request.class && waiter.identity.as_deref() == Some(identity)
            }) {
                state.coalesced = state.coalesced.saturating_add(1);
                if request.class == WorkClass::BackgroundContinue {
                    state.background_decisions_avoided =
                        state.background_decisions_avoided.saturating_add(1);
                }
                return Err(format!("{} request coalesced", request.class.as_key()));
            }
        }

        make_capacity_for(&mut state, request.class)?;
        state.next_id = state.next_id.saturating_add(1);
        let id = state.next_id;
        state.queue.push_back(Waiter {
            id,
            class: request.class,
            priority: request.class.priority(),
            identity: request.identity,
            deadline: Instant::now() + request.deadline,
            cancellation: request.cancellation.clone(),
        });
        state.queue_high_water_mark = state.queue_high_water_mark.max(state.queue.len());

        loop {
            if state.shutting_down || request.cancellation.is_cancelled() {
                remove_waiter(&mut state, id);
                return Err(if state.shutting_down {
                    "workload governor is shutting down".to_string()
                } else {
                    format!("{} request cancelled", request.class.as_key())
                });
            }
            let deadline = state
                .queue
                .iter()
                .find(|waiter| waiter.id == id)
                .map(|waiter| waiter.deadline);
            let Some(deadline) = deadline else {
                return Err(format!("{} request superseded", request.class.as_key()));
            };
            if Instant::now() >= deadline {
                remove_waiter(&mut state, id);
                return Err(format!(
                    "workload governor timed out waiting for {}",
                    request.class.as_key()
                ));
            }
            let can_overlap = allowed_with_active(&state.active, request.class);
            let is_highest = state
                .queue
                .iter()
                .filter(|waiter| allowed_with_active(&state.active, waiter.class))
                .max_by_key(|waiter| (waiter.priority, std::cmp::Reverse(waiter.id)))
                .is_some_and(|waiter| waiter.id == id);
            if can_overlap && is_highest {
                state.queue.retain(|waiter| waiter.id != id);
                state.active.insert(
                    group.unwrap(),
                    ActiveWork {
                        id,
                        class: request.class,
                        cancellation: request.cancellation.clone(),
                    },
                );
                *state.started.entry(request.class).or_default() += 1;
                return Ok(Permit {
                    governor: self,
                    id,
                    class: request.class,
                    group,
                    cancellation: request.cancellation,
                    started: Instant::now(),
                    failed: false,
                });
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            let (next, wait_result) = self
                .changed
                .wait_timeout(state, remaining)
                .map_err(|_| "workload governor wait poisoned".to_string())?;
            state = next;
            if wait_result.timed_out() {
                remove_waiter(&mut state, id);
                return Err(format!(
                    "workload governor timed out waiting for {}",
                    request.class.as_key()
                ));
            }
        }
    }

    pub fn note_coalesced(&self, background_avoided: bool) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.coalesced = state.coalesced.saturating_add(1);
        if background_avoided {
            state.background_decisions_avoided =
                state.background_decisions_avoided.saturating_add(1);
        }
    }

    pub fn diagnostics(&self) -> WorkloadDiagnostics {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let active_operations = state
            .active
            .values()
            .map(|active| active.class.as_key())
            .collect();
        let mut queued_by_class = BTreeMap::new();
        let mut queue_capacity_by_class = BTreeMap::new();
        let mut rejected_by_class = BTreeMap::new();
        for class in all_classes() {
            queued_by_class.insert(
                class.as_key(),
                state
                    .queue
                    .iter()
                    .filter(|waiter| waiter.class == class)
                    .count() as u64,
            );
            queue_capacity_by_class.insert(class.as_key(), class.queue_capacity() as u64);
            rejected_by_class.insert(
                class.as_key(),
                state.rejected.get(&class).copied().unwrap_or(0),
            );
        }
        let mut duration_percentiles_ms = BTreeMap::new();
        for (class, samples) in &state.durations_ms {
            let mut sorted: Vec<_> = samples.iter().copied().collect();
            sorted.sort_unstable();
            if !sorted.is_empty() {
                duration_percentiles_ms.insert(
                    class.as_key(),
                    DurationPercentiles {
                        p50: sorted[(sorted.len() - 1) * 50 / 100],
                        p95: sorted[(sorted.len() - 1) * 95 / 100],
                    },
                );
            }
        }
        WorkloadDiagnostics {
            active_operations,
            queued_operation_count: state.queue.len() as u64,
            queue_capacity: TOTAL_QUEUE_CAPACITY as u64,
            queue_high_water_mark: state.queue_high_water_mark as u64,
            queued_by_class,
            queue_capacity_by_class,
            rejected_by_class,
            coalesced_requests: state.coalesced,
            cancelled_or_superseded_requests: state.cancelled_or_superseded,
            background_decisions_avoided: state.background_decisions_avoided,
            shutting_down: state.shutting_down,
            duration_percentiles_ms,
        }
    }

    fn shutdown_inner(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.shutting_down = true;
        let queued = state.queue.len() as u64;
        for waiter in &state.queue {
            waiter.cancellation.cancel();
        }
        for active in state.active.values() {
            active.cancellation.cancel();
        }
        state.queue.clear();
        state.cancelled_or_superseded = state.cancelled_or_superseded.saturating_add(queued);
        self.changed.notify_all();
    }
}

fn make_capacity_for(state: &mut State, class: WorkClass) -> Result<(), String> {
    let group_count = state
        .queue
        .iter()
        .filter(|waiter| waiter.class.group() == class.group())
        .count();
    if state.queue.len() < TOTAL_QUEUE_CAPACITY && group_count < class.queue_capacity() {
        return Ok(());
    }
    if matches!(
        class,
        WorkClass::ManualContinue | WorkClass::ScreenshotCapture
    ) {
        if let Some(index) = state.queue.iter().position(|waiter| {
            matches!(
                waiter.class,
                WorkClass::BackgroundContinue
                    | WorkClass::AuditExport
                    | WorkClass::MaintenanceCleanup
                    | WorkClass::DerivedEvidenceUpdate
            )
        }) {
            if let Some(waiter) = state.queue.remove(index) {
                waiter.cancellation.cancel();
                state.cancelled_or_superseded = state.cancelled_or_superseded.saturating_add(1);
                return Ok(());
            }
        }
    }
    *state.rejected.entry(class).or_default() += 1;
    Err(format!("{} queue is at capacity", class.as_key()))
}

fn supersede_background_continue(state: &mut State) {
    let mut removed = 0_u64;
    state.queue.retain(|waiter| {
        let superseded = matches!(
            waiter.class,
            WorkClass::BackgroundContinue | WorkClass::IslandRefresh
        );
        if superseded {
            waiter.cancellation.cancel();
            removed = removed.saturating_add(1);
        }
        !superseded
    });
    if let Some(active) = state.active.get(&Group::Continue) {
        if matches!(
            active.class,
            WorkClass::BackgroundContinue | WorkClass::IslandRefresh
        ) {
            active.cancellation.cancel();
        }
    }
    state.cancelled_or_superseded = state.cancelled_or_superseded.saturating_add(removed);
}

fn remove_waiter(state: &mut State, id: u64) {
    let before = state.queue.len();
    state.queue.retain(|waiter| waiter.id != id);
    if state.queue.len() != before {
        state.cancelled_or_superseded = state.cancelled_or_superseded.saturating_add(1);
    }
}

fn allowed_with_active(active: &BTreeMap<Group, ActiveWork>, class: WorkClass) -> bool {
    let Some(group) = class.group() else {
        return true;
    };
    if active.contains_key(&group) {
        return false;
    }
    match class {
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

fn all_classes() -> [WorkClass; 11] {
    [
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
    ]
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
    pub queue_capacity: u64,
    pub queue_high_water_mark: u64,
    pub queued_by_class: BTreeMap<String, u64>,
    pub queue_capacity_by_class: BTreeMap<String, u64>,
    pub rejected_by_class: BTreeMap<String, u64>,
    pub coalesced_requests: u64,
    pub cancelled_or_superseded_requests: u64,
    pub background_decisions_avoided: u64,
    pub shutting_down: bool,
    pub duration_percentiles_ms: BTreeMap<String, DurationPercentiles>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn policy_classifies_every_required_work_class() {
        assert!(all_classes()
            .iter()
            .all(|class| class.priority() >= Priority::Low));
    }

    #[test]
    fn failure_does_not_poison_coordinator() {
        let governor = WorkloadGovernor::default();
        {
            let mut permit = governor.acquire(WorkClass::AuditExport).unwrap();
            permit.mark_failed();
        }
        assert!(governor.acquire(WorkClass::AuditExport).is_ok());
    }

    #[test]
    fn manual_continue_precedes_queued_maintenance() {
        let governor = Arc::new(WorkloadGovernor::default());
        let held = governor.acquire(WorkClass::ScreenshotCapture).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let order = Arc::new(Mutex::new(Vec::new()));
        let spawn = |class, label| {
            let governor = governor.clone();
            let barrier = barrier.clone();
            let order = order.clone();
            thread::spawn(move || {
                barrier.wait();
                let _permit = governor.acquire(class).unwrap();
                order.lock().unwrap().push(label);
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
    fn queue_saturation_is_finite_and_observable() {
        let governor = Arc::new(WorkloadGovernor::default());
        let held = governor.acquire(WorkClass::MaintenanceCleanup).unwrap();
        let mut workers = Vec::new();
        for index in 0..16 {
            let governor = governor.clone();
            workers.push(thread::spawn(move || {
                governor
                    .acquire_request(
                        WorkRequest::new(WorkClass::MaintenanceCleanup)
                            .identity(format!("maintenance-{index}"))
                            .deadline(Duration::from_millis(30)),
                    )
                    .map(|_| ())
            }));
        }
        thread::sleep(Duration::from_millis(5));
        let diagnostics = governor.diagnostics();
        assert!(diagnostics.queued_operation_count <= 1);
        assert!(diagnostics.queue_high_water_mark <= diagnostics.queue_capacity);
        drop(held);
        for worker in workers {
            let _ = worker.join();
        }
    }

    #[test]
    fn equivalent_requests_coalesce() {
        let governor = Arc::new(WorkloadGovernor::default());
        let held = governor.acquire(WorkClass::AuditExport).unwrap();
        let first_governor = governor.clone();
        let first = thread::spawn(move || {
            first_governor
                .acquire_request(
                    WorkRequest::new(WorkClass::AuditExport)
                        .identity("decision-1")
                        .deadline(Duration::from_millis(50)),
                )
                .map(|_| ())
        });
        thread::sleep(Duration::from_millis(5));
        let duplicate = governor.acquire_request(
            WorkRequest::new(WorkClass::AuditExport)
                .identity("decision-1")
                .deadline(Duration::from_millis(5)),
        );
        let duplicate_error = match duplicate {
            Err(error) => error,
            Ok(_) => panic!("duplicate audit request should coalesce"),
        };
        assert!(duplicate_error.contains("coalesced"));
        drop(held);
        let _ = first.join();
        assert_eq!(governor.diagnostics().coalesced_requests, 1);
    }

    #[test]
    fn manual_continue_supersedes_queued_background() {
        let governor = Arc::new(WorkloadGovernor::default());
        let held = governor.acquire(WorkClass::BackgroundContinue).unwrap();
        let queued_governor = governor.clone();
        let queued = thread::spawn(move || {
            queued_governor
                .acquire_request(
                    WorkRequest::new(WorkClass::BackgroundContinue)
                        .identity("background-2")
                        .deadline(Duration::from_millis(100)),
                )
                .map(|_| ())
        });
        thread::sleep(Duration::from_millis(5));
        let manual_governor = governor.clone();
        let manual = thread::spawn(move || {
            manual_governor
                .acquire_request(
                    WorkRequest::new(WorkClass::ManualContinue)
                        .deadline(Duration::from_millis(100)),
                )
                .map(|_| ())
        });
        thread::sleep(Duration::from_millis(5));
        assert!(held.is_cancelled());
        drop(held);
        let queued_error = queued.join().unwrap().err().unwrap_or_default();
        assert!(queued_error.contains("superseded") || queued_error.contains("cancelled"));
        assert!(manual.join().unwrap().is_ok());
    }

    #[test]
    fn shutdown_cancels_active_and_queued_work_and_wakes_waiters() {
        let governor = Arc::new(WorkloadGovernor::default());
        let held = governor.acquire(WorkClass::AuditExport).unwrap();
        let waiter_governor = governor.clone();
        let waiter =
            thread::spawn(move || waiter_governor.acquire(WorkClass::AuditExport).map(|_| ()));
        thread::sleep(Duration::from_millis(5));
        governor.shutdown_inner();
        assert!(held.is_cancelled());
        assert!(waiter.join().unwrap().is_err());
        assert!(governor.acquire(WorkClass::BackgroundContinue).is_err());
    }

    #[test]
    fn timed_acquire_removes_a_waiter_instead_of_waiting_forever() {
        let governor = WorkloadGovernor::default();
        let _held = governor.acquire(WorkClass::BackgroundContinue).unwrap();
        let error = match governor
            .acquire_with_timeout(WorkClass::ManualContinue, Duration::from_millis(10))
        {
            Err(error) => error,
            Ok(_) => panic!("manual Continue should time out behind active Continue"),
        };
        assert!(error.contains("timed out"));
        assert_eq!(governor.diagnostics().queued_operation_count, 0);
    }

    #[test]
    fn manual_continue_can_use_committed_evidence_while_background_capture_finishes() {
        let governor = WorkloadGovernor::default();
        let _capture = governor.acquire(WorkClass::ScreenshotCapture).unwrap();
        let manual = governor
            .acquire_with_timeout(WorkClass::ManualContinue, Duration::from_millis(10))
            .unwrap();
        assert_eq!(governor.diagnostics().active_operations.len(), 2);
        drop(manual);
    }
}
