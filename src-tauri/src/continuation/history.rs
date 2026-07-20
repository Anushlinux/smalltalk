use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{ContinueDecisionRequest, ContinueDecisionResult};

pub(crate) const CONTINUE_HISTORY_OUTPUT_SCHEMA_V1: &str = "smalltalk.continue_history_output.v1";
pub(crate) const CONTINUE_HISTORY_PAGE_SCHEMA_V1: &str = "smalltalk.continue_history_page.v1";

const HISTORY_TABLE: &str = "continue_answer_history";
const HISTORY_SCHEMA_VERSION: i64 = 1;
const DEFAULT_PAGE_SIZE: usize = 25;
const MAX_PAGE_SIZE: usize = 50;
const MAX_HISTORY_ENTRIES: i64 = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinueHistoryCursorV1 {
    pub created_at_ms: i64,
    pub decision_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinueHistoryAnswerRowV1 {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinueHistorySummaryV1 {
    pub decision_id: String,
    pub created_at_ms: i64,
    pub origin: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinueHistoryPageV1 {
    pub schema: String,
    pub items: Vec<ContinueHistorySummaryV1>,
    pub next_cursor: Option<ContinueHistoryCursorV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContinueHistoryOutputV1 {
    pub schema: String,
    pub decision_id: String,
    pub created_at_ms: i64,
    pub origin: String,
    pub title: String,
    pub rows: Vec<ContinueHistoryAnswerRowV1>,
}

pub(crate) fn ensure_continue_history_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS continue_answer_history (
          decision_id TEXT PRIMARY KEY,
          schema_version INTEGER NOT NULL,
          created_at_ms INTEGER NOT NULL,
          origin TEXT NOT NULL CHECK(origin IN ('island', 'main_app')),
          title TEXT NOT NULL,
          rows_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_continue_answer_history_newest
          ON continue_answer_history(created_at_ms DESC, decision_id DESC);
        ",
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn record_explicit_continue_output(
    conn: &Connection,
    request: &ContinueDecisionRequest,
    created_at_ms: i64,
    result: &ContinueDecisionResult,
) -> Result<bool, String> {
    let Some(origin) = explicit_request_origin(request) else {
        return Ok(false);
    };
    if result.decision_id.trim().is_empty() {
        return Ok(false);
    }
    if crate::session_island::continue_decision_is_failed_empty_refresh(result) {
        return Ok(false);
    }
    let output = product_facing_output(result, created_at_ms, origin);
    persist_continue_history_output(conn, &output)?;
    Ok(true)
}

pub(crate) fn list_continue_history(
    conn: &Connection,
    cursor: Option<&ContinueHistoryCursorV1>,
    limit: Option<usize>,
) -> Result<ContinueHistoryPageV1, String> {
    if !has_continue_history_schema(conn)? {
        return Ok(ContinueHistoryPageV1 {
            schema: CONTINUE_HISTORY_PAGE_SCHEMA_V1.to_string(),
            items: Vec::new(),
            next_cursor: None,
        });
    }
    let page_size = limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let query_limit = i64::try_from(page_size + 1).map_err(|error| error.to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT decision_id, created_at_ms, origin, title
             FROM continue_answer_history
             WHERE schema_version = ?1
               AND ((?2 IS NULL)
                OR created_at_ms < ?2
                OR (created_at_ms = ?2 AND decision_id < ?3))
             ORDER BY created_at_ms DESC, decision_id DESC
             LIMIT ?4",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            params![
                HISTORY_SCHEMA_VERSION,
                cursor.map(|cursor| cursor.created_at_ms),
                cursor.map(|cursor| cursor.decision_id.as_str()),
                query_limit
            ],
            |row| {
                Ok(ContinueHistorySummaryV1 {
                    decision_id: row.get(0)?,
                    created_at_ms: row.get(1)?,
                    origin: row.get(2)?,
                    title: row.get(3)?,
                })
            },
        )
        .map_err(|error| error.to_string())?;
    let mut items = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let has_more = items.len() > page_size;
    if has_more {
        items.pop();
    }
    let next_cursor = has_more.then(|| {
        let last = items
            .last()
            .expect("a positive page size always retains an item");
        ContinueHistoryCursorV1 {
            created_at_ms: last.created_at_ms,
            decision_id: last.decision_id.clone(),
        }
    });
    Ok(ContinueHistoryPageV1 {
        schema: CONTINUE_HISTORY_PAGE_SCHEMA_V1.to_string(),
        items,
        next_cursor,
    })
}

pub(crate) fn get_continue_history_output(
    conn: &Connection,
    decision_id: &str,
) -> Result<Option<ContinueHistoryOutputV1>, String> {
    if !has_continue_history_schema(conn)? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT created_at_ms, origin, title, rows_json
         FROM continue_answer_history
         WHERE decision_id = ?1 AND schema_version = ?2",
        params![decision_id, HISTORY_SCHEMA_VERSION],
        |row| {
            let rows_json: String = row.get(3)?;
            let rows = serde_json::from_str(&rows_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(ContinueHistoryOutputV1 {
                schema: CONTINUE_HISTORY_OUTPUT_SCHEMA_V1.to_string(),
                decision_id: decision_id.to_string(),
                created_at_ms: row.get(0)?,
                origin: row.get(1)?,
                title: row.get(2)?,
                rows,
            })
        },
    )
    .optional()
    .map_err(|error| error.to_string())
}

pub(crate) fn clear_continue_history(conn: &Connection) -> Result<(), String> {
    if !has_continue_history_schema(conn)? {
        return Ok(());
    }
    conn.execute("DELETE FROM continue_answer_history", [])
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn explicit_request_origin(request: &ContinueDecisionRequest) -> Option<&'static str> {
    let is_manual = request.request_trigger.as_deref() == Some("manual")
        || request.island_trigger_reason.as_deref() == Some("user_pressed_continue");
    if !is_manual {
        return None;
    }
    let is_island = request
        .island_source
        .as_deref()
        .is_some_and(|source| !source.trim().is_empty())
        || request.island_trigger_reason.as_deref() == Some("user_pressed_continue");
    Some(if is_island { "island" } else { "main_app" })
}

fn product_facing_output(
    result: &ContinueDecisionResult,
    created_at_ms: i64,
    origin: &str,
) -> ContinueHistoryOutputV1 {
    let state = crate::session_island::island_state_from_continue_decision(
        result,
        crate::session_island::IslandFreshness {
            evidence_watermark_ms: None,
            newest_evidence_ms: None,
            decision_updated_at_ms: Some(created_at_ms),
            decision_stale: false,
        },
        crate::session_island::IslandStateContext {
            local_memory_running: true,
            has_local_memory: true,
        },
    );
    let answer = state.semantic_answer.as_ref();
    let title = answer
        .and_then(semantic_answer_title)
        .unwrap_or_else(|| "Couldn’t recover the task".to_string());
    let mut rows = Vec::new();
    append_row(
        &mut rows,
        "Task object",
        answer.and_then(|answer| answer.task_object.as_deref()),
    );
    append_row(
        &mut rows,
        "Current activity — observed surface",
        answer.and_then(|answer| answer.current_activity.observed_surface.as_deref()),
    );
    append_row(
        &mut rows,
        "Current activity — immediate operation",
        answer.and_then(|answer| answer.current_activity.immediate_user_operation.as_deref()),
    );
    append_row(
        &mut rows,
        "Current activity — operation effect",
        answer.and_then(|answer| {
            answer
                .current_activity
                .semantic_effect_of_operation
                .as_deref()
        }),
    );
    append_row(
        &mut rows,
        "Current activity — current subtask",
        answer.and_then(|answer| answer.current_activity.current_subtask.as_deref()),
    );
    append_row(
        &mut rows,
        "Current activity — relationship to primary",
        answer.map(|answer| answer.current_activity.relationship_to_primary.as_str()),
    );
    append_row(
        &mut rows,
        "Last meaningful progress",
        answer.and_then(|answer| answer.last_meaningful_progress.as_deref()),
    );
    append_row(
        &mut rows,
        "Unfinished state",
        answer.and_then(|answer| answer.unfinished_state.as_deref()),
    );
    append_row(
        &mut rows,
        "Next action",
        answer
            .and_then(|answer| answer.next_action.as_deref())
            .or(state.next_action.as_deref()),
    );
    append_row(
        &mut rows,
        "Where summary",
        answer.and_then(|answer| answer.where_summary.as_deref()),
    );
    ContinueHistoryOutputV1 {
        schema: CONTINUE_HISTORY_OUTPUT_SCHEMA_V1.to_string(),
        decision_id: result.decision_id.clone(),
        created_at_ms,
        origin: origin.to_string(),
        title,
        rows,
    }
}

fn append_row(rows: &mut Vec<ContinueHistoryAnswerRowV1>, label: &str, value: Option<&str>) {
    let Some(value) = nonempty(value) else {
        return;
    };
    rows.push(ContinueHistoryAnswerRowV1 {
        label: label.to_string(),
        value,
    });
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn semantic_answer_title(
    answer: &crate::continuation::task_truth_v2::production::TaskTruthPublicAnswerV1,
) -> Option<String> {
    [
        answer.task_summary.as_deref(),
        answer.current_subtask.as_deref(),
        answer.current_activity.current_subtask.as_deref(),
        answer.next_action.as_deref(),
        answer.unfinished_state.as_deref(),
        answer.last_meaningful_progress.as_deref(),
    ]
    .into_iter()
    .find_map(nonempty)
}

fn persist_continue_history_output(
    conn: &Connection,
    output: &ContinueHistoryOutputV1,
) -> Result<(), String> {
    ensure_continue_history_schema(conn)?;
    let rows_json = serde_json::to_string(&output.rows).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO continue_answer_history (
           decision_id, schema_version, created_at_ms, origin, title, rows_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            output.decision_id,
            HISTORY_SCHEMA_VERSION,
            output.created_at_ms,
            output.origin,
            output.title,
            rows_json,
        ],
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "DELETE FROM continue_answer_history
         WHERE decision_id IN (
           SELECT decision_id FROM continue_answer_history
           ORDER BY created_at_ms DESC, decision_id DESC
           LIMIT -1 OFFSET ?1
         )",
        params![MAX_HISTORY_ENTRIES],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn has_continue_history_schema(conn: &Connection) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
         )",
        params![HISTORY_TABLE],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(id: &str, created_at_ms: i64, title: &str) -> ContinueHistoryOutputV1 {
        ContinueHistoryOutputV1 {
            schema: CONTINUE_HISTORY_OUTPUT_SCHEMA_V1.to_string(),
            decision_id: id.to_string(),
            created_at_ms,
            origin: "main_app".to_string(),
            title: title.to_string(),
            rows: vec![ContinueHistoryAnswerRowV1 {
                label: "Next action".to_string(),
                value: format!("Continue {id}"),
            }],
        }
    }

    #[test]
    fn schema_migration_is_idempotent_and_versioned() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_continue_history_schema(&conn).unwrap();
        ensure_continue_history_schema(&conn).unwrap();
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(continue_answer_history)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            columns,
            vec![
                "decision_id",
                "schema_version",
                "created_at_ms",
                "origin",
                "title",
                "rows_json"
            ]
        );
    }

    #[test]
    fn reads_are_query_only_and_empty_before_migration() {
        let conn = Connection::open_in_memory().unwrap();
        let schema_version_before: i64 = conn
            .query_row("PRAGMA schema_version", [], |row| row.get(0))
            .unwrap();

        assert!(list_continue_history(&conn, None, None)
            .unwrap()
            .items
            .is_empty());
        assert!(get_continue_history_output(&conn, "missing")
            .unwrap()
            .is_none());
        let schema_version_after: i64 = conn
            .query_row("PRAGMA schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(schema_version_after, schema_version_before);
    }

    #[test]
    fn explicit_request_filter_and_origin_classification_are_strict() {
        let mut request = ContinueDecisionRequest::default();
        request.request_trigger = Some("background".into());
        request.island_source = Some("island_primary".into());
        assert_eq!(explicit_request_origin(&request), None);
        request.request_trigger = Some("manual".into());
        assert_eq!(explicit_request_origin(&request), Some("island"));
        request.island_source = None;
        assert_eq!(explicit_request_origin(&request), Some("main_app"));
        request.request_trigger = Some("island".into());
        request.island_trigger_reason = Some("user_pressed_continue".into());
        assert_eq!(explicit_request_origin(&request), Some("island"));
        request.request_trigger = Some("startup".into());
        request.island_trigger_reason = Some("evidence_changed".into());
        assert_eq!(explicit_request_origin(&request), None);
    }

    #[test]
    fn partial_semantic_answer_uses_admitted_current_step_as_history_title() {
        let answer = crate::continuation::task_truth_v2::production::TaskTruthPublicAnswerV1 {
            task_summary: None,
            current_subtask: Some("Review the admitted Continue response".into()),
            unfinished_state: Some("The exact primary task remains uncertain".into()),
            ..Default::default()
        };

        assert_eq!(
            semantic_answer_title(&answer).as_deref(),
            Some("Review the admitted Continue response")
        );
    }

    #[test]
    fn stored_output_is_immutable_and_deduplicated_by_decision_id() {
        let conn = Connection::open_in_memory().unwrap();
        persist_continue_history_output(&conn, &output("same", 10, "Original title")).unwrap();
        persist_continue_history_output(&conn, &output("same", 20, "Replacement title")).unwrap();
        let stored = get_continue_history_output(&conn, "same").unwrap().unwrap();
        assert_eq!(stored.created_at_ms, 10);
        assert_eq!(stored.title, "Original title");
    }

    #[test]
    fn cursor_pages_are_newest_first_and_limit_is_bounded() {
        let conn = Connection::open_in_memory().unwrap();
        for index in 0..55 {
            persist_continue_history_output(
                &conn,
                &output(
                    &format!("decision-{index:02}"),
                    index,
                    &format!("Title {index}"),
                ),
            )
            .unwrap();
        }
        let first = list_continue_history(&conn, None, Some(200)).unwrap();
        assert_eq!(first.items.len(), 50);
        assert_eq!(first.items[0].decision_id, "decision-54");
        assert_eq!(first.items[49].decision_id, "decision-05");
        let second = list_continue_history(&conn, first.next_cursor.as_ref(), None).unwrap();
        assert_eq!(second.items.len(), 5);
        assert_eq!(second.items[0].decision_id, "decision-04");
        assert_eq!(second.items[4].decision_id, "decision-00");
        assert!(second.next_cursor.is_none());
    }

    #[test]
    fn equal_timestamps_page_without_duplicates_or_gaps() {
        let conn = Connection::open_in_memory().unwrap();
        for id in ["a", "b", "c"] {
            persist_continue_history_output(&conn, &output(id, 10, id)).unwrap();
        }
        let first = list_continue_history(&conn, None, Some(2)).unwrap();
        assert_eq!(
            first
                .items
                .iter()
                .map(|item| item.decision_id.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "b"]
        );
        let second = list_continue_history(&conn, first.next_cursor.as_ref(), Some(2)).unwrap();
        assert_eq!(second.items[0].decision_id, "a");
    }

    #[test]
    fn retention_keeps_only_newest_one_hundred_outputs() {
        let conn = Connection::open_in_memory().unwrap();
        for index in 0..105 {
            persist_continue_history_output(
                &conn,
                &output(&format!("decision-{index:03}"), index, "Saved answer"),
            )
            .unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_answer_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 100);
        assert!(get_continue_history_output(&conn, "decision-004")
            .unwrap()
            .is_none());
        assert!(get_continue_history_output(&conn, "decision-005")
            .unwrap()
            .is_some());
    }

    #[test]
    fn unresolved_product_copy_and_ordered_rows_round_trip_exactly() {
        let conn = Connection::open_in_memory().unwrap();
        let unresolved = ContinueHistoryOutputV1 {
            schema: CONTINUE_HISTORY_OUTPUT_SCHEMA_V1.to_string(),
            decision_id: "unresolved".into(),
            created_at_ms: 99,
            origin: "island".into(),
            title: "Couldn’t recover the task".into(),
            rows: vec![
                ContinueHistoryAnswerRowV1 {
                    label: "Next action".into(),
                    value: "Continue working until clearer evidence is available.".into(),
                },
                ContinueHistoryAnswerRowV1 {
                    label: "Where summary".into(),
                    value: "Current location is unclear.".into(),
                },
            ],
        };
        persist_continue_history_output(&conn, &unresolved).unwrap();
        assert_eq!(
            get_continue_history_output(&conn, "unresolved")
                .unwrap()
                .unwrap(),
            unresolved
        );
    }

    #[test]
    fn persisted_shape_contains_only_product_facing_fields_and_clears() {
        let conn = Connection::open_in_memory().unwrap();
        persist_continue_history_output(&conn, &output("private-check", 1, "Safe title")).unwrap();
        let stored = get_continue_history_output(&conn, "private-check")
            .unwrap()
            .unwrap();
        let stored_json = serde_json::to_value(&stored).unwrap();
        let stored_fields = stored_json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            stored_fields,
            std::collections::BTreeSet::from([
                "created_at_ms",
                "decision_id",
                "origin",
                "rows",
                "schema",
                "title",
            ])
        );
        let row_fields = stored_json["rows"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            row_fields,
            std::collections::BTreeSet::from(["label", "value"])
        );
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name = 'continue_answer_history'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        for private_field in [
            "browser_url",
            "document_path",
            "evidence_id",
            "provider",
            "response_id",
            "screenshot_path",
            "typed_text",
            "clipboard",
        ] {
            assert!(
                !sql.contains(private_field) && !stored_json.to_string().contains(private_field),
                "private field {private_field} leaked into the history schema or saved output"
            );
        }
        clear_continue_history(&conn).unwrap();
        assert!(list_continue_history(&conn, None, None)
            .unwrap()
            .items
            .is_empty());
    }
}
