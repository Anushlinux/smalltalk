use crate::capture_core::quality::{evaluate_surface, SurfacePolicyInput};
use crate::capture_core::workstate::{
    AnswerabilityActivity, AnswerabilityBundle, AnswerabilityEdge, AnswerabilitySurface,
    AnswerabilityTarget, AnswerabilityWorkstream, CutoffPoint, EvidenceArtifact, EvidenceNeed,
    Observation, ResumeAnchor,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

static WORKSTATE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Default)]
pub struct SurfaceObservationInput {
    pub session_id: String,
    pub ts_ms: i64,
    pub event_id: Option<String>,
    pub event_type: String,
    pub source: String,
    pub app_pid: Option<i64>,
    pub app_bundle_id: Option<String>,
    pub app_name: Option<String>,
    pub window_id: Option<i64>,
    pub window_title: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub focused_object: Option<String>,
    pub visible_text: Option<String>,
    pub selected_text: Option<String>,
    pub viewport_signature: Option<String>,
    pub privacy_state: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkstateIngestResult {
    pub observation_id: String,
    pub surface_id: Option<String>,
    pub workstate_id: String,
    pub workstream_id: Option<String>,
    pub evidence_need: Option<EvidenceNeed>,
}

#[derive(Debug, Clone, Default)]
pub struct EvidenceLink {
    pub evidence_need_id: Option<String>,
    pub observation_id: Option<String>,
    pub surface_id: Option<String>,
    pub workstate_id: Option<String>,
    pub workstream_id: Option<String>,
}

pub fn init_workstate_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS observations (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          ts_ms INTEGER NOT NULL,
          kind TEXT NOT NULL,
          source TEXT NOT NULL,
          surface_id TEXT,
          workstate_id TEXT,
          workstream_id TEXT,
          frame_id TEXT,
          event_ids_json TEXT NOT NULL,
          summary TEXT NOT NULL,
          confidence REAL NOT NULL,
          payload_json TEXT NOT NULL,
          privacy_state TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_observations_session_ts
          ON observations(session_id, ts_ms);
        CREATE INDEX IF NOT EXISTS idx_observations_surface_ts
          ON observations(surface_id, ts_ms);

        CREATE TABLE IF NOT EXISTS surfaces (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          surface_key TEXT NOT NULL,
          surface_type TEXT NOT NULL,
          app_name TEXT,
          app_bundle_id TEXT,
          app_pid INTEGER,
          window_id INTEGER,
          window_title TEXT,
          url_ref TEXT,
          document_ref TEXT,
          first_seen_ts INTEGER NOT NULL,
          last_seen_ts INTEGER NOT NULL,
          last_focused_ts INTEGER,
          last_meaningful_change_ts INTEGER,
          last_proof_frame_id TEXT,
          privacy_state TEXT NOT NULL,
          confidence REAL NOT NULL,
          metadata_json TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_surfaces_session_key
          ON surfaces(session_id, surface_key);
        CREATE INDEX IF NOT EXISTS idx_surfaces_session_seen
          ON surfaces(session_id, last_seen_ts DESC);

        CREATE TABLE IF NOT EXISTS surface_states (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          surface_id TEXT NOT NULL,
          ts_ms INTEGER NOT NULL,
          surface_state TEXT NOT NULL,
          focused_object TEXT,
          visible_text_hash TEXT,
          visible_text_excerpt_redacted TEXT,
          selected_text_hash TEXT,
          selected_text_excerpt_redacted TEXT,
          viewport_signature TEXT,
          content_units_hash TEXT,
          ocr_state TEXT NOT NULL,
          ax_state TEXT NOT NULL,
          last_event_id TEXT,
          last_frame_id TEXT,
          quality_json TEXT NOT NULL,
          confidence REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_surface_states_surface_ts
          ON surface_states(surface_id, ts_ms DESC);

        CREATE TABLE IF NOT EXISTS workstates (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          active_surface_id TEXT,
          active_workstream_id TEXT,
          current_focus_surface_id TEXT,
          current_activity_summary TEXT NOT NULL,
          status TEXT NOT NULL,
          confidence REAL NOT NULL,
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL,
          metadata_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workstates_session_updated
          ON workstates(session_id, updated_at_ms DESC);

        CREATE TABLE IF NOT EXISTS workstreams (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          start_ts INTEGER NOT NULL,
          end_ts INTEGER,
          status TEXT NOT NULL,
          primary_surface_id TEXT,
          current_focus_surface_id TEXT,
          branch_from_workstream_id TEXT,
          label_hypothesis TEXT,
          intent_summary TEXT,
          confidence REAL NOT NULL,
          cutoff_observation_id TEXT,
          resume_anchor_id TEXT,
          missing_evidence_json TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workstreams_session_updated
          ON workstreams(session_id, updated_at_ms DESC);

        CREATE TABLE IF NOT EXISTS workstream_edges (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          from_workstream_id TEXT NOT NULL,
          to_workstream_id TEXT NOT NULL,
          kind TEXT NOT NULL,
          ts_ms INTEGER NOT NULL,
          observation_id TEXT,
          confidence REAL NOT NULL,
          reason TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cutoff_points (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          workstate_id TEXT NOT NULL,
          workstream_id TEXT,
          observation_id TEXT NOT NULL,
          surface_id TEXT,
          ts_ms INTEGER NOT NULL,
          reason TEXT NOT NULL,
          confidence REAL NOT NULL,
          status TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS resume_anchors (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          workstate_id TEXT NOT NULL,
          workstream_id TEXT,
          surface_id TEXT,
          anchor_type TEXT NOT NULL,
          value_redacted TEXT NOT NULL,
          frame_id TEXT,
          observation_id TEXT,
          confidence REAL NOT NULL,
          why_this_anchor TEXT NOT NULL,
          openability_json TEXT NOT NULL,
          privacy_state TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_resume_anchors_session
          ON resume_anchors(session_id, created_at_ms DESC);

        CREATE TABLE IF NOT EXISTS evidence_needs (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          workstate_id TEXT NOT NULL,
          workstream_id TEXT,
          observation_id TEXT,
          surface_id TEXT,
          need_type TEXT NOT NULL,
          reason TEXT NOT NULL,
          modality TEXT NOT NULL,
          priority INTEGER NOT NULL,
          status TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL,
          resolved_at_ms INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_evidence_needs_session
          ON evidence_needs(session_id, created_at_ms DESC);

        CREATE TABLE IF NOT EXISTS evidence_artifacts (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          workstate_id TEXT,
          workstream_id TEXT,
          surface_id TEXT,
          observation_id TEXT,
          evidence_need_id TEXT,
          kind TEXT NOT NULL,
          ref_table TEXT NOT NULL,
          ref_id TEXT NOT NULL,
          role TEXT NOT NULL,
          summary TEXT NOT NULL,
          privacy_state TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL,
          metadata_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_evidence_artifacts_session
          ON evidence_artifacts(session_id, created_at_ms DESC);
        ",
    )
    .map_err(to_string)
}

pub fn ingest_observation(
    conn: &Connection,
    input: SurfaceObservationInput,
) -> Result<WorkstateIngestResult, String> {
    init_workstate_schema(conn)?;
    let privacy_state = input
        .privacy_state
        .clone()
        .unwrap_or_else(|| "normal".to_string());
    let surface_key = surface_key(&input);
    let surface_type = infer_surface_type(&input);
    let surface_id = upsert_surface(conn, &input, &surface_key, &surface_type, &privacy_state)?;
    let surface_policy = evaluate_surface(&SurfacePolicyInput {
        app_name: input.app_name.clone().unwrap_or_default(),
        app_bundle_id: input.app_bundle_id.clone().unwrap_or_default(),
        window_title: input.window_title.clone().unwrap_or_default(),
        browser_url: input.browser_url.clone().unwrap_or_default(),
        surface_type: surface_type.clone(),
        visible_text: input.visible_text.clone().unwrap_or_default(),
        selected_text_present: input
            .selected_text
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        is_debug_artifact: false,
        privacy_excluded: privacy_state == "excluded" || privacy_state == "never_send",
        page_body_unit_count: if input
            .visible_text
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            0
        } else {
            1
        },
        chrome_unit_count: 0,
    });
    let state_id = insert_surface_state(conn, &input, &surface_id, &surface_policy.surface_state)?;
    let workstate_id = upsert_workstate(conn, &input, &surface_id, &state_id)?;
    let workstream_id = upsert_workstream(conn, &input, &surface_id, &workstate_id)?;
    let observation_id = next_id("obs", input.ts_ms);
    let event_ids = input.event_id.clone().into_iter().collect::<Vec<_>>();
    let summary = observation_summary(&input, &surface_type);
    conn.execute(
        "INSERT INTO observations (
            id, session_id, ts_ms, kind, source, surface_id, workstate_id, workstream_id,
            frame_id, event_ids_json, summary, confidence, payload_json, privacy_state, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10, ?11, ?12, ?13, ?3)",
        params![
            observation_id,
            input.session_id,
            input.ts_ms,
            observation_kind(&input.event_type),
            input.source,
            surface_id,
            workstate_id,
            workstream_id,
            serde_json::to_string(&event_ids).map_err(to_string)?,
            summary,
            0.72_f64,
            input.payload.clone().unwrap_or_else(|| json!({})).to_string(),
            privacy_state,
        ],
    )
    .map_err(to_string)?;

    maybe_record_cutoff(
        conn,
        &input,
        &workstate_id,
        workstream_id.as_deref(),
        &surface_id,
        &observation_id,
    )?;
    maybe_record_resume_anchor(
        conn,
        &input,
        &workstate_id,
        workstream_id.as_deref(),
        &surface_id,
        &observation_id,
        &privacy_state,
    )?;
    let evidence_need = maybe_create_evidence_need(
        conn,
        &input,
        &surface_policy.surface_state,
        &workstate_id,
        workstream_id.as_deref(),
        &surface_id,
        &observation_id,
        &privacy_state,
    )?;

    Ok(WorkstateIngestResult {
        observation_id,
        surface_id: Some(surface_id),
        workstate_id,
        workstream_id,
        evidence_need,
    })
}

pub fn record_frame_artifact(
    conn: &Connection,
    session_id: &str,
    frame_id: &str,
    trigger: &str,
    link: Option<&EvidenceLink>,
    ts_ms: i64,
) -> Result<(), String> {
    init_workstate_schema(conn)?;
    let artifact_id = next_id("artifact", ts_ms);
    let evidence_need_id = link.and_then(|item| item.evidence_need_id.clone());
    let observation_id = link.and_then(|item| item.observation_id.clone());
    let surface_id = link.and_then(|item| item.surface_id.clone());
    let workstate_id = link.and_then(|item| item.workstate_id.clone());
    let workstream_id = link.and_then(|item| item.workstream_id.clone());
    conn.execute(
        "INSERT INTO evidence_artifacts (
            id, session_id, workstate_id, workstream_id, surface_id, observation_id,
            evidence_need_id, kind, ref_table, ref_id, role, summary, privacy_state,
            created_at_ms, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'frame', 'frames', ?8, ?9, ?10, 'normal', ?11, ?12)",
        params![
            artifact_id,
            session_id,
            workstate_id,
            workstream_id,
            surface_id,
            observation_id,
            evidence_need_id,
            frame_id,
            trigger,
            format!("proof frame captured for {}", trigger),
            ts_ms,
            json!({ "capture_trigger": trigger }).to_string(),
        ],
    )
    .map_err(to_string)?;
    if let Some(need_id) = link.and_then(|item| item.evidence_need_id.as_deref()) {
        conn.execute(
            "UPDATE evidence_needs
             SET status = 'resolved', resolved_at_ms = ?2
             WHERE id = ?1",
            params![need_id, ts_ms],
        )
        .map_err(to_string)?;
    }
    if let Some(surface_id) = link.and_then(|item| item.surface_id.as_deref()) {
        conn.execute(
            "UPDATE surfaces
             SET last_proof_frame_id = ?2
             WHERE id = ?1",
            params![surface_id, frame_id],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

pub fn load_answerability_bundle(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<AnswerabilityBundle>, String> {
    init_workstate_schema(conn)?;
    let observations = load_observations(conn, session_id, 64)?;
    if observations.is_empty() {
        return Ok(None);
    }
    let surfaces = load_answerability_surfaces(conn, session_id, 12)?;
    let workstreams = load_answerability_workstreams(conn, session_id)?;
    let anchors = load_resume_anchors(conn, session_id, 12)?;
    let cutoffs = load_cutoff_points(conn, session_id, 8)?;
    let artifacts = load_evidence_artifacts(conn, session_id, 24)?;
    let current_surface = surfaces.first().cloned();
    let current_focus = current_surface.as_ref().map(|surface| AnswerabilityTarget {
        evidence_handle: observations
            .last()
            .map(|item| format!("observation:{}", item.id))
            .unwrap_or_else(|| "observation:unknown".to_string()),
        surface_id: Some(surface.surface_id.clone()),
        app: surface.app.clone(),
        title: surface.title.clone(),
        surface_type: surface.surface_type.clone(),
        surface_state: surface.surface_state.clone(),
        anchor: anchors.first().map(|anchor| anchor.value_redacted.clone()),
        reason: "Latest observed surface in workstate memory.".to_string(),
        confidence: surface.confidence,
    });
    let resume_work_target = choose_resume_target(&surfaces, &anchors, &observations);
    let current_activity = observations
        .last()
        .map(|observation| AnswerabilityActivity {
            evidence_handle: format!("observation:{}", observation.id),
            activity_type: activity_type_for_observation(&observation.kind),
            summary: observation.summary.clone(),
            reason: "Derived from the latest memory observation, not from selected screenshots."
                .to_string(),
            confidence: observation.confidence,
        });
    let mut missing_evidence = Vec::new();
    if artifacts.is_empty() {
        missing_evidence.push(
            "No proof frame is attached; resume claim is based on local observations and surface state."
                .to_string(),
        );
    }
    if anchors.is_empty() {
        missing_evidence.push("No concrete resume anchor has been proven yet.".to_string());
    }

    Ok(Some(AnswerabilityBundle {
        schema: "smalltalk.answerability_bundle.v1".to_string(),
        session_id: session_id.to_string(),
        current_focus,
        current_activity,
        resume_work_target: resume_work_target.clone(),
        return_target: resume_work_target,
        active_surfaces: surfaces,
        workstreams,
        branch_timeline: load_workstream_edges(conn, session_id)?,
        candidate_cutoff_points: cutoffs,
        candidate_resume_anchors: anchors,
        observations,
        evidence_artifacts: artifacts,
        missing_evidence,
        privacy_state: "local_redacted".to_string(),
    }))
}

fn upsert_surface(
    conn: &Connection,
    input: &SurfaceObservationInput,
    surface_key: &str,
    surface_type: &str,
    privacy_state: &str,
) -> Result<String, String> {
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM surfaces WHERE session_id = ?1 AND surface_key = ?2",
            params![input.session_id, surface_key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?
    {
        conn.execute(
            "UPDATE surfaces
             SET last_seen_ts = ?2,
                 last_focused_ts = ?2,
                 last_meaningful_change_ts = CASE WHEN ?3 = 1 THEN ?2 ELSE last_meaningful_change_ts END,
                 app_name = COALESCE(?4, app_name),
                 app_bundle_id = COALESCE(?5, app_bundle_id),
                 app_pid = COALESCE(?6, app_pid),
                 window_id = COALESCE(?7, window_id),
                 window_title = COALESCE(?8, window_title),
                 metadata_json = ?9
             WHERE id = ?1",
            params![
                id,
                input.ts_ms,
                is_meaningful_event(&input.event_type) as i64,
                input.app_name,
                input.app_bundle_id,
                input.app_pid,
                input.window_id,
                input.window_title,
                surface_metadata_json(input),
            ],
        )
        .map_err(to_string)?;
        return Ok(id);
    }

    let id = next_id("surface", input.ts_ms);
    conn.execute(
        "INSERT INTO surfaces (
            id, session_id, surface_key, surface_type, app_name, app_bundle_id, app_pid,
            window_id, window_title, url_ref, document_ref, first_seen_ts, last_seen_ts,
            last_focused_ts, last_meaningful_change_ts, privacy_state, confidence, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, ?12, ?12, ?13, 0.72, ?14)",
        params![
            id,
            input.session_id,
            surface_key,
            surface_type,
            input.app_name,
            input.app_bundle_id,
            input.app_pid,
            input.window_id,
            input.window_title,
            input.browser_url.as_deref().and_then(redacted_url_ref),
            input.document_path.as_deref().map(redacted_path_ref),
            input.ts_ms,
            privacy_state,
            surface_metadata_json(input),
        ],
    )
    .map_err(to_string)?;
    Ok(id)
}

fn insert_surface_state(
    conn: &Connection,
    input: &SurfaceObservationInput,
    surface_id: &str,
    surface_state: &str,
) -> Result<String, String> {
    let id = next_id("state", input.ts_ms);
    let visible = input.visible_text.as_deref().and_then(non_empty);
    let selected = input.selected_text.as_deref().and_then(non_empty);
    conn.execute(
        "INSERT INTO surface_states (
            id, session_id, surface_id, ts_ms, surface_state, focused_object,
            visible_text_hash, visible_text_excerpt_redacted, selected_text_hash,
            selected_text_excerpt_redacted, viewport_signature, content_units_hash,
            ocr_state, ax_state, last_event_id, last_frame_id, quality_json, confidence
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, 'not_run', ?12, ?13, NULL, ?14, 0.72)",
        params![
            id,
            input.session_id,
            surface_id,
            input.ts_ms,
            surface_state,
            input.focused_object,
            visible.map(stable_hash),
            visible.map(redacted_excerpt),
            selected.map(stable_hash),
            selected.map(redacted_excerpt),
            input.viewport_signature,
            if input.visible_text.as_deref().unwrap_or("").trim().is_empty() {
                "thin"
            } else {
                "available"
            },
            input.event_id,
            json!({ "source": input.source, "event_type": input.event_type }).to_string(),
        ],
    )
    .map_err(to_string)?;
    Ok(id)
}

fn upsert_workstate(
    conn: &Connection,
    input: &SurfaceObservationInput,
    surface_id: &str,
    _state_id: &str,
) -> Result<String, String> {
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM workstates WHERE session_id = ?1 ORDER BY updated_at_ms DESC LIMIT 1",
            params![input.session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?
    {
        conn.execute(
            "UPDATE workstates
             SET active_surface_id = ?2,
                 current_focus_surface_id = ?2,
                 current_activity_summary = ?3,
                 updated_at_ms = ?4
             WHERE id = ?1",
            params![id, surface_id, activity_summary(input), input.ts_ms],
        )
        .map_err(to_string)?;
        return Ok(id);
    }
    let id = next_id("workstate", input.ts_ms);
    conn.execute(
        "INSERT INTO workstates (
            id, session_id, active_surface_id, current_focus_surface_id,
            current_activity_summary, status, confidence, created_at_ms, updated_at_ms,
            metadata_json
         ) VALUES (?1, ?2, ?3, ?3, ?4, 'active', 0.68, ?5, ?5, ?6)",
        params![
            id,
            input.session_id,
            surface_id,
            activity_summary(input),
            input.ts_ms,
            json!({ "vertical_slice": true }).to_string(),
        ],
    )
    .map_err(to_string)?;
    Ok(id)
}

fn upsert_workstream(
    conn: &Connection,
    input: &SurfaceObservationInput,
    surface_id: &str,
    workstate_id: &str,
) -> Result<Option<String>, String> {
    let existing = conn
        .query_row(
            "SELECT id, current_focus_surface_id FROM workstreams
             WHERE session_id = ?1 AND status = 'active'
             ORDER BY updated_at_ms DESC LIMIT 1",
            params![input.session_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(to_string)?;
    if let Some((id, prior_surface_id)) = existing {
        let is_branch = prior_surface_id
            .as_ref()
            .is_some_and(|prior| prior != surface_id)
            && matches!(
                input.event_type.as_str(),
                "app_switch" | "window_focus" | "navigation_changed"
            );
        if is_branch {
            let edge_id = next_id("edge", input.ts_ms);
            conn.execute(
                "INSERT INTO workstream_edges (
                    id, session_id, from_workstream_id, to_workstream_id, kind, ts_ms,
                    observation_id, confidence, reason
                 ) VALUES (?1, ?2, ?3, ?3, 'branch', ?4, NULL, 0.58, ?5)",
                params![
                    edge_id,
                    input.session_id,
                    id,
                    input.ts_ms,
                    "surface changed while preserving active workstream compatibility",
                ],
            )
            .map_err(to_string)?;
        }
        conn.execute(
            "UPDATE workstreams
             SET current_focus_surface_id = ?2,
                 intent_summary = ?3,
                 updated_at_ms = ?4
             WHERE id = ?1",
            params![id, surface_id, activity_summary(input), input.ts_ms],
        )
        .map_err(to_string)?;
        conn.execute(
            "UPDATE workstates SET active_workstream_id = ?2 WHERE id = ?1",
            params![workstate_id, id],
        )
        .map_err(to_string)?;
        return Ok(Some(id));
    }

    let id = next_id("workstream", input.ts_ms);
    conn.execute(
        "INSERT INTO workstreams (
            id, session_id, start_ts, status, primary_surface_id, current_focus_surface_id,
            label_hypothesis, intent_summary, confidence, missing_evidence_json,
            created_at_ms, updated_at_ms
         ) VALUES (?1, ?2, ?3, 'active', ?4, ?4, ?5, ?6, 0.64, '[]', ?3, ?3)",
        params![
            id,
            input.session_id,
            input.ts_ms,
            surface_id,
            input
                .window_title
                .clone()
                .unwrap_or_else(|| "active work".to_string()),
            activity_summary(input),
        ],
    )
    .map_err(to_string)?;
    conn.execute(
        "UPDATE workstates SET active_workstream_id = ?2 WHERE id = ?1",
        params![workstate_id, id],
    )
    .map_err(to_string)?;
    Ok(Some(id))
}

fn maybe_create_evidence_need(
    conn: &Connection,
    input: &SurfaceObservationInput,
    surface_state: &str,
    workstate_id: &str,
    workstream_id: Option<&str>,
    surface_id: &str,
    observation_id: &str,
    privacy_state: &str,
) -> Result<Option<EvidenceNeed>, String> {
    let event = input.event_type.as_str();
    let capture_default = matches!(event, "session_start" | "manual");
    let surface_needs_proof = matches!(
        event,
        "app_switch" | "window_focus" | "accessibility_change" | "ax_notification"
    ) && surface_last_proof_frame(conn, surface_id)?.is_none()
        && !matches!(
            surface_state,
            "smalltalk_self_surface" | "privacy_excluded_surface"
        );
    let low_confidence = surface_state == "need_more_evidence"
        && !matches!(
            event,
            "scroll" | "click" | "idle" | "key_down" | "clipboard"
        );
    if !(capture_default || surface_needs_proof || low_confidence) {
        return Ok(None);
    }
    if matches!(
        event,
        "scroll" | "click" | "idle" | "key_down" | "clipboard"
    ) {
        return Ok(None);
    }
    if privacy_state == "excluded" || privacy_state == "never_send" {
        return Ok(None);
    }

    let need = EvidenceNeed {
        id: next_id("need", input.ts_ms),
        session_id: input.session_id.clone(),
        workstate_id: workstate_id.to_string(),
        workstream_id: workstream_id.map(str::to_string),
        observation_id: Some(observation_id.to_string()),
        surface_id: Some(surface_id.to_string()),
        need_type: if capture_default {
            "baseline_or_manual_proof".to_string()
        } else {
            "surface_state_proof".to_string()
        },
        reason: evidence_reason(event, surface_state),
        modality: "frame".to_string(),
        priority: if capture_default { 90 } else { 55 },
        status: "open".to_string(),
        created_at_ms: input.ts_ms,
    };
    conn.execute(
        "INSERT INTO evidence_needs (
            id, session_id, workstate_id, workstream_id, observation_id, surface_id,
            need_type, reason, modality, priority, status, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            need.id,
            need.session_id,
            need.workstate_id,
            need.workstream_id,
            need.observation_id,
            need.surface_id,
            need.need_type,
            need.reason,
            need.modality,
            need.priority,
            need.status,
            need.created_at_ms,
        ],
    )
    .map_err(to_string)?;
    Ok(Some(need))
}

fn maybe_record_cutoff(
    conn: &Connection,
    input: &SurfaceObservationInput,
    workstate_id: &str,
    workstream_id: Option<&str>,
    surface_id: &str,
    observation_id: &str,
) -> Result<(), String> {
    if !matches!(
        input.event_type.as_str(),
        "idle" | "session_stop" | "window_focus"
    ) {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO cutoff_points (
            id, session_id, workstate_id, workstream_id, observation_id, surface_id,
            ts_ms, reason, confidence, status
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0.55, 'candidate')",
        params![
            next_id("cutoff", input.ts_ms),
            input.session_id,
            workstate_id,
            workstream_id,
            observation_id,
            surface_id,
            input.ts_ms,
            format!("{} may mark a pause or cutoff", input.event_type),
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn maybe_record_resume_anchor(
    conn: &Connection,
    input: &SurfaceObservationInput,
    workstate_id: &str,
    workstream_id: Option<&str>,
    surface_id: &str,
    observation_id: &str,
    privacy_state: &str,
) -> Result<(), String> {
    let anchor = input
        .selected_text
        .as_deref()
        .and_then(non_empty)
        .map(redacted_excerpt)
        .or_else(|| {
            input
                .visible_text
                .as_deref()
                .and_then(non_empty)
                .map(redacted_excerpt)
        })
        .or_else(|| input.window_title.clone());
    let Some(value) = anchor.and_then(non_empty_owned) else {
        return Ok(());
    };
    conn.execute(
        "INSERT INTO resume_anchors (
            id, session_id, workstate_id, workstream_id, surface_id, anchor_type,
            value_redacted, observation_id, confidence, why_this_anchor, openability_json,
            privacy_state, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0.58, ?9, ?10, ?11, ?12)",
        params![
            next_id("anchor", input.ts_ms),
            input.session_id,
            workstate_id,
            workstream_id,
            surface_id,
            if input.browser_url.is_some() {
                "url"
            } else {
                "visible_text"
            },
            value,
            observation_id,
            "Best local anchor currently available from surface state.",
            json!({ "openable": input.browser_url.is_some() || input.document_path.is_some() })
                .to_string(),
            privacy_state,
            input.ts_ms,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn load_observations(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<Observation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, ts_ms, kind, source, surface_id, workstate_id,
                    workstream_id, frame_id, event_ids_json, summary, confidence,
                    payload_json, privacy_state
             FROM observations
             WHERE session_id = ?1
             ORDER BY ts_ms DESC, id DESC
             LIMIT ?2",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id, limit as i64], |row| {
            let event_ids_json: String = row.get(9)?;
            let payload_json: String = row.get(12)?;
            Ok(Observation {
                id: row.get(0)?,
                session_id: row.get(1)?,
                ts_ms: row.get(2)?,
                kind: row.get(3)?,
                source: row.get(4)?,
                surface_id: row.get(5)?,
                workstate_id: row.get(6)?,
                workstream_id: row.get(7)?,
                frame_id: row.get(8)?,
                event_ids: serde_json::from_str(&event_ids_json).unwrap_or_default(),
                summary: row.get(10)?,
                confidence: row.get(11)?,
                payload: serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({})),
                privacy_state: row.get(13)?,
            })
        })
        .map_err(to_string)?;
    let mut items = rows.collect::<Result<Vec<_>, _>>().map_err(to_string)?;
    items.reverse();
    Ok(items)
}

fn load_answerability_surfaces(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<AnswerabilitySurface>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.surface_key, s.surface_type, s.app_name, s.window_title,
                    s.last_seen_ts, COALESCE(st.surface_state, 'need_more_evidence'),
                    COALESCE(st.confidence, s.confidence)
             FROM surfaces s
             LEFT JOIN surface_states st ON st.id = (
               SELECT id FROM surface_states
               WHERE surface_id = s.id
               ORDER BY ts_ms DESC, id DESC
               LIMIT 1
             )
             WHERE s.session_id = ?1
             ORDER BY s.last_seen_ts DESC, s.id DESC
             LIMIT ?2",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id, limit as i64], |row| {
            Ok(AnswerabilitySurface {
                surface_id: row.get(0)?,
                surface_key: row.get(1)?,
                surface_type: row.get(2)?,
                app: row.get(3)?,
                title: row.get(4)?,
                last_observed_at_ms: row.get(5)?,
                surface_state: row.get(6)?,
                confidence: row.get(7)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_answerability_workstreams(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<AnswerabilityWorkstream>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, status, primary_surface_id, current_focus_surface_id,
                    COALESCE(intent_summary, ''), confidence
             FROM workstreams
             WHERE session_id = ?1
             ORDER BY updated_at_ms DESC, id DESC
             LIMIT 8",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id], |row| {
            Ok(AnswerabilityWorkstream {
                workstream_id: row.get(0)?,
                status: row.get(1)?,
                primary_surface_id: row.get(2)?,
                current_focus_surface_id: row.get(3)?,
                current_activity_summary: row.get(4)?,
                confidence: row.get(5)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_resume_anchors(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<ResumeAnchor>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, workstate_id, workstream_id, surface_id, anchor_type,
                    value_redacted, frame_id, observation_id, confidence, why_this_anchor,
                    privacy_state
             FROM resume_anchors
             WHERE session_id = ?1
             ORDER BY created_at_ms DESC, id DESC
             LIMIT ?2",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id, limit as i64], |row| {
            Ok(ResumeAnchor {
                id: row.get(0)?,
                session_id: row.get(1)?,
                workstate_id: row.get(2)?,
                workstream_id: row.get(3)?,
                surface_id: row.get(4)?,
                anchor_type: row.get(5)?,
                value_redacted: row.get(6)?,
                frame_id: row.get(7)?,
                observation_id: row.get(8)?,
                confidence: row.get(9)?,
                why_this_anchor: row.get(10)?,
                privacy_state: row.get(11)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_cutoff_points(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<CutoffPoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, workstate_id, workstream_id, observation_id,
                    surface_id, reason, confidence
             FROM cutoff_points
             WHERE session_id = ?1
             ORDER BY ts_ms DESC, id DESC
             LIMIT ?2",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id, limit as i64], |row| {
            Ok(CutoffPoint {
                id: row.get(0)?,
                session_id: row.get(1)?,
                workstate_id: row.get(2)?,
                workstream_id: row.get(3)?,
                observation_id: row.get(4)?,
                surface_id: row.get(5)?,
                reason: row.get(6)?,
                confidence: row.get(7)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_evidence_artifacts(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<EvidenceArtifact>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, workstate_id, workstream_id, surface_id, observation_id,
                    evidence_need_id, kind, ref_table, ref_id, role, summary, privacy_state,
                    created_at_ms
             FROM evidence_artifacts
             WHERE session_id = ?1
             ORDER BY created_at_ms DESC, id DESC
             LIMIT ?2",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id, limit as i64], |row| {
            Ok(EvidenceArtifact {
                id: row.get(0)?,
                session_id: row.get(1)?,
                workstate_id: row.get(2)?,
                workstream_id: row.get(3)?,
                surface_id: row.get(4)?,
                observation_id: row.get(5)?,
                evidence_need_id: row.get(6)?,
                kind: row.get(7)?,
                ref_table: row.get(8)?,
                ref_id: row.get(9)?,
                role: row.get(10)?,
                summary: row.get(11)?,
                privacy_state: row.get(12)?,
                created_at_ms: row.get(13)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn load_workstream_edges(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<AnswerabilityEdge>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT from_workstream_id, to_workstream_id, kind, reason, confidence
             FROM workstream_edges
             WHERE session_id = ?1
             ORDER BY ts_ms ASC, id ASC
             LIMIT 24",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id], |row| {
            Ok(AnswerabilityEdge {
                from_workstream_id: row.get(0)?,
                to_workstream_id: row.get(1)?,
                kind: row.get(2)?,
                reason: row.get(3)?,
                confidence: row.get(4)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn choose_resume_target(
    surfaces: &[AnswerabilitySurface],
    anchors: &[ResumeAnchor],
    observations: &[Observation],
) -> Option<AnswerabilityTarget> {
    let surface = surfaces
        .iter()
        .find(|surface| {
            matches!(
                surface.surface_state.as_str(),
                "current_work_surface" | "actionable_task_surface"
            )
        })
        .or_else(|| surfaces.first())?;
    let anchor = anchors
        .iter()
        .find(|anchor| anchor.surface_id.as_deref() == Some(surface.surface_id.as_str()))
        .or_else(|| anchors.first());
    Some(AnswerabilityTarget {
        evidence_handle: anchor
            .map(|anchor| format!("anchor:{}", anchor.id))
            .or_else(|| {
                observations
                    .last()
                    .map(|obs| format!("observation:{}", obs.id))
            })
            .unwrap_or_else(|| format!("surface:{}", surface.surface_id)),
        surface_id: Some(surface.surface_id.clone()),
        app: surface.app.clone(),
        title: surface.title.clone(),
        surface_type: surface.surface_type.clone(),
        surface_state: surface.surface_state.clone(),
        anchor: anchor.map(|anchor| anchor.value_redacted.clone()),
        reason: "Selected from workstate memory before proof-frame selection.".to_string(),
        confidence: surface
            .confidence
            .min(anchor.map(|item| item.confidence).unwrap_or(0.56)),
    })
}

fn surface_last_proof_frame(conn: &Connection, surface_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT last_proof_frame_id FROM surfaces WHERE id = ?1",
        params![surface_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()
    .map(|value| value.flatten())
    .map_err(to_string)
}

pub fn surface_key(input: &SurfaceObservationInput) -> String {
    let app = input
        .app_bundle_id
        .as_deref()
        .or(input.app_name.as_deref())
        .unwrap_or("unknown-app")
        .trim()
        .to_lowercase();
    let identity = input
        .browser_url
        .as_deref()
        .map(|url| format!("url:{}", stable_hash(url)))
        .or_else(|| {
            input
                .document_path
                .as_deref()
                .map(|path| format!("doc:{}", stable_hash(path)))
        })
        .or_else(|| input.window_id.map(|id| format!("window-id:{}", id)))
        .or_else(|| {
            input
                .window_title
                .as_deref()
                .map(|title| format!("title:{}", title.trim().to_lowercase()))
        })
        .unwrap_or_else(|| "unknown-surface".to_string());
    format!("{}::{}", app, identity)
}

fn infer_surface_type(input: &SurfaceObservationInput) -> String {
    let app = input.app_name.as_deref().unwrap_or("").to_lowercase();
    let title = input.window_title.as_deref().unwrap_or("").to_lowercase();
    if input.browser_url.is_some()
        || ["chrome", "safari", "arc", "brave", "edge", "helium"]
            .iter()
            .any(|needle| app.contains(needle))
    {
        if title.contains("chatgpt") || title.contains("claude") {
            "chat_conversation".to_string()
        } else {
            "browser_tab".to_string()
        }
    } else if app.contains("terminal") || app.contains("iterm") {
        "terminal".to_string()
    } else if app.contains("cursor") || app.contains("code") || title.ends_with(".rs") {
        "code_editor".to_string()
    } else if input.document_path.is_some() {
        "document".to_string()
    } else {
        "unknown".to_string()
    }
}

fn observation_kind(event_type: &str) -> String {
    match event_type {
        "scroll" | "scroll_stop" => "scroll_position_changed",
        "click" => "click_interaction",
        "key_down" | "typing_pause" => "typing_burst_updated",
        "clipboard" => "clipboard_metadata_changed",
        "idle" => "surface_stable",
        "app_switch" | "window_focus" => "surface_focus_changed",
        "session_start" => "manual_checkpoint",
        "manual" => "manual_checkpoint",
        other => other,
    }
    .to_string()
}

fn observation_summary(input: &SurfaceObservationInput, surface_type: &str) -> String {
    let app = input.app_name.as_deref().unwrap_or("unknown app");
    let title = input.window_title.as_deref().unwrap_or("unknown surface");
    format!(
        "{} observed on {} / {} ({})",
        observation_kind(&input.event_type),
        app,
        title,
        surface_type
    )
}

fn activity_summary(input: &SurfaceObservationInput) -> String {
    match input.event_type.as_str() {
        "key_down" | "typing_pause" => "editing or composing without raw keystrokes",
        "scroll" | "scroll_stop" => "reading or scanning within the same surface",
        "click" => "interacting with the current surface",
        "clipboard" => "copying or transferring local context",
        "idle" => "surface is stable or paused",
        "app_switch" | "window_focus" => "changing or returning to a work surface",
        "manual" | "session_start" => "manual or baseline checkpoint",
        _ => "observing active work surface",
    }
    .to_string()
}

fn activity_type_for_observation(kind: &str) -> String {
    match kind {
        "typing_burst_updated" => "editing".to_string(),
        "scroll_position_changed" => "focused_reading".to_string(),
        "clipboard_metadata_changed" => "copying_evidence".to_string(),
        "surface_stable" => "paused".to_string(),
        _ => "current_surface".to_string(),
    }
}

fn evidence_reason(event_type: &str, surface_state: &str) -> String {
    if event_type == "session_start" {
        "session baseline proof for a new workstate".to_string()
    } else if event_type == "manual" {
        "explicit user proof capture".to_string()
    } else {
        format!(
            "surface state '{}' needs proof after {}",
            surface_state, event_type
        )
    }
}

fn is_meaningful_event(event_type: &str) -> bool {
    !matches!(event_type, "idle" | "scroll")
}

fn redacted_url_ref(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let host = trimmed
        .split("://")
        .nth(1)
        .unwrap_or(trimmed)
        .split('/')
        .next()
        .unwrap_or("unknown-host");
    Some(format!("url_host:{}#{}", host, stable_hash(trimmed)))
}

fn redacted_path_ref(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or("document");
    format!("path_name:{}#{}", name, stable_hash(path))
}

fn surface_metadata_json(input: &SurfaceObservationInput) -> String {
    json!({
        "has_url": input.browser_url.is_some(),
        "has_document": input.document_path.is_some(),
        "event_type": input.event_type,
    })
    .to_string()
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn non_empty_owned(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn redacted_excerpt(value: &str) -> String {
    value
        .split_whitespace()
        .take(36)
        .collect::<Vec<_>>()
        .join(" ")
}

fn stable_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn next_id(prefix: &str, ts_ms: i64) -> String {
    let seq = WORKSTATE_ID.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{}", prefix, ts_ms, seq)
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_workstate_schema(&conn).unwrap();
        conn
    }

    fn input(event_type: &str) -> SurfaceObservationInput {
        SurfaceObservationInput {
            session_id: "session-1".to_string(),
            ts_ms: 1000,
            event_id: Some(format!("evt-{}", event_type)),
            event_type: event_type.to_string(),
            source: "test".to_string(),
            app_name: Some("Helium".to_string()),
            window_title: Some("Claude".to_string()),
            browser_url: Some("https://claude.ai/chat/abc".to_string()),
            visible_text: Some("Workstate memory architecture implementation notes".to_string()),
            ..SurfaceObservationInput::default()
        }
    }

    #[test]
    fn scroll_observation_does_not_create_evidence_need() {
        let conn = conn();
        let result = ingest_observation(&conn, input("scroll")).unwrap();

        assert!(result.evidence_need.is_none());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn click_observation_does_not_create_evidence_need() {
        let conn = conn();
        let result = ingest_observation(&conn, input("click")).unwrap();

        assert!(result.evidence_need.is_none());
    }

    #[test]
    fn idle_observation_records_cutoff_without_default_frame_need() {
        let conn = conn();
        let result = ingest_observation(&conn, input("idle")).unwrap();

        assert!(result.evidence_need.is_none());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cutoff_points", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn session_start_creates_baseline_evidence_need() {
        let conn = conn();
        let result = ingest_observation(&conn, input("session_start")).unwrap();

        assert!(result.evidence_need.is_some());
    }

    #[test]
    fn answerability_bundle_builds_without_frames() {
        let conn = conn();
        ingest_observation(&conn, input("scroll")).unwrap();
        ingest_observation(
            &conn,
            SurfaceObservationInput {
                ts_ms: 2000,
                event_type: "idle".to_string(),
                event_id: Some("evt-idle".to_string()),
                ..input("idle")
            },
        )
        .unwrap();

        let bundle = load_answerability_bundle(&conn, "session-1")
            .unwrap()
            .unwrap();

        assert!(bundle.evidence_artifacts.is_empty());
        assert!(!bundle.observations.is_empty());
        assert!(bundle
            .missing_evidence
            .iter()
            .any(|item| item.contains("No proof frame")));
    }
}
