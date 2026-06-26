#[derive(Debug, Clone, Default)]
pub struct SurfacePolicyInput {
    pub app_name: String,
    pub app_bundle_id: String,
    pub window_title: String,
    pub browser_url: String,
    pub surface_type: String,
    pub visible_text: String,
    pub selected_text_present: bool,
    pub is_debug_artifact: bool,
    pub privacy_excluded: bool,
    pub page_body_unit_count: usize,
    pub chrome_unit_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfacePolicyDecision {
    pub surface_state: String,
    pub model_eligible: bool,
    pub resume_target_eligible: bool,
    pub needs_more_evidence: bool,
    pub chrome_heavy: bool,
    pub score_adjustment: f64,
    pub quality_flags: Vec<String>,
}

pub fn evaluate_surface(input: &SurfacePolicyInput) -> SurfacePolicyDecision {
    let app = input.app_name.to_lowercase();
    let bundle = input.app_bundle_id.to_lowercase();
    let title = input.window_title.to_lowercase();
    let url = input.browser_url.to_lowercase();
    let text = input.visible_text.to_lowercase();
    let chrome_heavy =
        browser_chrome_heavy(&text, input.chrome_unit_count, input.page_body_unit_count);

    let mut flags = Vec::new();
    let mut state = if input.privacy_excluded {
        "privacy_excluded_surface"
    } else if input.is_debug_artifact || looks_like_debug_artifact(&title, &url, &text) {
        "debug_export_surface"
    } else if looks_like_smalltalk_self(&app, &bundle, &title) {
        "smalltalk_self_surface"
    } else if looks_like_system_monitor(&app, &bundle, &title, &text) {
        "system_monitor_surface"
    } else if looks_like_finder_overview(&app, &bundle, &title, &text) {
        "finder_overview_surface"
    } else if looks_like_transient(&url, &title) {
        "transient_system_surface"
    } else if looks_like_window_overview(&app, &title, &text) {
        "mixed_window_overview"
    } else if looks_like_passive_feed(&url, &title, &text) {
        "passive_feed_surface"
    } else if actionable_text(&text) || form_like_task(&input.surface_type, &text) {
        "actionable_task_surface"
    } else if matches!(
        input.surface_type.as_str(),
        "chat_conversation" | "code_editor" | "terminal" | "notes_doc"
    ) {
        "current_work_surface"
    } else if matches!(input.surface_type.as_str(), "browser_tab" | "pdf" | "media") {
        "source_or_discovery_surface"
    } else {
        "need_more_evidence"
    }
    .to_string();

    if chrome_heavy {
        flags.push("json_visible_text_was_browser_chrome_heavy".to_string());
    }
    if input.page_body_unit_count == 0 && is_browserish(&app, &url, &input.surface_type) {
        flags.push("missing_page_body_content_units".to_string());
    }
    if state == "need_more_evidence" {
        flags.push("surface_state_need_more_evidence".to_string());
    }
    if input.surface_type == "unknown" {
        flags.push("surface_type_unknown".to_string());
    }
    if chrome_heavy && state == "current_work_surface" {
        state = "need_more_evidence".to_string();
        flags.push("chrome_heavy_current_work_downgraded".to_string());
    }

    let resume_target_eligible = matches!(
        state.as_str(),
        "current_work_surface" | "actionable_task_surface"
    );
    let model_eligible = !matches!(
        state.as_str(),
        "privacy_excluded_surface" | "smalltalk_self_surface" | "debug_export_surface"
    );
    let needs_more_evidence = matches!(
        state.as_str(),
        "need_more_evidence"
            | "system_monitor_surface"
            | "finder_overview_surface"
            | "source_or_discovery_surface"
    );
    let score_adjustment = match state.as_str() {
        "actionable_task_surface" => 0.24,
        "current_work_surface" => 0.10,
        "source_or_discovery_surface" => -0.18,
        "need_more_evidence" => -0.45,
        "passive_feed_surface" => -0.50,
        "mixed_window_overview" | "transient_system_surface" => -0.55,
        "system_monitor_surface" | "finder_overview_surface" => -0.68,
        "smalltalk_self_surface" | "debug_export_surface" | "privacy_excluded_surface" => -0.80,
        _ => 0.0,
    };

    SurfacePolicyDecision {
        surface_state: state,
        model_eligible,
        resume_target_eligible,
        needs_more_evidence,
        chrome_heavy,
        score_adjustment,
        quality_flags: flags,
    }
}

pub fn surface_state_can_be_resume_target(surface_state: &str) -> bool {
    matches!(
        surface_state,
        "current_work_surface" | "actionable_task_surface"
    )
}

pub fn surface_state_is_context_only(surface_state: &str) -> bool {
    matches!(
        surface_state,
        "passive_feed_surface"
            | "debug_export_surface"
            | "transient_system_surface"
            | "mixed_window_overview"
            | "source_or_discovery_surface"
            | "system_monitor_surface"
            | "finder_overview_surface"
            | "smalltalk_self_surface"
            | "privacy_excluded_surface"
    )
}

fn is_browserish(app: &str, url: &str, surface_type: &str) -> bool {
    !url.is_empty()
        || [
            "safari", "chrome", "arc", "brave", "edge", "vivaldi", "opera", "chromium",
        ]
        .iter()
        .any(|needle| app.contains(needle))
        || matches!(surface_type, "browser_tab" | "chat_conversation")
}

fn looks_like_debug_artifact(title: &str, url: &str, text: &str) -> bool {
    [title, url, text].iter().any(|value| {
        value.contains("resume-query-bundle")
            || value.contains("cloud-resume-result")
            || value.contains("safe-ai-export")
            || value.contains("smalltalk.capture_session")
            || value.contains("smalltalk.resume_query")
    })
}

fn looks_like_smalltalk_self(app: &str, bundle: &str, title: &str) -> bool {
    app == "smalltalk" || bundle.contains("com.smalltalk") || title == "smalltalk"
}

fn looks_like_system_monitor(app: &str, bundle: &str, title: &str, text: &str) -> bool {
    app.contains("activity monitor")
        || bundle.contains("activitymonitor")
        || title.contains("activity monitor")
        || text.contains("all processes")
        || text.contains("code helper (plugin)")
        || text.contains("windowserver")
}

fn looks_like_finder_overview(app: &str, bundle: &str, title: &str, text: &str) -> bool {
    (app == "finder" || bundle == "com.apple.finder")
        && (title == "documents"
            || title == "desktop"
            || text.contains("favourites")
            || text.contains("locations")
            || text.contains("icloud drive"))
}

fn looks_like_transient(url: &str, title: &str) -> bool {
    matches!(url, "about:blank" | "chrome://newtab/" | "edge://newtab/")
        || matches!(title, "new tab" | "untitled")
}

fn looks_like_window_overview(app: &str, title: &str, text: &str) -> bool {
    app.contains("dock")
        || title.contains("mission control")
        || title.contains("window overview")
        || title.contains("app expose")
        || title.contains("app exposé")
        || text.contains("mission control")
}

fn looks_like_passive_feed(url: &str, title: &str, text: &str) -> bool {
    let feed = url.contains("x.com/home")
        || url.contains("twitter.com/home")
        || url.contains("linkedin.com/feed")
        || title.contains("home / x")
        || title.contains("linkedin");
    feed && (text.contains("for you")
        || text.contains("following")
        || text.contains("what is happening")
        || text.contains("who to follow")
        || text.contains("promoted"))
        && !form_like_task("browser_tab", text)
}

fn actionable_text(text: &str) -> bool {
    [
        "resume here",
        "continue from",
        "build this next",
        "next action",
        "implement",
        "fix ",
        "todo",
        "start from:",
        "do next:",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn form_like_task(surface_type: &str, text: &str) -> bool {
    if surface_type != "browser_tab" && surface_type != "unknown" {
        return false;
    }
    [
        "name",
        "email",
        "apply",
        "application",
        "submit",
        "full time",
        "founding engineer",
        "hardest problem",
        "cover letter",
        "resume",
    ]
    .iter()
    .filter(|marker| text.contains(**marker))
    .count()
        >= 3
}

pub fn browser_chrome_heavy(text: &str, chrome_units: usize, page_body_units: usize) -> bool {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return true;
    }
    if chrome_units >= 2 && page_body_units == 0 {
        return true;
    }
    let marker_count = [
        "developer tools",
        "pinned",
        "file edit view",
        "history bookmarks",
        "profiles window",
        "address and search",
        "new tab",
        "tab search",
        "extensions",
        "side panel",
        "reload",
        "bookmarks",
        "downloads",
    ]
    .iter()
    .filter(|marker| compact.contains(**marker))
    .count();
    let meaningful = [
        "critical extraction bug",
        "for browser pages",
        "next action",
        "resume",
        "implement",
        "founding engineer",
    ]
    .iter()
    .any(|marker| compact.contains(*marker));
    marker_count >= 2 && !meaningful
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(app: &str, title: &str, surface_type: &str, text: &str) -> SurfacePolicyInput {
        SurfacePolicyInput {
            app_name: app.to_string(),
            window_title: title.to_string(),
            surface_type: surface_type.to_string(),
            visible_text: text.to_string(),
            page_body_unit_count: 1,
            ..SurfacePolicyInput::default()
        }
    }

    #[test]
    fn activity_monitor_is_context_only_not_resume_target() {
        let decision = evaluate_surface(&input(
            "Activity Monitor",
            "Activity Monitor - All Processes",
            "unknown",
            "WindowServer Code Helper (Plugin) 466.2 MB",
        ));

        assert_eq!(decision.surface_state, "system_monitor_surface");
        assert!(!decision.resume_target_eligible);
        assert!(decision.score_adjustment < -0.6);
    }

    #[test]
    fn finder_documents_overview_is_not_resume_target() {
        let decision = evaluate_surface(&input(
            "Finder",
            "Documents",
            "unknown",
            "Favourites Applications Desktop Locations iCloud Drive",
        ));

        assert_eq!(decision.surface_state, "finder_overview_surface");
        assert!(!decision.resume_target_eligible);
    }

    #[test]
    fn application_form_is_actionable() {
        let decision = evaluate_surface(&input(
            "Helium",
            "Founding Engineer - Full Time",
            "browser_tab",
            "Founding Engineer Full Time application Name Email hardest problem submit",
        ));

        assert_eq!(decision.surface_state, "actionable_task_surface");
        assert!(decision.resume_target_eligible);
    }

    #[test]
    fn browser_chrome_without_body_is_need_more_evidence() {
        let mut item = input(
            "Chrome",
            "Data Optimization",
            "chat_conversation",
            "Developer Tools pinned File Edit View History Bookmarks Extensions",
        );
        item.chrome_unit_count = 3;
        item.page_body_unit_count = 0;

        let decision = evaluate_surface(&item);

        assert!(decision.chrome_heavy);
        assert!(!decision.resume_target_eligible);
    }
}
