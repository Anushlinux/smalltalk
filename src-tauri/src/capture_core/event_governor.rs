#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEventKind {
    AppSwitch,
    WindowFocus,
    AccessibilityChange,
    Click,
    KeyDown,
    Scroll,
    Clipboard,
    Idle,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureEvent {
    pub kind: CaptureEventKind,
    pub surface_key: String,
    pub content_hash: Option<String>,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureScheduleDecision {
    pub store_observation: bool,
    pub schedule_frame: bool,
    pub trigger: &'static str,
    pub settle_delay_ms: u64,
    pub reason: &'static str,
}

pub fn decide_capture(
    event: &CaptureEvent,
    previous: Option<&CaptureEvent>,
) -> CaptureScheduleDecision {
    let same_surface = previous.is_some_and(|prior| prior.surface_key == event.surface_key);
    let same_content = previous
        .and_then(|prior| prior.content_hash.as_ref())
        .zip(event.content_hash.as_ref())
        .is_some_and(|(left, right)| left == right);

    match event.kind {
        CaptureEventKind::Manual => CaptureScheduleDecision {
            store_observation: true,
            schedule_frame: true,
            trigger: "manual",
            settle_delay_ms: 0,
            reason: "explicit user capture",
        },
        CaptureEventKind::AppSwitch => CaptureScheduleDecision {
            store_observation: true,
            schedule_frame: !same_surface,
            trigger: "app_switch",
            settle_delay_ms: 300,
            reason: "frontmost application changed",
        },
        CaptureEventKind::WindowFocus | CaptureEventKind::AccessibilityChange => {
            CaptureScheduleDecision {
                store_observation: true,
                schedule_frame: !same_surface || !same_content,
                trigger: "window_focus",
                settle_delay_ms: 300,
                reason: "focused surface changed",
            }
        }
        CaptureEventKind::Click => CaptureScheduleDecision {
            store_observation: true,
            schedule_frame: !same_surface || event.content_hash.is_none() || !same_content,
            trigger: "click",
            settle_delay_ms: 220,
            reason: "click may commit navigation or selection",
        },
        CaptureEventKind::KeyDown => CaptureScheduleDecision {
            store_observation: true,
            schedule_frame: true,
            trigger: "typing_pause",
            settle_delay_ms: 850,
            reason: "typing is captured after pause or commit",
        },
        CaptureEventKind::Scroll => CaptureScheduleDecision {
            store_observation: true,
            schedule_frame: !same_content,
            trigger: "scroll_stop",
            settle_delay_ms: 500,
            reason: "scroll bursts collapse to final settled viewport",
        },
        CaptureEventKind::Clipboard => CaptureScheduleDecision {
            store_observation: true,
            schedule_frame: true,
            trigger: "clipboard",
            settle_delay_ms: 220,
            reason: "clipboard can indicate task transfer",
        },
        CaptureEventKind::Idle => CaptureScheduleDecision {
            store_observation: true,
            schedule_frame: !same_content,
            trigger: "idle",
            settle_delay_ms: 0,
            reason: "idle is fallback and must prove changed content",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: CaptureEventKind, surface: &str, hash: &str) -> CaptureEvent {
        CaptureEvent {
            kind,
            surface_key: surface.to_string(),
            content_hash: Some(hash.to_string()),
            ts_ms: 1,
        }
    }

    #[test]
    fn scroll_on_same_content_stores_observation_but_skips_frame() {
        let previous = event(CaptureEventKind::Scroll, "browser:a", "same");
        let next = event(CaptureEventKind::Scroll, "browser:a", "same");

        let decision = decide_capture(&next, Some(&previous));

        assert!(decision.store_observation);
        assert!(!decision.schedule_frame);
        assert_eq!(decision.trigger, "scroll_stop");
    }

    #[test]
    fn typing_is_coalesced_into_delayed_frame() {
        let decision = decide_capture(&event(CaptureEventKind::KeyDown, "editor", "a"), None);

        assert!(decision.schedule_frame);
        assert_eq!(decision.trigger, "typing_pause");
        assert!(decision.settle_delay_ms >= 800);
    }

    #[test]
    fn app_switch_to_same_surface_does_not_need_new_frame() {
        let previous = event(CaptureEventKind::AppSwitch, "codex", "a");
        let next = event(CaptureEventKind::AppSwitch, "codex", "b");

        let decision = decide_capture(&next, Some(&previous));

        assert!(decision.store_observation);
        assert!(!decision.schedule_frame);
    }
}
