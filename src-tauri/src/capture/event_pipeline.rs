//! Bounded transport for privacy-safe UI event metadata.
//!
//! The Swift producer already collapses scroll and Accessibility bursts before
//! writing JSON lines. This transport is the second, authoritative pressure
//! boundary. It reserves space for identity and error events, coalesces noisy
//! events by surface, and exposes every loss as a counter plus a synthetic
//! diagnostic event. It never stores typed characters or clipboard contents.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

pub const HIGH_CAPACITY: usize = 64;
pub const NORMAL_CAPACITY: usize = 96;
pub const PRESSURE_CAPACITY: usize = 160;
pub const TOTAL_CAPACITY: usize = HIGH_CAPACITY + NORMAL_CAPACITY + PRESSURE_CAPACITY;
// The capture loop receives one event before this drain, so one transaction is
// bounded to 32 events.
pub const MAX_DRAIN_COUNT: usize = 31;
pub const MAX_DRAIN_TIME: Duration = Duration::from_millis(12);
const HIGH_VALUE_WAIT: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EventPriority {
    Pressure,
    Normal,
    High,
}

#[derive(Debug, Clone)]
struct QueuedEvent {
    raw: String,
    priority: EventPriority,
    key: String,
    first_ts_ms: i64,
    last_ts_ms: i64,
    count: u64,
}

#[derive(Debug, Default)]
struct State {
    high: VecDeque<QueuedEvent>,
    normal: VecDeque<QueuedEvent>,
    pressure: VecDeque<QueuedEvent>,
    high_water_mark: usize,
    coalesced: u64,
    dropped_pressure: u64,
    dropped_normal: u64,
    dropped_high: u64,
    pending_overflow_diagnostic: bool,
    shutdown: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub struct EventPipelineDiagnostics {
    pub queue_depth: u64,
    pub queue_capacity: u64,
    pub high_queue_depth: u64,
    pub normal_queue_depth: u64,
    pub pressure_queue_depth: u64,
    pub high_water_mark: u64,
    pub coalesced_count: u64,
    pub dropped_count: u64,
    pub dropped_pressure_count: u64,
    pub dropped_normal_count: u64,
    pub dropped_high_count: u64,
    pub shutdown: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrainResult {
    pub drained: usize,
    pub hit_count_budget: bool,
    pub hit_time_budget: bool,
}

#[derive(Debug)]
pub struct EventPipeline {
    state: Mutex<State>,
    changed: Condvar,
}

impl Default for EventPipeline {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
        }
    }
}

impl EventPipeline {
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Admit a producer line without allowing low-value traffic to occupy the
    /// high-value reservation. High-value input gets one short bounded wait so
    /// the consumer can make space; it never waits indefinitely.
    pub fn push(&self, raw: String) -> bool {
        let mut event = classify(&raw);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.shutdown {
            return false;
        }

        if coalesce_existing(&mut state, &mut event) {
            state.coalesced = state.coalesced.saturating_add(1);
            self.changed.notify_one();
            return true;
        }

        if event.priority == EventPriority::High && state.high.len() >= HIGH_CAPACITY {
            let (next, _) = self
                .changed
                .wait_timeout_while(state, HIGH_VALUE_WAIT, |state| {
                    !state.shutdown && state.high.len() >= HIGH_CAPACITY
                })
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = next;
            if state.shutdown {
                return false;
            }
            if coalesce_existing(&mut state, &mut event) {
                state.coalesced = state.coalesced.saturating_add(1);
                self.changed.notify_one();
                return true;
            }
        }

        let admitted = match event.priority {
            EventPriority::High if state.high.len() < HIGH_CAPACITY => {
                state.high.push_back(event);
                true
            }
            EventPriority::Normal if state.normal.len() < NORMAL_CAPACITY => {
                state.normal.push_back(event);
                true
            }
            EventPriority::Pressure if state.pressure.len() < PRESSURE_CAPACITY => {
                state.pressure.push_back(event);
                true
            }
            EventPriority::Pressure => {
                // Pressure events are replaceable summaries. Keep the newest
                // surface sample and account for the displaced one.
                state.pressure.pop_front();
                state.pressure.push_back(event);
                state.dropped_pressure = state.dropped_pressure.saturating_add(1);
                state.pending_overflow_diagnostic = true;
                true
            }
            EventPriority::Normal => {
                state.normal.pop_front();
                state.normal.push_back(event);
                state.dropped_normal = state.dropped_normal.saturating_add(1);
                state.pending_overflow_diagnostic = true;
                true
            }
            EventPriority::High => {
                // A finite system cannot promise lossless delivery under an
                // infinite high-value producer. Preserve the reserved lane,
                // record the exceptional loss, and surface a synthetic event.
                state.dropped_high = state.dropped_high.saturating_add(1);
                state.pending_overflow_diagnostic = true;
                false
            }
        };
        update_high_water_mark(&mut state);
        self.changed.notify_one();
        admitted
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Option<String> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (mut state, _) = self
            .changed
            .wait_timeout_while(state, timeout, |state| {
                !state.shutdown && queue_depth(state) == 0 && !state.pending_overflow_diagnostic
            })
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let value = pop_next(&mut state);
        self.changed.notify_all();
        value
    }

    pub fn drain_bounded<F>(&self, mut consume: F) -> DrainResult
    where
        F: FnMut(String),
    {
        let started = Instant::now();
        let mut result = DrainResult::default();
        while result.drained < MAX_DRAIN_COUNT {
            if started.elapsed() >= MAX_DRAIN_TIME {
                result.hit_time_budget = true;
                break;
            }
            let next = {
                let mut state = self
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let next = pop_next(&mut state);
                if next.is_some() {
                    self.changed.notify_all();
                }
                next
            };
            let Some(next) = next else {
                break;
            };
            consume(next);
            result.drained += 1;
        }
        if result.drained == MAX_DRAIN_COUNT {
            result.hit_count_budget = true;
        }
        result
    }

    pub fn diagnostics(&self) -> EventPipelineDiagnostics {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        EventPipelineDiagnostics {
            queue_depth: queue_depth(&state) as u64,
            queue_capacity: TOTAL_CAPACITY as u64,
            high_queue_depth: state.high.len() as u64,
            normal_queue_depth: state.normal.len() as u64,
            pressure_queue_depth: state.pressure.len() as u64,
            high_water_mark: state.high_water_mark as u64,
            coalesced_count: state.coalesced,
            dropped_count: state
                .dropped_pressure
                .saturating_add(state.dropped_normal)
                .saturating_add(state.dropped_high),
            dropped_pressure_count: state.dropped_pressure,
            dropped_normal_count: state.dropped_normal,
            dropped_high_count: state.dropped_high,
            shutdown: state.shutdown,
        }
    }

    pub fn shutdown(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.shutdown = true;
        self.changed.notify_all();
    }
}

fn queue_depth(state: &State) -> usize {
    state.high.len() + state.normal.len() + state.pressure.len()
}

fn update_high_water_mark(state: &mut State) {
    state.high_water_mark = state.high_water_mark.max(queue_depth(state));
}

fn coalesce_existing(state: &mut State, incoming: &mut QueuedEvent) -> bool {
    let queue = match incoming.priority {
        EventPriority::High => &mut state.high,
        EventPriority::Normal => &mut state.normal,
        EventPriority::Pressure => &mut state.pressure,
    };
    let Some(existing) = queue.iter_mut().rev().find(|item| item.key == incoming.key) else {
        return false;
    };
    if incoming.last_ts_ms.saturating_sub(existing.last_ts_ms) > coalescing_window_ms(&incoming.raw)
    {
        return false;
    }
    existing.last_ts_ms = incoming.last_ts_ms;
    existing.count = existing.count.saturating_add(incoming.count);
    existing.raw = with_aggregate_metadata(
        &incoming.raw,
        existing.count,
        existing.first_ts_ms,
        existing.last_ts_ms,
    );
    true
}

fn pop_next(state: &mut State) -> Option<String> {
    if state.pending_overflow_diagnostic {
        state.pending_overflow_diagnostic = false;
        return Some(
            json!({
                "ts_ms": now_millis(),
                "event_type": "pipeline_diagnostics",
                "payload": {
                    "dropped_pressure": state.dropped_pressure.to_string(),
                    "dropped_normal": state.dropped_normal.to_string(),
                    "dropped_high": state.dropped_high.to_string(),
                    "coalesced": state.coalesced.to_string()
                }
            })
            .to_string(),
        );
    }
    state
        .high
        .pop_front()
        .or_else(|| state.normal.pop_front())
        .or_else(|| state.pressure.pop_front())
        .map(|event| event.raw)
}

fn classify(raw: &str) -> QueuedEvent {
    let value = serde_json::from_str::<Value>(raw).unwrap_or(Value::Null);
    let event_type = value
        .get("event_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let key_category = value
        .get("key_category")
        .and_then(Value::as_str)
        .unwrap_or("");
    let is_repeat = value
        .get("is_repeat")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let notification = value
        .get("payload")
        .and_then(Value::as_object)
        .and_then(|payload| payload.get("notification"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let priority = match event_type {
        "app_switch"
        | "window_focus"
        | "clipboard"
        | "error"
        | "permission_change"
        | "helper_started"
        | "source_diagnostics"
        | "pipeline_diagnostics" => EventPriority::High,
        "key_down" if matches!(key_category, "enter" | "shortcut" | "escape") => {
            EventPriority::High
        }
        "scroll" | "ax_notification" | "accessibility_change" => EventPriority::Pressure,
        "key_down" if is_repeat || key_category == "char" => EventPriority::Pressure,
        _ => EventPriority::Normal,
    };
    let ts_ms = value
        .get("ts_ms")
        .and_then(Value::as_i64)
        .unwrap_or_else(now_millis);
    let surface = format!(
        "{}:{}:{}",
        value
            .get("app_bundle_id")
            .or_else(|| value.get("app_name"))
            .and_then(Value::as_str)
            .unwrap_or("unknown_app"),
        value
            .get("window_title")
            .and_then(Value::as_str)
            .unwrap_or("unknown_window"),
        notification
    );
    let detail = match event_type {
        "key_down" => key_category,
        "click" => value
            .get("button")
            .and_then(Value::as_str)
            .unwrap_or("click"),
        _ => event_type,
    };
    QueuedEvent {
        raw: raw.to_string(),
        priority,
        key: format!("{event_type}:{surface}:{detail}"),
        first_ts_ms: ts_ms,
        last_ts_ms: ts_ms,
        count: 1,
    }
}

fn coalescing_window_ms(raw: &str) -> i64 {
    let value = serde_json::from_str::<Value>(raw).unwrap_or(Value::Null);
    match value
        .get("event_type")
        .and_then(Value::as_str)
        .unwrap_or("")
    {
        "scroll" => 650,
        "ax_notification" | "accessibility_change" => 1_800,
        "key_down" => 400,
        "click" => 250,
        "source_diagnostics" | "pipeline_diagnostics" => 30_000,
        _ => 0,
    }
}

fn with_aggregate_metadata(raw: &str, count: u64, first_ts_ms: i64, last_ts_ms: i64) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(raw) else {
        return raw.to_string();
    };
    let Some(object) = value.as_object_mut() else {
        return raw.to_string();
    };
    let payload = object.entry("payload").or_insert_with(|| json!({}));
    if !payload.is_object() {
        *payload = json!({});
    }
    if let Some(payload) = payload.as_object_mut() {
        payload.insert("coalesced_count".into(), Value::String(count.to_string()));
        payload.insert(
            "coalesced_first_ts_ms".into(),
            Value::String(first_ts_ms.to_string()),
        );
        payload.insert(
            "coalesced_last_ts_ms".into(),
            Value::String(last_ts_ms.to_string()),
        );
    }
    value.to_string()
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn event(event_type: &str, ts_ms: i64) -> String {
        json!({
            "ts_ms": ts_ms,
            "event_type": event_type,
            "app_bundle_id": "com.example.editor",
            "window_title": "bounded fixture",
            "key_category": if event_type == "key_down" { "char" } else { "" }
        })
        .to_string()
    }

    #[test]
    fn sixty_minute_equivalent_pressure_is_bounded() {
        let pipeline = EventPipeline::default();
        for index in 0..216_000 {
            pipeline.push(event("scroll", index * 16));
        }
        let diagnostics = pipeline.diagnostics();
        assert!(diagnostics.queue_depth <= diagnostics.queue_capacity);
        assert!(diagnostics.high_water_mark <= diagnostics.queue_capacity);
        assert!(diagnostics.dropped_count > 0 || diagnostics.coalesced_count > 0);
    }

    #[test]
    fn noisy_pressure_cannot_consume_high_value_reservation() {
        let pipeline = EventPipeline::default();
        for index in 0..10_000 {
            pipeline.push(event("scroll", index * 700));
        }
        assert!(pipeline.push(event("app_switch", 9_000_000)));
        let first = pipeline.recv_timeout(Duration::from_millis(1)).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&first).unwrap()["event_type"],
            "pipeline_diagnostics"
        );
        let second = pipeline.recv_timeout(Duration::from_millis(1)).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&second).unwrap()["event_type"],
            "app_switch"
        );
    }

    #[test]
    fn bounded_drain_yields_with_work_remaining() {
        let pipeline = EventPipeline::default();
        for index in 0..TOTAL_CAPACITY {
            pipeline.push(event("click", index as i64 * 300));
        }
        let result = pipeline.drain_bounded(|_| {});
        assert!(result.drained <= MAX_DRAIN_COUNT);
        assert!(result.hit_count_budget || result.hit_time_budget);
        assert!(pipeline.diagnostics().queue_depth > 0);
    }

    #[test]
    fn shutdown_wakes_and_rejects_producers() {
        let pipeline = EventPipeline::default();
        pipeline.shutdown();
        assert!(!pipeline.push(event("app_switch", 1)));
        assert!(pipeline.diagnostics().shutdown);
    }

    #[test]
    fn continuous_producer_yields_to_stop_and_idle_checks() {
        let pipeline = EventPipeline::shared();
        let producing = Arc::new(AtomicBool::new(true));
        let producer_pipeline = pipeline.clone();
        let producer_flag = producing.clone();
        let producer = std::thread::spawn(move || {
            let mut index = 0_i64;
            while producer_flag.load(Ordering::Acquire) {
                producer_pipeline.push(event("scroll", index * 16));
                index = index.saturating_add(1);
            }
        });
        let idle_started = Instant::now() - Duration::from_secs(120);
        let stop_requested = Arc::new(AtomicBool::new(false));
        let stop_flag = stop_requested.clone();
        let stopper = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            stop_flag.store(true, Ordering::Release);
        });

        let turn_started = Instant::now();
        let mut idle_observed = false;
        while !stop_requested.load(Ordering::Acquire) {
            pipeline.drain_bounded(|_| {});
            idle_observed |= idle_started.elapsed() >= Duration::from_secs(120);
        }
        producing.store(false, Ordering::Release);
        pipeline.shutdown();
        producer.join().unwrap();
        stopper.join().unwrap();

        assert!(idle_observed);
        assert!(turn_started.elapsed() < Duration::from_secs(1));
        assert!(pipeline.diagnostics().queue_depth <= TOTAL_CAPACITY as u64);
    }
}
