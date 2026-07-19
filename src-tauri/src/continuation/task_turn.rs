use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::activity_recap::sanitize_public_text;

pub(crate) const CURRENT_TASK_TURN_SCHEMA_V2: &str = "smalltalk.current_task_turn.v2";
const MAX_GOAL_CHARS: usize = 280;
const MAX_STATE_CHARS: usize = 220;
const MAX_OBJECT_CHARS: usize = 96;
const RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1_000;

macro_rules! typed_label_enum {
    ($name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub(crate) fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }

            fn parse(value: &str) -> Result<Self, String> {
                match value {
                    $($label => Ok(Self::$variant),)+
                    invalid => Err(format!("invalid {} label: {invalid}", stringify!($name))),
                }
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.label()))
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                let raw = value.as_str()?;
                Self::parse(raw).map_err(|error| FromSqlError::Other(Box::new(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, error),
                )))
            }
        }
    };
}

typed_label_enum!(TaskExecutionState {
    Active => "active",
    Blocked => "blocked",
    Completed => "completed",
    Superseded => "superseded",
    Suspended => "suspended",
    IdleAfterProgress => "idle_after_progress",
    Unclear => "unclear",
});
typed_label_enum!(TaskTurnActor {
    User => "user",
    AssistantOrAgent => "assistant_or_agent",
    Tool => "tool",
    System => "system",
    Unknown => "unknown",
});
typed_label_enum!(TaskTurnWaitingOn {
    None => "none",
    User => "user",
    Agent => "agent",
    External => "external",
    Unknown => "unknown",
});
typed_label_enum!(TaskTurnRelation {
    NewTask => "new_task",
    Continuation => "continuation",
    Clarification => "clarification",
    ChildSupportStep => "child_support_step",
    Correction => "correction",
    Supersedes => "supersedes",
    Unknown => "unknown",
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskTurnScope {
    pub task_turn_id: String,
    pub current_span_ids: Vec<String>,
    pub current_source_record_ids: Vec<String>,
    pub current_semantic_text: String,
    pub completion_evidence_text: String,
    pub task_object: Option<String>,
    pub task_kind: String,
    pub execution_state: String,
    pub current_actor: TaskTurnActor,
    pub waiting_on: String,
    pub relation_to_prior: String,
    pub has_current_agent_state: bool,
    pub typed_boundary_confident: bool,
    pub attribution_confidence: f64,
    pub quality_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentTaskTurn {
    pub schema_version: String,
    pub task_turn_id: String,
    pub session_id: Option<String>,
    pub surface_key_hash: Option<String>,
    pub artifact_id: Option<String>,
    pub workstream_id: Option<String>,
    pub started_at_ms: i64,
    pub last_observed_at_ms: i64,
    pub latest_user_goal_summary: Option<String>,
    pub latest_user_goal_hash: Option<String>,
    pub latest_user_goal_redaction_status: String,
    pub task_object: Option<String>,
    pub task_object_hash: Option<String>,
    pub task_object_redaction_status: String,
    pub task_kind: String,
    pub current_actor: TaskTurnActor,
    pub actor_activity_state: Option<String>,
    pub actor_activity_state_hash: Option<String>,
    pub actor_activity_state_redaction_status: String,
    pub execution_state: TaskExecutionState,
    pub waiting_on: TaskTurnWaitingOn,
    pub relation_to_prior: TaskTurnRelation,
    pub prior_task_turn_id: Option<String>,
    pub supersedes_task_turn_id: Option<String>,
    pub parent_task_turn_id: Option<String>,
    pub latest_user_span_ids: Vec<String>,
    pub current_state_span_ids: Vec<String>,
    pub prior_boundary_span_ids: Vec<String>,
    pub supporting_action_ids: Vec<String>,
    pub supporting_event_ids: Vec<String>,
    pub evidence_quality: String,
    pub goal_confidence: f64,
    pub task_object_confidence: f64,
    pub actor_state_confidence: f64,
    pub execution_state_confidence: f64,
    pub waiting_on_confidence: f64,
    pub relation_confidence: f64,
    pub attribution_confidence: f64,
    pub missing_evidence: Vec<String>,
    pub quality_flags: Vec<String>,
    pub reason_codes: Vec<String>,
    pub revision: i64,
    pub selected: bool,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TaskTurnResolution {
    pub frame_scopes: HashMap<String, TaskTurnScope>,
    pub selected_task_turn_id: Option<String>,
    pub alternative_task_turn_ids: Vec<String>,
    pub selection_reason_codes: Vec<String>,
    pub conflict_flags: Vec<String>,
    pub missing_evidence: Vec<String>,
}

#[derive(Debug, Clone)]
struct SalientRow {
    frame_id: String,
    session_id: Option<String>,
    surface_key: Option<String>,
    artifact_id: Option<String>,
    observed_at_ms: i64,
    user_span_ids: Vec<String>,
    agent_span_ids: Vec<String>,
    prior_span_ids: Vec<String>,
    user_summary: Option<String>,
    user_hash: Option<String>,
    agent_summary: Option<String>,
    agent_hash: Option<String>,
    sampling_confidence: f64,
    missing_evidence: Vec<String>,
    quality_flags: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProvisionalTurn {
    row: SalientRow,
    id: String,
    goal_summary: Option<String>,
    goal_hash: Option<String>,
    actor_summary: Option<String>,
    task_object: Option<String>,
    task_kind: String,
    current_actor: String,
    execution_state: String,
    waiting_on: String,
    completion_evidence: Option<String>,
    relation: String,
    prior_id: Option<String>,
    supersedes_id: Option<String>,
    parent_id: Option<String>,
    source_record_ids: Vec<String>,
    attribution_confidence: f64,
    reason_codes: Vec<String>,
}

pub(crate) fn ensure_task_turn_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS continue_task_turns (
          task_turn_id TEXT PRIMARY KEY,
          schema_version TEXT NOT NULL,
          session_id TEXT,
          surface_key_hash TEXT,
          artifact_id TEXT,
          workstream_id TEXT,
          started_at_ms INTEGER NOT NULL,
          last_observed_at_ms INTEGER NOT NULL,
          latest_user_goal_summary TEXT,
          latest_user_goal_hash TEXT,
          latest_user_goal_redaction_status TEXT NOT NULL DEFAULT 'missing',
          task_object TEXT,
          task_object_hash TEXT,
          task_object_redaction_status TEXT NOT NULL DEFAULT 'missing',
          task_kind TEXT NOT NULL DEFAULT 'unknown',
          current_actor TEXT NOT NULL DEFAULT 'unknown',
          actor_activity_state TEXT,
          actor_activity_state_hash TEXT,
          actor_activity_state_redaction_status TEXT NOT NULL DEFAULT 'missing',
          execution_state TEXT NOT NULL DEFAULT 'unclear',
          waiting_on TEXT NOT NULL DEFAULT 'unknown',
          relation_to_prior TEXT NOT NULL DEFAULT 'unknown',
          prior_task_turn_id TEXT,
          supersedes_task_turn_id TEXT,
          parent_task_turn_id TEXT,
          latest_user_span_ids_json TEXT NOT NULL DEFAULT '[]',
          current_state_span_ids_json TEXT NOT NULL DEFAULT '[]',
          prior_boundary_span_ids_json TEXT NOT NULL DEFAULT '[]',
          supporting_action_ids_json TEXT NOT NULL DEFAULT '[]',
          supporting_event_ids_json TEXT NOT NULL DEFAULT '[]',
          evidence_quality TEXT NOT NULL DEFAULT 'thin',
          goal_confidence REAL NOT NULL DEFAULT 0.0,
          task_object_confidence REAL NOT NULL DEFAULT 0.0,
          actor_state_confidence REAL NOT NULL DEFAULT 0.0,
          execution_state_confidence REAL NOT NULL DEFAULT 0.0,
          waiting_on_confidence REAL NOT NULL DEFAULT 0.0,
          relation_confidence REAL NOT NULL DEFAULT 0.0,
          attribution_confidence REAL NOT NULL DEFAULT 0.0,
          missing_evidence_json TEXT NOT NULL DEFAULT '[]',
          quality_flags_json TEXT NOT NULL DEFAULT '[]',
          reason_codes_json TEXT NOT NULL DEFAULT '[]',
          material_fingerprint TEXT NOT NULL,
          revision INTEGER NOT NULL DEFAULT 1,
          selected INTEGER NOT NULL DEFAULT 0,
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_task_turns_selected
          ON continue_task_turns(selected, last_observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_task_turns_session_time
          ON continue_task_turns(session_id, last_observed_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_continue_task_turns_artifact_time
          ON continue_task_turns(artifact_id, last_observed_at_ms DESC);

        CREATE TABLE IF NOT EXISTS continue_task_turn_evidence (
          task_turn_id TEXT NOT NULL,
          source_frame_id TEXT NOT NULL,
          source_span_id TEXT NOT NULL,
          source_record_id TEXT,
          field_name TEXT NOT NULL,
          source_text_hash TEXT,
          redaction_status TEXT NOT NULL,
          evidence_role TEXT NOT NULL,
          observed_at_ms INTEGER NOT NULL,
          created_at_ms INTEGER NOT NULL,
          PRIMARY KEY(task_turn_id, source_span_id, field_name)
        );
        CREATE INDEX IF NOT EXISTS idx_continue_task_turn_evidence_frame
          ON continue_task_turn_evidence(source_frame_id, task_turn_id);

        CREATE TABLE IF NOT EXISTS continue_task_turn_relations (
          task_turn_id TEXT NOT NULL,
          related_task_turn_id TEXT NOT NULL,
          relation_kind TEXT NOT NULL,
          confidence REAL NOT NULL,
          reason_code TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL,
          updated_at_ms INTEGER NOT NULL,
          PRIMARY KEY(task_turn_id, related_task_turn_id, relation_kind)
        );

        CREATE TABLE IF NOT EXISTS continue_task_turn_lifecycle (
          id TEXT PRIMARY KEY,
          task_turn_id TEXT NOT NULL,
          revision INTEGER NOT NULL,
          observed_at_ms INTEGER NOT NULL,
          previous_execution_state TEXT,
          execution_state TEXT NOT NULL,
          previous_current_actor TEXT,
          current_actor TEXT NOT NULL,
          previous_waiting_on TEXT,
          waiting_on TEXT NOT NULL,
          transition_reason TEXT NOT NULL,
          source_kind TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_task_turn_history_turn
          ON continue_task_turn_lifecycle(task_turn_id, revision DESC);
        ",
    )
    .map_err(to_string)?;
    ensure_column(
        conn,
        "continue_task_turns",
        "latest_user_goal_redaction_status",
        "TEXT NOT NULL DEFAULT 'missing'",
    )?;
    ensure_column(conn, "continue_task_turns", "task_object_hash", "TEXT")?;
    ensure_column(
        conn,
        "continue_task_turns",
        "task_object_redaction_status",
        "TEXT NOT NULL DEFAULT 'missing'",
    )?;
    ensure_column(
        conn,
        "continue_task_turns",
        "actor_activity_state_redaction_status",
        "TEXT NOT NULL DEFAULT 'missing'",
    )?;
    Ok(())
}

pub(crate) fn resolve_provisional_task_turns(
    conn: &Connection,
    frame_ids: &[String],
) -> Result<TaskTurnResolution, String> {
    ensure_task_turn_schema(conn)?;
    if frame_ids.is_empty() || !table_exists(conn, "continue_salient_turn_evidence")? {
        return Ok(TaskTurnResolution::default());
    }

    let mut rows = Vec::new();
    for frame_id in frame_ids {
        if let Some(row) = load_salient_row(conn, frame_id)? {
            rows.push(row);
        }
    }
    // A prior boundary is history-only. It may explain chronology once a new
    // current user goal exists, but visible historical text alone must never
    // create or select a task turn.
    let prior_only_evidence_observed = rows.iter().any(|row| {
        row.user_span_ids.is_empty()
            && row.agent_span_ids.is_empty()
            && !row.prior_span_ids.is_empty()
    });
    let latest_sample_row = rows
        .iter()
        .max_by_key(|row| (row.observed_at_ms, numeric_id(&row.frame_id)))
        .cloned();
    rows.retain(|row| !row.user_span_ids.is_empty());
    rows.sort_by_key(|row| (row.observed_at_ms, numeric_id(&row.frame_id)));
    if rows.is_empty() {
        let preserved = if prior_only_evidence_observed {
            None
        } else {
            latest_sample_row
                .as_ref()
                .map(|row| preserve_selected_turn_across_empty_sample(conn, row))
                .transpose()?
                .flatten()
        };
        let manual_boundary_without_lineage = latest_sample_row
            .as_ref()
            .map(|row| frame_is_manual_continue_boundary(conn, &row.frame_id))
            .transpose()?
            .unwrap_or(false)
            && preserved.is_none();
        if (prior_only_evidence_observed || manual_boundary_without_lineage) && preserved.is_none()
        {
            clear_selected_task_turn(conn)?;
        }
        return Ok(TaskTurnResolution {
            selected_task_turn_id: preserved.as_ref().map(|turn| turn.task_turn_id.clone()),
            selection_reason_codes: preserved
                .as_ref()
                .map(|_| vec!["same_surface_empty_sample_preserved".to_string()])
                .unwrap_or_default(),
            missing_evidence: vec!["current_user_goal_evidence".to_string()],
            ..TaskTurnResolution::default()
        });
    }

    let mut remaining_identities =
        rows.iter()
            .fold(HashMap::<String, usize>::new(), |mut map, row| {
                *map.entry(salient_identity_key(row)).or_default() += 1;
                map
            });
    let mut seen_turn_ids = BTreeSet::new();
    let mut result = TaskTurnResolution::default();
    let mut prior = load_prior_turn(conn, &rows[0])?;
    let mut resolved_by_id: HashMap<String, CurrentTaskTurn> = HashMap::new();

    for row in rows {
        let identity = salient_identity_key(&row);
        let has_later_reobservation = remaining_identities
            .get(&identity)
            .copied()
            .unwrap_or_default()
            > 1;
        if let Some(remaining) = remaining_identities.get_mut(&identity) {
            *remaining = remaining.saturating_sub(1);
        }
        let mut provisional = build_provisional(conn, row, prior.as_ref())?;
        if has_later_reobservation {
            if let Some(existing) = load_task_turn(conn, &provisional.id)? {
                provisional.relation = existing.relation_to_prior.label().to_string();
                provisional.prior_id = existing.prior_task_turn_id;
                provisional.supersedes_id = existing.supersedes_task_turn_id;
                provisional.parent_id = existing.parent_task_turn_id;
            }
        }
        if seen_turn_ids.contains(&provisional.id) {
            provisional.relation = "continuation".to_string();
            provisional.supersedes_id = None;
            provisional
                .reason_codes
                .push("same_task_reobserved_after_support_detour".to_string());
        }
        persist_provisional(conn, &provisional)?;
        let persisted = load_task_turn(conn, &provisional.id)?
            .ok_or_else(|| format!("task turn {} missing after persistence", provisional.id))?;
        let scope = scope_from_turn(&persisted, &provisional);
        result
            .frame_scopes
            .insert(provisional.row.frame_id.clone(), scope);
        resolved_by_id.insert(provisional.id.clone(), persisted.clone());
        seen_turn_ids.insert(provisional.id.clone());
        prior = Some(persisted);
    }

    let selected = prior.as_ref().map(|turn| {
        if turn.relation_to_prior == TaskTurnRelation::ChildSupportStep {
            turn.parent_task_turn_id
                .clone()
                .unwrap_or_else(|| turn.task_turn_id.clone())
        } else {
            turn.task_turn_id.clone()
        }
    });
    let previous_selected = conn
        .query_row(
            "SELECT task_turn_id FROM continue_task_turns WHERE selected != 0
             ORDER BY last_observed_at_ms DESC, task_turn_id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?;
    if previous_selected != selected {
        let changed_at = now_ms();
        conn.execute(
            "UPDATE continue_task_turns SET selected=0, revision=revision+1,
             updated_at_ms=?1 WHERE selected != 0",
            params![changed_at],
        )
        .map_err(to_string)?;
        if let Some(id) = &selected {
            conn.execute(
                "UPDATE continue_task_turns SET selected=1, revision=revision+1,
                 updated_at_ms=?2 WHERE task_turn_id=?1",
                params![id, changed_at],
            )
            .map_err(to_string)?;
        }
    }
    result.selected_task_turn_id = selected.clone();
    result.selection_reason_codes = vec!["newest_typed_task_turn_evidence".to_string()];
    result.alternative_task_turn_ids = resolved_by_id
        .keys()
        .filter(|id| Some(id.as_str()) != selected.as_deref())
        .cloned()
        .collect();
    result.alternative_task_turn_ids.sort();
    result.conflict_flags = result
        .frame_scopes
        .values()
        .flat_map(|scope| scope.quality_flags.iter())
        .filter(|flag| flag.contains("ambiguous") || flag.contains("flattened"))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    result.missing_evidence = resolved_by_id
        .values()
        .flat_map(|turn| turn.missing_evidence.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(result)
}

fn frame_is_manual_continue_boundary(conn: &Connection, frame_id: &str) -> Result<bool, String> {
    if !table_exists(conn, "frames")? || !column_exists(conn, "frames", "capture_trigger")? {
        return Ok(false);
    }
    conn.query_row(
        "SELECT capture_trigger='manual_continue_boundary' FROM frames WHERE CAST(id AS TEXT)=?1",
        params![frame_id],
        |row| row.get::<_, bool>(0),
    )
    .optional()
    .map(|value| value.unwrap_or(false))
    .map_err(to_string)
}

fn preserve_selected_turn_across_empty_sample(
    conn: &Connection,
    row: &SalientRow,
) -> Result<Option<CurrentTaskTurn>, String> {
    let selected_id = conn
        .query_row(
            "SELECT task_turn_id FROM continue_task_turns
             WHERE selected=1 ORDER BY updated_at_ms DESC, task_turn_id DESC LIMIT 1",
            [],
            |result| result.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?;
    let Some(selected) = selected_id
        .map(|id| load_task_turn(conn, &id))
        .transpose()?
        .flatten()
    else {
        return Ok(None);
    };
    let row_surface_key_hash = row
        .surface_key
        .as_deref()
        .map(|value| stable_hash(format!("task-turn-surface|{value}").as_bytes()));
    let exact_lineage = row.session_id == selected.session_id
        && row_surface_key_hash.is_some()
        && row_surface_key_hash == selected.surface_key_hash;
    let bounded = row.observed_at_ms >= selected.last_observed_at_ms
        && row
            .observed_at_ms
            .saturating_sub(selected.last_observed_at_ms)
            <= 15_000;
    Ok((exact_lineage
        && bounded
        && selected.attribution_confidence >= 0.64
        && !selected.latest_user_span_ids.is_empty())
    .then_some(selected))
}

pub(crate) fn finalize_task_turns(
    conn: &Connection,
    task_turn_ids: &[String],
) -> Result<(), String> {
    if !table_exists(conn, "continue_task_turns")? || task_turn_ids.is_empty() {
        return Ok(());
    }
    if !table_exists(conn, "continue_task_actions")?
        || !column_exists(conn, "continue_task_actions", "task_turn_id")?
    {
        return Ok(());
    }
    for id in task_turn_ids.iter().collect::<BTreeSet<_>>() {
        finalize_one(conn, id)?;
    }
    Ok(())
}

/// Attaches semantic task turns to an already-established workstream without
/// participating in workstream ranking or return-target eligibility.  The
/// bridge is deliberately restricted to persisted action -> episode ->
/// workstream memberships, and resolves ties deterministically.
pub(crate) fn link_task_turn_workstreams(conn: &Connection) -> Result<(), String> {
    for table in [
        "continue_task_turns",
        "continue_task_actions",
        "continue_episode_actions",
        "continue_workstream_episodes",
    ] {
        if !table_exists(conn, table)? {
            return Ok(());
        }
    }
    if !column_exists(conn, "continue_task_actions", "task_turn_id")? {
        return Ok(());
    }
    let score = if column_exists(conn, "continue_workstream_episodes", "membership_score")? {
        "COALESCE(we.membership_score, 0.0)"
    } else {
        "0.0"
    };
    let order = if column_exists(conn, "continue_workstream_episodes", "order_index")? {
        "COALESCE(we.order_index, 0)"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT DISTINCT ta.task_turn_id, we.workstream_id, {score} AS membership_score,
                {order} AS membership_order, ea.episode_id
         FROM continue_task_actions ta
         JOIN continue_episode_actions ea ON ea.action_id = ta.id
         JOIN continue_workstream_episodes we ON we.episode_id = ea.episode_id
         WHERE ta.task_turn_id IS NOT NULL AND ta.task_turn_id != ''
         ORDER BY ta.task_turn_id, membership_score DESC, membership_order DESC,
                  we.workstream_id, ea.episode_id"
    );
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let candidates = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    let chosen = candidates.into_iter().fold(
        BTreeMap::<String, String>::new(),
        |mut values, (turn, workstream)| {
            values.entry(turn).or_insert(workstream);
            values
        },
    );
    for (turn_id, workstream_id) in chosen {
        let before = load_task_turn(conn, &turn_id)?;
        let Some(before) = before else {
            continue;
        };
        let unchanged = before.workstream_id.as_deref() == Some(workstream_id.as_str());
        let revision = before.revision + i64::from(!unchanged);
        let now = now_ms();
        if !unchanged {
            conn.execute(
                "UPDATE continue_task_turns SET workstream_id=?2, revision=?3,
                 material_fingerprint=?4, updated_at_ms=?5 WHERE task_turn_id=?1",
                params![
                    turn_id,
                    workstream_id,
                    revision,
                    material_fingerprint(&[
                        before.latest_user_goal_hash.as_deref().unwrap_or(""),
                        before.execution_state.label(),
                        before.current_actor.label(),
                        before.waiting_on.label(),
                        before.relation_to_prior.label(),
                        &workstream_id,
                    ]),
                    now,
                ],
            )
            .map_err(to_string)?;
        }
        if table_exists(conn, "continue_task_turn_workstream_memberships")? {
            let action_ids = load_membership_action_ids(conn, &turn_id, &workstream_id)?;
            let evidence_json = json_string(&action_ids)?;
            let reason_codes = json_string(&vec![
                "action_episode_workstream_membership".to_string(),
                "current_task_turn_justifies_selection".to_string(),
            ])?;
            conn.execute(
                "INSERT INTO continue_task_turn_workstream_memberships (
                   task_turn_id, workstream_id, relation_to_current_task,
                   membership_score, selected, evidence_action_ids_json,
                   reason_codes_json, revision, created_at_ms, updated_at_ms
                 ) VALUES (?1,?2,'same_task',1.0,1,?3,?4,?5,?6,?6)
                 ON CONFLICT(task_turn_id, workstream_id) DO UPDATE SET
                   relation_to_current_task='same_task', membership_score=1.0,
                   selected=1, evidence_action_ids_json=excluded.evidence_action_ids_json,
                   reason_codes_json=excluded.reason_codes_json,
                   revision=excluded.revision, updated_at_ms=excluded.updated_at_ms",
                params![
                    turn_id,
                    workstream_id,
                    evidence_json,
                    reason_codes,
                    revision,
                    now
                ],
            )
            .map_err(to_string)?;
        }
    }
    Ok(())
}

fn load_membership_action_ids(
    conn: &Connection,
    task_turn_id: &str,
    workstream_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT ta.id
             FROM continue_task_actions ta
             JOIN continue_episode_actions ea ON ea.action_id=ta.id
             JOIN continue_workstream_episodes we ON we.episode_id=ea.episode_id
             WHERE ta.task_turn_id=?1 AND we.workstream_id=?2
             ORDER BY ta.created_at_ms, ta.id",
        )
        .map_err(to_string)?;
    let values = stmt
        .query_map(params![task_turn_id, workstream_id], |row| {
            row.get::<_, String>(0)
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    Ok(values)
}

pub(crate) fn selected_current_task_turn(
    conn: &Connection,
) -> Result<Option<CurrentTaskTurn>, String> {
    if !table_exists(conn, "continue_task_turns")? {
        return Ok(None);
    }
    let id = conn
        .query_row(
            "SELECT task_turn_id FROM continue_task_turns
             WHERE selected != 0 AND schema_version = ?1
             ORDER BY last_observed_at_ms DESC, task_turn_id DESC LIMIT 1",
            params![CURRENT_TASK_TURN_SCHEMA_V2],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?;
    id.map(|id| load_task_turn(conn, &id))
        .transpose()
        .map(Option::flatten)
}

fn clear_selected_task_turn(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "continue_task_turns")? {
        return Ok(());
    }
    conn.execute(
        "UPDATE continue_task_turns
         SET selected = 0, revision = revision + 1, updated_at_ms = ?1
         WHERE selected != 0",
        params![now_ms()],
    )
    .map_err(to_string)?;
    Ok(())
}

pub(crate) fn current_task_turn_accuracy_slots(
    conn: &Connection,
) -> Result<BTreeMap<String, Value>, String> {
    let Some(turn) = selected_current_task_turn(conn)? else {
        return Ok(BTreeMap::new());
    };
    let prior = turn
        .prior_task_turn_id
        .as_deref()
        .map(|id| load_task_turn(conn, id))
        .transpose()?
        .flatten();
    Ok(BTreeMap::from([
        (
            "latest_user_goal".to_string(),
            json!(turn.latest_user_goal_summary),
        ),
        (
            "current_agent_state".to_string(),
            json!(turn.actor_activity_state),
        ),
        ("task_summary".to_string(), json!(task_summary_slug(&turn))),
        ("execution_state".to_string(), json!(turn.execution_state)),
        ("current_actor".to_string(), json!(turn.current_actor)),
        ("waiting_on".to_string(), json!(turn.waiting_on)),
        ("turn_relation".to_string(), json!(turn.relation_to_prior)),
        (
            "prior_task".to_string(),
            json!(prior.as_ref().map(task_summary_slug)),
        ),
        (
            "prior_task_state".to_string(),
            json!(prior.as_ref().map(|value| value.execution_state)),
        ),
    ]))
}

pub(crate) fn task_turn_audit_json(conn: &Connection, limit: usize) -> Result<Value, String> {
    if !table_exists(conn, "continue_task_turns")? {
        return Ok(json!({
            "schema": CURRENT_TASK_TURN_SCHEMA_V2,
            "selected": Value::Null,
            "turns": [],
            "relations": [],
            "lifecycle_history": [],
            "compatibility": "no_task_turn_table"
        }));
    }
    let limit = limit.clamp(1, 500) as i64;
    let mut stmt = conn
        .prepare(
            "SELECT task_turn_id FROM continue_task_turns
             ORDER BY selected DESC, last_observed_at_ms DESC LIMIT ?1",
        )
        .map_err(to_string)?;
    let ids = stmt
        .query_map(params![limit], |row| row.get::<_, String>(0))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    let turns = ids
        .iter()
        .filter_map(|id| load_task_turn(conn, id).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    let relations = load_json_rows(
        conn,
        "SELECT task_turn_id, related_task_turn_id, relation_kind, confidence, reason_code
         FROM continue_task_turn_relations ORDER BY updated_at_ms DESC LIMIT ?1",
        limit,
        |row| {
            Ok(json!({
                "task_turn_id": row.get::<_, String>(0)?,
                "related_task_turn_id": row.get::<_, String>(1)?,
                "relation_kind": row.get::<_, String>(2)?,
                "confidence": row.get::<_, f64>(3)?,
                "reason_code": row.get::<_, String>(4)?,
            }))
        },
    )?;
    let history = load_json_rows(
        conn,
        "SELECT task_turn_id, revision, observed_at_ms, previous_execution_state,
                execution_state, previous_current_actor, current_actor,
                previous_waiting_on, waiting_on, transition_reason, source_kind
         FROM continue_task_turn_lifecycle
         ORDER BY observed_at_ms DESC, revision DESC LIMIT ?1",
        limit,
        |row| {
            Ok(json!({
                "task_turn_id": row.get::<_, String>(0)?,
                "revision": row.get::<_, i64>(1)?,
                "observed_at_ms": row.get::<_, i64>(2)?,
                "previous_execution_state": row.get::<_, Option<String>>(3)?,
                "execution_state": row.get::<_, String>(4)?,
                "previous_current_actor": row.get::<_, Option<String>>(5)?,
                "current_actor": row.get::<_, String>(6)?,
                "previous_waiting_on": row.get::<_, Option<String>>(7)?,
                "waiting_on": row.get::<_, String>(8)?,
                "transition_reason": row.get::<_, String>(9)?,
                "source_kind": row.get::<_, String>(10)?,
            }))
        },
    )?;
    let selected = turns
        .iter()
        .find(|turn| turn.selected)
        .map(|turn| &turn.task_turn_id);
    Ok(json!({
        "schema": CURRENT_TASK_TURN_SCHEMA_V2,
        "selected": selected,
        "turns": turns,
        "relations": relations,
        "lifecycle_history": history,
        "summary_limits": {
            "latest_user_goal_summary_chars": MAX_GOAL_CHARS,
            "actor_activity_state_chars": MAX_STATE_CHARS,
            "task_object_chars": MAX_OBJECT_CHARS,
        }
    }))
}

pub(crate) fn latest_task_turn_marker(
    conn: &Connection,
) -> Result<(Option<String>, Option<i64>, Option<i64>), String> {
    if !table_exists(conn, "continue_task_turns")? {
        return Ok((None, None, None));
    }
    conn.query_row(
        "SELECT task_turn_id, revision, updated_at_ms FROM continue_task_turns
         WHERE selected != 0 AND schema_version = ?1
         ORDER BY last_observed_at_ms DESC, task_turn_id DESC LIMIT 1",
        params![CURRENT_TASK_TURN_SCHEMA_V2],
        |row| Ok((Some(row.get(0)?), Some(row.get(1)?), Some(row.get(2)?))),
    )
    .optional()
    .map(|value| value.unwrap_or((None, None, None)))
    .map_err(to_string)
}

pub(crate) fn prune_task_turns(conn: &Connection, now_ms: i64) -> Result<usize, String> {
    if !table_exists(conn, "continue_task_turns")? {
        return Ok(0);
    }
    let cutoff = now_ms.saturating_sub(RETENTION_MS);
    let mut stmt = conn
        .prepare(
            "SELECT task_turn_id FROM continue_task_turns
             WHERE selected = 0 AND last_observed_at_ms < ?1",
        )
        .map_err(to_string)?;
    let ids = stmt
        .query_map(params![cutoff], |row| row.get::<_, String>(0))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    delete_turns(conn, &ids)?;
    Ok(ids.len())
}

pub(crate) fn clear_task_turns_for_frames(
    conn: &Connection,
    frame_ids: &[String],
) -> Result<(), String> {
    if frame_ids.is_empty() || !table_exists(conn, "continue_task_turn_evidence")? {
        return Ok(());
    }
    let mut affected = BTreeSet::new();
    for frame_id in frame_ids {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT task_turn_id FROM continue_task_turn_evidence WHERE source_frame_id = ?1",
            )
            .map_err(to_string)?;
        affected.extend(
            stmt.query_map(params![frame_id], |row| row.get::<_, String>(0))
                .map_err(to_string)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(to_string)?,
        );
        conn.execute(
            "DELETE FROM continue_task_turn_evidence WHERE source_frame_id = ?1",
            params![frame_id],
        )
        .map_err(to_string)?;
    }
    let mut orphaned = Vec::new();
    for id in affected {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM continue_task_turn_evidence WHERE task_turn_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(to_string)?;
        if count == 0 {
            orphaned.push(id);
        }
    }
    delete_turns(conn, &orphaned)?;
    Ok(())
}

fn load_salient_row(conn: &Connection, frame_id: &str) -> Result<Option<SalientRow>, String> {
    conn.query_row(
        "SELECT frame_id, session_id, surface_key, artifact_id, observed_at_ms,
                latest_user_span_ids_json, current_agent_span_ids_json,
                prior_boundary_span_ids_json, salient_user_goal_sample,
                salient_user_goal_hash, salient_agent_state_sample,
                salient_agent_state_hash, prior_boundary_sample, prior_boundary_hash,
                sampling_confidence, missing_roles_json, fallback_flags_json
         FROM continue_salient_turn_evidence WHERE frame_id = ?1",
        params![frame_id],
        |row| {
            Ok(SalientRow {
                frame_id: row.get(0)?,
                session_id: row.get(1)?,
                surface_key: row.get(2)?,
                artifact_id: row.get(3)?,
                observed_at_ms: row.get(4)?,
                user_span_ids: parse_vec(row.get::<_, String>(5)?),
                agent_span_ids: parse_vec(row.get::<_, String>(6)?),
                prior_span_ids: parse_vec(row.get::<_, String>(7)?),
                user_summary: row.get(8)?,
                user_hash: row.get(9)?,
                agent_summary: row.get(10)?,
                agent_hash: row.get(11)?,
                sampling_confidence: row.get(14)?,
                missing_evidence: parse_vec(row.get::<_, String>(15)?),
                quality_flags: parse_vec(row.get::<_, String>(16)?),
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn salient_identity_key(row: &SalientRow) -> String {
    format!(
        "{}|{}",
        row.session_id.as_deref().unwrap_or("none"),
        row.user_hash.as_deref().unwrap_or(&row.frame_id)
    )
}

fn build_provisional(
    conn: &Connection,
    row: SalientRow,
    prior: Option<&CurrentTaskTurn>,
) -> Result<ProvisionalTurn, String> {
    if row.user_span_ids.is_empty() {
        return Err("current task turn requires current user goal evidence".to_string());
    }
    let privacy_blocked = any_privacy_blocked(conn, &row.user_span_ids)?;
    let has_user_goal = true;
    let goal_summary = if privacy_blocked {
        None
    } else {
        row.user_summary
            .clone()
            .and_then(|value| sanitize_public_text(value, MAX_GOAL_CHARS))
    };
    let actor_summary = if privacy_blocked {
        None
    } else {
        row.agent_summary
            .clone()
            .and_then(|value| sanitize_public_text(value, MAX_STATE_CHARS))
    };
    let goal_hash = row
        .user_hash
        .clone()
        .or_else(|| goal_summary.as_deref().map(text_hash));
    let stable_boundary = row
        .user_span_ids
        .first()
        .and_then(|span| source_record_for_span(conn, span).ok().flatten())
        .map(|(_, record_id, _)| record_id)
        .or_else(|| goal_hash.clone())
        .unwrap_or_else(|| format!("frame:{}", row.frame_id));
    let boundary_identity = goal_hash.clone().unwrap_or(stable_boundary);
    let id_material = format!(
        "{}|{}|{}",
        CURRENT_TASK_TURN_SCHEMA_V2,
        row.session_id.as_deref().unwrap_or("none"),
        boundary_identity
    );
    let id = format!("task-turn-{}", stable_hash(id_material.as_bytes()));
    let same_persisted_turn = prior.is_some_and(|turn| turn.task_turn_id == id);
    let same_as_prior = prior
        .and_then(|turn| turn.latest_user_goal_hash.as_deref())
        .zip(goal_hash.as_deref())
        .is_some_and(|(left, right)| left == right);
    let relation = if same_persisted_turn {
        prior
            .map(|turn| turn.relation_to_prior.label().to_string())
            .unwrap_or_else(|| "new_task".to_string())
    } else {
        relation_to_prior(goal_summary.as_deref(), prior, same_as_prior)
    };
    let (current_actor, execution_state, waiting_on, completion_evidence, actor_reasons) =
        provisional_axes(actor_summary.as_deref(), has_user_goal);
    let task_kind = classify_task_kind(goal_summary.as_deref());
    let task_object = canonical_task_object(goal_summary.as_deref());
    let mut reason_codes = vec!["provisional_boundary_from_p6_02_typed_evidence".to_string()];
    reason_codes.extend(actor_reasons);
    reason_codes.push(format!("relation_{relation}"));
    let source_record_ids = row
        .user_span_ids
        .iter()
        .chain(row.agent_span_ids.iter())
        .filter_map(|span| source_record_for_span(conn, span).ok().flatten())
        .map(|(_, record, _)| record)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let attribution_confidence = if row.user_span_ids.is_empty() {
        if !row.prior_span_ids.is_empty() && execution_state == "completed" {
            row.sampling_confidence.min(0.72)
        } else {
            row.sampling_confidence.min(0.45)
        }
    } else if row
        .quality_flags
        .iter()
        .any(|flag| flag.contains("flattened"))
    {
        row.sampling_confidence.min(0.55)
    } else {
        row.sampling_confidence
    };
    let supersedes_id = if same_persisted_turn {
        prior.and_then(|turn| turn.supersedes_task_turn_id.clone())
    } else {
        (relation == "supersedes")
            .then(|| prior.map(|turn| turn.task_turn_id.clone()))
            .flatten()
    };
    let parent_id = if same_persisted_turn {
        prior.and_then(|turn| turn.parent_task_turn_id.clone())
    } else {
        (relation == "child_support_step")
            .then(|| prior.map(|turn| turn.task_turn_id.clone()))
            .flatten()
    };
    let prior_id = if same_persisted_turn {
        prior.and_then(|turn| turn.prior_task_turn_id.clone())
    } else {
        prior.map(|turn| turn.task_turn_id.clone())
    };
    Ok(ProvisionalTurn {
        row,
        id,
        goal_summary,
        goal_hash,
        actor_summary,
        task_object,
        task_kind,
        current_actor,
        execution_state,
        waiting_on,
        completion_evidence,
        relation,
        prior_id,
        supersedes_id,
        parent_id,
        source_record_ids,
        attribution_confidence: round_confidence(attribution_confidence),
        reason_codes,
    })
}

fn relation_to_prior(
    goal: Option<&str>,
    prior: Option<&CurrentTaskTurn>,
    same_as_prior: bool,
) -> String {
    let Some(prior) = prior else {
        return "new_task".to_string();
    };
    if same_as_prior {
        return "continuation".to_string();
    }
    let lower = goal.unwrap_or_default().to_ascii_lowercase();
    if contains_any(
        &lower,
        &["actually", "instead", "no,", "correction", "rather than"],
    ) {
        return "correction".to_string();
    }
    let clarification_cue = contains_any(
        &lower,
        &[
            "what do you mean",
            "can you explain",
            "could you explain",
            "why did",
            "how does that",
            "which one",
            "clarify",
        ],
    );
    if clarification_cue
        || token_overlap(
            goal.unwrap_or_default(),
            prior
                .latest_user_goal_summary
                .as_deref()
                .unwrap_or_default(),
        ) >= 0.55
    {
        return "clarification".to_string();
    }
    if contains_any(
        &lower,
        &[
            "search docs",
            "check docs",
            "look up",
            "run the tests",
            "terminal",
        ],
    ) {
        return "child_support_step".to_string();
    }
    if !matches!(
        prior.execution_state,
        TaskExecutionState::Completed | TaskExecutionState::Superseded
    ) {
        "supersedes".to_string()
    } else {
        "new_task".to_string()
    }
}

fn provisional_axes(
    agent_state: Option<&str>,
    has_user_goal: bool,
) -> (String, String, String, Option<String>, Vec<String>) {
    let lower = agent_state.unwrap_or_default().to_ascii_lowercase();
    if !lower.is_empty() && is_completion_text(&lower) {
        return (
            "assistant_or_agent".to_string(),
            "completed".to_string(),
            "none".to_string(),
            sanitize_public_text(agent_state.unwrap_or_default().to_string(), MAX_STATE_CHARS),
            vec!["attributed_agent_completion_after_user_boundary".to_string()],
        );
    }
    if !lower.is_empty() {
        let waiting = if contains_any(
            &lower,
            &["waiting for you", "need you to", "please provide"],
        ) {
            "user"
        } else if contains_any(
            &lower,
            &["waiting for external", "pending external", "blocked by"],
        ) {
            "external"
        } else {
            "agent"
        };
        let execution = if contains_any(&lower, &["blocked", "cannot proceed", "can't proceed"]) {
            "blocked"
        } else {
            "active"
        };
        return (
            "assistant_or_agent".to_string(),
            execution.to_string(),
            waiting.to_string(),
            None,
            vec!["current_agent_state_after_user_boundary".to_string()],
        );
    }
    if has_user_goal {
        return (
            "user".to_string(),
            "active".to_string(),
            "agent".to_string(),
            None,
            vec!["latest_user_goal_without_agent_state".to_string()],
        );
    }
    (
        "unknown".to_string(),
        "unclear".to_string(),
        "unknown".to_string(),
        None,
        vec!["missing_typed_user_and_agent_state".to_string()],
    )
}

fn persist_provisional(conn: &Connection, turn: &ProvisionalTurn) -> Result<(), String> {
    let existing = load_task_turn(conn, &turn.id)?;
    let started_at = existing
        .as_ref()
        .map(|value| value.started_at_ms.min(turn.row.observed_at_ms))
        .unwrap_or(turn.row.observed_at_ms);
    let last_observed = existing
        .as_ref()
        .map(|value| value.last_observed_at_ms.max(turn.row.observed_at_ms))
        .unwrap_or(turn.row.observed_at_ms);
    let summary_source_spans = &turn.row.user_span_ids;
    let goal_spans = union_strings(
        existing.as_ref().map(|value| &value.latest_user_span_ids),
        &turn.row.user_span_ids,
    );
    let state_spans = union_strings(
        existing.as_ref().map(|value| &value.current_state_span_ids),
        &turn.row.agent_span_ids,
    );
    let prior_spans = union_strings(
        existing
            .as_ref()
            .map(|value| &value.prior_boundary_span_ids),
        &turn.row.prior_span_ids,
    );
    let evidence_quality = if turn.row.user_span_ids.is_empty() {
        "thin"
    } else if turn.attribution_confidence >= 0.74 && turn.row.quality_flags.is_empty() {
        "strong"
    } else {
        "partial"
    };
    let mut missing = turn.row.missing_evidence.clone();
    if turn.goal_summary.is_none() {
        push_unique(&mut missing, "public_safe_latest_user_goal".to_string());
    }
    let mut flags = turn.row.quality_flags.clone();
    if turn.relation == "unknown" && existing.is_some() {
        push_unique(&mut flags, "ambiguous_task_relation".to_string());
    }
    let goal_confidence = if turn.goal_summary.is_some() {
        turn.attribution_confidence
    } else {
        turn.attribution_confidence.min(0.4)
    };
    let object_confidence = if turn.task_object.is_some() {
        goal_confidence.min(0.84)
    } else {
        0.0
    };
    let actor_confidence = if turn.actor_summary.is_some() {
        turn.attribution_confidence
    } else {
        turn.attribution_confidence.min(0.62)
    };
    let relation_confidence = if turn.relation == "unknown" {
        0.35
    } else if turn.relation == "continuation" {
        turn.attribution_confidence
    } else {
        turn.attribution_confidence.min(0.82)
    };
    let surface_key_hash = turn
        .row
        .surface_key
        .as_deref()
        .map(|value| stable_hash(format!("task-turn-surface|{value}").as_bytes()));
    let goal_spans_json = json_string(&goal_spans)?;
    let state_spans_json = json_string(&state_spans)?;
    let flags_json = json_string(&flags)?;
    let fingerprint = material_fingerprint(&[
        turn.goal_summary.as_deref().unwrap_or(""),
        turn.task_object.as_deref().unwrap_or(""),
        &turn.task_kind,
        &turn.current_actor,
        turn.actor_summary.as_deref().unwrap_or(""),
        &turn.execution_state,
        &turn.waiting_on,
        &turn.relation,
        &goal_spans_json,
        &state_spans_json,
        &flags_json,
    ]);
    let previous_fingerprint = existing
        .as_ref()
        .and_then(|_| load_material_fingerprint(conn, &turn.id).ok().flatten());
    let material_changed =
        existing.is_none() || previous_fingerprint.as_deref() != Some(fingerprint.as_str());
    let revision = existing
        .as_ref()
        .map(|value| value.revision + i64::from(material_changed))
        .unwrap_or(1);
    let now = now_ms();
    let updated_at = existing
        .as_ref()
        .filter(|_| !material_changed)
        .map(|value| value.updated_at_ms)
        .unwrap_or(now);
    let goal_redaction =
        summary_redaction_status(turn.goal_summary.as_deref(), summary_source_spans);
    let object_hash = turn.task_object.as_deref().map(text_hash);
    let object_redaction =
        summary_redaction_status(turn.task_object.as_deref(), summary_source_spans);
    let actor_redaction =
        summary_redaction_status(turn.actor_summary.as_deref(), &turn.row.agent_span_ids);
    conn.execute(
        "INSERT INTO continue_task_turns (
           task_turn_id, schema_version, session_id, surface_key_hash, artifact_id,
           workstream_id, started_at_ms, last_observed_at_ms, latest_user_goal_summary,
           latest_user_goal_hash, latest_user_goal_redaction_status, task_object,
           task_object_hash, task_object_redaction_status, task_kind, current_actor,
           actor_activity_state, actor_activity_state_hash,
           actor_activity_state_redaction_status, execution_state, waiting_on,
           relation_to_prior, prior_task_turn_id, supersedes_task_turn_id,
           parent_task_turn_id, latest_user_span_ids_json, current_state_span_ids_json,
           prior_boundary_span_ids_json, supporting_action_ids_json,
           supporting_event_ids_json, evidence_quality, goal_confidence,
           task_object_confidence, actor_state_confidence, execution_state_confidence,
           waiting_on_confidence, relation_confidence, attribution_confidence,
           missing_evidence_json, quality_flags_json, reason_codes_json,
           material_fingerprint, revision, selected, created_at_ms, updated_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                   ?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,
                   ?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,
                   ?45,?46)
         ON CONFLICT(task_turn_id) DO UPDATE SET
           schema_version=excluded.schema_version,
           session_id=excluded.session_id,
           surface_key_hash=excluded.surface_key_hash,
           artifact_id=excluded.artifact_id,
           workstream_id=excluded.workstream_id,
           started_at_ms=excluded.started_at_ms,
           last_observed_at_ms=excluded.last_observed_at_ms,
           latest_user_goal_summary=excluded.latest_user_goal_summary,
           latest_user_goal_hash=excluded.latest_user_goal_hash,
           latest_user_goal_redaction_status=excluded.latest_user_goal_redaction_status,
           task_object=excluded.task_object,
           task_object_hash=excluded.task_object_hash,
           task_object_redaction_status=excluded.task_object_redaction_status,
           task_kind=excluded.task_kind,
           current_actor=excluded.current_actor,
           actor_activity_state=excluded.actor_activity_state,
           actor_activity_state_hash=excluded.actor_activity_state_hash,
           actor_activity_state_redaction_status=excluded.actor_activity_state_redaction_status,
           execution_state=excluded.execution_state,
           waiting_on=excluded.waiting_on,
           relation_to_prior=excluded.relation_to_prior,
           prior_task_turn_id=excluded.prior_task_turn_id,
           supersedes_task_turn_id=excluded.supersedes_task_turn_id,
           parent_task_turn_id=excluded.parent_task_turn_id,
           latest_user_span_ids_json=excluded.latest_user_span_ids_json,
           current_state_span_ids_json=excluded.current_state_span_ids_json,
           prior_boundary_span_ids_json=excluded.prior_boundary_span_ids_json,
           supporting_action_ids_json=excluded.supporting_action_ids_json,
           supporting_event_ids_json=excluded.supporting_event_ids_json,
           evidence_quality=excluded.evidence_quality,
           goal_confidence=excluded.goal_confidence,
           task_object_confidence=excluded.task_object_confidence,
           actor_state_confidence=excluded.actor_state_confidence,
           execution_state_confidence=excluded.execution_state_confidence,
           waiting_on_confidence=excluded.waiting_on_confidence,
           relation_confidence=excluded.relation_confidence,
           attribution_confidence=excluded.attribution_confidence,
           missing_evidence_json=excluded.missing_evidence_json,
           quality_flags_json=excluded.quality_flags_json,
           reason_codes_json=excluded.reason_codes_json,
           material_fingerprint=excluded.material_fingerprint,
           revision=excluded.revision,
           selected=excluded.selected,
           updated_at_ms=excluded.updated_at_ms",
        params![
            turn.id,
            CURRENT_TASK_TURN_SCHEMA_V2,
            turn.row.session_id,
            surface_key_hash,
            turn.row.artifact_id,
            existing
                .as_ref()
                .and_then(|value| value.workstream_id.clone()),
            started_at,
            last_observed,
            turn.goal_summary,
            turn.goal_hash,
            goal_redaction,
            turn.task_object,
            object_hash,
            object_redaction,
            turn.task_kind,
            turn.current_actor,
            turn.actor_summary,
            turn.row.agent_hash,
            actor_redaction,
            turn.execution_state,
            turn.waiting_on,
            turn.relation,
            turn.prior_id,
            turn.supersedes_id,
            turn.parent_id,
            goal_spans_json,
            state_spans_json,
            json_string(&prior_spans)?,
            existing
                .as_ref()
                .map(|value| json_string(&value.supporting_action_ids))
                .transpose()?
                .unwrap_or_else(|| "[]".to_string()),
            existing
                .as_ref()
                .map(|value| json_string(&value.supporting_event_ids))
                .transpose()?
                .unwrap_or_else(|| "[]".to_string()),
            evidence_quality,
            goal_confidence,
            object_confidence,
            actor_confidence,
            actor_confidence,
            actor_confidence,
            relation_confidence,
            turn.attribution_confidence,
            json_string(&missing)?,
            flags_json,
            json_string(&turn.reason_codes)?,
            fingerprint,
            revision,
            existing.as_ref().is_some_and(|value| value.selected) as i64,
            now,
            updated_at,
        ],
    )
    .map_err(to_string)?;
    persist_evidence_links(conn, turn, summary_source_spans, &state_spans, now)?;
    persist_relation(conn, turn, now)?;
    if existing.as_ref().is_none_or(|value| {
        value.execution_state.label() != turn.execution_state
            || value.current_actor.label() != turn.current_actor
            || value.waiting_on.label() != turn.waiting_on
    }) {
        persist_history(
            conn,
            &turn.id,
            revision,
            turn.row.observed_at_ms,
            existing.as_ref().map(|value| value.execution_state.label()),
            &turn.execution_state,
            existing.as_ref().map(|value| value.current_actor.label()),
            &turn.current_actor,
            existing.as_ref().map(|value| value.waiting_on.label()),
            &turn.waiting_on,
            "provisional_typed_evidence",
            "p6_02_ordered_evidence",
        )?;
    }
    if let Some(prior_id) = &turn.supersedes_id {
        mark_superseded(conn, prior_id, &turn.id, turn.row.observed_at_ms)?;
    }
    Ok(())
}

fn persist_evidence_links(
    conn: &Connection,
    turn: &ProvisionalTurn,
    goal_spans: &[String],
    state_spans: &[String],
    now: i64,
) -> Result<(), String> {
    for (field, role, spans) in [
        ("latest_user_goal_summary", "latest_user_goal", goal_spans),
        ("task_object", "latest_user_goal", goal_spans),
        ("actor_activity_state", "current_agent_state", state_spans),
        ("execution_state", "current_agent_state", state_spans),
        ("current_actor", "current_agent_state", state_spans),
        ("waiting_on", "current_agent_state", state_spans),
    ] {
        for span in spans {
            let (frame, record, hash, privacy) = source_link_for_span(conn, span)?;
            conn.execute(
                "INSERT OR REPLACE INTO continue_task_turn_evidence (
                   task_turn_id, source_frame_id, source_span_id, source_record_id,
                   field_name, source_text_hash, redaction_status, evidence_role,
                   observed_at_ms, created_at_ms
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    turn.id,
                    frame,
                    span,
                    record,
                    field,
                    hash,
                    privacy,
                    role,
                    turn.row.observed_at_ms,
                    now
                ],
            )
            .map_err(to_string)?;
        }
    }
    for span in &turn.row.prior_span_ids {
        let (frame, record, hash, privacy) = source_link_for_span(conn, span)?;
        conn.execute(
            "INSERT OR REPLACE INTO continue_task_turn_evidence (
               task_turn_id, source_frame_id, source_span_id, source_record_id,
               field_name, source_text_hash, redaction_status, evidence_role,
               observed_at_ms, created_at_ms
             ) VALUES (?1,?2,?3,?4,'relation_to_prior',?5,?6,'prior_boundary',?7,?8)",
            params![
                turn.id,
                frame,
                span,
                record,
                hash,
                privacy,
                turn.row.observed_at_ms,
                now
            ],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn persist_relation(conn: &Connection, turn: &ProvisionalTurn, now: i64) -> Result<(), String> {
    let Some(prior) = &turn.prior_id else {
        return Ok(());
    };
    if prior == &turn.id {
        return Ok(());
    }
    conn.execute(
        "INSERT OR REPLACE INTO continue_task_turn_relations (
           task_turn_id, related_task_turn_id, relation_kind, confidence,
           reason_code, created_at_ms, updated_at_ms
         ) VALUES (?1,?2,?3,?4,?5,
                   COALESCE((SELECT created_at_ms FROM continue_task_turn_relations
                             WHERE task_turn_id=?1 AND related_task_turn_id=?2 AND relation_kind=?3),?6),?6)",
        params![
            turn.id,
            prior,
            turn.relation,
            turn.attribution_confidence.min(0.88),
            format!("typed_boundary_{}", turn.relation),
            now
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn finalize_one(conn: &Connection, id: &str) -> Result<(), String> {
    let Some(before) = load_task_turn(conn, id)? else {
        return Ok(());
    };
    let kind_expr = if column_exists(conn, "continue_task_actions", "action_kind")? {
        "COALESCE(action_kind,'')"
    } else {
        "''"
    };
    let delta_expr = if column_exists(conn, "continue_task_actions", "semantic_delta_kind")? {
        "COALESCE(semantic_delta_kind,'')"
    } else {
        "''"
    };
    let after_expr = if column_exists(conn, "continue_task_actions", "semantic_after_hint")? {
        "COALESCE(semantic_after_hint,'')"
    } else {
        "''"
    };
    let event_expr = if column_exists(conn, "continue_task_actions", "evidence_event_ids_json")? {
        "COALESCE(evidence_event_ids_json,'[]')"
    } else {
        "'[]'"
    };
    let created_expr = if column_exists(conn, "continue_task_actions", "created_at_ms")? {
        "created_at_ms"
    } else {
        "0"
    };
    let sql = format!(
        "SELECT id, {kind_expr}, {delta_expr}, {after_expr}, {event_expr}, {created_expr}
         FROM continue_task_actions WHERE task_turn_id = ?1 ORDER BY {created_expr}, id"
    );
    let mut stmt = conn.prepare(&sql).map_err(to_string)?;
    let actions = stmt
        .query_map(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    if actions.is_empty() {
        return Ok(());
    }
    let action_ids = actions
        .iter()
        .map(|item| item.0.clone())
        .collect::<Vec<_>>();
    let mut event_ids = actions
        .iter()
        .flat_map(|item| parse_vec(item.4.clone()))
        .collect::<Vec<_>>();
    if table_exists(conn, "continue_task_action_events")? {
        for action_id in &action_ids {
            let mut stmt = conn
                .prepare(
                    "SELECT event_id FROM continue_task_action_events WHERE action_id=?1 ORDER BY order_index",
                )
                .map_err(to_string)?;
            event_ids.extend(
                stmt.query_map(params![action_id], |row| row.get::<_, String>(0))
                    .map_err(to_string)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(to_string)?,
            );
        }
    }
    event_ids.sort();
    event_ids.dedup();
    let latest = actions.last().expect("actions is non-empty");
    let combined = format!("{} {} {}", latest.1, latest.2, latest.3).to_ascii_lowercase();
    let (execution, actor, waiting, reason) = if is_completion_text(&combined) {
        (
            "completed",
            "assistant_or_agent",
            "none",
            "turn_scoped_action_completion",
        )
    } else if contains_any(&combined, &["blocked", "error", "failure", "failed"]) {
        (
            "blocked",
            "assistant_or_agent",
            "external",
            "turn_scoped_action_blocked",
        )
    } else if contains_any(
        &combined,
        &[
            "editing",
            "implement",
            "trace",
            "investigat",
            "navigat",
            "search",
            "test",
        ],
    ) {
        (
            "active",
            "assistant_or_agent",
            "agent",
            "turn_scoped_action_active",
        )
    } else {
        (
            before.execution_state.label(),
            before.current_actor.label(),
            before.waiting_on.label(),
            "turn_scoped_action_no_axis_change",
        )
    };
    let changed = execution != before.execution_state.label()
        || actor != before.current_actor.label()
        || waiting != before.waiting_on.label()
        || action_ids != before.supporting_action_ids
        || event_ids != before.supporting_event_ids;
    if !changed {
        return Ok(());
    }
    let revision = before.revision + 1;
    let action_ids_json = json_string(&action_ids)?;
    let event_ids_json = json_string(&event_ids)?;
    let fingerprint = material_fingerprint(&[
        before.latest_user_goal_summary.as_deref().unwrap_or(""),
        before.task_object.as_deref().unwrap_or(""),
        &before.task_kind,
        execution,
        actor,
        waiting,
        before.relation_to_prior.label(),
        &action_ids_json,
        &event_ids_json,
    ]);
    conn.execute(
        "UPDATE continue_task_turns SET execution_state=?2, current_actor=?3,
         waiting_on=?4, supporting_action_ids_json=?5, supporting_event_ids_json=?6,
         execution_state_confidence=MAX(execution_state_confidence,0.82),
         actor_state_confidence=MAX(actor_state_confidence,0.78),
         waiting_on_confidence=MAX(waiting_on_confidence,0.78),
         reason_codes_json=?7, material_fingerprint=?8, revision=?9, updated_at_ms=?10
         WHERE task_turn_id=?1",
        params![
            id,
            execution,
            actor,
            waiting,
            action_ids_json,
            event_ids_json,
            json_string(&union_strings(
                Some(&before.reason_codes),
                &[reason.to_string()]
            ))?,
            fingerprint,
            revision,
            now_ms(),
        ],
    )
    .map_err(to_string)?;
    persist_history(
        conn,
        id,
        revision,
        latest.5,
        Some(before.execution_state.label()),
        execution,
        Some(before.current_actor.label()),
        actor,
        Some(before.waiting_on.label()),
        waiting,
        reason,
        "turn_scoped_actions",
    )
}

fn mark_superseded(
    conn: &Connection,
    prior_id: &str,
    new_id: &str,
    observed_at_ms: i64,
) -> Result<(), String> {
    let Some(prior) = load_task_turn(conn, prior_id)? else {
        return Ok(());
    };
    if matches!(
        prior.execution_state,
        TaskExecutionState::Completed | TaskExecutionState::Superseded
    ) {
        return Ok(());
    }
    let revision = prior.revision + 1;
    conn.execute(
        "UPDATE continue_task_turns SET execution_state='superseded', selected=0,
         revision=?2, updated_at_ms=?3, reason_codes_json=?4,
         material_fingerprint=?5 WHERE task_turn_id=?1",
        params![
            prior_id,
            revision,
            now_ms(),
            json_string(&union_strings(
                Some(&prior.reason_codes),
                &[format!("superseded_by:{new_id}")]
            ))?,
            material_fingerprint(&[
                prior.latest_user_goal_summary.as_deref().unwrap_or(""),
                "superseded",
                new_id
            ])
        ],
    )
    .map_err(to_string)?;
    persist_history(
        conn,
        prior_id,
        revision,
        observed_at_ms,
        Some(prior.execution_state.label()),
        "superseded",
        Some(prior.current_actor.label()),
        prior.current_actor.label(),
        Some(prior.waiting_on.label()),
        prior.waiting_on.label(),
        "new_high_confidence_user_goal",
        "provisional_resolver",
    )
}

#[allow(clippy::too_many_arguments)]
fn persist_history(
    conn: &Connection,
    task_turn_id: &str,
    revision: i64,
    observed_at_ms: i64,
    previous_execution: Option<&str>,
    execution: &str,
    previous_actor: Option<&str>,
    actor: &str,
    previous_waiting: Option<&str>,
    waiting: &str,
    reason: &str,
    source: &str,
) -> Result<(), String> {
    let material =
        format!("{task_turn_id}|{revision}|{execution}|{actor}|{waiting}|{reason}|{source}");
    conn.execute(
        "INSERT OR IGNORE INTO continue_task_turn_lifecycle (
           id, task_turn_id, revision, observed_at_ms, previous_execution_state,
           execution_state, previous_current_actor, current_actor, previous_waiting_on,
           waiting_on, transition_reason, source_kind, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            format!("task-turn-history-{}", stable_hash(material.as_bytes())),
            task_turn_id,
            revision,
            observed_at_ms,
            previous_execution,
            execution,
            previous_actor,
            actor,
            previous_waiting,
            waiting,
            reason,
            source,
            now_ms()
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn load_prior_turn(conn: &Connection, row: &SalientRow) -> Result<Option<CurrentTaskTurn>, String> {
    let id = conn
        .query_row(
            "SELECT task_turn_id FROM continue_task_turns
             WHERE ((?1 IS NULL AND session_id IS NULL) OR session_id=?1)
               AND last_observed_at_ms <= ?2
             ORDER BY last_observed_at_ms DESC, task_turn_id DESC LIMIT 1",
            params![row.session_id, row.observed_at_ms],
            |result| result.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?;
    id.map(|id| load_task_turn(conn, &id))
        .transpose()
        .map(Option::flatten)
}

fn load_task_turn(conn: &Connection, id: &str) -> Result<Option<CurrentTaskTurn>, String> {
    conn.query_row(
        "SELECT schema_version, task_turn_id, session_id, surface_key_hash, artifact_id,
                workstream_id, started_at_ms, last_observed_at_ms,
                latest_user_goal_summary, latest_user_goal_hash,
                latest_user_goal_redaction_status, task_object, task_object_hash,
                task_object_redaction_status, task_kind, current_actor,
                actor_activity_state, actor_activity_state_hash,
                actor_activity_state_redaction_status, execution_state, waiting_on,
                relation_to_prior, prior_task_turn_id, supersedes_task_turn_id,
                parent_task_turn_id, latest_user_span_ids_json,
                current_state_span_ids_json, prior_boundary_span_ids_json,
                supporting_action_ids_json, supporting_event_ids_json, evidence_quality,
                goal_confidence, task_object_confidence, actor_state_confidence,
                execution_state_confidence, waiting_on_confidence, relation_confidence,
                attribution_confidence, missing_evidence_json, quality_flags_json,
                reason_codes_json, revision, selected, updated_at_ms
         FROM continue_task_turns WHERE task_turn_id=?1",
        params![id],
        |row| {
            Ok(CurrentTaskTurn {
                schema_version: row.get(0)?,
                task_turn_id: row.get(1)?,
                session_id: row.get(2)?,
                surface_key_hash: row.get(3)?,
                artifact_id: row.get(4)?,
                workstream_id: row.get(5)?,
                started_at_ms: row.get(6)?,
                last_observed_at_ms: row.get(7)?,
                latest_user_goal_summary: row.get(8)?,
                latest_user_goal_hash: row.get(9)?,
                latest_user_goal_redaction_status: row.get(10)?,
                task_object: row.get(11)?,
                task_object_hash: row.get(12)?,
                task_object_redaction_status: row.get(13)?,
                task_kind: row.get(14)?,
                current_actor: row.get(15)?,
                actor_activity_state: row.get(16)?,
                actor_activity_state_hash: row.get(17)?,
                actor_activity_state_redaction_status: row.get(18)?,
                execution_state: row.get(19)?,
                waiting_on: row.get(20)?,
                relation_to_prior: row.get(21)?,
                prior_task_turn_id: row.get(22)?,
                supersedes_task_turn_id: row.get(23)?,
                parent_task_turn_id: row.get(24)?,
                latest_user_span_ids: parse_vec(row.get::<_, String>(25)?),
                current_state_span_ids: parse_vec(row.get::<_, String>(26)?),
                prior_boundary_span_ids: parse_vec(row.get::<_, String>(27)?),
                supporting_action_ids: parse_vec(row.get::<_, String>(28)?),
                supporting_event_ids: parse_vec(row.get::<_, String>(29)?),
                evidence_quality: row.get(30)?,
                goal_confidence: row.get(31)?,
                task_object_confidence: row.get(32)?,
                actor_state_confidence: row.get(33)?,
                execution_state_confidence: row.get(34)?,
                waiting_on_confidence: row.get(35)?,
                relation_confidence: row.get(36)?,
                attribution_confidence: row.get(37)?,
                missing_evidence: parse_vec(row.get::<_, String>(38)?),
                quality_flags: parse_vec(row.get::<_, String>(39)?),
                reason_codes: parse_vec(row.get::<_, String>(40)?),
                revision: row.get(41)?,
                selected: row.get::<_, i64>(42)? != 0,
                updated_at_ms: row.get(43)?,
            })
        },
    )
    .optional()
    .map_err(to_string)
}

fn scope_from_turn(turn: &CurrentTaskTurn, provisional: &ProvisionalTurn) -> TaskTurnScope {
    let mut spans = turn.latest_user_span_ids.clone();
    spans.extend(turn.current_state_span_ids.iter().cloned());
    spans.sort();
    spans.dedup();
    let semantic = [
        turn.latest_user_goal_summary.as_deref(),
        turn.actor_activity_state.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    TaskTurnScope {
        task_turn_id: turn.task_turn_id.clone(),
        current_span_ids: spans,
        current_source_record_ids: provisional.source_record_ids.clone(),
        current_semantic_text: semantic,
        completion_evidence_text: provisional.completion_evidence.clone().unwrap_or_default(),
        task_object: turn.task_object.clone(),
        task_kind: turn.task_kind.clone(),
        execution_state: turn.execution_state.label().to_string(),
        current_actor: turn.current_actor,
        waiting_on: turn.waiting_on.label().to_string(),
        relation_to_prior: turn.relation_to_prior.label().to_string(),
        has_current_agent_state: !turn.current_state_span_ids.is_empty(),
        typed_boundary_confident: !turn.latest_user_span_ids.is_empty()
            && turn.attribution_confidence >= 0.64
            && !turn
                .quality_flags
                .iter()
                .any(|flag| flag.contains("flattened")),
        attribution_confidence: turn.attribution_confidence,
        quality_flags: turn.quality_flags.clone(),
    }
}

fn canonical_task_object(goal: Option<&str>) -> Option<String> {
    let original = goal?.trim();
    let lower = original.to_ascii_lowercase().replace(['-', '_'], " ");
    let known = [
        ("capture button", "island_capture_button"),
        ("continue card copy", "continue_card_copy"),
        ("continue card", "continue_card"),
        ("activity recap", "activity_recap"),
        ("task turn", "current_task_turn"),
    ];
    if let Some((_, label)) = known.iter().find(|(needle, _)| lower.contains(needle)) {
        return Some((*label).to_string());
    }
    let stop = [
        "implement",
        "update",
        "fix",
        "investigate",
        "trace",
        "review",
        "check",
        "please",
        "can",
        "could",
        "would",
        "why",
        "how",
        "what",
        "the",
        "a",
        "an",
        "this",
        "that",
        "is",
        "does",
        "do",
        "to",
        "for",
        "in",
        "on",
        "and",
        "it",
        "well",
    ];
    let tokens = original
        .split(|ch: char| !ch.is_alphanumeric() && ch != '-' && ch != '_')
        .filter(|token| token.len() > 2 && !stop.contains(&token.to_ascii_lowercase().as_str()))
        .take(6)
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return None;
    }
    sanitize_public_text(tokens.join(" "), MAX_OBJECT_CHARS)
}

fn classify_task_kind(goal: Option<&str>) -> String {
    let lower = goal.unwrap_or_default().to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "why",
            "how does",
            "investigat",
            "trace",
            "understand",
            "what does",
        ],
    ) {
        "investigation"
    } else if contains_any(
        &lower,
        &[
            "implement",
            "update",
            "change",
            "add",
            "fix",
            "refactor",
            "build",
        ],
    ) {
        "implementation"
    } else if contains_any(&lower, &["review", "verify", "check", "test", "audit"]) {
        "verification"
    } else if contains_any(&lower, &["write", "document", "docs", "copy"]) {
        "documentation"
    } else {
        "unknown"
    }
    .to_string()
}

fn task_summary_slug(turn: &CurrentTaskTurn) -> String {
    let object = turn.task_object.as_deref().unwrap_or("current_task");
    match (object, turn.task_kind.as_str()) {
        ("island_capture_button", "investigation") => "capture_button_investigation".to_string(),
        ("continue_card_copy", "implementation") => "continue_card_copy_update".to_string(),
        _ => format!("{object}_{}", turn.task_kind),
    }
}

fn is_completion_text(lower: &str) -> bool {
    let lower = lower.to_ascii_lowercase();
    contains_any(
        &lower,
        &[
            "completed successfully",
            "implementation is complete",
            "update is complete",
            " is complete",
            "implemented and verified",
            "finished the",
            "fixed and tested",
            "all tests passed",
            "verification passed",
            "task is done",
        ],
    )
}

fn token_overlap(left: &str, right: &str) -> f64 {
    fn tokens(value: &str) -> BTreeSet<String> {
        value
            .to_ascii_lowercase()
            .split(|ch: char| !ch.is_alphanumeric())
            .filter(|token| token.len() > 3)
            .map(str::to_string)
            .collect()
    }
    let left = tokens(left);
    let right = tokens(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    left.intersection(&right).count() as f64 / left.len().min(right.len()) as f64
}

fn any_privacy_blocked(conn: &Connection, spans: &[String]) -> Result<bool, String> {
    for span in spans {
        if let Some(status) = conn
            .query_row(
                "SELECT privacy_status FROM continue_ordered_evidence_spans WHERE span_id=?1",
                params![span],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(to_string)?
        {
            if contains_any(
                &status.to_ascii_lowercase(),
                &["blocked", "private", "sensitive", "excluded"],
            ) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn source_record_for_span(
    conn: &Connection,
    span: &str,
) -> Result<Option<(String, String, String)>, String> {
    conn.query_row(
        "SELECT frame_id, primary_source_record_id, text_hash
         FROM continue_ordered_evidence_spans WHERE span_id=?1",
        params![span],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(to_string)
}

fn source_link_for_span(
    conn: &Connection,
    span: &str,
) -> Result<(String, Option<String>, Option<String>, String), String> {
    conn.query_row(
        "SELECT frame_id, primary_source_record_id, text_hash, privacy_status
         FROM continue_ordered_evidence_spans WHERE span_id=?1",
        params![span],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .optional()
    .map_err(to_string)?
    .map(|(frame, record, hash, privacy)| (frame, Some(record), Some(hash), privacy))
    .ok_or_else(|| format!("task-turn source span missing: {span}"))
}

fn load_material_fingerprint(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT material_fingerprint FROM continue_task_turns WHERE task_turn_id=?1",
        params![id],
        |row| row.get(0),
    )
    .optional()
    .map_err(to_string)
}

fn delete_turns(conn: &Connection, ids: &[String]) -> Result<(), String> {
    for id in ids {
        for sql in [
            "DELETE FROM continue_task_turn_evidence WHERE task_turn_id=?1",
            "DELETE FROM continue_task_turn_relations WHERE task_turn_id=?1 OR related_task_turn_id=?1",
            "DELETE FROM continue_task_turn_lifecycle WHERE task_turn_id=?1",
            "DELETE FROM continue_task_turns WHERE task_turn_id=?1",
        ] {
            conn.execute(sql, params![id]).map_err(to_string)?;
        }
    }
    Ok(())
}

fn load_json_rows<F>(conn: &Connection, sql: &str, limit: i64, map: F) -> Result<Vec<Value>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
{
    let mut stmt = conn.prepare(sql).map_err(to_string)?;
    let values = stmt
        .query_map(params![limit], map)
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    Ok(values)
}

fn union_strings(existing: Option<&Vec<String>>, new: &[String]) -> Vec<String> {
    existing
        .into_iter()
        .flatten()
        .chain(new.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn parse_vec(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn json_string<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(to_string)
}

fn material_fingerprint(parts: &[&str]) -> String {
    stable_hash(parts.join("\u{1f}").as_bytes())
}

fn text_hash(value: &str) -> String {
    stable_hash(format!("task-turn-public-text|{}", normalize_text(value)).as_bytes())
}

fn normalize_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn numeric_id(value: &str) -> i64 {
    value.parse().unwrap_or(i64::MAX)
}

fn round_confidence(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 1000.0).round() / 1000.0
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(to_string)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    if !table_exists(conn, table)? {
        return Ok(false);
    }
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(to_string)?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    Ok(columns.iter().any(|value| value == column))
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    if !column_exists(conn, table, column)? {
        conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {definition}"
        ))
        .map_err(to_string)?;
    }
    Ok(())
}

fn summary_redaction_status(summary: Option<&str>, source_spans: &[String]) -> &'static str {
    if summary.is_some() {
        "bounded_public_safe_summary"
    } else if source_spans.is_empty() {
        "missing_source"
    } else {
        "redacted_or_unsafe"
    }
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE continue_ordered_evidence_spans (
              span_id TEXT PRIMARY KEY,
              frame_id TEXT NOT NULL,
              primary_source_record_id TEXT NOT NULL,
              text_hash TEXT NOT NULL,
              privacy_status TEXT NOT NULL
            );
            CREATE TABLE continue_salient_turn_evidence (
              frame_id TEXT PRIMARY KEY,
              session_id TEXT,
              surface_key TEXT,
              artifact_id TEXT,
              observed_at_ms INTEGER NOT NULL,
              latest_user_span_ids_json TEXT NOT NULL,
              current_agent_span_ids_json TEXT NOT NULL,
              prior_boundary_span_ids_json TEXT NOT NULL,
              salient_user_goal_sample TEXT,
              salient_user_goal_hash TEXT,
              salient_agent_state_sample TEXT,
              salient_agent_state_hash TEXT,
              prior_boundary_sample TEXT,
              prior_boundary_hash TEXT,
              sampling_confidence REAL NOT NULL,
              missing_roles_json TEXT NOT NULL,
              fallback_flags_json TEXT NOT NULL
            );
            CREATE TABLE continue_task_actions (
              id TEXT PRIMARY KEY,
              task_turn_id TEXT,
              action_kind TEXT,
              semantic_delta_kind TEXT,
              semantic_after_hint TEXT,
              evidence_event_ids_json TEXT,
              created_at_ms INTEGER
            );
            CREATE TABLE frames (
              id INTEGER PRIMARY KEY,
              capture_trigger TEXT
            );
            ",
        )
        .unwrap();
        conn
    }

    fn insert_evidence(
        conn: &Connection,
        frame: &str,
        at: i64,
        user: Option<&str>,
        agent: Option<&str>,
        prior: Option<&str>,
        privacy: &str,
    ) {
        let user_id = user.map(|_| format!("user-{frame}"));
        let agent_id = agent.map(|_| format!("agent-{frame}"));
        let prior_id = prior.map(|_| format!("prior-{frame}"));
        for (id, text) in [
            user_id.as_deref().zip(user),
            agent_id.as_deref().zip(agent),
            prior_id.as_deref().zip(prior),
        ]
        .into_iter()
        .flatten()
        {
            conn.execute(
                "INSERT INTO continue_ordered_evidence_spans VALUES (?1,?2,?3,?4,?5)",
                params![id, frame, format!("source-{id}"), text_hash(text), privacy],
            )
            .unwrap();
        }
        let ids = |id: Option<String>| {
            serde_json::to_string(&id.into_iter().collect::<Vec<_>>()).unwrap()
        };
        conn.execute(
            "INSERT INTO continue_salient_turn_evidence VALUES (
               ?1,'session','surface','artifact',?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,
               0.92,'[]','[]')",
            params![
                frame,
                at,
                ids(user_id),
                ids(agent_id),
                ids(prior_id),
                user,
                user.map(text_hash),
                agent,
                agent.map(text_hash),
                prior,
                prior.map(text_hash),
            ],
        )
        .unwrap();
    }

    fn selected(conn: &Connection) -> CurrentTaskTurn {
        selected_current_task_turn(conn).unwrap().unwrap()
    }

    #[test]
    fn new_task_after_completed_task_is_current() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            None,
            None,
            Some("The Continue-card copy update is complete"),
            "normal",
        );
        insert_evidence(
            &conn,
            "2",
            200,
            Some("Investigate why the island Capture button does not work"),
            Some("I will trace the Swift and Rust code paths now"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into(), "2".into()]).unwrap();
        let turn = selected(&conn);
        assert_eq!(turn.relation_to_prior, TaskTurnRelation::NewTask);
        assert_eq!(turn.execution_state, TaskExecutionState::Active);
        assert_eq!(turn.task_object.as_deref(), Some("island_capture_button"));
        assert!(turn.prior_task_turn_id.is_none());
        let slots = current_task_turn_accuracy_slots(&conn).unwrap();
        assert_eq!(slots["task_summary"], json!("capture_button_investigation"));
        assert_eq!(slots["prior_task"], Value::Null);
    }

    #[test]
    fn prior_boundary_only_is_history_not_a_current_task() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            None,
            None,
            Some("The Continue-card copy update is complete"),
            "normal",
        );

        let resolution = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();

        assert!(resolution.selected_task_turn_id.is_none());
        assert_eq!(
            resolution.missing_evidence,
            vec!["current_user_goal_evidence".to_string()]
        );
        assert!(selected_current_task_turn(&conn).unwrap().is_none());
        assert_eq!(latest_task_turn_marker(&conn).unwrap(), (None, None, None));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_task_turns", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn prior_only_evidence_clears_current_selection_without_reusing_prior_identity() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Implement the Capture button"),
            Some("I am tracing the handler"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let original = selected(&conn);
        insert_evidence(
            &conn,
            "2",
            200,
            None,
            None,
            Some("The old task is complete"),
            "normal",
        );

        let resolution = resolve_provisional_task_turns(&conn, &["2".into()]).unwrap();

        assert!(resolution.selected_task_turn_id.is_none());
        assert!(selected_current_task_turn(&conn).unwrap().is_none());
        assert_eq!(latest_task_turn_marker(&conn).unwrap(), (None, None, None));
        let ids: Vec<String> = conn
            .prepare("SELECT task_turn_id FROM continue_task_turns ORDER BY task_turn_id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(ids, vec![original.task_turn_id]);
    }

    #[test]
    fn selected_accessor_never_falls_back_to_unselected_history() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Implement the Capture button"),
            Some("I am tracing the handler"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        conn.execute("UPDATE continue_task_turns SET selected = 0", [])
            .unwrap();

        assert!(selected_current_task_turn(&conn).unwrap().is_none());
        assert_eq!(latest_task_turn_marker(&conn).unwrap(), (None, None, None));
    }

    #[test]
    fn clearing_selected_frame_does_not_promote_older_history() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Implement the settings panel"),
            Some("I am editing the panel"),
            None,
            "normal",
        );
        insert_evidence(
            &conn,
            "2",
            200,
            Some("Investigate the Capture button"),
            Some("I am tracing it"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into(), "2".into()]).unwrap();

        clear_task_turns_for_frames(&conn, &["2".into()]).unwrap();

        assert!(selected_current_task_turn(&conn).unwrap().is_none());
        assert_eq!(latest_task_turn_marker(&conn).unwrap(), (None, None, None));
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_task_turns", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(remaining, 1);
    }

    #[test]
    fn prior_spans_link_only_to_history_relation_fields() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Investigate the Capture button"),
            Some("I am tracing it"),
            Some("The earlier settings task is complete"),
            "normal",
        );

        let resolution = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let turn = selected(&conn);
        assert_eq!(turn.schema_version, CURRENT_TASK_TURN_SCHEMA_V2);
        assert!(!resolution.frame_scopes["1"]
            .current_source_record_ids
            .contains(&"source-prior-1".to_string()));
        let prior_links: Vec<(String, String)> = conn
            .prepare(
                "SELECT field_name, evidence_role FROM continue_task_turn_evidence
                 WHERE source_span_id = 'prior-1' ORDER BY field_name, evidence_role",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            prior_links,
            vec![(
                "relation_to_prior".to_string(),
                "prior_boundary".to_string()
            )]
        );
    }

    #[test]
    fn surface_activity_without_semantic_spans_does_not_fabricate_a_turn() {
        let conn = fixture();
        insert_evidence(&conn, "1", 100, None, None, None, "normal");

        let resolution = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();

        assert!(resolution.selected_task_turn_id.is_none());
        assert!(resolution.frame_scopes.is_empty());
        assert!(selected_current_task_turn(&conn).unwrap().is_none());
    }

    #[test]
    fn lca_06_same_surface_manual_sample_preserves_only_the_prior_causal_turn() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Repair the live Continue runtime"),
            Some("I am working on the repair"),
            None,
            "normal",
        );
        let first = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let original = first.selected_task_turn_id.unwrap();

        insert_evidence(&conn, "2", 200, None, None, None, "normal");
        conn.execute(
            "INSERT INTO frames (id, capture_trigger) VALUES (2, 'manual_continue_boundary')",
            [],
        )
        .unwrap();
        let second = resolve_provisional_task_turns(&conn, &["2".into()]).unwrap();
        assert_eq!(
            second.selected_task_turn_id.as_deref(),
            Some(original.as_str())
        );
        assert_eq!(
            second.selection_reason_codes,
            vec!["same_surface_empty_sample_preserved".to_string()]
        );
    }

    #[test]
    fn lca_06_wrong_surface_manual_sample_does_not_reuse_the_selected_turn() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Repair the live Continue runtime"),
            Some("I am working on the repair"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();

        insert_evidence(&conn, "2", 200, None, None, None, "normal");
        conn.execute(
            "UPDATE continue_salient_turn_evidence SET surface_key='other-surface' WHERE frame_id='2'",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO frames (id, capture_trigger) VALUES (2, 'manual_continue_boundary')",
            [],
        )
        .unwrap();
        let resolution = resolve_provisional_task_turns(&conn, &["2".into()]).unwrap();
        assert!(resolution.selected_task_turn_id.is_none());
        assert!(selected_current_task_turn(&conn).unwrap().is_none());
    }

    #[test]
    fn clarification_is_distinct_from_new_task() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Update the Continue card copy"),
            Some("The implementation is complete"),
            None,
            "normal",
        );
        insert_evidence(
            &conn,
            "2",
            200,
            Some("Can you explain why that Continue card copy changed?"),
            Some("I will explain the reasoning"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into(), "2".into()]).unwrap();
        assert_eq!(
            selected(&conn).relation_to_prior,
            TaskTurnRelation::Clarification
        );
    }

    #[test]
    fn unfinished_prior_is_superseded_by_unrelated_goal() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Implement the settings panel"),
            Some("I am editing the panel"),
            None,
            "normal",
        );
        insert_evidence(
            &conn,
            "2",
            200,
            Some("Investigate the Capture button"),
            Some("I will trace it"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into(), "2".into()]).unwrap();
        let turn = selected(&conn);
        assert_eq!(turn.relation_to_prior, TaskTurnRelation::Supersedes);
        assert_eq!(
            load_task_turn(&conn, turn.prior_task_turn_id.as_deref().unwrap())
                .unwrap()
                .unwrap()
                .execution_state,
            TaskExecutionState::Superseded
        );
    }

    #[test]
    fn agent_working_status_is_active_not_completed() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Investigate the Capture button"),
            Some("I am tracing the Swift and Rust code now"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let turn = selected(&conn);
        assert_eq!(turn.execution_state, TaskExecutionState::Active);
        assert_eq!(turn.current_actor, TaskTurnActor::AssistantOrAgent);
        assert_eq!(turn.waiting_on, TaskTurnWaitingOn::Agent);
    }

    #[test]
    fn completion_cue_is_scoped_to_matching_turn() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            None,
            None,
            Some("Verification passed for the Continue card copy"),
            "normal",
        );
        insert_evidence(
            &conn,
            "2",
            200,
            Some("Investigate the Capture button"),
            Some("I am tracing code"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into(), "2".into()]).unwrap();
        assert_eq!(selected(&conn).execution_state, TaskExecutionState::Active);
    }

    #[test]
    fn side_panel_terminal_completion_does_not_complete_chat_turn() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Investigate the Capture button"),
            Some("I am tracing code"),
            Some("Old terminal: all tests passed"),
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        assert_eq!(selected(&conn).execution_state, TaskExecutionState::Active);
    }

    #[test]
    fn child_support_step_keeps_parent_selected() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Investigate the Capture button"),
            Some("I am tracing code"),
            None,
            "normal",
        );
        let first = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let parent = first.selected_task_turn_id.unwrap();
        insert_evidence(
            &conn,
            "2",
            200,
            Some("Check docs for the button API"),
            Some("I am searching the docs"),
            None,
            "normal",
        );
        let second = resolve_provisional_task_turns(&conn, &["2".into()]).unwrap();
        assert_eq!(
            second.selected_task_turn_id.as_deref(),
            Some(parent.as_str())
        );
        assert_eq!(
            second.frame_scopes["2"].relation_to_prior,
            "child_support_step"
        );
    }

    #[test]
    fn same_task_reobserved_after_support_detour_is_continuation() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Update the Continue-card copy"),
            Some("The update is complete"),
            None,
            "normal",
        );
        let goal = "What does the island Capture button do?";
        let state = "I will trace the Swift bridge and Rust handler";
        insert_evidence(&conn, "2", 200, Some(goal), Some(state), None, "normal");
        insert_evidence(&conn, "3", 300, Some(goal), Some(state), None, "normal");
        let first =
            resolve_provisional_task_turns(&conn, &["1".into(), "2".into(), "3".into()]).unwrap();
        assert_eq!(first.frame_scopes["3"].relation_to_prior, "continuation");
        let current = selected(&conn);
        assert_eq!(current.relation_to_prior, TaskTurnRelation::Continuation);
        assert_ne!(
            current.prior_task_turn_id.as_deref(),
            Some(current.task_turn_id.as_str())
        );
        let prior = load_task_turn(&conn, current.prior_task_turn_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(prior.execution_state, TaskExecutionState::Completed);
        assert_eq!(task_summary_slug(&prior), "continue_card_copy_update");
        let marker = latest_task_turn_marker(&conn).unwrap();
        resolve_provisional_task_turns(&conn, &["1".into(), "2".into(), "3".into()]).unwrap();
        assert_eq!(marker, latest_task_turn_marker(&conn).unwrap());
    }

    #[test]
    fn finalizer_never_crosses_task_turn_boundary() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Investigate the Capture button"),
            Some("I am tracing code"),
            None,
            "normal",
        );
        let resolution = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let current = resolution.selected_task_turn_id.unwrap();
        conn.execute(
            "INSERT INTO continue_task_actions VALUES ('old','other','reviewing_output',
             'completed_successfully','Verification passed','[]',200)",
            [],
        )
        .unwrap();
        finalize_task_turns(&conn, &[current]).unwrap();
        assert_eq!(selected(&conn).execution_state, TaskExecutionState::Active);
    }

    #[test]
    fn deterministic_ids_revisions_and_markers_are_idempotent() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Investigate the Capture button"),
            Some("I am tracing code"),
            None,
            "normal",
        );
        let first = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let marker = latest_task_turn_marker(&conn).unwrap();
        let second = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        assert_eq!(first.selected_task_turn_id, second.selected_task_turn_id);
        assert_eq!(marker, latest_task_turn_marker(&conn).unwrap());
    }

    #[test]
    fn old_database_without_task_turn_table_is_safe() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(selected_current_task_turn(&conn).unwrap().is_none());
        assert_eq!(latest_task_turn_marker(&conn).unwrap(), (None, None, None));
        assert!(task_turn_audit_json(&conn, 10).unwrap()["turns"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn privacy_safe_summaries_keep_only_hashes_and_links() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Implement secret Capture button token"),
            Some("I am tracing code"),
            None,
            "sensitive",
        );
        resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let turn = selected(&conn);
        assert!(turn.latest_user_goal_summary.is_none());
        assert_eq!(turn.latest_user_goal_redaction_status, "redacted_or_unsafe");
        assert!(turn.latest_user_goal_hash.is_some());
        let links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM continue_task_turn_evidence",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(links > 0);
    }

    #[test]
    fn historical_actions_cannot_bootstrap_provisional_boundary() {
        let conn = fixture();
        conn.execute(
            "INSERT INTO continue_task_actions VALUES ('historical','fake-turn',
             'reviewing_output','completed_successfully','Verification passed','[]',50)",
            [],
        )
        .unwrap();
        insert_evidence(&conn, "1", 100, None, None, None, "normal");
        let resolution = resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        assert_ne!(
            resolution.selected_task_turn_id.as_deref(),
            Some("fake-turn")
        );
        assert!(resolution.selected_task_turn_id.is_none());
        assert!(selected_current_task_turn(&conn).unwrap().is_none());
    }

    #[test]
    fn lifecycle_axes_round_trip_independently() {
        let conn = fixture();
        insert_evidence(
            &conn,
            "1",
            100,
            Some("Implement the Capture button"),
            Some("Blocked by an external signing service"),
            None,
            "normal",
        );
        resolve_provisional_task_turns(&conn, &["1".into()]).unwrap();
        let turn = selected(&conn);
        assert_eq!(turn.execution_state, TaskExecutionState::Blocked);
        assert_eq!(turn.current_actor, TaskTurnActor::AssistantOrAgent);
        assert_eq!(turn.waiting_on, TaskTurnWaitingOn::External);
    }
}
