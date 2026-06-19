use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const ACCESSIBILITY_SCRIPT: &str = r#"
on replaceText(sourceText, oldText, newText)
  set oldDelims to AppleScript's text item delimiters
  set AppleScript's text item delimiters to oldText
  set parts to every text item of sourceText
  set AppleScript's text item delimiters to newText
  set joinedText to parts as text
  set AppleScript's text item delimiters to oldDelims
  return joinedText
end replaceText

on cleanText(rawValue)
  try
    if rawValue is missing value then return ""
    set cleaned to rawValue as text
  on error
    return ""
  end try
  set cleaned to my replaceText(cleaned, return, " ")
  set cleaned to my replaceText(cleaned, linefeed, " ")
  set cleaned to my replaceText(cleaned, tab, " ")
  return cleaned
end cleanText

on appendPiece(pieceList, rawValue)
  set cleaned to my cleanText(rawValue)
  if cleaned is "" then return pieceList
  if pieceList does not contain cleaned then set end of pieceList to cleaned
  return pieceList
end appendPiece

on joinPieces(pieceList)
  if (count of pieceList) is 0 then return ""
  set oldDelims to AppleScript's text item delimiters
  set AppleScript's text item delimiters to " "
  set joinedText to pieceList as text
  set AppleScript's text item delimiters to oldDelims
  return joinedText
end joinPieces

on collectElement(theElement, depth)
  if depth > 6 then return ""
  set outputText to ""
  try
    set roleText to ""
    try
      set roleText to role of theElement as text
    end try

    set pieces to {}
    try
      set pieces to my appendPiece(pieces, name of theElement)
    end try
    try
      set pieces to my appendPiece(pieces, value of theElement)
    end try
    try
      set pieces to my appendPiece(pieces, description of theElement)
    end try

    set nodeText to my joinPieces(pieces)
    if nodeText is not "" then
      set outputText to outputText & "NODE" & tab & (depth as text) & tab & my cleanText(roleText) & tab & nodeText & linefeed
    end if

    if depth < 6 then
      try
        set childElements to UI elements of theElement
        repeat with childElement in childElements
          set outputText to outputText & my collectElement(childElement, depth + 1)
        end repeat
      end try
    end if
  end try
  return outputText
end collectElement

on getBrowserUrl(appName)
  set foundUrl to ""
  try
    ignoring case
      if appName contains "Safari" then
        tell application "Safari" to set foundUrl to URL of front document
      else if appName contains "Google Chrome" then
        tell application "Google Chrome" to set foundUrl to URL of active tab of front window
      else if appName contains "Chrome" then
        tell application "Google Chrome" to set foundUrl to URL of active tab of front window
      else if appName contains "Brave" then
        tell application "Brave Browser" to set foundUrl to URL of active tab of front window
      else if appName contains "Microsoft Edge" then
        tell application "Microsoft Edge" to set foundUrl to URL of active tab of front window
      else if appName contains "Arc" then
        tell application "Arc" to set foundUrl to URL of active tab of front window
      else if appName contains "Chromium" then
        tell application "Chromium" to set foundUrl to URL of active tab of front window
      else if appName contains "Vivaldi" then
        tell application "Vivaldi" to set foundUrl to URL of active tab of front window
      else if appName contains "Opera" then
        tell application "Opera" to set foundUrl to URL of active tab of front window
      end if
    end ignoring
  end try
  return my cleanText(foundUrl)
end getBrowserUrl

tell application "System Events"
  set frontProc to first application process whose frontmost is true
  set appName to my cleanText(name of frontProc)
  set windowName to ""
  set documentValue to ""

  try
    set windowName to my cleanText(name of front window of frontProc)
  end try

  try
    set documentValue to my cleanText(value of attribute "AXDocument" of front window of frontProc)
  end try

  set browserUrl to my getBrowserUrl(appName)
  if browserUrl is "" then
    if documentValue starts with "http://" or documentValue starts with "https://" then set browserUrl to documentValue
  end if

  set outputText to "APP" & tab & appName & linefeed
  set outputText to outputText & "WINDOW" & tab & windowName & linefeed
  set outputText to outputText & "BROWSER_URL" & tab & browserUrl & linefeed
  set outputText to outputText & "DOCUMENT" & tab & documentValue & linefeed

  try
    set outputText to outputText & my collectElement(front window of frontProc, 0)
  end try

  return outputText
end tell
"#;

const VISION_OCR_SWIFT: &str = include_str!("../scripts/vision_ocr.swift");

#[derive(Debug, Clone)]
pub struct CaptureState {
    inner: Arc<Mutex<CaptureRuntime>>,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CaptureRuntime::default())),
        }
    }
}

#[derive(Debug, Default)]
struct CaptureRuntime {
    running: bool,
    stop_signal: Option<Arc<AtomicBool>>,
    worker: Option<JoinHandle<()>>,
    last_error: Option<String>,
    last_frame: Option<CaptureFrame>,
    started_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureStatus {
    pub running: bool,
    pub frame_count: i64,
    pub started_at: Option<i64>,
    pub last_error: Option<String>,
    pub latest_frame: Option<CaptureFrame>,
    pub data_dir: String,
    pub database_path: String,
    pub screenshot_tool: bool,
    pub accessibility_tool: bool,
    pub ocr_tool: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureFrame {
    pub id: i64,
    pub captured_at: i64,
    pub snapshot_path: String,
    pub app_name: Option<String>,
    pub window_name: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub focused: bool,
    pub capture_trigger: String,
    pub text_source: Option<String>,
    pub accessibility_text: Option<String>,
    pub accessibility_tree_json: Option<String>,
    pub full_text: Option<String>,
    pub content_hash: Option<String>,
    pub image_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub frame: CaptureFrame,
    pub snippet: String,
    pub rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessibilityNode {
    role: String,
    text: String,
    depth: u8,
}

#[derive(Debug, Default)]
struct AccessibilityContext {
    app_name: Option<String>,
    window_name: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    text: String,
    nodes: Vec<AccessibilityNode>,
    error: Option<String>,
}

#[derive(Debug, Default)]
struct OcrOutput {
    text: String,
    text_json: String,
    engine: String,
    error: Option<String>,
}

#[derive(Debug)]
struct CapturePaths {
    root_dir: PathBuf,
    snapshot_dir: PathBuf,
    helper_dir: PathBuf,
    db_path: PathBuf,
}

#[derive(Debug)]
struct CaptureOutcome {
    frame: Option<CaptureFrame>,
    image_hash: String,
    content_hash: Option<String>,
}

#[tauri::command]
pub fn start_capture(app: AppHandle, state: State<CaptureState>) -> Result<CaptureStatus, String> {
    ensure_db(&app)?;

    {
        let mut runtime = lock_runtime(state.inner())?;
        if runtime.running {
            return capture_status(app, state);
        }

        let stop_signal = Arc::new(AtomicBool::new(false));
        let thread_stop = stop_signal.clone();
        let thread_app = app.clone();
        let thread_state = state.inner.clone();

        runtime.running = true;
        runtime.started_at = Some(now_millis());
        runtime.last_error = None;
        runtime.stop_signal = Some(stop_signal);
        runtime.worker = Some(thread::spawn(move || {
            capture_loop(thread_app, thread_state, thread_stop);
        }));
    }

    capture_status(app, state)
}

#[tauri::command]
pub fn stop_capture(app: AppHandle, state: State<CaptureState>) -> Result<CaptureStatus, String> {
    let handle = {
        let mut runtime = lock_runtime(state.inner())?;
        if let Some(stop_signal) = runtime.stop_signal.take() {
            stop_signal.store(true, Ordering::Relaxed);
        }
        runtime.running = false;
        runtime.started_at = None;
        runtime.worker.take()
    };

    if let Some(handle) = handle {
        let _ = handle.join();
    }

    capture_status(app, state)
}

#[tauri::command]
pub fn capture_once(app: AppHandle, state: State<CaptureState>) -> Result<CaptureFrame, String> {
    let outcome = capture_frame(&app, "manual", false, None, None)?;
    let frame = outcome
        .frame
        .ok_or_else(|| "capture was skipped before a frame was stored".to_string())?;

    {
        let mut runtime = lock_runtime(state.inner())?;
        runtime.last_error = None;
        runtime.last_frame = Some(frame.clone());
    }

    let _ = app.emit("capture-status", capture_status_snapshot(&app, &state));
    Ok(frame)
}

#[tauri::command]
pub fn capture_status(app: AppHandle, state: State<CaptureState>) -> Result<CaptureStatus, String> {
    capture_status_snapshot(&app, &state)
}

#[tauri::command]
pub fn search_captures(
    app: AppHandle,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(30).clamp(1, 100);
    let conn = open_db(&app)?;
    if query.trim().is_empty() {
        return latest_frames(&conn, limit);
    }

    let fts_query = fts_query(&query);
    if fts_query.is_empty() {
        return latest_frames(&conn, limit);
    }

    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.captured_at, f.snapshot_path, f.app_name, f.window_name,
                    f.browser_url, f.document_path, f.focused, f.capture_trigger,
                    f.text_source, f.accessibility_text, f.accessibility_tree_json,
                    f.full_text, f.content_hash, f.image_hash,
                    snippet(frames_fts, 0, '[', ']', ' ... ', 18) AS snippet,
                    bm25(frames_fts) AS rank
             FROM frames_fts
             JOIN frames f ON f.id = frames_fts.rowid
             WHERE frames_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )
        .map_err(to_string)?;

    let rows = stmt
        .query_map(params![fts_query, limit], |row| {
            Ok(SearchResult {
                frame: frame_from_row(row)?,
                snippet: row.get(15)?,
                rank: row.get(16)?,
            })
        })
        .map_err(to_string)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

#[tauri::command]
pub fn get_frame(app: AppHandle, frame_id: i64) -> Result<Option<CaptureFrame>, String> {
    let conn = open_db(&app)?;
    conn.query_row(
        "SELECT id, captured_at, snapshot_path, app_name, window_name,
                browser_url, document_path, focused, capture_trigger,
                text_source, accessibility_text, accessibility_tree_json,
                full_text, content_hash, image_hash
         FROM frames
         WHERE id = ?1",
        params![frame_id],
        frame_from_row,
    )
    .optional()
    .map_err(to_string)
}

#[tauri::command]
pub fn get_frame_image(app: AppHandle, frame_id: i64) -> Result<Option<String>, String> {
    let Some(frame) = get_frame(app, frame_id)? else {
        return Ok(None);
    };
    let bytes = fs::read(&frame.snapshot_path).map_err(to_string)?;
    Ok(Some(format!(
        "data:image/jpeg;base64,{}",
        base64_encode(&bytes)
    )))
}

fn capture_loop(app: AppHandle, state: Arc<Mutex<CaptureRuntime>>, stop_signal: Arc<AtomicBool>) {
    let mut last_visual_check = Instant::now()
        .checked_sub(Duration::from_secs(5))
        .unwrap_or_else(Instant::now);
    let mut last_idle_capture = Instant::now();
    let mut previous_image_hash: Option<String> = None;
    let mut previous_content_hash: Option<String> = None;

    match capture_frame(&app, "manual", false, None, None) {
        Ok(outcome) => {
            previous_image_hash = Some(outcome.image_hash);
            previous_content_hash = outcome.content_hash;
            if let Some(frame) = outcome.frame {
                update_success(&state, frame.clone());
                let _ = app.emit("capture-frame", frame);
            }
        }
        Err(error) => update_error(&state, error),
    }

    while !stop_signal.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(250));

        if last_visual_check.elapsed() >= Duration::from_secs(3) {
            last_visual_check = Instant::now();
            match capture_frame(
                &app,
                "visual_change",
                true,
                previous_image_hash.as_deref(),
                previous_content_hash.as_deref(),
            ) {
                Ok(outcome) => {
                    previous_image_hash = Some(outcome.image_hash);
                    previous_content_hash = outcome.content_hash.or(previous_content_hash);
                    if let Some(frame) = outcome.frame {
                        last_idle_capture = Instant::now();
                        update_success(&state, frame.clone());
                        let _ = app.emit("capture-frame", frame);
                    }
                }
                Err(error) => update_error(&state, error),
            }
        }

        if last_idle_capture.elapsed() >= Duration::from_secs(30) {
            last_idle_capture = Instant::now();
            match capture_frame(&app, "idle", false, None, None) {
                Ok(outcome) => {
                    previous_image_hash = Some(outcome.image_hash);
                    previous_content_hash = outcome.content_hash;
                    if let Some(frame) = outcome.frame {
                        update_success(&state, frame.clone());
                        let _ = app.emit("capture-frame", frame);
                    }
                }
                Err(error) => update_error(&state, error),
            }
        }
    }

    if let Ok(mut runtime) = state.lock() {
        runtime.running = false;
        runtime.started_at = None;
        runtime.stop_signal = None;
    }

    let _ = app.emit("capture-status", capture_status_snapshot_inner(&app, &state));
}

fn capture_frame(
    app: &AppHandle,
    capture_trigger: &str,
    dedupe: bool,
    previous_image_hash: Option<&str>,
    previous_content_hash: Option<&str>,
) -> Result<CaptureOutcome, String> {
    let paths = capture_paths(app)?;
    fs::create_dir_all(&paths.snapshot_dir).map_err(to_string)?;
    ensure_db(app)?;

    let captured_at = now_millis();
    let day = day_bucket(captured_at);
    let day_dir = paths.snapshot_dir.join(day);
    fs::create_dir_all(&day_dir).map_err(to_string)?;

    let snapshot_path = day_dir.join(format!("{}_main.jpg", captured_at));
    capture_screenshot(&snapshot_path)?;
    let image_hash = stable_hash_bytes(&fs::read(&snapshot_path).map_err(to_string)?);

    let context = collect_accessibility_context();
    let accessibility_text = non_empty(context.text.clone());
    let accessibility_tree_json = if context.nodes.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&context.nodes).map_err(to_string)?)
    };
    let a11y_is_thin = accessibility_text
        .as_deref()
        .map(|_| accessibility_is_thin(&context))
        .unwrap_or(true);

    let ocr = if accessibility_text.is_none() || a11y_is_thin {
        run_ocr(&paths, &snapshot_path).unwrap_or_else(|error| OcrOutput {
            error: Some(error),
            ..OcrOutput::default()
        })
    } else {
        OcrOutput::default()
    };

    let ocr_text = non_empty(ocr.text.clone());
    let (text_source, full_text) = resolve_text(accessibility_text.as_deref(), ocr_text.as_deref(), a11y_is_thin);
    let content_hash = full_text.as_deref().map(|text| stable_hash_bytes(text.as_bytes()));

    if dedupe {
        let same_image = previous_image_hash.is_some_and(|prev| prev == image_hash);
        let same_content = match (previous_content_hash, content_hash.as_deref()) {
            (Some(prev), Some(current)) => prev == current,
            _ => false,
        };
        if same_image || same_content {
            let _ = fs::remove_file(&snapshot_path);
            return Ok(CaptureOutcome {
                frame: None,
                image_hash,
                content_hash,
            });
        }
    }

    let conn = open_db(app)?;
    conn.execute(
        "INSERT INTO frames (
            captured_at, snapshot_path, app_name, window_name, browser_url,
            document_path, focused, capture_trigger, text_source,
            accessibility_text, accessibility_tree_json, full_text,
            content_hash, image_hash, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            captured_at,
            snapshot_path.to_string_lossy().to_string(),
            context.app_name,
            context.window_name,
            context.browser_url,
            context.document_path,
            true,
            capture_trigger,
            text_source,
            accessibility_text,
            accessibility_tree_json,
            full_text,
            content_hash,
            image_hash,
            captured_at,
        ],
    )
    .map_err(to_string)?;

    let frame_id = conn.last_insert_rowid();
    if let Some(text) = ocr_text {
        conn.execute(
            "INSERT INTO ocr_text (frame_id, text, text_json, ocr_engine)
             VALUES (?1, ?2, ?3, ?4)",
            params![frame_id, text, ocr.text_json, ocr.engine],
        )
        .map_err(to_string)?;
    }

    let mut frame = get_frame(app.clone(), frame_id)?.ok_or_else(|| "stored frame missing".to_string())?;
    if frame.full_text.is_none() {
        let mut notes = Vec::new();
        if let Some(error) = context.error {
            notes.push(format!("accessibility: {}", error));
        }
        if let Some(error) = ocr.error {
            notes.push(format!("ocr: {}", error));
        }
        if !notes.is_empty() {
            frame.full_text = Some(notes.join("\n"));
        }
    }

    Ok(CaptureOutcome {
        frame: Some(frame),
        image_hash,
        content_hash,
    })
}

fn resolve_text(
    accessibility_text: Option<&str>,
    ocr_text: Option<&str>,
    a11y_is_thin: bool,
) -> (Option<&'static str>, Option<String>) {
    match (accessibility_text, ocr_text) {
        (Some(a11y), Some(ocr)) if a11y_is_thin => {
            (Some("hybrid"), Some(format!("{}\n{}", a11y, ocr)))
        }
        (Some(a11y), _) => (Some("accessibility"), Some(a11y.to_string())),
        (None, Some(ocr)) => (Some("ocr"), Some(ocr.to_string())),
        (None, None) => (None, None),
    }
}

fn capture_status_snapshot(app: &AppHandle, state: &State<CaptureState>) -> Result<CaptureStatus, String> {
    capture_status_snapshot_inner(app, &state.inner)
}

fn capture_status_snapshot_inner(
    app: &AppHandle,
    runtime_state: &Arc<Mutex<CaptureRuntime>>,
) -> Result<CaptureStatus, String> {
    let paths = capture_paths(app)?;
    let _ = ensure_db(app);
    let conn = open_db(app).ok();

    let frame_count = conn
        .as_ref()
        .and_then(|conn| conn.query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0)).ok())
        .unwrap_or(0);

    let latest_frame = conn.as_ref().and_then(|conn| {
        conn.query_row(
            "SELECT id, captured_at, snapshot_path, app_name, window_name,
                    browser_url, document_path, focused, capture_trigger,
                    text_source, accessibility_text, accessibility_tree_json,
                    full_text, content_hash, image_hash
             FROM frames
             ORDER BY captured_at DESC
             LIMIT 1",
            [],
            frame_from_row,
        )
        .optional()
        .ok()
        .flatten()
    });

    let runtime = runtime_state
        .lock()
        .map_err(|_| "capture runtime lock poisoned".to_string())?;

    Ok(CaptureStatus {
        running: runtime.running,
        frame_count,
        started_at: runtime.started_at,
        last_error: runtime.last_error.clone(),
        latest_frame: latest_frame.or_else(|| runtime.last_frame.clone()),
        data_dir: paths.root_dir.to_string_lossy().to_string(),
        database_path: paths.db_path.to_string_lossy().to_string(),
        screenshot_tool: Path::new("/usr/sbin/screencapture").exists(),
        accessibility_tool: Path::new("/usr/bin/osascript").exists(),
        ocr_tool: Path::new("/usr/bin/swiftc").exists()
            || Path::new("/usr/bin/swift").exists()
            || command_in_path("tesseract"),
    })
}

fn latest_frames(conn: &Connection, limit: u32) -> Result<Vec<SearchResult>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, captured_at, snapshot_path, app_name, window_name,
                    browser_url, document_path, focused, capture_trigger,
                    text_source, accessibility_text, accessibility_tree_json,
                    full_text, content_hash, image_hash,
                    substr(coalesce(full_text, ''), 1, 260) AS snippet
             FROM frames
             ORDER BY captured_at DESC
             LIMIT ?1",
        )
        .map_err(to_string)?;

    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(SearchResult {
                frame: frame_from_row(row)?,
                snippet: row.get(15)?,
                rank: 0.0,
            })
        })
        .map_err(to_string)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn frame_from_row(row: &Row<'_>) -> rusqlite::Result<CaptureFrame> {
    Ok(CaptureFrame {
        id: row.get(0)?,
        captured_at: row.get(1)?,
        snapshot_path: row.get(2)?,
        app_name: row.get(3)?,
        window_name: row.get(4)?,
        browser_url: row.get(5)?,
        document_path: row.get(6)?,
        focused: row.get(7)?,
        capture_trigger: row.get(8)?,
        text_source: row.get(9)?,
        accessibility_text: row.get(10)?,
        accessibility_tree_json: row.get(11)?,
        full_text: row.get(12)?,
        content_hash: row.get(13)?,
        image_hash: row.get(14)?,
    })
}

fn ensure_db(app: &AppHandle) -> Result<(), String> {
    let conn = open_db(app)?;
    init_db(&conn)
}

fn open_db(app: &AppHandle) -> Result<Connection, String> {
    let paths = capture_paths(app)?;
    fs::create_dir_all(&paths.root_dir).map_err(to_string)?;
    let conn = Connection::open(&paths.db_path).map_err(to_string)?;
    conn.busy_timeout(Duration::from_secs(5)).map_err(to_string)?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    init_db(&conn)?;
    Ok(conn)
}

fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS frames (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          captured_at INTEGER NOT NULL,
          snapshot_path TEXT NOT NULL,
          app_name TEXT,
          window_name TEXT,
          browser_url TEXT,
          document_path TEXT,
          focused INTEGER NOT NULL DEFAULT 1,
          capture_trigger TEXT NOT NULL,
          text_source TEXT,
          accessibility_text TEXT,
          accessibility_tree_json TEXT,
          full_text TEXT,
          content_hash TEXT,
          image_hash TEXT,
          created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS ocr_text (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          frame_id INTEGER NOT NULL REFERENCES frames(id) ON DELETE CASCADE,
          text TEXT,
          text_json TEXT,
          ocr_engine TEXT
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS frames_fts
        USING fts5(full_text, app_name, window_name, browser_url, document_path);

        CREATE TRIGGER IF NOT EXISTS frames_ai AFTER INSERT ON frames BEGIN
          INSERT INTO frames_fts(rowid, full_text, app_name, window_name, browser_url, document_path)
          VALUES (new.id, new.full_text, new.app_name, new.window_name, new.browser_url, new.document_path);
        END;

        CREATE TRIGGER IF NOT EXISTS frames_ad AFTER DELETE ON frames BEGIN
          DELETE FROM frames_fts WHERE rowid = old.id;
        END;

        CREATE INDEX IF NOT EXISTS idx_frames_captured_at ON frames(captured_at DESC);
        CREATE INDEX IF NOT EXISTS idx_ocr_text_frame_id ON ocr_text(frame_id);
        ",
    )
    .map_err(to_string)
}

fn capture_paths(app: &AppHandle) -> Result<CapturePaths, String> {
    let root_dir = app
        .path()
        .app_data_dir()
        .map_err(to_string)?
        .join("capture");
    Ok(CapturePaths {
        snapshot_dir: root_dir.join("snapshots"),
        helper_dir: root_dir.join("helpers"),
        db_path: root_dir.join("smalltalk-capture.sqlite"),
        root_dir,
    })
}

fn capture_screenshot(path: &Path) -> Result<(), String> {
    let output = Command::new("/usr/sbin/screencapture")
        .arg("-x")
        .arg("-t")
        .arg("jpg")
        .arg(path)
        .output()
        .map_err(|error| format!("screencapture failed to start: {}", error))?;

    if output.status.success() && path.exists() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "screencapture failed; Screen Recording permission may be missing".to_string()
        } else {
            stderr
        })
    }
}

fn collect_accessibility_context() -> AccessibilityContext {
    let output = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(ACCESSIBILITY_SCRIPT)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return AccessibilityContext {
                error: Some(format!("osascript failed to start: {}", error)),
                ..AccessibilityContext::default()
            }
        }
    };

    if !output.status.success() {
        return AccessibilityContext {
            error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            ..AccessibilityContext::default()
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_accessibility_output(&stdout)
}

fn parse_accessibility_output(stdout: &str) -> AccessibilityContext {
    let mut context = AccessibilityContext::default();
    let mut text_parts = Vec::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(4, '\t');
        match parts.next() {
            Some("APP") => context.app_name = non_empty(parts.next().unwrap_or("").to_string()),
            Some("WINDOW") => context.window_name = non_empty(parts.next().unwrap_or("").to_string()),
            Some("BROWSER_URL") => {
                context.browser_url = non_empty(parts.next().unwrap_or("").to_string())
            }
            Some("DOCUMENT") => {
                let raw = parts.next().unwrap_or("").to_string();
                context.document_path = parse_document_path(&raw);
                if context.browser_url.is_none()
                    && (raw.starts_with("http://") || raw.starts_with("https://"))
                {
                    context.browser_url = Some(raw);
                }
            }
            Some("NODE") => {
                let depth = parts
                    .next()
                    .and_then(|value| value.parse::<u8>().ok())
                    .unwrap_or(0);
                let role = parts.next().unwrap_or("").trim().to_string();
                let text = parts.next().unwrap_or("").trim().to_string();
                if !text.is_empty() {
                    text_parts.push(text.clone());
                    context.nodes.push(AccessibilityNode { role, text, depth });
                }
            }
            _ => {}
        }
    }

    context.text = compact_join(text_parts);
    context
}

fn parse_document_path(raw: &str) -> Option<String> {
    if !raw.starts_with("file://") {
        return None;
    }
    let mut path = raw.trim_start_matches("file://").to_string();
    if let Some(rest) = path.strip_prefix("localhost/") {
        path = format!("/{}", rest);
    }
    percent_decode(&path).or(Some(path))
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                output.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        output.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(output).ok()
}

fn accessibility_is_thin(context: &AccessibilityContext) -> bool {
    const CANVAS_PATTERNS: &[&str] = &[
        "google docs",
        "google sheets",
        "google slides",
        "figma",
        "excalidraw",
        "miro",
        "canva",
        "tldraw",
    ];
    const CHROME_ROLES: &[&str] = &[
        "button",
        "menu",
        "menu item",
        "menuitem",
        "toolbar",
        "tab",
        "checkbox",
        "radio button",
        "slider",
        "scroll bar",
        "pop up button",
        "AXButton",
        "AXMenuItem",
        "AXToolbar",
        "AXTab",
        "AXCheckBox",
        "AXRadioButton",
    ];

    let window = context.window_name.as_deref().unwrap_or("").to_lowercase();
    let url = context.browser_url.as_deref().unwrap_or("").to_lowercase();
    if CANVAS_PATTERNS
        .iter()
        .any(|pattern| window.contains(pattern) || url.contains(pattern))
    {
        return true;
    }

    let total_chars: usize = context.nodes.iter().map(|node| node.text.len()).sum();
    if total_chars < 100 {
        return true;
    }

    let content_chars: usize = context
        .nodes
        .iter()
        .filter(|node| {
            let role = node.role.to_lowercase();
            !CHROME_ROLES
                .iter()
                .any(|chrome_role| role == chrome_role.to_lowercase() || role.contains(chrome_role))
        })
        .map(|node| node.text.len())
        .sum();

    (content_chars as f64 / total_chars as f64) < 0.3
}

fn run_ocr(paths: &CapturePaths, image_path: &Path) -> Result<OcrOutput, String> {
    if cfg!(target_os = "macos") {
        match run_vision_ocr(paths, image_path) {
            Ok(output) if !output.text.trim().is_empty() => return Ok(output),
            Ok(output) => {
                if command_in_path("tesseract") {
                    return run_tesseract(image_path).or(Ok(output));
                }
                return Ok(output);
            }
            Err(error) => {
                if command_in_path("tesseract") {
                    return run_tesseract(image_path);
                }
                return Err(error);
            }
        }
    }

    run_tesseract(image_path)
}

fn run_vision_ocr(paths: &CapturePaths, image_path: &Path) -> Result<OcrOutput, String> {
    fs::create_dir_all(&paths.helper_dir).map_err(to_string)?;
    let source_path = paths.helper_dir.join("vision_ocr.swift");
    let helper_path = paths.helper_dir.join("vision_ocr");

    let should_write = fs::read_to_string(&source_path)
        .map(|existing| existing != VISION_OCR_SWIFT)
        .unwrap_or(true);
    if should_write {
        fs::write(&source_path, VISION_OCR_SWIFT).map_err(to_string)?;
        let _ = fs::remove_file(&helper_path);
    }

    if !helper_path.exists() {
        let output = Command::new("/usr/bin/swiftc")
            .arg("-O")
            .arg(&source_path)
            .arg("-framework")
            .arg("Vision")
            .arg("-framework")
            .arg("AppKit")
            .arg("-o")
            .arg(&helper_path)
            .output()
            .map_err(|error| format!("swiftc failed to start: {}", error))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return run_vision_ocr_interpreted(&source_path, image_path).or(Err(stderr));
        }
    }

    let output = Command::new(&helper_path)
        .arg(image_path)
        .output()
        .map_err(|error| format!("vision helper failed to start: {}", error))?;
    parse_vision_output(output, "AppleVision")
}

fn run_vision_ocr_interpreted(source_path: &Path, image_path: &Path) -> Result<OcrOutput, String> {
    let output = Command::new("/usr/bin/swift")
        .arg(source_path)
        .arg(image_path)
        .output()
        .map_err(|error| format!("swift failed to start: {}", error))?;
    parse_vision_output(output, "AppleVision")
}

fn parse_vision_output(output: std::process::Output, engine: &str) -> Result<OcrOutput, String> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "vision OCR failed".to_string()
        } else {
            stderr
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: Value = serde_json::from_str(stdout.trim()).map_err(to_string)?;
    let text = payload
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let text_json = payload
        .get("text_elements")
        .map(Value::to_string)
        .unwrap_or_else(|| "[]".to_string());
    let error = payload
        .get("error")
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(OcrOutput {
        text,
        text_json,
        engine: engine.to_string(),
        error,
    })
}

fn run_tesseract(image_path: &Path) -> Result<OcrOutput, String> {
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .output()
        .map_err(|error| format!("tesseract failed to start: {}", error))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(OcrOutput {
        text: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        text_json: "[]".to_string(),
        engine: "Tesseract".to_string(),
        error: None,
    })
}

fn command_in_path(name: &str) -> bool {
    Command::new("/usr/bin/which")
        .arg(name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn lock_runtime(state: &CaptureState) -> Result<std::sync::MutexGuard<'_, CaptureRuntime>, String> {
    state
        .inner
        .lock()
        .map_err(|_| "capture runtime lock poisoned".to_string())
}

fn update_success(state: &Arc<Mutex<CaptureRuntime>>, frame: CaptureFrame) {
    if let Ok(mut runtime) = state.lock() {
        runtime.last_error = None;
        runtime.last_frame = Some(frame);
    }
}

fn update_error(state: &Arc<Mutex<CaptureRuntime>>, error: String) {
    if let Ok(mut runtime) = state.lock() {
        runtime.last_error = Some(error);
    }
}

fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .map(|token| format!("{}*", token))
        .collect::<Vec<_>>()
        .join(" ")
}

fn compact_join(parts: Vec<String>) -> String {
    let mut output = Vec::new();
    for part in parts {
        let trimmed = part.trim();
        if !trimmed.is_empty() && !output.iter().any(|existing: &String| existing == trimmed) {
            output.push(trimmed.to_string());
        }
    }
    output.join("\n")
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn stable_hash_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);

        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn day_bucket(timestamp_ms: i64) -> String {
    let days = timestamp_ms.div_euclid(86_400_000);
    format!("day-{}", days)
}

fn to_string<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}
