use rusqlite::types::ValueRef as SqlValueRef;
use rusqlite::{params, Connection, OptionalExtension, Row, ToSql};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
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
    try
      set pieces to my appendPiece(pieces, value of attribute "AXSelectedText" of theElement)
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
const ACCESSIBILITY_SNAPSHOT_SWIFT: &str = include_str!("../scripts/accessibility_snapshot.swift");
const CAPTURE_EVENTS_SWIFT: &str = include_str!("../scripts/capture_events.swift");
const WINDOW_SNAPSHOT_SWIFT: &str = include_str!("../scripts/window_snapshot.swift");
const IMAGE_MASK_SWIFT: &str = r#"
import AppKit
import Foundation

struct MaskRect: Decodable {
    let x: Double
    let y: Double
    let w: Double
    let h: Double
}

let args = CommandLine.arguments
guard args.count >= 4 else {
    fputs("usage: image_mask <input> <output> <rects-json>\n", stderr)
    exit(2)
}

let input = URL(fileURLWithPath: args[1])
let output = URL(fileURLWithPath: args[2])
let rectData = args[3].data(using: .utf8) ?? Data()
let rects = (try? JSONDecoder().decode([MaskRect].self, from: rectData)) ?? []

guard let image = NSImage(contentsOf: input) else {
    fputs("could not load input image\n", stderr)
    exit(3)
}

let size = image.size
guard let bitmap = NSBitmapImageRep(
    bitmapDataPlanes: nil,
    pixelsWide: max(1, Int(size.width.rounded())),
    pixelsHigh: max(1, Int(size.height.rounded())),
    bitsPerSample: 8,
    samplesPerPixel: 4,
    hasAlpha: true,
    isPlanar: false,
    colorSpaceName: .deviceRGB,
    bytesPerRow: 0,
    bitsPerPixel: 0
) else {
    fputs("could not create bitmap\n", stderr)
    exit(4)
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
image.draw(in: NSRect(origin: .zero, size: size))
NSColor.black.setFill()
for rect in rects {
    let x = max(0.0, min(rect.x, size.width))
    let y = max(0.0, min(rect.y, size.height))
    let w = max(0.0, min(rect.w, size.width - x))
    let h = max(0.0, min(rect.h, size.height - y))
    NSBezierPath(rect: NSRect(x: x, y: size.height - y - h, width: w, height: h)).fill()
}
NSGraphicsContext.restoreGraphicsState()

guard let png = bitmap.representation(using: .png, properties: [:]) else {
    fputs("could not encode png\n", stderr)
    exit(5)
}

try FileManager.default.createDirectory(
    at: output.deletingLastPathComponent(),
    withIntermediateDirectories: true
)
try png.write(to: output)
"#;
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

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
    active_session_id: Option<String>,
    last_session: Option<CaptureSession>,
    last_export: Option<SessionExportSummary>,
    started_at: Option<i64>,
    skipped_samples: i64,
    last_skipped_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureStatus {
    pub running: bool,
    pub frame_count: i64,
    pub event_count: i64,
    pub transition_count: i64,
    pub content_unit_count: i64,
    pub session_count: i64,
    pub active_session: Option<CaptureSession>,
    pub latest_session: Option<CaptureSession>,
    pub last_export: Option<SessionExportSummary>,
    pub started_at: Option<i64>,
    pub last_error: Option<String>,
    pub latest_frame: Option<CaptureFrame>,
    pub skipped_samples: i64,
    pub last_skipped_at: Option<i64>,
    pub data_dir: String,
    pub database_path: String,
    pub screenshot_tool: bool,
    pub accessibility_tool: bool,
    pub ocr_tool: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionCounts {
    pub frames: i64,
    pub events: i64,
    pub triggers: i64,
    pub transitions: i64,
    pub content_units: i64,
    pub ax_nodes: i64,
    pub ocr_text_rows: i64,
    pub ocr_spans: i64,
    pub app_contexts: i64,
    pub window_snapshots: i64,
    pub windows: i64,
    pub frame_diffs: i64,
    pub clipboard_events: i64,
    pub typing_bursts: i64,
    pub presence_samples: i64,
    pub sensitive_regions: i64,
}

impl Default for SessionCounts {
    fn default() -> Self {
        Self {
            frames: 0,
            events: 0,
            triggers: 0,
            transitions: 0,
            content_units: 0,
            ax_nodes: 0,
            ocr_text_rows: 0,
            ocr_spans: 0,
            app_contexts: 0,
            window_snapshots: 0,
            windows: 0,
            frame_diffs: 0,
            clipboard_events: 0,
            typing_bursts: 0,
            presence_samples: 0,
            sensitive_regions: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureSession {
    pub id: String,
    pub sequence: i64,
    pub started_at: i64,
    pub stopped_at: Option<i64>,
    pub status: String,
    pub export_path: Option<String>,
    pub counts: SessionCounts,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionExportSummary {
    pub session_id: String,
    pub session_sequence: i64,
    pub generated_at: i64,
    pub kind: String,
    pub folder_name: String,
    pub path: String,
    pub byte_size: i64,
    pub file_count: i64,
    pub warning_count: i64,
    pub counts: SessionCounts,
}

#[derive(Debug, Clone, Serialize)]
pub struct StopCaptureOutput {
    pub status: CaptureStatus,
    pub session: Option<CaptureSession>,
    pub export: Option<SessionExportSummary>,
    pub preview: Option<Value>,
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
    pub capture_provider: Option<String>,
    pub scope: Option<String>,
    pub display_id: Option<String>,
    pub window_id: Option<i64>,
    pub app_pid: Option<i64>,
    pub app_bundle_id: Option<String>,
    pub screen_scale: Option<f64>,
    pub pixel_width: Option<i64>,
    pub pixel_height: Option<i64>,
    pub full_screenshot_path: Option<String>,
    pub active_window_crop_path: Option<String>,
    pub active_element_crop_path: Option<String>,
    pub phash: Option<String>,
    pub privacy_status: Option<String>,
    pub capture_trigger_id: Option<String>,
    pub previous_frame_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub frame: CaptureFrame,
    pub snippet: String,
    pub rank: f64,
}

const FRAME_COLUMNS: &str = "id, captured_at, snapshot_path, app_name, window_name,
    browser_url, document_path, focused, capture_trigger, text_source,
    accessibility_text, accessibility_tree_json, full_text, content_hash, image_hash,
    capture_provider, scope, display_id, window_id, app_pid, app_bundle_id,
    screen_scale, pixel_width, pixel_height, full_screenshot_path,
    active_window_crop_path, active_element_crop_path, phash, privacy_status,
    capture_trigger_id, previous_frame_id, session_id";

const FRAME_COLUMNS_F: &str = "f.id, f.captured_at, f.snapshot_path, f.app_name, f.window_name,
    f.browser_url, f.document_path, f.focused, f.capture_trigger, f.text_source,
    f.accessibility_text, f.accessibility_tree_json, f.full_text, f.content_hash, f.image_hash,
    f.capture_provider, f.scope, f.display_id, f.window_id, f.app_pid, f.app_bundle_id,
    f.screen_scale, f.pixel_width, f.pixel_height, f.full_screenshot_path,
    f.active_window_crop_path, f.active_element_crop_path, f.phash, f.privacy_status,
    f.capture_trigger_id, f.previous_frame_id, f.session_id";

const INSERT_FRAME_SQL: &str = "INSERT INTO frames (
    captured_at, snapshot_path, app_name, window_name, browser_url,
    document_path, focused, capture_trigger, text_source,
    accessibility_text, accessibility_tree_json, full_text,
    content_hash, image_hash, created_at, capture_provider, scope,
    display_id, window_id, app_pid, app_bundle_id, screen_scale,
    pixel_width, pixel_height, full_screenshot_path, active_window_crop_path,
    active_element_crop_path, phash, privacy_status, capture_trigger_id,
    previous_frame_id, session_id
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
          ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28,
          ?29, ?30, ?31, ?32)";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AccessibilityNode {
    #[serde(default)]
    local_id: Option<String>,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    role: String,
    #[serde(default)]
    subrole: Option<String>,
    #[serde(default)]
    role_description: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    help: Option<String>,
    #[serde(default)]
    identifier: Option<String>,
    #[serde(default)]
    document: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    selected_text: Option<String>,
    #[serde(default)]
    selected_text_range: Option<Value>,
    #[serde(default)]
    visible_character_range: Option<Value>,
    #[serde(default)]
    number_of_characters: Option<i64>,
    #[serde(default)]
    focused: Option<bool>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    selected: Option<bool>,
    #[serde(default)]
    bounds: Option<Rect>,
    #[serde(default)]
    actions: Vec<String>,
    #[serde(default)]
    children_count: Option<i64>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    depth: u8,
}

const EVENT_LOOP_WAKE_INTERVAL: Duration = Duration::from_millis(100);
const MIN_CAPTURE_INTERVAL: Duration = Duration::from_millis(600);
const IDLE_CAPTURE_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Default)]
struct AccessibilityContext {
    app_pid: Option<i64>,
    app_bundle_id: Option<String>,
    app_name: Option<String>,
    window_id: Option<i64>,
    window_name: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    text: String,
    nodes: Vec<AccessibilityNode>,
    error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SemanticFingerprint {
    app_name: Option<String>,
    window_name: Option<String>,
    browser_url: Option<String>,
    document_path: Option<String>,
    text_hash: Option<String>,
}

#[derive(Debug, Default)]
struct OcrOutput {
    text: String,
    text_json: String,
    engine: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct FrameMetadata {
    capture_provider: String,
    scope: String,
    display_id: Option<String>,
    window_id: Option<i64>,
    app_pid: Option<i64>,
    app_bundle_id: Option<String>,
    screen_scale: f64,
    pixel_width: Option<i64>,
    pixel_height: Option<i64>,
    full_screenshot_path: String,
    active_window_crop_path: Option<String>,
    active_element_crop_path: Option<String>,
    phash: Option<String>,
    privacy_status: String,
}

#[derive(Debug, Clone, Default)]
struct PrivacyDecision {
    skip_capture: bool,
    status: String,
    regions: Vec<DetectedSensitiveRegion>,
}

#[derive(Debug, Clone)]
struct DetectedSensitiveRegion {
    region_type: String,
    bounds: Option<Rect>,
    source: String,
    confidence: f64,
    action_taken: String,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WindowSnapshotPayload {
    ts_ms: i64,
    active_window_id: Option<i64>,
    active_app_pid: Option<i64>,
    active_app_bundle_id: Option<String>,
    screen_count: i64,
    windows: Vec<WindowPayload>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WindowPayload {
    cg_window_id: Option<i64>,
    owner_pid: Option<i64>,
    owner_name: Option<String>,
    bundle_id: Option<String>,
    window_title: Option<String>,
    layer: Option<i64>,
    alpha: Option<f64>,
    is_onscreen: Option<bool>,
    is_active: bool,
    bounds: Option<Rect>,
    workspace: Option<i64>,
    raw: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct VisionOcrElement {
    text: String,
    confidence: Option<f64>,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
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
    semantic_fingerprint: SemanticFingerprint,
}

#[derive(Debug, Clone)]
struct PendingTrigger {
    id: String,
    capture_trigger: String,
    caused_by_event_ids: Vec<String>,
    pre_frame_id: Option<String>,
    settle_delay_ms: i64,
    ready_at: Instant,
}

#[derive(Debug, Clone, Default)]
struct TypingBurstState {
    id: Option<String>,
    started_at_ms: i64,
    ended_at_ms: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct UiEventRecord {
    #[serde(default)]
    id: String,
    ts_ms: i64,
    event_type: String,
    app_pid: Option<i64>,
    app_bundle_id: Option<String>,
    app_name: Option<String>,
    window_id: Option<i64>,
    window_title: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    button: Option<String>,
    scroll_dx: Option<f64>,
    scroll_dy: Option<f64>,
    key_category: Option<String>,
    modifier_flags: Option<String>,
    is_repeat: Option<bool>,
    #[serde(default)]
    payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UiEventSummary {
    pub id: String,
    pub ts_ms: i64,
    pub event_type: String,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub key_category: Option<String>,
    pub payload_json: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureTriggerSummary {
    pub id: String,
    pub ts_ms: i64,
    pub trigger_type: String,
    pub caused_by_event_ids: String,
    pub pre_frame_id: Option<String>,
    pub post_frame_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransitionSummary {
    pub id: String,
    pub trigger_id: String,
    pub primary_event_id: Option<String>,
    pub pre_frame_id: Option<String>,
    pub post_frame_id: Option<String>,
    pub ts_start_ms: i64,
    pub ts_end_ms: i64,
    pub transition_type: Option<String>,
    pub confidence: Option<f64>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Timeline {
    pub events: Vec<UiEventSummary>,
    pub triggers: Vec<CaptureTriggerSummary>,
    pub transitions: Vec<TransitionSummary>,
    pub frames: Vec<CaptureFrame>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AxNodeSummary {
    pub id: String,
    pub parent_id: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub focused: Option<bool>,
    pub bounds_x: Option<f64>,
    pub bounds_y: Option<f64>,
    pub bounds_w: Option<f64>,
    pub bounds_h: Option<f64>,
    pub depth: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrSpanSummary {
    pub id: String,
    pub engine: String,
    pub text: String,
    pub confidence: Option<f64>,
    pub bounds_x: f64,
    pub bounds_y: f64,
    pub bounds_w: f64,
    pub bounds_h: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentUnitSummary {
    pub id: String,
    pub source: String,
    pub unit_type: String,
    pub semantic_role: Option<String>,
    pub text: Option<String>,
    pub bounds_x: Option<f64>,
    pub bounds_y: Option<f64>,
    pub bounds_w: Option<f64>,
    pub bounds_h: Option<f64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppContextSummary {
    pub id: String,
    pub adapter_id: String,
    pub object_type: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub selected_text: Option<String>,
    pub focused_object: Option<String>,
    pub confidence: Option<f64>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensitiveRegionSummary {
    pub id: String,
    pub region_type: String,
    pub bounds_x: Option<f64>,
    pub bounds_y: Option<f64>,
    pub bounds_w: Option<f64>,
    pub bounds_h: Option<f64>,
    pub source: String,
    pub confidence: Option<f64>,
    pub action_taken: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowSummary {
    pub cg_window_id: Option<i64>,
    pub owner_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub is_active: bool,
    pub bounds_x: Option<f64>,
    pub bounds_y: Option<f64>,
    pub bounds_w: Option<f64>,
    pub bounds_h: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationSignals {
    pub screenshot_present: bool,
    pub has_ax: bool,
    pub has_ocr: bool,
    pub has_content_units: bool,
    pub has_app_context: bool,
    pub has_window_graph: bool,
    pub has_transition: bool,
    pub has_event_provenance: bool,
    pub has_sensitive_regions: bool,
    pub ax_node_count: usize,
    pub ocr_span_count: usize,
    pub content_unit_count: usize,
    pub app_context_count: usize,
    pub window_count: usize,
    pub transition_count: usize,
    pub event_count: usize,
    pub missing_signals: Vec<String>,
    pub trust_label: String,
    pub trust_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameDetail {
    pub frame: CaptureFrame,
    pub verification: VerificationSignals,
    pub events: Vec<UiEventSummary>,
    pub ax_nodes: Vec<AxNodeSummary>,
    pub ocr_spans: Vec<OcrSpanSummary>,
    pub content_units: Vec<ContentUnitSummary>,
    pub app_contexts: Vec<AppContextSummary>,
    pub sensitive_regions: Vec<SensitiveRegionSummary>,
    pub windows: Vec<WindowSummary>,
    pub transitions: Vec<TransitionSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameConsistencyReport {
    pub frame_id: String,
    pub warnings: Vec<FrameQualityWarning>,
    pub confidence_adjustment: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameQualityWarning {
    pub id: String,
    pub frame_id: String,
    pub warning_type: String,
    pub severity: String,
    pub message: String,
    pub evidence_json: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaptureConfig {
    pub capture_v2_enabled: Option<bool>,
    pub screencapturekit_enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExclusionRuleInput {
    pub rule_type: String,
    pub pattern: String,
    pub action: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExclusionRule {
    pub id: String,
    pub rule_type: String,
    pub pattern: String,
    pub action: String,
    pub enabled: bool,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafeAiExportInput {
    pub lookback_minutes: Option<i64>,
    pub range_ms: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub current_frame_id: Option<i64>,
    pub include_images: Option<bool>,
    pub max_frames: Option<u32>,
    pub export_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeAiExportBundle {
    pub id: String,
    pub generated_at_ms: i64,
    pub export_type: String,
    pub lookback_start_ms: i64,
    pub lookback_end_ms: i64,
    pub input_frame_count: usize,
    pub exported_frame_count: usize,
    pub excluded_frame_count: usize,
    pub masked_image_count: usize,
    pub redacted_text_count: usize,
    pub frames: Vec<SafeAiFrame>,
    pub transitions: Vec<TransitionSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeAiFrame {
    pub frame_id: String,
    pub captured_at_ms: i64,
    pub app_name: Option<String>,
    pub app_bundle_id: Option<String>,
    pub window_name: Option<String>,
    pub window_id: Option<i64>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub phash: Option<String>,
    pub app_context_id: Option<String>,
    pub app_context_object_type: Option<String>,
    pub image_path_safe: Option<String>,
    pub active_window_crop_path_safe: Option<String>,
    pub top_content_units: Vec<CompactContentUnit>,
    pub text_source: Option<String>,
    pub text: Option<String>,
    pub evidence_strength: f64,
    pub privacy_status: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompactContentUnit {
    pub id: String,
    pub source: String,
    pub unit_type: String,
    pub semantic_role: Option<String>,
    pub text: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NativeStoryboardInput {
    pub lookback_minutes: Option<i64>,
    pub max_keyframes: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub current_frame_id: Option<i64>,
    pub include_images: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeStoryboardDossier {
    pub generated_at_ms: i64,
    pub lookback_start_ms: i64,
    pub lookback_end_ms: i64,
    pub current_frame_id: Option<String>,
    pub keyframes: Vec<StoryboardKeyframe>,
    pub transitions: Vec<StoryboardTransition>,
    pub dominant_surfaces: Vec<SurfaceSummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoryboardKeyframe {
    pub frame_id: String,
    pub kind: String,
    pub captured_at_ms: i64,
    pub app_name: Option<String>,
    pub app_bundle_id: Option<String>,
    pub window_name: Option<String>,
    pub browser_url: Option<String>,
    pub document_path: Option<String>,
    pub app_context_id: Option<String>,
    pub app_context_object_type: Option<String>,
    pub image_path_safe: Option<String>,
    pub active_window_crop_path_safe: Option<String>,
    pub top_content_units: Vec<CompactContentUnit>,
    pub text_source: Option<String>,
    pub evidence_strength: f64,
    pub selection_reason: String,
    pub privacy_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoryboardTransition {
    pub id: String,
    pub transition_type: String,
    pub pre_frame_id: Option<String>,
    pub post_frame_id: Option<String>,
    pub evidence_frame_ids: Vec<String>,
    pub evidence_event_ids: Vec<String>,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransitionClassifierInput {
    pub lookback_minutes: Option<i64>,
    pub range_ms: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub current_frame_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassifiedTransition {
    pub id: String,
    pub transition_type: String,
    pub pre_frame_id: Option<String>,
    pub post_frame_id: Option<String>,
    pub return_score: Option<f64>,
    pub evidence_frame_ids: Vec<String>,
    pub evidence_event_ids: Vec<String>,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceSummary {
    pub surface_key: String,
    pub app_name: Option<String>,
    pub window_name: Option<String>,
    pub url_or_document: Option<String>,
    pub frame_count: usize,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NativeResumeInput {
    pub lookback_minutes: Option<i64>,
    pub max_keyframes: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub current_frame_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeResumeCard {
    pub generated_at_ms: i64,
    pub lookback_minutes: i64,
    pub what_was_i_doing: String,
    pub what_was_i_reading: Option<String>,
    pub focus_now: String,
    pub why_this_focus: String,
    pub continue_from: ResumeContinueFrom,
    pub what_changed: Vec<String>,
    pub useful_evidence: Vec<String>,
    pub likely_distractions: Vec<String>,
    pub behavior_read: ResumeBehaviorRead,
    pub next_action: String,
    pub confidence: f64,
    pub evidence_frame_ids: Vec<String>,
    pub evidence_transition_ids: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResumeContinueFrom {
    pub frame_id: Option<String>,
    pub app_name: Option<String>,
    pub window_name: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub document_path: Option<String>,
    pub quote: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResumeBehaviorRead {
    pub mode: String,
    pub confidence: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResumeEvalReport {
    pub evaluated_at_ms: i64,
    pub case_count: usize,
    pub average_task_identification_score: f64,
    pub average_resume_target_score: f64,
    pub average_hallucination_control_score: f64,
    pub warnings_frequency: f64,
    pub unknown_transition_frequency: f64,
    pub redacted_frame_handling_correctness: f64,
    pub cases: Vec<ResumeEvalCaseReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResumeEvalCaseReport {
    pub session_id: Option<String>,
    pub task_identification: f64,
    pub reading_identification: f64,
    pub resume_target: f64,
    pub why_explanation: f64,
    pub distraction_handling: f64,
    pub hallucination_control: f64,
    pub warnings_count: usize,
    pub unknown_transition_count: usize,
    pub redacted_frame_handling_ok: bool,
}

struct CaptureEventSource {
    child: Child,
    reader: Option<JoinHandle<()>>,
    rx: Receiver<String>,
}

impl SemanticFingerprint {
    fn from_context(context: &AccessibilityContext) -> Self {
        Self {
            app_name: context.app_name.clone(),
            window_name: context.window_name.clone(),
            browser_url: context.browser_url.clone(),
            document_path: context.document_path.clone(),
            text_hash: non_empty(context.text.clone())
                .map(|text| stable_hash_bytes(text.as_bytes())),
        }
    }
}

#[tauri::command]
pub fn start_capture(app: AppHandle, state: State<CaptureState>) -> Result<CaptureStatus, String> {
    ensure_db(&app)?;

    {
        let runtime = lock_runtime(state.inner())?;
        if runtime.running {
            let status = capture_status(app, state)?;
            crate::session_island::update_session_island_from_status(
                &status,
                crate::session_island::SessionIslandState::RecordingCompact,
            );
            return Ok(status);
        }
    }

    crate::session_island::update_session_island(
        crate::session_island::SessionIslandSnapshot::starting(),
    );
    crate::session_island::show_session_island();

    let session = create_capture_session(&app)?;

    {
        let mut runtime = lock_runtime(state.inner())?;
        let stop_signal = Arc::new(AtomicBool::new(false));
        let thread_stop = stop_signal.clone();
        let thread_app = app.clone();
        let thread_state = state.inner.clone();
        let thread_session_id = session.id.clone();

        runtime.running = true;
        runtime.started_at = Some(session.started_at);
        runtime.last_error = None;
        runtime.skipped_samples = 0;
        runtime.last_skipped_at = None;
        runtime.active_session_id = Some(session.id.clone());
        runtime.last_session = Some(session);
        runtime.last_export = None;
        runtime.stop_signal = Some(stop_signal);
        runtime.worker = Some(thread::spawn(move || {
            capture_loop(thread_app, thread_state, thread_stop, thread_session_id);
        }));
    }

    let status = capture_status(app.clone(), state)?;
    crate::session_island::update_session_island_from_status(
        &status,
        crate::session_island::SessionIslandState::RecordingCompact,
    );
    Ok(status)
}

#[tauri::command]
pub fn stop_capture(
    app: AppHandle,
    state: State<CaptureState>,
) -> Result<StopCaptureOutput, String> {
    if let Ok(status) = capture_status_snapshot(&app, &state) {
        crate::session_island::update_session_island_from_status(
            &status,
            crate::session_island::SessionIslandState::Processing,
        );
    }

    let session_id = {
        let runtime = lock_runtime(state.inner())?;
        runtime.active_session_id.clone()
    };

    stop_runtime(state.inner())?;

    let mut stopped_session = None;
    let mut export = None;
    if let Some(session_id) = session_id {
        let (session, summary) = finish_capture_session(&app, &session_id)?;
        {
            let mut runtime = lock_runtime(state.inner())?;
            runtime.active_session_id = None;
            runtime.last_session = Some(session.clone());
            runtime.last_export = Some(summary.clone());
        }
        stopped_session = Some(session);
        export = Some(summary);
    }

    let status = capture_status(app.clone(), state)?;
    crate::session_island::update_session_island_from_status(
        &status,
        crate::session_island::SessionIslandState::StoppedToast,
    );
    crate::session_island::return_to_ready_after_stop(app.clone());
    let preview = stopped_session
        .as_ref()
        .zip(export.as_ref())
        .map(|(session, export)| {
            serde_json::json!({
                "schema": "smalltalk.capture_session.stop_preview.v1",
                "session": session,
                "export": export,
            })
        });

    Ok(StopCaptureOutput {
        status,
        session: stopped_session,
        export,
        preview,
    })
}

#[tauri::command]
pub fn capture_once(app: AppHandle, state: State<CaptureState>) -> Result<CaptureFrame, String> {
    let session_id = {
        let runtime = lock_runtime(state.inner())?;
        runtime
            .active_session_id
            .clone()
            .ok_or_else(|| "start a session before capturing a manual frame".to_string())?
    };
    let pre_frame_id = latest_frame_id_for_session(&app, &session_id)
        .ok()
        .flatten();
    let trigger_id =
        insert_system_capture_trigger(&app, &session_id, "manual", pre_frame_id.clone(), false)?;
    let outcome = capture_frame(
        &app,
        &session_id,
        "manual",
        false,
        None,
        None,
        None,
        None,
        Some(&trigger_id),
        pre_frame_id.as_deref(),
    )?;
    let frame = outcome
        .frame
        .ok_or_else(|| "capture was skipped before a frame was stored".to_string())?;
    let _ = finalize_capture_trigger_by_id(&app, &trigger_id, true);
    {
        let mut runtime = lock_runtime(state.inner())?;
        runtime.last_error = None;
        runtime.last_frame = Some(frame.clone());
    }

    if let Ok(status) = capture_status_snapshot(&app, &state) {
        let _ = app.emit("capture-status", status.clone());
        crate::session_island::update_session_island_from_status(
            &status,
            crate::session_island::SessionIslandState::RecordingCompact,
        );
    }
    Ok(frame)
}

#[tauri::command]
pub fn capture_status(app: AppHandle, state: State<CaptureState>) -> Result<CaptureStatus, String> {
    capture_status_snapshot(&app, &state)
}

#[tauri::command]
pub fn delete_all_frames(
    app: AppHandle,
    state: State<CaptureState>,
) -> Result<CaptureStatus, String> {
    stop_runtime(state.inner())?;
    clear_capture_store(&app)?;

    {
        let mut runtime = lock_runtime(state.inner())?;
        runtime.running = false;
        runtime.started_at = None;
        runtime.last_error = None;
        runtime.last_frame = None;
        runtime.active_session_id = None;
        runtime.last_session = None;
        runtime.last_export = None;
        runtime.skipped_samples = 0;
        runtime.last_skipped_at = None;
    }

    let status = capture_status_snapshot(&app, &state)?;
    let _ = app.emit("capture-status", status.clone());
    crate::session_island::update_session_island_from_status(
        &status,
        crate::session_island::SessionIslandState::Ready,
    );
    crate::session_island::show_session_island();
    Ok(status)
}

#[tauri::command]
pub fn search_captures(
    app: AppHandle,
    query: String,
    limit: Option<u32>,
    session_id: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(30).clamp(1, 100);
    let conn = open_db(&app)?;
    let scoped_session_id = session_id.or_else(|| latest_session_id(&conn).ok().flatten());
    if query.trim().is_empty() {
        return latest_frames(&conn, limit, scoped_session_id.as_deref());
    }

    let fts_query = fts_query(&query);
    if fts_query.is_empty() {
        return latest_frames(&conn, limit, scoped_session_id.as_deref());
    }

    let mut stmt = conn
        .prepare(&format!(
            "SELECT {},
                    snippet(frames_fts, 0, '[', ']', ' ... ', 18) AS snippet,
                    bm25(frames_fts) AS rank
             FROM frames_fts
             JOIN frames f ON f.id = frames_fts.rowid
             WHERE frames_fts MATCH ?1
               AND (?3 IS NULL OR f.session_id = ?3)
             ORDER BY rank
             LIMIT ?2",
            FRAME_COLUMNS_F
        ))
        .map_err(to_string)?;

    let rows = stmt
        .query_map(params![fts_query, limit, scoped_session_id], |row| {
            Ok(SearchResult {
                frame: frame_from_row(row)?,
                snippet: row.get(32)?,
                rank: row.get(33)?,
            })
        })
        .map_err(to_string)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

#[tauri::command]
pub fn get_frame(app: AppHandle, frame_id: i64) -> Result<Option<CaptureFrame>, String> {
    let conn = open_db(&app)?;
    conn.query_row(
        &format!(
            "SELECT {}
         FROM frames
         WHERE id = ?1",
            FRAME_COLUMNS
        ),
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

#[tauri::command]
pub fn get_frame_image_variant(
    app: AppHandle,
    frame_id: i64,
    variant: Option<String>,
) -> Result<Option<String>, String> {
    let Some(frame) = get_frame(app, frame_id)? else {
        return Ok(None);
    };
    let path = match variant.as_deref() {
        Some("window") | Some("preview") => frame
            .active_window_crop_path
            .as_deref()
            .unwrap_or(&frame.snapshot_path),
        _ => &frame.snapshot_path,
    };
    let bytes = fs::read(path).map_err(to_string)?;
    Ok(Some(format!(
        "data:image/jpeg;base64,{}",
        base64_encode(&bytes)
    )))
}

#[tauri::command]
pub fn start_native_capture(
    app: AppHandle,
    state: State<CaptureState>,
    config: Option<CaptureConfig>,
) -> Result<CaptureStatus, String> {
    let _ = config
        .as_ref()
        .and_then(|config| config.capture_v2_enabled)
        .unwrap_or(true);
    let _screencapturekit_requested = config
        .as_ref()
        .and_then(|config| config.screencapturekit_enabled)
        .unwrap_or(false);
    start_capture(app, state)
}

#[tauri::command]
pub fn stop_native_capture(
    app: AppHandle,
    state: State<CaptureState>,
) -> Result<CaptureStatus, String> {
    Ok(stop_capture(app, state)?.status)
}

#[tauri::command]
pub fn capture_once_v2(
    app: AppHandle,
    state: State<CaptureState>,
    reason: Option<String>,
) -> Result<CaptureFrame, String> {
    let _ = reason;
    capture_once(app, state)
}

#[tauri::command]
pub fn get_frame_v2(app: AppHandle, frame_id: i64) -> Result<Option<FrameDetail>, String> {
    get_frame_detail(app, frame_id)
}

#[tauri::command]
pub fn get_recent_timeline(
    app: AppHandle,
    range_ms: Option<i64>,
    session_id: Option<String>,
) -> Result<Timeline, String> {
    let conn = open_db(&app)?;
    let since = now_millis() - range_ms.unwrap_or(10 * 60 * 1000).max(1_000);
    let scoped_session_id = session_id.or_else(|| latest_session_id(&conn).ok().flatten());

    let events = query_ui_events(&conn, since, scoped_session_id.as_deref())?;
    let triggers = query_capture_triggers(&conn, since, scoped_session_id.as_deref())?;
    let transitions = query_transitions_since(&conn, since, scoped_session_id.as_deref())?;
    let frames = query_frames_since(&conn, since, scoped_session_id.as_deref())?;

    Ok(Timeline {
        events,
        triggers,
        transitions,
        frames,
    })
}

#[tauri::command]
pub fn get_frame_detail(app: AppHandle, frame_id: i64) -> Result<Option<FrameDetail>, String> {
    let Some(frame) = get_frame(app.clone(), frame_id)? else {
        return Ok(None);
    };
    let conn = open_db(&app)?;
    let frame_key = frame_id.to_string();
    let ax_nodes = query_ax_nodes(&conn, &frame_key)?;
    let ocr_spans = query_ocr_spans(&conn, &frame_key)?;
    let content_units = query_content_units_for_frame(&conn, &frame_key)?;
    let app_contexts = query_app_contexts(&conn, &frame_key)?;
    let sensitive_regions = query_sensitive_regions(&conn, &frame_key)?;
    let windows = query_windows_for_frame(&conn, &frame_key)?;
    let transitions = query_transitions_for_frame(&conn, &frame_key)?;
    let events = query_events_for_frame(&conn, &frame)?;
    let verification = build_verification_signals(
        &frame,
        &ax_nodes,
        &ocr_spans,
        &content_units,
        &app_contexts,
        &sensitive_regions,
        &windows,
        &transitions,
        &events,
    );
    Ok(Some(FrameDetail {
        frame,
        verification,
        events,
        ax_nodes,
        ocr_spans,
        content_units,
        app_contexts,
        sensitive_regions,
        windows,
        transitions,
    }))
}

#[tauri::command]
pub fn validate_frame_consistency(
    app: AppHandle,
    frame_id: i64,
) -> Result<Option<FrameConsistencyReport>, String> {
    let Some(frame) = get_frame(app.clone(), frame_id)? else {
        return Ok(None);
    };
    let conn = open_db(&app)?;
    Ok(Some(validate_frame_consistency_inner(&conn, &frame)?))
}

#[tauri::command]
pub fn get_transition(
    app: AppHandle,
    transition_id: String,
) -> Result<Option<TransitionSummary>, String> {
    let conn = open_db(&app)?;
    query_transition_by_id(&conn, &transition_id)
}

#[tauri::command]
pub fn search_content_units(
    app: AppHandle,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<ContentUnitSummary>, String> {
    let conn = open_db(&app)?;
    let like = format!("%{}%", query.trim());
    let limit = limit.unwrap_or(50).clamp(1, 100);
    let mut stmt = conn
        .prepare(
            "SELECT id, source, unit_type, semantic_role, text, bounds_x, bounds_y,
                    bounds_w, bounds_h, confidence
             FROM content_units
             WHERE text LIKE ?1
             ORDER BY created_at_ms DESC
             LIMIT ?2",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![like, limit], content_unit_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

#[tauri::command]
pub fn add_exclusion_rule(
    app: AppHandle,
    rule: ExclusionRuleInput,
) -> Result<ExclusionRule, String> {
    let conn = open_db(&app)?;
    let created_at_ms = now_millis();
    let id = next_id("rule");
    let enabled = rule.enabled.unwrap_or(true);
    conn.execute(
        "INSERT INTO exclusion_rules (
            id, rule_type, pattern, action, enabled, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            rule.rule_type,
            rule.pattern,
            rule.action,
            bool_to_i64(enabled),
            created_at_ms,
        ],
    )
    .map_err(to_string)?;
    Ok(ExclusionRule {
        id,
        rule_type: rule.rule_type,
        pattern: rule.pattern,
        action: rule.action,
        enabled,
        created_at_ms,
    })
}

#[tauri::command]
pub fn remove_exclusion_rule(app: AppHandle, rule_id: String) -> Result<bool, String> {
    let conn = open_db(&app)?;
    let changed = conn
        .execute(
            "DELETE FROM exclusion_rules WHERE id = ?1",
            params![rule_id],
        )
        .map_err(to_string)?;
    Ok(changed > 0)
}

#[tauri::command]
pub fn list_exclusion_rules(app: AppHandle) -> Result<Vec<ExclusionRule>, String> {
    let conn = open_db(&app)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, rule_type, pattern, action, enabled, created_at_ms
             FROM exclusion_rules
             ORDER BY created_at_ms ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ExclusionRule {
                id: row.get(0)?,
                rule_type: row.get(1)?,
                pattern: row.get(2)?,
                action: row.get(3)?,
                enabled: row.get::<_, i64>(4)? == 1,
                created_at_ms: row.get(5)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

#[tauri::command]
pub fn delete_recent_captures(app: AppHandle, range_ms: i64) -> Result<i64, String> {
    let conn = open_db(&app)?;
    let cutoff = now_millis() - range_ms.max(1_000);
    let mut stmt = conn
        .prepare(
            "SELECT snapshot_path, active_window_crop_path FROM frames WHERE captured_at >= ?1",
        )
        .map_err(to_string)?;
    let paths = stmt
        .query_map(params![cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    for (snapshot, crop) in paths {
        let _ = fs::remove_file(snapshot);
        if let Some(crop) = crop {
            let _ = fs::remove_file(crop);
        }
    }
    let deleted = conn
        .execute(
            "DELETE FROM frames WHERE captured_at >= ?1",
            params![cutoff],
        )
        .map_err(to_string)? as i64;
    Ok(deleted)
}

#[tauri::command]
pub fn export_debug_episode(app: AppHandle, range_ms: Option<i64>) -> Result<Value, String> {
    let bundle = build_safe_ai_export(
        app,
        SafeAiExportInput {
            lookback_minutes: None,
            range_ms,
            current_frame_id: None,
            include_images: Some(false),
            max_frames: Some(120),
            export_type: Some("debug_episode".to_string()),
        },
    )?;
    serde_json::to_value(bundle).map_err(to_string)
}

#[tauri::command]
pub fn get_episode_dossier(app: AppHandle, range_ms: Option<i64>) -> Result<Value, String> {
    let input = NativeStoryboardInput {
        lookback_minutes: range_ms.map(|value| (value / 60_000).max(1)),
        max_keyframes: Some(12),
        current_frame_id: None,
        include_images: Some(true),
    };
    serde_json::to_value(get_native_storyboard_dossier(app, Some(input))?).map_err(to_string)
}

#[tauri::command]
pub fn build_safe_ai_export(
    app: AppHandle,
    input: SafeAiExportInput,
) -> Result<SafeAiExportBundle, String> {
    let paths = capture_paths(&app)?;
    let conn = open_db(&app)?;
    let output_dir = paths.root_dir.join("safe-ai-exports");
    build_safe_ai_export_from_conn(&conn, &output_dir, input)
}

#[tauri::command]
pub fn get_native_storyboard_dossier(
    app: AppHandle,
    input: Option<NativeStoryboardInput>,
) -> Result<NativeStoryboardDossier, String> {
    let paths = capture_paths(&app)?;
    let conn = open_db(&app)?;
    get_native_storyboard_dossier_from_conn(&conn, &paths.root_dir.join("safe-ai-exports"), input)
}

fn get_native_storyboard_dossier_from_conn(
    conn: &Connection,
    output_root: &Path,
    input: Option<NativeStoryboardInput>,
) -> Result<NativeStoryboardDossier, String> {
    let input = input.unwrap_or(NativeStoryboardInput {
        lookback_minutes: None,
        max_keyframes: None,
        current_frame_id: None,
        include_images: None,
    });
    let max_keyframes = input.max_keyframes.unwrap_or(10).clamp(1, 12) as usize;
    let lookback_minutes = input.lookback_minutes.unwrap_or(20).max(1);
    let bundle = build_safe_ai_export_from_conn(
        conn,
        output_root,
        SafeAiExportInput {
            lookback_minutes: Some(lookback_minutes),
            range_ms: None,
            current_frame_id: input.current_frame_id,
            include_images: input.include_images.or(Some(true)),
            max_frames: Some(120),
            export_type: Some("native_storyboard".to_string()),
        },
    )?;
    Ok(build_storyboard_from_safe_export(bundle, max_keyframes))
}

#[tauri::command]
pub fn classify_episode_transitions(
    app: AppHandle,
    input: Option<TransitionClassifierInput>,
) -> Result<Vec<ClassifiedTransition>, String> {
    let input = input.unwrap_or(TransitionClassifierInput {
        lookback_minutes: None,
        range_ms: None,
        current_frame_id: None,
    });
    let bundle = build_safe_ai_export(
        app,
        SafeAiExportInput {
            lookback_minutes: input.lookback_minutes.or(Some(20)),
            range_ms: input.range_ms,
            current_frame_id: input.current_frame_id,
            include_images: Some(false),
            max_frames: Some(160),
            export_type: Some("transition_classifier".to_string()),
        },
    )?;
    Ok(classify_safe_episode_transitions(
        &bundle.frames,
        &bundle.transitions,
    ))
}

#[tauri::command]
pub fn get_native_resume_card(
    app: AppHandle,
    input: Option<NativeResumeInput>,
) -> Result<NativeResumeCard, String> {
    let paths = capture_paths(&app)?;
    let conn = open_db(&app)?;
    get_native_resume_card_from_conn(&conn, &paths.root_dir.join("safe-ai-exports"), input)
}

fn get_native_resume_card_from_conn(
    conn: &Connection,
    output_root: &Path,
    input: Option<NativeResumeInput>,
) -> Result<NativeResumeCard, String> {
    let input = input.unwrap_or(NativeResumeInput {
        lookback_minutes: None,
        max_keyframes: None,
        current_frame_id: None,
    });
    let lookback_minutes = input.lookback_minutes.unwrap_or(20).max(1);
    let dossier = get_native_storyboard_dossier_from_conn(
        conn,
        output_root,
        Some(NativeStoryboardInput {
            lookback_minutes: Some(lookback_minutes),
            max_keyframes: input.max_keyframes.or(Some(10)),
            current_frame_id: input.current_frame_id,
            include_images: Some(true),
        }),
    )?;
    Ok(build_resume_card_from_storyboard(dossier, lookback_minutes))
}

#[tauri::command]
pub fn run_resume_eval(app: AppHandle, eval_file_path: String) -> Result<ResumeEvalReport, String> {
    let raw = fs::read_to_string(&eval_file_path).map_err(to_string)?;
    let parsed: Value = serde_json::from_str(&raw).map_err(to_string)?;
    let cases = match parsed {
        Value::Array(cases) => cases,
        Value::Object(_) => vec![parsed],
        _ => return Err("eval file must contain a JSON object or array".to_string()),
    };
    let mut reports = Vec::new();
    for case in cases {
        reports.push(run_resume_eval_case(&app, &case)?);
    }
    Ok(summarize_resume_eval_reports(reports))
}

fn build_safe_ai_export_from_conn(
    conn: &Connection,
    output_root: &Path,
    input: SafeAiExportInput,
) -> Result<SafeAiExportBundle, String> {
    ensure_ai_export_audit_table(conn)?;
    let generated_at_ms = now_millis();
    let export_id = next_id("ai-export");
    let export_type = input.export_type.unwrap_or_else(|| "ai_export".to_string());
    let include_images = input.include_images.unwrap_or(true);
    let lookback_ms = input
        .range_ms
        .or_else(|| input.lookback_minutes.map(|minutes| minutes * 60_000))
        .unwrap_or(20 * 60_000)
        .max(1_000);
    let lookback_end_ms = input
        .current_frame_id
        .and_then(|id| frame_captured_at(conn, id).ok().flatten())
        .unwrap_or(generated_at_ms);
    let lookback_start_ms = lookback_end_ms.saturating_sub(lookback_ms);
    let max_frames = input.max_frames.unwrap_or(120).clamp(1, 240);
    let frames = query_frames_between(conn, lookback_start_ms, lookback_end_ms, max_frames)?;
    let transitions = query_transitions_between(conn, lookback_start_ms, lookback_end_ms)?;
    let export_dir = output_root.join(&export_id);
    fs::create_dir_all(&export_dir).map_err(to_string)?;

    let mut warnings = Vec::new();
    let mut safe_frames = Vec::new();
    let mut excluded_frame_count = 0_usize;
    let mut masked_image_count = 0_usize;
    let mut redacted_text_count = 0_usize;

    for frame in frames.iter() {
        let frame_key = frame.id.to_string();
        let sensitive_regions = query_sensitive_regions(conn, &frame_key)?;
        let app_contexts = query_app_contexts(conn, &frame_key)?;
        let content_units = query_content_units_for_frame(conn, &frame_key)?;
        let quality_warnings = query_frame_quality_warnings(conn, &frame_key).unwrap_or_default();
        let privacy = export_privacy_for_frame(frame, &sensitive_regions);
        if privacy.exclude_all {
            excluded_frame_count += 1;
            warnings.push(format!(
                "frame {} excluded by {}",
                frame.id, privacy.exclude_reason
            ));
            continue;
        }

        let mut frame_warnings = Vec::new();
        let context = app_contexts.first();
        let mut text_was_redacted = false;
        let redacted_text = frame.full_text.as_deref().and_then(|text| {
            let redacted = redact_text_for_ai(text);
            if redacted != text {
                text_was_redacted = true;
            }
            non_empty(redacted)
        });
        let compact_units = compact_content_units_for_ai(&content_units, &mut text_was_redacted);
        if privacy.redact_text && redacted_text.is_some() {
            text_was_redacted = true;
        }
        if text_was_redacted {
            redacted_text_count += 1;
            frame_warnings.push("text redacted before AI export".to_string());
        }
        for warning in &quality_warnings {
            frame_warnings.push(format!(
                "quality warning {}: {}",
                warning.warning_type, warning.message
            ));
        }

        let (image_path_safe, active_window_crop_path_safe) = if include_images {
            let mut snapshot_safe = None;
            let mut window_safe = None;
            if privacy.mask_images {
                if let Some(path) = derive_safe_image_for_export(
                    &export_dir,
                    frame.id,
                    "full_screenshot",
                    frame
                        .full_screenshot_path
                        .as_deref()
                        .unwrap_or(&frame.snapshot_path),
                    &sensitive_regions,
                    &mut frame_warnings,
                )? {
                    snapshot_safe = Some(path);
                    masked_image_count += 1;
                }
                if let Some(path) = frame.active_window_crop_path.as_deref() {
                    if let Some(path) = derive_safe_image_for_export(
                        &export_dir,
                        frame.id,
                        "active_window",
                        path,
                        &sensitive_regions,
                        &mut frame_warnings,
                    )? {
                        window_safe = Some(path);
                        masked_image_count += 1;
                    }
                }
            } else {
                snapshot_safe = copy_safe_image_for_export(
                    &export_dir,
                    frame.id,
                    "full_screenshot",
                    frame
                        .full_screenshot_path
                        .as_deref()
                        .unwrap_or(&frame.snapshot_path),
                    &mut frame_warnings,
                )?;
                if let Some(path) = frame.active_window_crop_path.as_deref() {
                    window_safe = copy_safe_image_for_export(
                        &export_dir,
                        frame.id,
                        "active_window",
                        path,
                        &mut frame_warnings,
                    )?;
                }
            }
            (snapshot_safe, window_safe)
        } else {
            (None, None)
        };

        warnings.extend(
            frame_warnings
                .iter()
                .map(|warning| format!("frame {}: {}", frame.id, warning)),
        );
        safe_frames.push(SafeAiFrame {
            frame_id: frame.id.to_string(),
            captured_at_ms: frame.captured_at,
            app_name: frame.app_name.clone(),
            app_bundle_id: frame.app_bundle_id.clone(),
            window_name: frame.window_name.clone(),
            window_id: frame.window_id,
            browser_url: redact_url_for_ai(frame.browser_url.as_deref(), &mut redacted_text_count),
            document_path: redact_path_for_ai(
                frame.document_path.as_deref(),
                &mut redacted_text_count,
            ),
            phash: frame.phash.clone(),
            app_context_id: context.map(|context| context.id.clone()),
            app_context_object_type: context.map(|context| context.object_type.clone()),
            image_path_safe,
            active_window_crop_path_safe,
            top_content_units: compact_units,
            text_source: frame.text_source.clone(),
            text: if privacy.redact_text {
                redacted_text
            } else {
                redacted_text.or_else(|| frame.full_text.clone().and_then(non_empty))
            },
            evidence_strength: apply_quality_adjustment(
                evidence_strength(frame, &content_units, &app_contexts),
                &quality_warnings,
            ),
            privacy_status: privacy.status,
            warnings: frame_warnings,
        });
    }

    if excluded_frame_count > 0 {
        warnings.push(format!(
            "{} frames excluded by privacy policy",
            excluded_frame_count
        ));
    }
    if masked_image_count > 0 {
        warnings.push(format!(
            "{} screenshots pixel-masked before export",
            masked_image_count
        ));
    }
    if redacted_text_count > 0 {
        warnings.push(format!(
            "{} frame/url/text fields redacted before export",
            redacted_text_count
        ));
    }

    let bundle = SafeAiExportBundle {
        id: export_id.clone(),
        generated_at_ms,
        export_type: export_type.clone(),
        lookback_start_ms,
        lookback_end_ms,
        input_frame_count: frames.len(),
        exported_frame_count: safe_frames.len(),
        excluded_frame_count,
        masked_image_count,
        redacted_text_count,
        frames: safe_frames,
        transitions,
        warnings,
    };
    insert_ai_export_audit(conn, &bundle)?;
    write_json_pretty(
        &export_dir.join("safe-ai-export.json"),
        &serde_json::to_value(&bundle).map_err(to_string)?,
    )?;
    Ok(bundle)
}

#[derive(Debug)]
struct FrameExportPrivacy {
    status: String,
    mask_images: bool,
    redact_text: bool,
    exclude_all: bool,
    exclude_reason: String,
}

fn export_privacy_for_frame(
    frame: &CaptureFrame,
    sensitive_regions: &[SensitiveRegionSummary],
) -> FrameExportPrivacy {
    let frame_status = frame
        .privacy_status
        .as_deref()
        .unwrap_or("normal")
        .to_string();
    let actions = sensitive_regions
        .iter()
        .filter_map(|region| region.action_taken.as_deref())
        .collect::<Vec<_>>();
    let exclude_all = frame_status.contains("skipped")
        || actions.iter().any(|action| {
            matches!(
                *action,
                "skip_capture" | "never_send_to_ai" | "excluded_app"
            )
        });
    let redact = frame_status == "redacted"
        || !sensitive_regions.is_empty()
        || actions.iter().any(|action| action.contains("redact"));
    FrameExportPrivacy {
        status: if exclude_all {
            "excluded".to_string()
        } else if redact {
            "redacted".to_string()
        } else {
            frame_status.clone()
        },
        mask_images: redact,
        redact_text: redact,
        exclude_all,
        exclude_reason: if actions.iter().any(|action| *action == "never_send_to_ai") {
            "never_send_to_ai".to_string()
        } else if frame_status.contains("skipped") {
            "skip_capture".to_string()
        } else {
            "privacy rule".to_string()
        },
    }
}

fn build_storyboard_from_safe_export(
    bundle: SafeAiExportBundle,
    max_keyframes: usize,
) -> NativeStoryboardDossier {
    let mut frames = bundle.frames.clone();
    frames.sort_by_key(|frame| frame.captured_at_ms);
    let mut selected: Vec<(String, SafeAiFrame, String)> = Vec::new();
    let mut seen = HashSet::new();
    if let Some(current) = frames.last() {
        push_storyboard_selection(
            &mut selected,
            &mut seen,
            "current_frame",
            current.clone(),
            "latest captured frame in the lookback window",
        );
    }
    if let Some(first) = frames.first() {
        push_storyboard_selection(
            &mut selected,
            &mut seen,
            "session_start",
            first.clone(),
            "earliest exported frame in the lookback window",
        );
    }
    for pair in frames.windows(2).rev() {
        let previous = &pair[0];
        let current = &pair[1];
        if surface_key_for_safe_frame(previous) != surface_key_for_safe_frame(current) {
            push_storyboard_selection(
                &mut selected,
                &mut seen,
                "last_major_app_or_window_switch",
                previous.clone(),
                "last frame before the current surface changed",
            );
            push_storyboard_selection(
                &mut selected,
                &mut seen,
                "branch_landing",
                current.clone(),
                "first frame on a changed app/window/url surface",
            );
            break;
        }
    }
    for frame in frames.iter().rev() {
        let text = frame
            .top_content_units
            .iter()
            .map(|unit| unit.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if text.len() > 400 {
            push_storyboard_selection(
                &mut selected,
                &mut seen,
                "last_high_attention_reading_frame",
                frame.clone(),
                "substantial visible text/content units",
            );
            break;
        }
    }
    for transition in bundle.transitions.iter().take(40) {
        let transition_type = transition.transition_type.as_deref().unwrap_or("unknown");
        let target_id = transition
            .post_frame_id
            .as_deref()
            .or(transition.pre_frame_id.as_deref());
        let Some(target_id) = target_id else {
            continue;
        };
        let Some(frame) = frames.iter().find(|frame| frame.frame_id == target_id) else {
            continue;
        };
        if transition_type.contains("typing") {
            push_storyboard_selection(
                &mut selected,
                &mut seen,
                "last_typing_pause_or_commit",
                frame.clone(),
                "linked to a typing transition",
            );
        } else if transition_type.contains("clipboard") {
            push_storyboard_selection(
                &mut selected,
                &mut seen,
                "last_clipboard_source",
                frame.clone(),
                "linked to clipboard evidence",
            );
        } else if transition_type.contains("scroll") {
            push_storyboard_selection(
                &mut selected,
                &mut seen,
                "last_scroll_stop_focus_frame",
                frame.clone(),
                "linked to a scroll/focus transition",
            );
        }
        if selected.len() >= max_keyframes {
            break;
        }
    }
    let classified_transitions =
        classify_safe_episode_transitions(&bundle.frames, &bundle.transitions);
    for transition in &classified_transitions {
        if selected.len() >= max_keyframes {
            break;
        }
        let kind = match transition.transition_type.as_str() {
            "returning_to_previous_task" => "return_to_previous_surface",
            "possible_distraction" | "background_media" => "possible_distraction",
            _ => continue,
        };
        let target_id = transition
            .post_frame_id
            .as_deref()
            .or_else(|| transition.evidence_frame_ids.last().map(String::as_str));
        let Some(target_id) = target_id else {
            continue;
        };
        let Some(frame) = frames.iter().find(|frame| frame.frame_id == target_id) else {
            continue;
        };
        push_storyboard_selection(
            &mut selected,
            &mut seen,
            kind,
            frame.clone(),
            &transition.reason,
        );
    }
    while selected.len() < max_keyframes {
        let Some(frame) = frames.pop() else {
            break;
        };
        push_storyboard_selection(
            &mut selected,
            &mut seen,
            "resume_candidate",
            frame,
            "recent non-duplicate exported evidence",
        );
    }
    selected.truncate(max_keyframes);
    selected.sort_by_key(|(_, frame, _)| frame.captured_at_ms);

    let keyframes = selected
        .into_iter()
        .map(|(kind, frame, reason)| StoryboardKeyframe {
            frame_id: frame.frame_id,
            kind,
            captured_at_ms: frame.captured_at_ms,
            app_name: frame.app_name,
            app_bundle_id: frame.app_bundle_id,
            window_name: frame.window_name,
            browser_url: frame.browser_url,
            document_path: frame.document_path,
            app_context_id: frame.app_context_id,
            app_context_object_type: frame.app_context_object_type,
            image_path_safe: frame.image_path_safe,
            active_window_crop_path_safe: frame.active_window_crop_path_safe,
            top_content_units: frame.top_content_units,
            text_source: frame.text_source,
            evidence_strength: frame.evidence_strength,
            selection_reason: reason,
            privacy_status: frame.privacy_status,
        })
        .collect::<Vec<_>>();
    let storyboard_transitions =
        classify_storyboard_transitions(&bundle.frames, &bundle.transitions);
    let dominant_surfaces = dominant_surfaces(&bundle.frames);
    NativeStoryboardDossier {
        generated_at_ms: bundle.generated_at_ms,
        lookback_start_ms: bundle.lookback_start_ms,
        lookback_end_ms: bundle.lookback_end_ms,
        current_frame_id: keyframes.last().map(|frame| frame.frame_id.clone()),
        keyframes,
        transitions: storyboard_transitions,
        dominant_surfaces,
        warnings: bundle.warnings,
    }
}

fn build_resume_card_from_storyboard(
    dossier: NativeStoryboardDossier,
    lookback_minutes: i64,
) -> NativeResumeCard {
    let current = dossier
        .keyframes
        .iter()
        .rev()
        .find(|frame| frame.kind == "current_frame")
        .or_else(|| dossier.keyframes.last());
    let reading = dossier
        .keyframes
        .iter()
        .rev()
        .find(|frame| {
            frame.kind == "last_high_attention_reading_frame"
                || frame
                    .app_context_object_type
                    .as_deref()
                    .is_some_and(|kind| {
                        matches!(
                            kind,
                            "browser_tab" | "chat_conversation" | "pdf" | "notes_doc"
                        )
                    })
        })
        .or(current);
    let classifier_target_id = resume_target_frame_id_from_transitions(&dossier);
    let classifier_target = classifier_target_id
        .as_deref()
        .and_then(|id| dossier.keyframes.iter().find(|frame| frame.frame_id == id));
    let continue_from = classifier_target
        .or_else(|| {
            dossier
                .keyframes
                .iter()
                .rev()
                .find(|frame| frame.kind == "return_to_previous_surface")
        })
        .or(reading)
        .or(current);
    let evidence_frame_ids = dossier
        .keyframes
        .iter()
        .map(|frame| frame.frame_id.clone())
        .collect::<Vec<_>>();
    let evidence_transition_ids = dossier
        .transitions
        .iter()
        .map(|transition| transition.id.clone())
        .collect::<Vec<_>>();
    let surface_names = dossier
        .dominant_surfaces
        .iter()
        .take(3)
        .map(|surface| surface.surface_key.clone())
        .collect::<Vec<_>>();
    let reading_quote = continue_from.and_then(best_quote_for_keyframe);
    let title = continue_from.and_then(|frame| {
        frame
            .window_name
            .clone()
            .or_else(|| frame.browser_url.clone())
            .or_else(|| frame.document_path.clone())
    });
    let what_was_i_reading = reading
        .and_then(best_quote_for_keyframe)
        .or_else(|| title.clone());
    let what_was_i_doing = if let Some(frame) = current {
        format!(
            "You were working in {}{}.",
            frame.app_name.as_deref().unwrap_or("the current app"),
            frame
                .window_name
                .as_deref()
                .map(|name| format!(" on {}", name))
                .unwrap_or_default()
        )
    } else {
        "There is not enough exported evidence to identify the current task.".to_string()
    };
    let focus_now = if let Some(frame) = continue_from {
        format!(
            "Continue from {}{}.",
            frame
                .app_name
                .as_deref()
                .unwrap_or("the strongest evidence frame"),
            frame
                .window_name
                .as_deref()
                .map(|name| format!(": {}", name))
                .unwrap_or_default()
        )
    } else {
        "Capture a fresh frame, then ask for a resume card again.".to_string()
    };
    let why_this_focus = if dossier.transitions.iter().any(|transition| {
        transition.transition_type == "returning_to_previous_task"
            || transition.transition_type == "verification_branch"
    }) {
        "The recent surfaces look like a branch followed by a return to a previous work surface."
            .to_string()
    } else if dossier.keyframes.len() >= 2 {
        "This is the strongest recent surface with readable exported evidence.".to_string()
    } else {
        "The exported evidence is thin, so this cue stays conservative.".to_string()
    };
    let behavior_mode = infer_behavior_mode(&dossier);
    let confidence = (dossier
        .keyframes
        .iter()
        .map(|frame| frame.evidence_strength)
        .sum::<f64>()
        / dossier.keyframes.len().max(1) as f64)
        .min(0.86);
    NativeResumeCard {
        generated_at_ms: now_millis(),
        lookback_minutes,
        what_was_i_doing,
        what_was_i_reading,
        focus_now,
        why_this_focus,
        continue_from: ResumeContinueFrom {
            frame_id: continue_from.map(|frame| frame.frame_id.clone()),
            app_name: continue_from.and_then(|frame| frame.app_name.clone()),
            window_name: continue_from.and_then(|frame| frame.window_name.clone()),
            title,
            url: continue_from.and_then(|frame| frame.browser_url.clone()),
            document_path: continue_from.and_then(|frame| frame.document_path.clone()),
            quote: reading_quote,
            reason: continue_from
                .map(|frame| frame.selection_reason.clone())
                .unwrap_or_else(|| "no safe keyframe available".to_string()),
        },
        what_changed: surface_names,
        useful_evidence: dossier
            .keyframes
            .iter()
            .take(6)
            .map(|frame| format!("frame {}: {}", frame.frame_id, frame.selection_reason))
            .collect(),
        likely_distractions: likely_distractions(&dossier),
        behavior_read: ResumeBehaviorRead {
            mode: behavior_mode,
            confidence,
            notes: vec!["Inference is based on screen/app evidence, not mental state.".to_string()],
        },
        next_action:
            "Resume from the cited frame and continue the last meaningful reading or writing step."
                .to_string(),
        confidence,
        evidence_frame_ids,
        evidence_transition_ids,
        warnings: dossier.warnings,
    }
}

fn resume_target_frame_id_from_transitions(dossier: &NativeStoryboardDossier) -> Option<String> {
    if let Some(return_transition) = dossier
        .transitions
        .iter()
        .find(|transition| transition.transition_type == "returning_to_previous_task")
    {
        return return_transition
            .post_frame_id
            .clone()
            .or_else(|| return_transition.evidence_frame_ids.last().cloned());
    }
    if let Some(distraction) = dossier.transitions.iter().find(|transition| {
        matches!(
            transition.transition_type.as_str(),
            "possible_distraction" | "background_media"
        )
    }) {
        return distraction.pre_frame_id.clone();
    }
    None
}

fn query_frames_between(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
    limit: u32,
) -> Result<Vec<CaptureFrame>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {}
             FROM frames
             WHERE captured_at >= ?1 AND captured_at <= ?2
             ORDER BY captured_at DESC, id DESC
             LIMIT ?3",
            FRAME_COLUMNS
        ))
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![start_ms, end_ms, limit], frame_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_transitions_between(
    conn: &Connection,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<TransitionSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, trigger_id, primary_event_id, pre_frame_id, post_frame_id,
                    ts_start_ms, ts_end_ms, transition_type, confidence, summary
             FROM event_transitions
             WHERE ts_start_ms >= ?1 AND ts_start_ms <= ?2
             ORDER BY ts_start_ms DESC
             LIMIT 160",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![start_ms, end_ms], transition_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn frame_captured_at(conn: &Connection, frame_id: i64) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT captured_at FROM frames WHERE id = ?1",
        params![frame_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(to_string)
}

fn compact_content_units_for_ai(
    content_units: &[ContentUnitSummary],
    text_was_redacted: &mut bool,
) -> Vec<CompactContentUnit> {
    let mut units = content_units
        .iter()
        .filter(|unit| !is_low_signal_content_unit(unit))
        .collect::<Vec<_>>();
    units.sort_by(|left, right| {
        content_role_priority(right)
            .cmp(&content_role_priority(left))
            .then_with(|| {
                right
                    .confidence
                    .partial_cmp(&left.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    units
        .into_iter()
        .take(12)
        .filter_map(|unit| {
            let text = unit.text.as_deref()?.trim();
            if text.is_empty() {
                return None;
            }
            let redacted = redact_text_for_ai(text);
            if redacted != text {
                *text_was_redacted = true;
            }
            non_empty(redacted).map(|text| CompactContentUnit {
                id: unit.id.clone(),
                source: unit.source.clone(),
                unit_type: unit.unit_type.clone(),
                semantic_role: unit.semantic_role.clone(),
                text,
                confidence: unit.confidence,
            })
        })
        .collect()
}

fn is_low_signal_content_unit(unit: &ContentUnitSummary) -> bool {
    let role = unit.semantic_role.as_deref().unwrap_or("").to_lowercase();
    let unit_type = unit.unit_type.to_lowercase();
    let text = unit.text.as_deref().unwrap_or("").trim();
    text.len() < 3
        || role.contains("browser_chrome")
        || role.contains("toolbar")
        || role.contains("system_menu")
        || role.contains("sidebar")
        || unit_type.contains("button")
}

fn content_role_priority(unit: &ContentUnitSummary) -> i32 {
    match unit.semantic_role.as_deref().unwrap_or("") {
        "main_content" => 80,
        "chat_message" => 78,
        "composer" => 74,
        "code_editor" => 72,
        "terminal_output" => 70,
        "search_result" => 55,
        "browser_chrome" | "toolbar" | "system_menu" | "app_sidebar" => 0,
        _ => 40,
    }
}

fn evidence_strength(
    frame: &CaptureFrame,
    content_units: &[ContentUnitSummary],
    app_contexts: &[AppContextSummary],
) -> f64 {
    let mut score = 0.18_f64;
    if Path::new(&frame.snapshot_path).exists() {
        score += 0.18;
    }
    if frame
        .full_text
        .as_deref()
        .is_some_and(|text| text.len() > 120)
    {
        score += 0.18;
    }
    if !content_units.is_empty() {
        score += 0.24;
    }
    if !app_contexts.is_empty() {
        score += 0.12;
    }
    if frame.browser_url.is_some() || frame.document_path.is_some() {
        score += 0.1;
    }
    score.min(1.0)
}

fn apply_quality_adjustment(score: f64, warnings: &[FrameQualityWarning]) -> f64 {
    let penalty = warnings
        .iter()
        .map(|warning| match warning.severity.as_str() {
            "high" => 0.18,
            "medium" => 0.1,
            _ => 0.04,
        })
        .sum::<f64>()
        .min(0.32);
    (score - penalty).max(0.0)
}

fn redact_url_for_ai(value: Option<&str>, redacted_count: &mut usize) -> Option<String> {
    value.and_then(|value| {
        let mut redacted = redact_text_for_ai(value);
        for needle in [
            "checkout", "payment", "bank", "health", "medical", "login", "auth",
        ] {
            if redacted.to_lowercase().contains(needle) {
                redacted = "[REDACTED_URL]".to_string();
                *redacted_count += 1;
                break;
            }
        }
        non_empty(redacted)
    })
}

fn redact_path_for_ai(value: Option<&str>, redacted_count: &mut usize) -> Option<String> {
    value.and_then(|value| {
        let redacted = redact_text_for_ai(value);
        if redacted != value {
            *redacted_count += 1;
        }
        non_empty(redacted)
    })
}

fn redact_text_for_ai(input: &str) -> String {
    let mut output = Vec::new();
    for raw_token in input.split_whitespace() {
        let token = raw_token.trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == ';');
        let lower = token.to_lowercase();
        let replacement = if looks_like_email(token) {
            Some("[REDACTED_EMAIL]")
        } else if looks_like_phone(token) {
            Some("[REDACTED_PHONE]")
        } else if looks_like_long_number(token) {
            Some("[REDACTED_NUMBER]")
        } else if looks_like_secret(token, &lower) {
            Some("[REDACTED_SECRET]")
        } else if looks_like_sensitive_surface(&lower) {
            Some("[REDACTED_SENSITIVE_SURFACE]")
        } else {
            None
        };
        output.push(replacement.unwrap_or(raw_token).to_string());
    }
    output.join(" ")
}

fn looks_like_email(token: &str) -> bool {
    let Some((local, domain)) = token.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && domain.len() >= 4
}

fn looks_like_phone(token: &str) -> bool {
    let digits = token.chars().filter(|char| char.is_ascii_digit()).count();
    digits >= 10
        && token
            .chars()
            .all(|char| char.is_ascii_digit() || "+-(). ".contains(char))
}

fn looks_like_long_number(token: &str) -> bool {
    token.chars().filter(|char| char.is_ascii_digit()).count() >= 12
}

fn looks_like_secret(token: &str, lower: &str) -> bool {
    lower.starts_with("sk-")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("access_token")
        || lower.contains("refresh_token")
        || lower.contains("bearer")
        || (lower.contains("secret") && token.len() > 8)
        || (token.len() >= 28
            && token.chars().any(|char| char.is_ascii_uppercase())
            && token.chars().any(|char| char.is_ascii_digit()))
}

fn looks_like_sensitive_surface(lower: &str) -> bool {
    [
        "password", "passcode", "banking", "checkout", "payment", "medical", "health",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn derive_safe_image_for_export(
    export_dir: &Path,
    frame_id: i64,
    label: &str,
    source: &str,
    sensitive_regions: &[SensitiveRegionSummary],
    warnings: &mut Vec<String>,
) -> Result<Option<String>, String> {
    if source.trim().is_empty() {
        warnings.push(format!("{} image path missing", label));
        return Ok(None);
    }
    let source_path = Path::new(source);
    let safe_dir = export_dir.join("images");
    fs::create_dir_all(&safe_dir).map_err(to_string)?;
    let output_path = safe_dir.join(format!("frame-{:06}-safe_{}.png", frame_id, label));
    let rects = mask_rects_for_export(sensitive_regions);
    if source_path.exists()
        && !rects.is_empty()
        && mask_image_with_swift(&safe_dir, source_path, &output_path, &rects).is_ok()
    {
        return Ok(Some(output_path.to_string_lossy().to_string()));
    }

    if !source_path.exists() {
        warnings.push(format!("source image for {} does not exist", label));
    } else if rects.is_empty() {
        warnings.push(format!(
            "{} had no bounded sensitive regions; wrote fully redacted placeholder",
            label
        ));
    } else {
        warnings.push(format!(
            "{} masking helper failed; wrote fully redacted placeholder",
            label
        ));
    }
    write_fully_redacted_png(&output_path)?;
    warnings.push(format!(
        "frame {} safe {} image is fully redacted",
        frame_id, label
    ));
    Ok(Some(output_path.to_string_lossy().to_string()))
}

fn copy_safe_image_for_export(
    export_dir: &Path,
    frame_id: i64,
    label: &str,
    source: &str,
    warnings: &mut Vec<String>,
) -> Result<Option<String>, String> {
    if source.trim().is_empty() {
        warnings.push(format!("{} image path missing", label));
        return Ok(None);
    }
    let source_path = Path::new(source);
    if !source_path.exists() {
        warnings.push(format!("source image for {} does not exist", label));
        return Ok(None);
    }
    let safe_dir = export_dir.join("images");
    fs::create_dir_all(&safe_dir).map_err(to_string)?;
    let output_path = safe_dir.join(format!("frame-{:06}-safe_{}.png", frame_id, label));
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "png" {
        fs::copy(source_path, &output_path).map_err(to_string)?;
    } else if convert_image_to_png(source_path, &output_path).is_err() {
        write_fully_redacted_png(&output_path)?;
        warnings.push(format!(
            "{} safe image fully redacted because PNG conversion failed",
            label
        ));
    }
    Ok(Some(output_path.to_string_lossy().to_string()))
}

fn mask_rects_for_export(sensitive_regions: &[SensitiveRegionSummary]) -> Vec<Rect> {
    sensitive_regions
        .iter()
        .filter_map(|region| {
            Some(Rect {
                x: region.bounds_x?,
                y: region.bounds_y?,
                w: region.bounds_w?,
                h: region.bounds_h?,
            })
        })
        .filter(|rect| rect.w > 0.0 && rect.h > 0.0)
        .collect()
}

fn mask_image_with_swift(
    helper_dir: &Path,
    source_path: &Path,
    output_path: &Path,
    rects: &[Rect],
) -> Result<(), String> {
    if !cfg!(target_os = "macos") || !Path::new("/usr/bin/swiftc").exists() {
        return Err("image masking helper unavailable".to_string());
    }
    let helper =
        ensure_export_swift_helper(helper_dir, "image_mask", IMAGE_MASK_SWIFT, &["AppKit"])?;
    let rect_json = serde_json::to_string(rects).map_err(to_string)?;
    let output = Command::new(helper)
        .arg(source_path)
        .arg(output_path)
        .arg(rect_json)
        .output()
        .map_err(|error| format!("image mask helper failed to start: {}", error))?;
    if output.status.success() && output_path.exists() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "image mask helper failed".to_string()
        } else {
            stderr
        })
    }
}

fn ensure_export_swift_helper(
    helper_dir: &Path,
    name: &str,
    source: &str,
    frameworks: &[&str],
) -> Result<PathBuf, String> {
    fs::create_dir_all(helper_dir).map_err(to_string)?;
    let source_path = helper_dir.join(format!("{}.swift", name));
    let helper_path = helper_dir.join(name);
    let should_write = fs::read_to_string(&source_path)
        .map(|existing| existing != source)
        .unwrap_or(true);
    if should_write {
        fs::write(&source_path, source).map_err(to_string)?;
        let _ = fs::remove_file(&helper_path);
    }
    if !helper_path.exists() {
        let mut command = Command::new("/usr/bin/swiftc");
        command.arg("-O").arg(&source_path);
        for framework in frameworks {
            command.arg("-framework").arg(framework);
        }
        command.arg("-o").arg(&helper_path);
        let output = command
            .output()
            .map_err(|error| format!("swiftc failed: {}", error))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
    }
    Ok(helper_path)
}

fn write_fully_redacted_png(path: &Path) -> Result<(), String> {
    const BLACK_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 96, 96, 96, 248, 15,
        0, 1, 5, 1, 2, 154, 45, 66, 181, 0, 0, 0, 0, 73, 69, 68, 174, 66, 96, 130,
    ];
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(to_string)?;
    }
    fs::write(path, BLACK_PNG).map_err(to_string)
}

fn push_storyboard_selection(
    selected: &mut Vec<(String, SafeAiFrame, String)>,
    seen: &mut HashSet<String>,
    kind: &str,
    frame: SafeAiFrame,
    reason: &str,
) {
    if seen.insert(frame.frame_id.clone()) {
        selected.push((kind.to_string(), frame, reason.to_string()));
    }
}

fn surface_key_for_safe_frame(frame: &SafeAiFrame) -> String {
    [
        frame.app_bundle_id.as_deref(),
        frame.app_name.as_deref(),
        frame.browser_url.as_deref(),
        frame.document_path.as_deref(),
        frame.window_name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .next()
    .unwrap_or("unknown")
    .to_lowercase()
}

fn classify_storyboard_transitions(
    frames: &[SafeAiFrame],
    transitions: &[TransitionSummary],
) -> Vec<StoryboardTransition> {
    classify_safe_episode_transitions(frames, transitions)
        .into_iter()
        .map(|transition| StoryboardTransition {
            id: transition.id,
            transition_type: transition.transition_type,
            pre_frame_id: transition.pre_frame_id,
            post_frame_id: transition.post_frame_id,
            evidence_frame_ids: transition.evidence_frame_ids,
            evidence_event_ids: transition.evidence_event_ids,
            confidence: transition.confidence,
            reason: transition.reason,
        })
        .collect()
}

fn classify_safe_episode_transitions(
    frames: &[SafeAiFrame],
    transitions: &[TransitionSummary],
) -> Vec<ClassifiedTransition> {
    let by_id = frames
        .iter()
        .map(|frame| (frame.frame_id.as_str(), frame))
        .collect::<HashMap<_, _>>();
    let chronological = sorted_safe_frames(frames);
    let mut classified = transitions
        .iter()
        .take(80)
        .map(|transition| {
            let base = transition.transition_type.as_deref().unwrap_or("unknown");
            let pre = transition
                .pre_frame_id
                .as_deref()
                .and_then(|id| by_id.get(id).copied());
            let post = transition
                .post_frame_id
                .as_deref()
                .and_then(|id| by_id.get(id).copied());
            let classified =
                classify_transition_with_episode_context(base, pre, post, &chronological);
            ClassifiedTransition {
                id: transition.id.clone(),
                transition_type: classified.transition_type,
                pre_frame_id: transition.pre_frame_id.clone(),
                post_frame_id: transition.post_frame_id.clone(),
                return_score: classified.return_score,
                evidence_frame_ids: evidence_frame_ids_for_transition(
                    pre,
                    post,
                    classified.match_frame,
                ),
                evidence_event_ids: transition.primary_event_id.clone().into_iter().collect(),
                confidence: classified.confidence,
                reason: classified.reason,
            }
        })
        .collect::<Vec<_>>();
    promote_verification_branches(&mut classified);
    classified
}

#[derive(Debug, Clone)]
struct EpisodeClassification {
    transition_type: String,
    reason: String,
    confidence: f64,
    return_score: Option<f64>,
    match_frame: Option<String>,
}

fn classify_transition_with_episode_context(
    base: &str,
    pre: Option<&SafeAiFrame>,
    post: Option<&SafeAiFrame>,
    chronological: &[&SafeAiFrame],
) -> EpisodeClassification {
    let Some(pre) = pre else {
        return EpisodeClassification {
            transition_type: base.to_string(),
            reason: "missing pre-frame evidence".to_string(),
            confidence: 0.35,
            return_score: None,
            match_frame: None,
        };
    };
    let Some(post) = post else {
        return EpisodeClassification {
            transition_type: base.to_string(),
            reason: "missing post-frame evidence".to_string(),
            confidence: 0.35,
            return_score: None,
            match_frame: None,
        };
    };
    if let Some((matched, score)) = best_return_match(post, chronological) {
        if score >= 0.72 && has_intervening_different_surface(matched, post, chronological) {
            return EpisodeClassification {
                transition_type: "returning_to_previous_task".to_string(),
                reason: format!(
                    "return score {:.2} matched earlier frame {} after an intervening surface",
                    score, matched.frame_id
                ),
                confidence: score.min(0.92),
                return_score: Some(score),
                match_frame: Some(matched.frame_id.clone()),
            };
        }
    }
    let same_surface = surface_key_for_safe_frame(pre) == surface_key_for_safe_frame(post);
    let overlap = text_overlap_score(pre, post);
    if same_surface && base.contains("scroll") {
        EpisodeClassification {
            transition_type: "continued_reading".to_string(),
            reason: "same surface with scroll/focus movement".to_string(),
            confidence: 0.68,
            return_score: None,
            match_frame: None,
        }
    } else if base.contains("typing") {
        EpisodeClassification {
            transition_type: "entered_input".to_string(),
            reason: "transition was linked to typing/input activity".to_string(),
            confidence: 0.66,
            return_score: None,
            match_frame: None,
        }
    } else if base.contains("clipboard") || base.contains("copy") {
        EpisodeClassification {
            transition_type: "copied_evidence".to_string(),
            reason: "transition was linked to clipboard activity".to_string(),
            confidence: 0.62,
            return_score: None,
            match_frame: None,
        }
    } else if !same_surface && overlap >= 0.22 {
        EpisodeClassification {
            transition_type: "branching_for_research".to_string(),
            reason: "surface changed but exported text shares terms".to_string(),
            confidence: 0.62,
            return_score: None,
            match_frame: None,
        }
    } else if same_surface {
        EpisodeClassification {
            transition_type: "continuing_same_task".to_string(),
            reason: "same app/window/url surface".to_string(),
            confidence: 0.58,
            return_score: None,
            match_frame: None,
        }
    } else if is_media_surface(post) {
        EpisodeClassification {
            transition_type: "background_media".to_string(),
            reason: "media-like destination surface".to_string(),
            confidence: 0.56,
            return_score: None,
            match_frame: None,
        }
    } else if overlap < 0.06 {
        EpisodeClassification {
            transition_type: "possible_distraction".to_string(),
            reason: "surface changed with low text overlap".to_string(),
            confidence: 0.5,
            return_score: None,
            match_frame: None,
        }
    } else {
        EpisodeClassification {
            transition_type: base.to_string(),
            reason: "kept original transition label with limited evidence".to_string(),
            confidence: 0.42,
            return_score: None,
            match_frame: None,
        }
    }
}

fn evidence_frame_ids_for_transition(
    pre: Option<&SafeAiFrame>,
    post: Option<&SafeAiFrame>,
    matched: Option<String>,
) -> Vec<String> {
    let mut ids = Vec::new();
    for id in [
        pre.map(|frame| frame.frame_id.clone()),
        post.map(|frame| frame.frame_id.clone()),
        matched,
    ]
    .into_iter()
    .flatten()
    {
        if !ids.iter().any(|existing| existing == &id) {
            ids.push(id);
        }
    }
    ids
}

fn promote_verification_branches(transitions: &mut [ClassifiedTransition]) {
    for index in 0..transitions.len() {
        if transitions[index].transition_type != "returning_to_previous_task" {
            continue;
        }
        let returned_to = transitions[index]
            .evidence_frame_ids
            .last()
            .cloned()
            .or_else(|| transitions[index].post_frame_id.clone());
        let Some(returned_to) = returned_to else {
            continue;
        };
        for prior in transitions.iter_mut().skip(index + 1) {
            if prior.transition_type == "branching_for_research"
                && prior.pre_frame_id.as_deref() == Some(returned_to.as_str())
            {
                prior.transition_type = "verification_branch".to_string();
                prior
                    .reason
                    .push_str("; later transition returned to the prior surface");
                prior.confidence = prior.confidence.max(0.7);
                break;
            }
        }
    }
}

fn sorted_safe_frames(frames: &[SafeAiFrame]) -> Vec<&SafeAiFrame> {
    let mut sorted = frames.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|frame| (frame.captured_at_ms, frame.frame_id.clone()));
    sorted
}

fn best_return_match<'a>(
    post: &SafeAiFrame,
    chronological: &[&'a SafeAiFrame],
) -> Option<(&'a SafeAiFrame, f64)> {
    chronological
        .iter()
        .copied()
        .filter(|candidate| candidate.captured_at_ms < post.captured_at_ms)
        .filter(|candidate| candidate.frame_id != post.frame_id)
        .map(|candidate| (candidate, return_score(candidate, post)))
        .max_by(|(_, left), (_, right)| {
            left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn has_intervening_different_surface(
    earlier: &SafeAiFrame,
    post: &SafeAiFrame,
    chronological: &[&SafeAiFrame],
) -> bool {
    let earlier_surface = surface_key_for_safe_frame(earlier);
    chronological.iter().any(|candidate| {
        candidate.captured_at_ms > earlier.captured_at_ms
            && candidate.captured_at_ms < post.captured_at_ms
            && surface_key_for_safe_frame(candidate) != earlier_surface
    })
}

fn return_score(a: &SafeAiFrame, b: &SafeAiFrame) -> f64 {
    (0.25 * app_context_similarity(a, b))
        + (0.20 * url_or_document_similarity(a, b))
        + (0.20 * visible_text_anchor_similarity(a, b))
        + (0.15 * text_overlap_score(a, b))
        + (0.10 * phash_similarity(a, b))
        + (0.10 * window_identity_similarity(a, b))
}

fn app_context_similarity(a: &SafeAiFrame, b: &SafeAiFrame) -> f64 {
    let same_object_type = a.app_context_object_type.is_some()
        && a.app_context_object_type == b.app_context_object_type;
    let same_app = a.app_bundle_id.is_some() && a.app_bundle_id == b.app_bundle_id;
    match (same_object_type, same_app) {
        (true, true) => 1.0,
        (true, false) | (false, true) => 0.65,
        _ => 0.0,
    }
}

fn url_or_document_similarity(a: &SafeAiFrame, b: &SafeAiFrame) -> f64 {
    let a_target = a.browser_url.as_deref().or(a.document_path.as_deref());
    let b_target = b.browser_url.as_deref().or(b.document_path.as_deref());
    match (a_target, b_target) {
        (Some(left), Some(right)) if left == right => 1.0,
        (Some(left), Some(right))
            if url_host(left) == url_host(right) && url_host(left).is_some() =>
        {
            0.65
        }
        (Some(left), Some(right)) if !left.is_empty() && !right.is_empty() => {
            token_jaccard(left, right)
        }
        _ => 0.0,
    }
}

fn visible_text_anchor_similarity(a: &SafeAiFrame, b: &SafeAiFrame) -> f64 {
    text_overlap_score(a, b)
}

fn phash_similarity(a: &SafeAiFrame, b: &SafeAiFrame) -> f64 {
    match (a.phash.as_deref(), b.phash.as_deref()) {
        (Some(left), Some(right)) if left == right => 1.0,
        (Some(left), Some(right)) => normalized_hex_similarity(left, right),
        _ => 0.0,
    }
}

fn window_identity_similarity(a: &SafeAiFrame, b: &SafeAiFrame) -> f64 {
    if a.window_id.is_some() && a.window_id == b.window_id {
        1.0
    } else if a.window_name.is_some() && a.window_name == b.window_name {
        1.0
    } else if a.app_name.is_some() && a.app_name == b.app_name {
        0.45
    } else {
        0.0
    }
}

fn normalized_hex_similarity(left: &str, right: &str) -> f64 {
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let compared = left.len().min(right.len());
    if compared == 0 {
        return 0.0;
    }
    let equal = left
        .chars()
        .zip(right.chars())
        .take(compared)
        .filter(|(left, right)| left == right)
        .count();
    equal as f64 / left.len().max(right.len()) as f64
}

fn url_host(value: &str) -> Option<String> {
    let after_scheme = value.split("://").nth(1).unwrap_or(value);
    after_scheme
        .split('/')
        .next()
        .map(str::to_lowercase)
        .filter(|host| !host.is_empty())
}

fn token_jaccard(left: &str, right: &str) -> f64 {
    let left_terms = text_terms(left);
    let right_terms = text_terms(right);
    if left_terms.is_empty() || right_terms.is_empty() {
        return 0.0;
    }
    let overlap = left_terms.intersection(&right_terms).count();
    let union = left_terms.union(&right_terms).count();
    overlap as f64 / union.max(1) as f64
}

fn text_terms(value: &str) -> HashSet<String> {
    value
        .split(|char: char| !char.is_ascii_alphanumeric())
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 4)
        .collect()
}

fn text_overlap_score(a: &SafeAiFrame, b: &SafeAiFrame) -> f64 {
    let a_terms = significant_terms(a);
    let b_terms = significant_terms(b);
    if a_terms.is_empty() || b_terms.is_empty() {
        return 0.0;
    }
    let overlap = a_terms.intersection(&b_terms).count();
    overlap as f64 / a_terms.len().min(b_terms.len()).max(1) as f64
}

fn significant_terms(frame: &SafeAiFrame) -> HashSet<String> {
    let text = frame
        .top_content_units
        .iter()
        .map(|unit| unit.text.as_str())
        .chain(frame.text.as_deref())
        .collect::<Vec<_>>()
        .join(" ");
    text.split(|char: char| !char.is_ascii_alphanumeric())
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 5)
        .take(80)
        .collect()
}

fn is_media_surface(frame: &SafeAiFrame) -> bool {
    let haystack = format!(
        "{} {} {}",
        frame.app_name.as_deref().unwrap_or(""),
        frame.window_name.as_deref().unwrap_or(""),
        frame.browser_url.as_deref().unwrap_or("")
    )
    .to_lowercase();
    ["youtube", "spotify", "netflix", "music", "video", "podcast"]
        .iter()
        .any(|needle| haystack.contains(needle))
}

fn dominant_surfaces(frames: &[SafeAiFrame]) -> Vec<SurfaceSummary> {
    let mut by_surface: HashMap<String, SurfaceSummary> = HashMap::new();
    for frame in frames {
        let key = surface_key_for_safe_frame(frame);
        by_surface
            .entry(key.clone())
            .and_modify(|surface| {
                surface.frame_count += 1;
                surface.first_seen_ms = surface.first_seen_ms.min(frame.captured_at_ms);
                surface.last_seen_ms = surface.last_seen_ms.max(frame.captured_at_ms);
            })
            .or_insert_with(|| SurfaceSummary {
                surface_key: key,
                app_name: frame.app_name.clone(),
                window_name: frame.window_name.clone(),
                url_or_document: frame.browser_url.clone().or(frame.document_path.clone()),
                frame_count: 1,
                first_seen_ms: frame.captured_at_ms,
                last_seen_ms: frame.captured_at_ms,
            });
    }
    let mut surfaces = by_surface.into_values().collect::<Vec<_>>();
    surfaces.sort_by(|a, b| b.frame_count.cmp(&a.frame_count));
    surfaces
}

fn best_quote_for_keyframe(frame: &StoryboardKeyframe) -> Option<String> {
    frame
        .top_content_units
        .iter()
        .find(|unit| unit.text.len() >= 30)
        .map(|unit| truncate_chars(&unit.text, 220))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        let mut out = value
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect::<String>();
        out.push_str("...");
        out
    }
}

fn infer_behavior_mode(dossier: &NativeStoryboardDossier) -> String {
    if dossier
        .transitions
        .iter()
        .any(|transition| transition.transition_type == "possible_distraction")
    {
        "switching".to_string()
    } else if dossier
        .keyframes
        .iter()
        .any(|frame| frame.kind == "last_typing_pause_or_commit")
    {
        "writing".to_string()
    } else if dossier
        .keyframes
        .iter()
        .any(|frame| frame.kind == "last_high_attention_reading_frame")
    {
        "focused_reading".to_string()
    } else {
        "unknown".to_string()
    }
}

fn likely_distractions(dossier: &NativeStoryboardDossier) -> Vec<String> {
    dossier
        .transitions
        .iter()
        .filter(|transition| transition.transition_type == "possible_distraction")
        .map(|transition| transition.reason.clone())
        .take(4)
        .collect()
}

fn ensure_ai_export_audit_table(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_export_audit (
          id TEXT PRIMARY KEY,
          created_at_ms INTEGER NOT NULL,
          export_type TEXT NOT NULL,
          lookback_start_ms INTEGER,
          lookback_end_ms INTEGER,
          input_frame_count INTEGER NOT NULL,
          exported_frame_count INTEGER NOT NULL,
          excluded_frame_count INTEGER NOT NULL,
          masked_image_count INTEGER NOT NULL,
          redacted_text_count INTEGER NOT NULL,
          warnings_json TEXT NOT NULL
        )",
        [],
    )
    .map_err(to_string)?;
    Ok(())
}

fn ensure_frame_quality_warnings_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS frame_quality_warnings (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          warning_type TEXT NOT NULL,
          severity TEXT NOT NULL,
          message TEXT NOT NULL,
          evidence_json TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_frame_quality_warnings_frame
          ON frame_quality_warnings(frame_id);
        ",
    )
    .map_err(to_string)
}

fn insert_ai_export_audit(conn: &Connection, bundle: &SafeAiExportBundle) -> Result<(), String> {
    ensure_ai_export_audit_table(conn)?;
    let warnings_json = serde_json::to_string(&bundle.warnings).map_err(to_string)?;
    conn.execute(
        "INSERT INTO ai_export_audit (
            id, created_at_ms, export_type, lookback_start_ms, lookback_end_ms,
            input_frame_count, exported_frame_count, excluded_frame_count,
            masked_image_count, redacted_text_count, warnings_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            bundle.id,
            bundle.generated_at_ms,
            bundle.export_type,
            bundle.lookback_start_ms,
            bundle.lookback_end_ms,
            bundle.input_frame_count as i64,
            bundle.exported_frame_count as i64,
            bundle.excluded_frame_count as i64,
            bundle.masked_image_count as i64,
            bundle.redacted_text_count as i64,
            warnings_json,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn run_resume_eval_case(app: &AppHandle, case: &Value) -> Result<ResumeEvalCaseReport, String> {
    let human = case.get("human_label").unwrap_or(&Value::Null);
    let model_output = match case.get("model_output") {
        Some(value) if !value.is_null() && value != &serde_json::json!({}) => value.clone(),
        _ => serde_json::to_value(get_native_resume_card(
            app.clone(),
            Some(NativeResumeInput {
                lookback_minutes: Some(20),
                max_keyframes: Some(10),
                current_frame_id: None,
            }),
        )?)
        .map_err(to_string)?,
    };
    let what_was_i_doing = model_output
        .get("what_was_i_doing")
        .and_then(Value::as_str)
        .unwrap_or("");
    let what_was_i_reading = model_output
        .get("what_was_i_reading")
        .and_then(Value::as_str)
        .unwrap_or("");
    let focus_now = model_output
        .get("focus_now")
        .and_then(Value::as_str)
        .unwrap_or("");
    let why_this_focus = model_output
        .get("why_this_focus")
        .and_then(Value::as_str)
        .unwrap_or("");
    let warnings_count = model_output
        .get("warnings")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let evidence_transition_ids = model_output
        .get("evidence_transition_ids")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let unknown_transition_count = if evidence_transition_ids == 0 { 1 } else { 0 };
    let model_all_text = serde_json::to_string(&model_output).map_err(to_string)?;
    let must_not_say_ok = human
        .get("must_not_say")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter().filter_map(Value::as_str).all(|forbidden| {
                !model_all_text
                    .to_lowercase()
                    .contains(&forbidden.to_lowercase())
            })
        })
        .unwrap_or(true);
    Ok(ResumeEvalCaseReport {
        session_id: case
            .get("session_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        task_identification: label_score(human, "what_was_i_doing", what_was_i_doing),
        reading_identification: label_score(human, "what_was_i_reading", what_was_i_reading),
        resume_target: resume_target_score(human, &model_output),
        why_explanation: label_score(human, "focus_now", focus_now).max(label_score(
            human,
            "why_explanation",
            why_this_focus,
        )),
        distraction_handling: distraction_score(human, &model_output),
        hallucination_control: if must_not_say_ok { 1.0 } else { 0.0 },
        warnings_count,
        unknown_transition_count,
        redacted_frame_handling_ok: !model_all_text.contains("sk-")
            && !model_all_text.contains("@example.com")
            && !model_all_text.to_lowercase().contains("password"),
    })
}

fn summarize_resume_eval_reports(cases: Vec<ResumeEvalCaseReport>) -> ResumeEvalReport {
    let count = cases.len().max(1) as f64;
    ResumeEvalReport {
        evaluated_at_ms: now_millis(),
        case_count: cases.len(),
        average_task_identification_score: cases
            .iter()
            .map(|case| case.task_identification)
            .sum::<f64>()
            / count,
        average_resume_target_score: cases.iter().map(|case| case.resume_target).sum::<f64>()
            / count,
        average_hallucination_control_score: cases
            .iter()
            .map(|case| case.hallucination_control)
            .sum::<f64>()
            / count,
        warnings_frequency: cases.iter().filter(|case| case.warnings_count > 0).count() as f64
            / count,
        unknown_transition_frequency: cases
            .iter()
            .filter(|case| case.unknown_transition_count > 0)
            .count() as f64
            / count,
        redacted_frame_handling_correctness: cases
            .iter()
            .filter(|case| case.redacted_frame_handling_ok)
            .count() as f64
            / count,
        cases,
    }
}

fn label_score(human: &Value, field: &str, actual: &str) -> f64 {
    let expected = human.get(field).and_then(Value::as_str).unwrap_or("");
    if expected.trim().is_empty() {
        return 0.0;
    }
    token_jaccard(expected, actual)
}

fn resume_target_score(human: &Value, model_output: &Value) -> f64 {
    let expected = human
        .get("continue_from_frame_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if expected.trim().is_empty() {
        return label_score(
            human,
            "focus_now",
            model_output
                .get("focus_now")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
    }
    let actual = model_output
        .get("continue_from")
        .and_then(|value| value.get("frame_id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if actual == expected {
        1.0
    } else {
        0.0
    }
}

fn distraction_score(human: &Value, model_output: &Value) -> f64 {
    let expected = human
        .get("distractions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if expected.trim().is_empty() {
        return 1.0;
    }
    let actual = model_output
        .get("likely_distractions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    token_jaccard(&expected, &actual)
}

fn capture_loop(
    app: AppHandle,
    state: Arc<Mutex<CaptureRuntime>>,
    stop_signal: Arc<AtomicBool>,
    session_id: String,
) {
    let mut last_idle_capture = Instant::now();
    let mut last_capture_at = Instant::now()
        .checked_sub(MIN_CAPTURE_INTERVAL)
        .unwrap_or_else(Instant::now);
    let mut previous_image_hash: Option<String> = None;
    let mut previous_content_hash: Option<String> = None;
    let mut previous_semantic_fingerprint: Option<SemanticFingerprint> = None;
    let mut pending_trigger: Option<PendingTrigger> = None;
    let mut typing_burst = TypingBurstState::default();
    let mut event_source = match capture_paths(&app)
        .and_then(|paths| start_capture_event_source(&paths))
    {
        Ok(source) => Some(source),
        Err(error) => {
            update_error_and_island(&app, &state, format!("event source unavailable: {}", error));
            None
        }
    };

    if capture_and_emit(
        &app,
        &state,
        &session_id,
        "session_start",
        false,
        &mut previous_image_hash,
        &mut previous_content_hash,
        &mut previous_semantic_fingerprint,
        &stop_signal,
        None,
        None,
    )
    .is_ok()
    {
        last_capture_at = Instant::now();
    }

    while !stop_signal.load(Ordering::Relaxed) {
        if let Some(source) = event_source.as_ref() {
            match source.rx.recv_timeout(EVENT_LOOP_WAKE_INTERVAL) {
                Ok(raw_trigger) => {
                    if let Err(error) = queue_event_trigger(
                        &app,
                        &session_id,
                        &mut pending_trigger,
                        &mut typing_burst,
                        &raw_trigger,
                    ) {
                        update_error_and_island(&app, &state, error);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    update_error_and_island(&app, &state, "event source stopped".to_string());
                    event_source = None;
                }
            }
        } else {
            thread::sleep(EVENT_LOOP_WAKE_INTERVAL);
        }

        if let Some(source) = event_source.as_ref() {
            while let Ok(raw_trigger) = source.rx.try_recv() {
                if let Err(error) = queue_event_trigger(
                    &app,
                    &session_id,
                    &mut pending_trigger,
                    &mut typing_burst,
                    &raw_trigger,
                ) {
                    update_error_and_island(&app, &state, error);
                }
            }
        }

        let pending_ready = pending_trigger
            .as_ref()
            .is_some_and(|trigger| Instant::now() >= trigger.ready_at);
        if pending_ready && last_capture_at.elapsed() >= MIN_CAPTURE_INTERVAL {
            if let Some(trigger) = pending_trigger.take() {
                match capture_and_emit(
                    &app,
                    &state,
                    &session_id,
                    &trigger.capture_trigger,
                    true,
                    &mut previous_image_hash,
                    &mut previous_content_hash,
                    &mut previous_semantic_fingerprint,
                    &stop_signal,
                    Some(trigger.id.clone()),
                    trigger.pre_frame_id.clone(),
                ) {
                    Ok(stored) => {
                        if stored {
                            last_idle_capture = Instant::now();
                        }
                        last_capture_at = Instant::now();
                    }
                    Err(error) => {
                        let _ = mark_capture_trigger_failed(&app, &trigger.id, &error);
                        update_error_and_island(&app, &state, error)
                    }
                }
            }
        }

        if last_idle_capture.elapsed() >= IDLE_CAPTURE_INTERVAL
            && last_capture_at.elapsed() >= MIN_CAPTURE_INTERVAL
        {
            match capture_and_emit(
                &app,
                &state,
                &session_id,
                "idle",
                true,
                &mut previous_image_hash,
                &mut previous_content_hash,
                &mut previous_semantic_fingerprint,
                &stop_signal,
                None,
                latest_frame_id_for_session(&app, &session_id)
                    .ok()
                    .flatten(),
            ) {
                Ok(stored) => {
                    last_idle_capture = Instant::now();
                    if stored {
                        last_capture_at = Instant::now();
                    }
                }
                Err(error) => update_error_and_island(&app, &state, error),
            }
        }
    }

    if let Some(source) = event_source {
        source.shutdown();
    }

    if let Ok(mut runtime) = state.lock() {
        runtime.running = false;
        runtime.started_at = None;
        runtime.stop_signal = None;
    }

    let _ = app.emit(
        "capture-status",
        capture_status_snapshot_inner(&app, &state),
    );
}

fn capture_and_emit(
    app: &AppHandle,
    state: &Arc<Mutex<CaptureRuntime>>,
    session_id: &str,
    capture_trigger: &str,
    dedupe: bool,
    previous_image_hash: &mut Option<String>,
    previous_content_hash: &mut Option<String>,
    previous_semantic_fingerprint: &mut Option<SemanticFingerprint>,
    stop_signal: &AtomicBool,
    capture_trigger_id: Option<String>,
    pre_frame_id: Option<String>,
) -> Result<bool, String> {
    let trigger_id = match capture_trigger_id {
        Some(id) => Some(id),
        None => Some(insert_system_capture_trigger(
            app,
            session_id,
            capture_trigger,
            pre_frame_id.clone(),
            dedupe,
        )?),
    };
    let outcome = capture_frame(
        app,
        session_id,
        capture_trigger,
        dedupe,
        previous_image_hash.as_deref(),
        previous_content_hash.as_deref(),
        None,
        Some(stop_signal),
        trigger_id.as_deref(),
        pre_frame_id.as_deref(),
    )?;

    *previous_image_hash = Some(outcome.image_hash.clone());
    *previous_content_hash = outcome
        .content_hash
        .clone()
        .or_else(|| previous_content_hash.clone());
    *previous_semantic_fingerprint = Some(outcome.semantic_fingerprint.clone());

    let stored = outcome.frame.is_some();
    if let Some(id) = trigger_id.as_deref() {
        let _ = finalize_capture_trigger_by_id(app, id, stored);
    }

    if let Some(frame) = outcome.frame {
        update_success(state, frame.clone());
        let _ = app.emit("capture-frame", frame);
        if let Ok(status) = capture_status_snapshot_inner(app, state) {
            crate::session_island::update_session_island_from_status(
                &status,
                crate::session_island::SessionIslandState::RecordingCompact,
            );
        }
        Ok(true)
    } else {
        update_skip(state);
        Ok(false)
    }
}

fn queue_event_trigger(
    app: &AppHandle,
    session_id: &str,
    pending: &mut Option<PendingTrigger>,
    typing_burst: &mut TypingBurstState,
    raw_event: &str,
) -> Result<(), String> {
    let Some(mut event) = parse_ui_event(raw_event) else {
        return Ok(());
    };

    if event.event_type == "helper_started" {
        return Ok(());
    }

    let (capture_trigger, settle_delay) = match normalize_event_trigger(&event.event_type) {
        Some(value) => value,
        None => return Ok(()),
    };

    let conn = open_db(app)?;
    if event.id.is_empty() {
        event.id = next_id("evt");
    }
    insert_ui_event(&conn, session_id, &event)?;
    record_event_side_effects(&conn, session_id, &event, typing_burst)?;

    let pre_frame_id = latest_frame_id_for_session_from_conn(&conn, session_id)?;
    let now = now_millis();
    if let Some(existing) = pending.as_mut() {
        existing.caused_by_event_ids.push(event.id.clone());
        existing.ready_at = Instant::now() + settle_delay;
        existing.settle_delay_ms = settle_delay.as_millis() as i64;
        if existing.capture_trigger != capture_trigger {
            existing.capture_trigger = "event_burst".to_string();
        }
        update_capture_trigger_events(&conn, existing)?;
        return Ok(());
    }

    let trigger = PendingTrigger {
        id: next_id("trg"),
        capture_trigger,
        caused_by_event_ids: vec![event.id.clone()],
        pre_frame_id,
        settle_delay_ms: settle_delay.as_millis() as i64,
        ready_at: Instant::now() + settle_delay,
    };
    insert_capture_trigger(&conn, session_id, &trigger, now, false, "event_bucket")?;
    *pending = Some(trigger);
    Ok(())
}

fn parse_ui_event(raw_event: &str) -> Option<UiEventRecord> {
    let trimmed = raw_event.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('{') {
        let mut event: UiEventRecord = serde_json::from_str(trimmed).ok()?;
        if event.ts_ms == 0 {
            event.ts_ms = now_millis();
        }
        return Some(event);
    }

    Some(UiEventRecord {
        id: String::new(),
        ts_ms: now_millis(),
        event_type: trimmed.to_string(),
        ..UiEventRecord::default()
    })
}

fn normalize_event_trigger(event_type: &str) -> Option<(String, Duration)> {
    match event_type.trim() {
        "app_switch" => Some(("app_switch".to_string(), Duration::from_millis(300))),
        "window_focus" => Some(("window_focus".to_string(), Duration::from_millis(300))),
        "accessibility_change" | "ax_notification" => Some((
            "accessibility_change".to_string(),
            Duration::from_millis(300),
        )),
        "click" => Some(("click".to_string(), Duration::from_millis(220))),
        "key_down" => Some(("typing_pause".to_string(), Duration::from_millis(850))),
        "scroll" => Some(("scroll_stop".to_string(), Duration::from_millis(500))),
        "clipboard" => Some(("clipboard".to_string(), Duration::from_millis(220))),
        _ => None,
    }
}

fn next_id(prefix: &str) -> String {
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{}", prefix, now_millis(), seq)
}

fn latest_frame_id_for_session(
    app: &AppHandle,
    session_id: &str,
) -> Result<Option<String>, String> {
    let conn = open_db(app)?;
    latest_frame_id_for_session_from_conn(&conn, session_id)
}

fn latest_frame_id_for_session_from_conn(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT CAST(id AS TEXT)
         FROM frames
         WHERE session_id = ?1
         ORDER BY captured_at DESC
         LIMIT 1",
        params![session_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(to_string)
}

fn insert_ui_event(
    conn: &Connection,
    session_id: &str,
    event: &UiEventRecord,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO ui_events (
            id, ts_ms, event_type, app_pid, app_bundle_id, app_name, window_id,
            window_title, x, y, button, scroll_dx, scroll_dy, key_category,
            modifier_flags, is_repeat, payload_json, created_at_ms, session_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                   ?14, ?15, ?16, ?17, ?18, ?19)",
        params![
            event.id,
            event.ts_ms,
            event.event_type,
            event.app_pid,
            event.app_bundle_id,
            event.app_name,
            event.window_id,
            event.window_title,
            event.x,
            event.y,
            event.button,
            event.scroll_dx,
            event.scroll_dy,
            event.key_category,
            event.modifier_flags,
            event.is_repeat.map(bool_to_i64),
            event.payload.as_ref().map(Value::to_string),
            now_millis(),
            session_id,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn record_event_side_effects(
    conn: &Connection,
    session_id: &str,
    event: &UiEventRecord,
    typing_burst: &mut TypingBurstState,
) -> Result<(), String> {
    match event.event_type.as_str() {
        "clipboard" => record_clipboard_event(conn, session_id, event),
        "key_down" => record_typing_event(conn, session_id, event, typing_burst),
        _ => Ok(()),
    }
}

fn record_clipboard_event(
    conn: &Connection,
    session_id: &str,
    event: &UiEventRecord,
) -> Result<(), String> {
    let payload = event.payload.as_ref().and_then(Value::as_object);
    let get_payload = |key: &str| {
        payload
            .and_then(|map| map.get(key))
            .and_then(Value::as_str)
            .map(str::to_string)
    };

    let change_count = get_payload("change_count")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    let content_type = get_payload("content_type").unwrap_or_else(|| "unknown".to_string());
    let byte_size = get_payload("byte_size").and_then(|value| value.parse::<i64>().ok());
    let source_frame_id = latest_frame_id_for_session_from_conn(conn, session_id)?;

    conn.execute(
        "INSERT OR IGNORE INTO clipboard_events (
            id, ts_ms, change_count, content_type, text_hash, redacted_preview,
            byte_size, source_frame_id, metadata_json, session_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            next_id("clip"),
            event.ts_ms,
            change_count,
            content_type,
            get_payload("text_hash"),
            get_payload("redacted_preview"),
            byte_size,
            source_frame_id,
            event.payload.as_ref().map(Value::to_string),
            session_id,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn record_typing_event(
    conn: &Connection,
    session_id: &str,
    event: &UiEventRecord,
    typing_burst: &mut TypingBurstState,
) -> Result<(), String> {
    let now = event.ts_ms;
    let should_start =
        typing_burst.id.is_none() || now.saturating_sub(typing_burst.ended_at_ms) > 1_200;
    if should_start {
        let id = next_id("type");
        let pre_frame_id = latest_frame_id_for_session_from_conn(conn, session_id)?;
        conn.execute(
            "INSERT INTO typing_bursts (
                id, started_at_ms, ended_at_ms, app_pid, app_bundle_id, app_name,
                window_id, window_title, char_count, backspace_count, enter_count,
                paste_count, shortcut_count, committed, commit_signal,
                raw_text_captured, pre_frame_id, session_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, 0, ?16, ?17)",
            params![
                id,
                now,
                now,
                event.app_pid,
                event.app_bundle_id,
                event.app_name,
                event.window_id,
                event.window_title,
                key_count(event, "char"),
                key_count(event, "backspace"),
                key_count(event, "enter"),
                paste_count(event),
                shortcut_count(event),
                if event.key_category.as_deref() == Some("enter") {
                    1
                } else {
                    0
                },
                if event.key_category.as_deref() == Some("enter") {
                    Some("enter")
                } else {
                    None
                },
                pre_frame_id,
                session_id,
            ],
        )
        .map_err(to_string)?;
        typing_burst.id = Some(id);
        typing_burst.started_at_ms = now;
        typing_burst.ended_at_ms = now;
        return Ok(());
    }

    if let Some(id) = typing_burst.id.as_deref() {
        conn.execute(
            "UPDATE typing_bursts
             SET ended_at_ms = ?2,
                 char_count = char_count + ?3,
                 backspace_count = backspace_count + ?4,
                 enter_count = enter_count + ?5,
                 paste_count = paste_count + ?6,
                 shortcut_count = shortcut_count + ?7,
                 committed = CASE WHEN ?8 = 1 THEN 1 ELSE committed END,
                 commit_signal = CASE WHEN ?8 = 1 THEN 'enter' ELSE commit_signal END
             WHERE id = ?1",
            params![
                id,
                now,
                key_count(event, "char"),
                key_count(event, "backspace"),
                key_count(event, "enter"),
                paste_count(event),
                shortcut_count(event),
                if event.key_category.as_deref() == Some("enter") {
                    1
                } else {
                    0
                },
            ],
        )
        .map_err(to_string)?;
        typing_burst.ended_at_ms = now;
    }

    Ok(())
}

fn key_count(event: &UiEventRecord, category: &str) -> i64 {
    if event.key_category.as_deref() == Some(category) && event.is_repeat != Some(true) {
        1
    } else {
        0
    }
}

fn shortcut_count(event: &UiEventRecord) -> i64 {
    if event.key_category.as_deref() == Some("shortcut") && event.is_repeat != Some(true) {
        1
    } else {
        0
    }
}

fn paste_count(event: &UiEventRecord) -> i64 {
    let payload = event
        .payload
        .as_ref()
        .map(Value::to_string)
        .unwrap_or_default();
    if event.key_category.as_deref() == Some("shortcut") && payload.contains("paste") {
        1
    } else {
        0
    }
}

fn insert_system_capture_trigger(
    app: &AppHandle,
    session_id: &str,
    trigger_type: &str,
    pre_frame_id: Option<String>,
    dedupe: bool,
) -> Result<String, String> {
    let conn = open_db(app)?;
    let trigger = PendingTrigger {
        id: next_id("trg"),
        capture_trigger: trigger_type.to_string(),
        caused_by_event_ids: Vec::new(),
        pre_frame_id,
        settle_delay_ms: 0,
        ready_at: Instant::now(),
    };
    insert_capture_trigger(
        &conn,
        session_id,
        &trigger,
        now_millis(),
        false,
        if dedupe { "layered" } else { "manual_bypass" },
    )?;
    Ok(trigger.id)
}

fn insert_capture_trigger(
    conn: &Connection,
    session_id: &str,
    trigger: &PendingTrigger,
    ts_ms: i64,
    rate_limited: bool,
    dedupe_policy: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO capture_triggers (
            id, ts_ms, trigger_type, caused_by_event_ids, settle_delay_ms,
            rate_limited, dedupe_policy, pre_frame_id, status, session_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'scheduled', ?9)",
        params![
            trigger.id,
            ts_ms,
            trigger.capture_trigger,
            serde_json::to_string(&trigger.caused_by_event_ids).map_err(to_string)?,
            trigger.settle_delay_ms,
            bool_to_i64(rate_limited),
            dedupe_policy,
            trigger.pre_frame_id,
            session_id,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn update_capture_trigger_events(
    conn: &Connection,
    trigger: &PendingTrigger,
) -> Result<(), String> {
    conn.execute(
        "UPDATE capture_triggers
         SET trigger_type = ?2,
             caused_by_event_ids = ?3,
             settle_delay_ms = ?4
         WHERE id = ?1",
        params![
            trigger.id,
            trigger.capture_trigger,
            serde_json::to_string(&trigger.caused_by_event_ids).map_err(to_string)?,
            trigger.settle_delay_ms,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn finalize_capture_trigger_by_id(
    app: &AppHandle,
    trigger_id: &str,
    stored: bool,
) -> Result<(), String> {
    let conn = open_db(app)?;
    let post_frame_id = conn
        .query_row(
            "SELECT CAST(id AS TEXT)
             FROM frames
             WHERE capture_trigger_id = ?1
             ORDER BY captured_at DESC
             LIMIT 1",
            params![trigger_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?;

    let status = if stored { "captured" } else { "skipped" };
    conn.execute(
        "UPDATE capture_triggers
         SET status = ?2, post_frame_id = ?3
         WHERE id = ?1",
        params![trigger_id, status, post_frame_id],
    )
    .map_err(to_string)?;

    let trigger = conn
        .query_row(
            "SELECT ts_ms, trigger_type, caused_by_event_ids, pre_frame_id, post_frame_id,
                    session_id
             FROM capture_triggers
             WHERE id = ?1",
            params![trigger_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(to_string)?;

    let Some((ts_ms, trigger_type, caused_by_event_ids, pre_frame_id, post_frame_id, session_id)) =
        trigger
    else {
        return Ok(());
    };
    let event_ids: Vec<String> = serde_json::from_str(&caused_by_event_ids).unwrap_or_default();
    let primary_event_id = event_ids.first().cloned();
    let transition_type = classify_transition_type(
        &conn,
        &trigger_type,
        pre_frame_id.as_deref(),
        post_frame_id.as_deref(),
        primary_event_id.as_deref(),
    )?;
    let summary = transition_summary(&trigger_type, &transition_type, stored);

    conn.execute(
        "INSERT OR IGNORE INTO event_transitions (
            id, trigger_id, primary_event_id, pre_frame_id, post_frame_id,
            ts_start_ms, ts_end_ms, transition_type, confidence, summary,
            changed_region_json, session_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            next_id("tx"),
            trigger_id,
            primary_event_id,
            pre_frame_id,
            post_frame_id,
            ts_ms,
            now_millis(),
            transition_type,
            0.68_f64,
            summary,
            "{}",
            session_id,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn mark_capture_trigger_failed(
    app: &AppHandle,
    trigger_id: &str,
    error: &str,
) -> Result<(), String> {
    let conn = open_db(app)?;
    conn.execute(
        "UPDATE capture_triggers SET status = 'failed', error = ?2 WHERE id = ?1",
        params![trigger_id, error],
    )
    .map_err(to_string)?;
    Ok(())
}

fn classify_transition_type(
    conn: &Connection,
    trigger_type: &str,
    pre_frame_id: Option<&str>,
    post_frame_id: Option<&str>,
    primary_event_id: Option<&str>,
) -> Result<String, String> {
    if post_frame_id.is_none() {
        return Ok("same_screen_idle".to_string());
    }
    match trigger_type {
        "app_switch" => return Ok("switched_app".to_string()),
        "scroll_stop" => return Ok("scrolled_to_new_section".to_string()),
        "typing_pause" => return Ok("entered_input".to_string()),
        "clipboard" => return Ok("copying_evidence".to_string()),
        _ => {}
    }

    if let Some(event_id) = primary_event_id {
        let event_type = conn
            .query_row(
                "SELECT event_type FROM ui_events WHERE id = ?1",
                params![event_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(to_string)?;
        if event_type.as_deref() == Some("click") {
            return Ok("unknown".to_string());
        }
    }

    let Some(pre_frame_id) = pre_frame_id else {
        return Ok("unknown".to_string());
    };
    let Some(post_frame_id) = post_frame_id else {
        return Ok("unknown".to_string());
    };
    let same_surface = conn
        .query_row(
            "SELECT
               COALESCE(a.app_name, '') = COALESCE(b.app_name, '')
               AND COALESCE(a.window_name, '') = COALESCE(b.window_name, '')
               AND COALESCE(a.browser_url, '') = COALESCE(b.browser_url, '')
             FROM frames a, frames b
             WHERE CAST(a.id AS TEXT) = ?1 AND CAST(b.id AS TEXT) = ?2",
            params![pre_frame_id, post_frame_id],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(to_string)?
        .unwrap_or(false);

    Ok(if same_surface {
        "continuing_same_task".to_string()
    } else {
        "new_task".to_string()
    })
}

fn transition_summary(trigger_type: &str, transition_type: &str, stored: bool) -> String {
    if !stored {
        return format!(
            "{} produced no stored post-frame after dedupe",
            trigger_type
        );
    }
    format!("{} classified as {}", trigger_type, transition_type)
}

fn start_capture_event_source(paths: &CapturePaths) -> Result<CaptureEventSource, String> {
    if !cfg!(target_os = "macos") {
        return Err("native UI event capture is only implemented for macOS".to_string());
    }

    let helper_path = ensure_swift_helper(
        paths,
        "capture_events",
        CAPTURE_EVENTS_SWIFT,
        &["ApplicationServices", "AppKit"],
    )?;
    let mut child = Command::new(helper_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("capture event helper failed to start: {}", error))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "capture event helper did not expose stdout".to_string())?;
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    Ok(CaptureEventSource {
        child,
        reader: Some(reader),
        rx,
    })
}

impl CaptureEventSource {
    fn shutdown(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn capture_frame(
    app: &AppHandle,
    session_id: &str,
    capture_trigger: &str,
    dedupe: bool,
    previous_image_hash: Option<&str>,
    previous_content_hash: Option<&str>,
    precollected_context: Option<AccessibilityContext>,
    cancellation: Option<&AtomicBool>,
    capture_trigger_id: Option<&str>,
    previous_frame_id: Option<&str>,
) -> Result<CaptureOutcome, String> {
    let paths = capture_paths(app)?;
    fs::create_dir_all(&paths.snapshot_dir).map_err(to_string)?;
    ensure_db(app)?;

    let captured_at = now_millis();
    let day = day_bucket(captured_at);
    let day_dir = paths.snapshot_dir.join(day);
    fs::create_dir_all(&day_dir).map_err(to_string)?;

    let context = precollected_context.unwrap_or_else(|| collect_accessibility_context(&paths));
    let semantic_fingerprint = SemanticFingerprint::from_context(&context);
    let privacy = privacy_decision(app, &context)?;
    if privacy.skip_capture {
        return Ok(CaptureOutcome {
            frame: None,
            image_hash: stable_hash_bytes(
                format!("skipped:{}:{}", capture_trigger, captured_at).as_bytes(),
            ),
            content_hash: None,
            semantic_fingerprint,
        });
    }

    let window_snapshot = collect_window_snapshot(&paths).ok();
    let active_window_id = context.window_id.or_else(|| {
        window_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.active_window_id)
    });

    let snapshot_path = day_dir.join(format!("{}_full.jpg", captured_at));
    capture_screenshot(&snapshot_path)?;
    let image_bytes = fs::read(&snapshot_path).map_err(to_string)?;
    let image_hash = stable_hash_bytes(&image_bytes);
    let image_dimensions = jpeg_dimensions(&image_bytes);
    let active_window_crop_path = if let Some(window_id) = active_window_id {
        let crop_path = day_dir.join(format!("{}_window.jpg", captured_at));
        match capture_window_screenshot(window_id, &crop_path) {
            Ok(()) => Some(crop_path),
            Err(_) => None,
        }
    } else {
        None
    };

    if cancellation.is_some_and(|signal| signal.load(Ordering::Relaxed)) {
        let _ = fs::remove_file(&snapshot_path);
        if let Some(path) = active_window_crop_path.as_ref() {
            let _ = fs::remove_file(path);
        }
        return Ok(CaptureOutcome {
            frame: None,
            image_hash,
            content_hash: None,
            semantic_fingerprint: SemanticFingerprint::default(),
        });
    }

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
    let (text_source, full_text) = resolve_text(
        accessibility_text.as_deref(),
        ocr_text.as_deref(),
        a11y_is_thin,
    );
    let content_hash = full_text
        .as_deref()
        .map(|text| stable_hash_bytes(text.as_bytes()));

    if cancellation.is_some_and(|signal| signal.load(Ordering::Relaxed)) {
        let _ = fs::remove_file(&snapshot_path);
        if let Some(path) = active_window_crop_path.as_ref() {
            let _ = fs::remove_file(path);
        }
        return Ok(CaptureOutcome {
            frame: None,
            image_hash,
            content_hash,
            semantic_fingerprint,
        });
    }

    if should_skip_dedup(
        dedupe,
        previous_image_hash,
        &image_hash,
        previous_content_hash,
        content_hash.as_deref(),
    ) {
        let _ = fs::remove_file(&snapshot_path);
        if let Some(path) = active_window_crop_path.as_ref() {
            let _ = fs::remove_file(path);
        }
        return Ok(CaptureOutcome {
            frame: None,
            image_hash,
            content_hash,
            semantic_fingerprint,
        });
    }

    if cancellation.is_some_and(|signal| signal.load(Ordering::Relaxed)) {
        let _ = fs::remove_file(&snapshot_path);
        if let Some(path) = active_window_crop_path.as_ref() {
            let _ = fs::remove_file(path);
        }
        return Ok(CaptureOutcome {
            frame: None,
            image_hash,
            content_hash,
            semantic_fingerprint,
        });
    }

    let conn = open_db(app)?;
    let metadata = FrameMetadata {
        capture_provider: "screencapture_cli".to_string(),
        scope: if active_window_id.is_some() {
            "active_window".to_string()
        } else {
            "active_display".to_string()
        },
        display_id: window_snapshot
            .as_ref()
            .and_then(|_| Some("main".to_string())),
        window_id: active_window_id,
        app_pid: context.app_pid.or_else(|| {
            window_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.active_app_pid)
        }),
        app_bundle_id: context.app_bundle_id.clone().or_else(|| {
            window_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.active_app_bundle_id.clone())
        }),
        screen_scale: 1.0,
        pixel_width: image_dimensions.map(|(width, _)| width),
        pixel_height: image_dimensions.map(|(_, height)| height),
        full_screenshot_path: snapshot_path.to_string_lossy().to_string(),
        active_window_crop_path: active_window_crop_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        active_element_crop_path: None,
        phash: Some(image_hash.clone()),
        privacy_status: privacy.status.clone(),
    };

    conn.execute(
        INSERT_FRAME_SQL,
        params![
            captured_at,
            snapshot_path.to_string_lossy().to_string(),
            context.app_name.clone(),
            context.window_name.clone(),
            context.browser_url.clone(),
            context.document_path.clone(),
            true,
            capture_trigger,
            text_source,
            accessibility_text.clone(),
            accessibility_tree_json,
            full_text.clone(),
            content_hash.clone(),
            image_hash,
            captured_at,
            metadata.capture_provider,
            metadata.scope,
            metadata.display_id,
            metadata.window_id,
            metadata.app_pid,
            metadata.app_bundle_id,
            metadata.screen_scale,
            metadata.pixel_width,
            metadata.pixel_height,
            metadata.full_screenshot_path,
            metadata.active_window_crop_path,
            metadata.active_element_crop_path,
            metadata.phash,
            metadata.privacy_status,
            capture_trigger_id,
            previous_frame_id,
            session_id,
        ],
    )
    .map_err(to_string)?;

    let frame_id = conn.last_insert_rowid();
    let mut ocr_text_for_fts = None;
    if let Some(text) = ocr_text {
        ocr_text_for_fts = Some(text.clone());
        conn.execute(
            "INSERT INTO ocr_text (frame_id, text, text_json, ocr_engine)
             VALUES (?1, ?2, ?3, ?4)",
            params![frame_id, text, ocr.text_json, ocr.engine],
        )
        .map_err(to_string)?;
    }
    if let Some(snapshot) = window_snapshot.as_ref() {
        persist_window_snapshot(&conn, frame_id, snapshot)?;
    }
    let ax_node_ids = persist_ax_nodes(&conn, frame_id, &context)?;
    let ocr_span_ids = persist_ocr_spans(
        &conn,
        frame_id,
        &ocr,
        metadata.pixel_width,
        metadata.pixel_height,
    )?;
    persist_app_contexts(&conn, frame_id, &context)?;
    persist_content_units(
        &conn,
        frame_id,
        &context,
        &ax_node_ids,
        &ocr_span_ids,
        metadata.pixel_width,
        metadata.pixel_height,
    )?;
    persist_sensitive_regions(&conn, frame_id, &context, &privacy)?;
    persist_presence_sample(&conn, session_id, &context)?;
    let _ = validate_frame_consistency_inner(
        &conn,
        &CaptureFrame {
            id: frame_id,
            captured_at,
            snapshot_path: snapshot_path.to_string_lossy().to_string(),
            app_name: context.app_name.clone(),
            window_name: context.window_name.clone(),
            browser_url: context.browser_url.clone(),
            document_path: context.document_path.clone(),
            focused: true,
            capture_trigger: capture_trigger.to_string(),
            text_source: text_source.map(str::to_string),
            accessibility_text: accessibility_text.clone(),
            accessibility_tree_json: None,
            full_text: full_text.clone(),
            content_hash: content_hash.clone(),
            image_hash: None,
            capture_provider: Some(metadata.capture_provider.clone()),
            scope: Some(metadata.scope.clone()),
            display_id: metadata.display_id.clone(),
            window_id: metadata.window_id,
            app_pid: metadata.app_pid,
            app_bundle_id: metadata.app_bundle_id.clone(),
            screen_scale: Some(metadata.screen_scale),
            pixel_width: metadata.pixel_width,
            pixel_height: metadata.pixel_height,
            full_screenshot_path: Some(metadata.full_screenshot_path.clone()),
            active_window_crop_path: metadata.active_window_crop_path.clone(),
            active_element_crop_path: metadata.active_element_crop_path.clone(),
            phash: metadata.phash.clone(),
            privacy_status: Some(metadata.privacy_status.clone()),
            capture_trigger_id: capture_trigger_id.map(str::to_string),
            previous_frame_id: previous_frame_id.map(str::to_string),
            session_id: Some(session_id.to_string()),
        },
    );
    if let Some(previous) = previous_frame_id {
        persist_frame_diff(
            &conn,
            session_id,
            previous,
            &frame_id.to_string(),
            capture_trigger,
            full_text.as_deref(),
            ocr_text_for_fts.as_deref(),
        )?;
    }

    let mut frame =
        get_frame(app.clone(), frame_id)?.ok_or_else(|| "stored frame missing".to_string())?;
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
        semantic_fingerprint,
    })
}

fn persist_window_snapshot(
    conn: &Connection,
    frame_id: i64,
    snapshot: &WindowSnapshotPayload,
) -> Result<(), String> {
    let snapshot_id = next_id("win-snap");
    conn.execute(
        "INSERT INTO window_snapshots (
            id, frame_id, ts_ms, active_window_id, active_app_pid,
            active_app_bundle_id, screen_count, raw_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            snapshot_id,
            frame_id.to_string(),
            snapshot.ts_ms,
            snapshot.active_window_id,
            snapshot.active_app_pid,
            snapshot.active_app_bundle_id,
            snapshot.screen_count,
            serde_json::to_string(snapshot).map_err(to_string)?,
        ],
    )
    .map_err(to_string)?;

    for window in &snapshot.windows {
        let bounds = window.bounds.clone().unwrap_or_default();
        conn.execute(
            "INSERT INTO windows (
                id, window_snapshot_id, cg_window_id, owner_pid, owner_name,
                bundle_id, window_title, layer, alpha, is_onscreen, is_active,
                bounds_x, bounds_y, bounds_w, bounds_h, workspace, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                next_id("win"),
                snapshot_id.clone(),
                window.cg_window_id,
                window.owner_pid,
                window.owner_name.clone(),
                window.bundle_id.clone(),
                window.window_title.clone(),
                window.layer,
                window.alpha,
                window.is_onscreen.map(bool_to_i64),
                bool_to_i64(window.is_active),
                window.bounds.as_ref().map(|_| bounds.x),
                window.bounds.as_ref().map(|_| bounds.y),
                window.bounds.as_ref().map(|_| bounds.w),
                window.bounds.as_ref().map(|_| bounds.h),
                window.workspace,
                serde_json::to_string(&window.raw).map_err(to_string)?,
            ],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn persist_ax_nodes(
    conn: &Connection,
    frame_id: i64,
    context: &AccessibilityContext,
) -> Result<HashMap<String, String>, String> {
    let mut ids = HashMap::new();
    for (index, node) in context.nodes.iter().enumerate() {
        let local_id = node
            .local_id
            .clone()
            .unwrap_or_else(|| format!("node-{}", index));
        let id = format!("ax-{}-{}", frame_id, sanitize_id(&local_id));
        ids.insert(local_id.clone(), id.clone());
        let parent_id = node
            .parent_id
            .as_ref()
            .and_then(|parent| ids.get(parent))
            .cloned();
        let bounds = node.bounds.clone().unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO ax_nodes (
                id, frame_id, parent_id, app_pid, window_id, role, subrole,
                role_description, title, value, description, help, identifier,
                document, url, selected_text, selected_text_range_json,
                visible_character_range_json, number_of_characters, focused,
                enabled, selected, bounds_x, bounds_y, bounds_w, bounds_h,
                actions_json, children_count, depth, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22,
                       ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30)",
            params![
                id,
                frame_id.to_string(),
                parent_id,
                context.app_pid,
                context.window_id,
                non_empty(node.role.clone()),
                node.subrole.clone(),
                node.role_description.clone(),
                node.title.clone(),
                node.value.clone(),
                node.description.clone(),
                node.help.clone(),
                node.identifier.clone(),
                node.document.clone(),
                node.url.clone(),
                node.selected_text.clone(),
                node.selected_text_range.as_ref().map(Value::to_string),
                node.visible_character_range.as_ref().map(Value::to_string),
                node.number_of_characters,
                node.focused.map(bool_to_i64),
                node.enabled.map(bool_to_i64),
                node.selected.map(bool_to_i64),
                node.bounds.as_ref().map(|_| bounds.x),
                node.bounds.as_ref().map(|_| bounds.y),
                node.bounds.as_ref().map(|_| bounds.w),
                node.bounds.as_ref().map(|_| bounds.h),
                serde_json::to_string(&node.actions).map_err(to_string)?,
                node.children_count,
                i64::from(node.depth),
                serde_json::to_string(node).map_err(to_string)?,
            ],
        )
        .map_err(to_string)?;
    }
    Ok(ids)
}

fn persist_ocr_spans(
    conn: &Connection,
    frame_id: i64,
    ocr: &OcrOutput,
    pixel_width: Option<i64>,
    pixel_height: Option<i64>,
) -> Result<Vec<String>, String> {
    let elements: Vec<VisionOcrElement> = serde_json::from_str(&ocr.text_json).unwrap_or_default();
    let width = pixel_width.unwrap_or(1).max(1) as f64;
    let height = pixel_height.unwrap_or(1).max(1) as f64;
    let mut ids = Vec::new();

    for (index, element) in elements.iter().enumerate() {
        if element.text.trim().is_empty() {
            continue;
        }
        let id = format!("ocr-{}-{}", frame_id, index);
        ids.push(id.clone());
        let normalized = serde_json::json!({
            "left": element.left,
            "top": element.top,
            "width": element.width,
            "height": element.height
        });
        conn.execute(
            "INSERT OR REPLACE INTO ocr_spans (
                id, frame_id, engine, text, confidence, lang, block_index,
                line_index, word_index, bounds_x, bounds_y, bounds_w, bounds_h,
                normalized_bounds_json, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0, ?6, NULL, ?7, ?8, ?9,
                       ?10, ?11, ?12)",
            params![
                id,
                frame_id.to_string(),
                ocr.engine.clone(),
                element.text.trim(),
                element.confidence,
                index as i64,
                element.left * width,
                element.top * height,
                element.width * width,
                element.height * height,
                normalized.to_string(),
                serde_json::to_string(element).map_err(to_string)?,
            ],
        )
        .map_err(to_string)?;
    }
    Ok(ids)
}

fn persist_app_contexts(
    conn: &Connection,
    frame_id: i64,
    context: &AccessibilityContext,
) -> Result<(), String> {
    let app_name = context.app_name.as_deref().unwrap_or("");
    let url = context.browser_url.clone();
    let document_path = context.document_path.clone();
    let selected_text = context
        .nodes
        .iter()
        .find_map(|node| node.selected_text.clone());

    let (adapter_id, object_type, confidence) = classify_app_context(app_name, url.as_deref());
    let title = context
        .window_name
        .clone()
        .or_else(|| context.app_name.clone());
    let primary_id = url
        .clone()
        .or_else(|| document_path.clone())
        .or_else(|| title.clone());
    let focused_object = context
        .nodes
        .iter()
        .find(|node| node.focused == Some(true))
        .map(node_text);
    let metadata = serde_json::json!({
        "app_name": context.app_name.clone(),
        "bundle_id": context.app_bundle_id.clone(),
        "browser_url": context.browser_url.clone(),
        "document_path": context.document_path.clone(),
        "adapter_version": 1
    });

    conn.execute(
        "INSERT INTO app_contexts (
            id, frame_id, adapter_id, object_type, primary_id, title, url,
            file_path, repo_path, line_number, selected_text, focused_object,
            confidence, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9, ?10, ?11, ?12)",
        params![
            next_id("appctx"),
            frame_id.to_string(),
            adapter_id,
            object_type,
            primary_id,
            title,
            url,
            document_path,
            selected_text,
            focused_object,
            confidence,
            metadata.to_string(),
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn persist_content_units(
    conn: &Connection,
    frame_id: i64,
    context: &AccessibilityContext,
    ax_node_ids: &HashMap<String, String>,
    ocr_span_ids: &[String],
    pixel_width: Option<i64>,
    pixel_height: Option<i64>,
) -> Result<(), String> {
    let now = now_millis();
    let width = pixel_width.unwrap_or(1).max(1) as f64;
    let height = pixel_height.unwrap_or(1).max(1) as f64;

    for (index, node) in context.nodes.iter().enumerate() {
        let text = node_text(node);
        if text.trim().is_empty() {
            continue;
        }
        let local_id = node
            .local_id
            .clone()
            .unwrap_or_else(|| format!("node-{}", index));
        let ax_node_id = ax_node_ids.get(&local_id).cloned();
        let bounds = node.bounds.clone().unwrap_or_default();
        let has_bounds = node.bounds.is_some();
        let unit_type = unit_type_from_role(&node.role);
        let semantic_role = semantic_role_for_text(&text, &node.role);
        conn.execute(
            "INSERT INTO content_units (
                id, frame_id, window_id, source, unit_type, text, text_hash,
                semantic_role, ax_node_id, ocr_span_ids, bounds_x, bounds_y,
                bounds_w, bounds_h, visible_ratio, center_distance, confidence,
                created_at_ms, raw_json
             ) VALUES (?1, ?2, ?3, 'ax', ?4, ?5, ?6, ?7, ?8, '[]', ?9, ?10,
                       ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                format!("unit-{}-ax-{}", frame_id, index),
                frame_id.to_string(),
                context.window_id,
                unit_type,
                text,
                stable_hash_bytes(text.as_bytes()),
                semantic_role,
                ax_node_id,
                if has_bounds { Some(bounds.x) } else { None },
                if has_bounds { Some(bounds.y) } else { None },
                if has_bounds { Some(bounds.w) } else { None },
                if has_bounds { Some(bounds.h) } else { None },
                if has_bounds { Some(1.0_f64) } else { None },
                if has_bounds {
                    Some(center_distance(&bounds, width, height))
                } else {
                    None
                },
                if node.focused == Some(true) {
                    0.92
                } else {
                    0.78
                },
                now,
                serde_json::to_string(node).map_err(to_string)?,
            ],
        )
        .map_err(to_string)?;
    }

    for (index, span_id) in ocr_span_ids.iter().enumerate() {
        let span = conn
            .query_row(
                "SELECT text, bounds_x, bounds_y, bounds_w, bounds_h FROM ocr_spans WHERE id = ?1",
                params![span_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, f64>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, f64>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(to_string)?;
        let Some((text, x, y, w, h)) = span else {
            continue;
        };
        if text.trim().is_empty() || context.text.contains(&text) {
            continue;
        }
        let ocr_span_ids = serde_json::to_string(&vec![span_id]).map_err(to_string)?;
        conn.execute(
            "INSERT INTO content_units (
                id, frame_id, source, unit_type, text, text_hash, semantic_role,
                ocr_span_ids, bounds_x, bounds_y, bounds_w, bounds_h, visible_ratio,
                center_distance, confidence, created_at_ms, raw_json
             ) VALUES (?1, ?2, 'ocr', 'unknown', ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                       ?10, 1.0, ?11, 0.64, ?12, ?13)",
            params![
                format!("unit-{}-ocr-{}", frame_id, index),
                frame_id.to_string(),
                text,
                stable_hash_bytes(text.as_bytes()),
                semantic_role_for_text(&text, ""),
                ocr_span_ids,
                x,
                y,
                w,
                h,
                center_distance(&Rect { x, y, w, h }, width, height),
                now,
                serde_json::json!({ "ocr_span_id": span_id }).to_string(),
            ],
        )
        .map_err(to_string)?;
    }

    Ok(())
}

fn persist_sensitive_regions(
    conn: &Connection,
    frame_id: i64,
    _context: &AccessibilityContext,
    privacy: &PrivacyDecision,
) -> Result<(), String> {
    for region in &privacy.regions {
        let bounds = region.bounds.clone().unwrap_or_default();
        conn.execute(
            "INSERT INTO sensitive_regions (
                id, frame_id, region_type, bounds_x, bounds_y, bounds_w, bounds_h,
                source, confidence, action_taken, metadata_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                next_id("sens"),
                frame_id.to_string(),
                region.region_type,
                region.bounds.as_ref().map(|_| bounds.x),
                region.bounds.as_ref().map(|_| bounds.y),
                region.bounds.as_ref().map(|_| bounds.w),
                region.bounds.as_ref().map(|_| bounds.h),
                region.source,
                region.confidence,
                region.action_taken,
                region.metadata_json,
            ],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn persist_presence_sample(
    conn: &Connection,
    session_id: &str,
    context: &AccessibilityContext,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO presence_samples (
            id, ts_ms, idle_seconds, display_asleep, screen_locked,
            active_input_recently, cursor_moved_recently, frontmost_app_bundle_id,
            session_id
         ) VALUES (?1, ?2, NULL, 0, 0, 1, 1, ?3, ?4)",
        params![
            next_id("presence"),
            now_millis(),
            context.app_bundle_id.clone(),
            session_id,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn persist_frame_diff(
    conn: &Connection,
    session_id: &str,
    previous_frame_id: &str,
    frame_id: &str,
    capture_trigger: &str,
    full_text: Option<&str>,
    ocr_text: Option<&str>,
) -> Result<(), String> {
    let previous = conn
        .query_row(
            "SELECT app_name, window_name, browser_url, content_hash, image_hash
             FROM frames WHERE CAST(id AS TEXT) = ?1",
            params![previous_frame_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(to_string)?;
    let current = conn
        .query_row(
            "SELECT app_name, window_name, browser_url, content_hash, image_hash
             FROM frames WHERE CAST(id AS TEXT) = ?1",
            params![frame_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(to_string)?;

    let diff_type = match capture_trigger {
        "scroll_stop" => "scrolled_to_new_section",
        "app_switch" => "switched_app",
        "typing_pause" => "entered_input",
        _ => {
            if previous.as_ref().map(|p| (&p.0, &p.1, &p.2))
                == current.as_ref().map(|c| (&c.0, &c.1, &c.2))
            {
                "same_screen_idle"
            } else {
                "unknown"
            }
        }
    };

    let text_hash = full_text
        .or(ocr_text)
        .map(|text| stable_hash_bytes(text.as_bytes()));
    conn.execute(
        "INSERT INTO frame_diffs (
            id, from_frame_id, to_frame_id, ts_ms, same_app, same_window,
            visual_phash_distance, changed_region_json, added_text_hashes,
            removed_text_hashes, stable_text_hashes, ax_tree_delta_json,
            ocr_delta_json, diff_type, confidence, summary, session_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, '{}', ?7, '[]', '[]', '{}',
                   '{}', ?8, ?9, ?10, ?11)",
        params![
            next_id("diff"),
            previous_frame_id,
            frame_id,
            now_millis(),
            previous
                .as_ref()
                .zip(current.as_ref())
                .map(|(p, c)| bool_to_i64(p.0 == c.0)),
            previous
                .as_ref()
                .zip(current.as_ref())
                .map(|(p, c)| bool_to_i64(p.1 == c.1)),
            serde_json::to_string(&text_hash.into_iter().collect::<Vec<_>>()).map_err(to_string)?,
            diff_type,
            0.58_f64,
            format!("{} -> {}", previous_frame_id, frame_id),
            session_id,
        ],
    )
    .map_err(to_string)?;
    Ok(())
}

fn classify_app_context(app_name: &str, url: Option<&str>) -> (&'static str, &'static str, f64) {
    let app = app_name.to_lowercase();
    let url = url.unwrap_or("").to_lowercase();
    if is_browser_app(&app) {
        if url.contains("chatgpt.com") || url.contains("chat.openai.com") {
            return ("chatgpt_browser_adapter", "chat_conversation", 0.9);
        }
        if url.contains("claude.ai") || url.contains("perplexity.ai") {
            return ("ai_chat_browser_adapter", "chat_conversation", 0.86);
        }
        if url.contains("notion.so") || url.contains("notion.site") {
            return ("notion_browser_adapter", "notes_doc", 0.86);
        }
        if url.contains("linear.app") {
            return ("linear_browser_adapter", "notes_doc", 0.82);
        }
        if is_media_url(&url) {
            return ("media_browser_adapter", "media", 0.86);
        }
        if url.ends_with(".pdf") || url.contains("/pdf") {
            return ("pdf_browser_adapter", "pdf", 0.78);
        }
        return ("browser_adapter", "browser_tab", 0.82);
    }
    if app.contains("chatgpt") || app.contains("claude") || app.contains("perplexity") {
        return ("native_chat_adapter", "chat_conversation", 0.86);
    }
    if app.contains("cursor")
        || app.contains("visual studio code")
        || app.contains("code")
        || app.contains("xcode")
        || app.contains("intellij")
    {
        return ("ide_adapter", "code_editor", 0.66);
    }
    if app.contains("terminal") || app.contains("iterm") || app.contains("warp") {
        return ("terminal_adapter", "terminal", 0.66);
    }
    if app.contains("preview") || app.contains("pdf") {
        return ("pdf_adapter", "pdf", 0.64);
    }
    if app.contains("finder") {
        return ("finder_adapter", "finder", 0.7);
    }
    if app.contains("youtube") || app.contains("spotify") || app.contains("music") {
        return ("media_app_adapter", "media", 0.8);
    }
    if app.contains("slack")
        || app.contains("discord")
        || app.contains("messages")
        || app.contains("whatsapp")
    {
        return ("messaging_adapter", "messaging", 0.58);
    }
    if app.contains("notion") || app.contains("linear") || app.contains("notes") {
        return ("notes_task_adapter", "notes_doc", 0.58);
    }
    ("generic_ax_adapter", "unknown", 0.44)
}

fn is_browser_app(app: &str) -> bool {
    [
        "safari", "chrome", "arc", "brave", "edge", "vivaldi", "opera", "chromium",
    ]
    .iter()
    .any(|needle| app.contains(needle))
}

fn is_media_url(url: &str) -> bool {
    [
        "youtube.com",
        "youtu.be",
        "spotify.com",
        "music.apple.com",
        "netflix.com",
        "twitch.tv",
    ]
    .iter()
    .any(|needle| url.contains(needle))
}

fn unit_type_from_role(role: &str) -> &'static str {
    let role = role.to_lowercase();
    if role.contains("button") {
        "button"
    } else if role.contains("textfield") || role.contains("text area") || role.contains("input") {
        "input"
    } else if role.contains("link") {
        "link"
    } else if role.contains("menu") {
        "menu_item"
    } else if role.contains("cell") {
        "table_cell"
    } else if role.contains("image") {
        "image"
    } else if role.contains("heading") {
        "heading"
    } else if role.contains("statictext") || role.contains("text") {
        "paragraph"
    } else {
        "unknown"
    }
}

fn semantic_role_for_text(text: &str, role: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let role = role.to_lowercase();
    if role.contains("toolbar") || role.contains("menubar") || role.contains("menu") {
        return Some("toolbar".to_string());
    }
    if lower.contains("address and search bar")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower == "back"
        || lower == "forward"
        || lower == "reload"
        || lower == "new tab"
    {
        return Some("browser_chrome".to_string());
    }
    if lower.contains("history")
        || lower.contains("settings")
        || lower.contains("sidebar")
        || lower.contains("workspace")
    {
        return Some("app_sidebar".to_string());
    }
    if role.contains("textfield") || role.contains("text area") {
        return Some("composer".to_string());
    }
    if lower.contains("error") || lower.contains("failed") || lower.contains("exception") {
        return Some("error".to_string());
    }
    if lower.contains("search") {
        return Some("search_result".to_string());
    }
    if lower.contains("function ")
        || lower.contains("const ")
        || lower.contains("let ")
        || lower.contains("class ")
    {
        return Some("code_editor".to_string());
    }
    if lower.contains("$ ") || lower.contains("error:") || lower.contains("warning:") {
        return Some("terminal_output".to_string());
    }
    if lower.contains("assistant") || lower.contains("chatgpt") || lower.contains("claude") {
        return Some("chat_message".to_string());
    }
    Some("main_content".to_string())
}

fn node_text(node: &AccessibilityNode) -> String {
    let mut pieces = Vec::new();
    for value in [
        node.text.as_str(),
        node.title.as_deref().unwrap_or(""),
        node.value.as_deref().unwrap_or(""),
        node.description.as_deref().unwrap_or(""),
        node.selected_text.as_deref().unwrap_or(""),
        node.document.as_deref().unwrap_or(""),
        node.url.as_deref().unwrap_or(""),
    ] {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !pieces.iter().any(|piece: &&str| *piece == trimmed) {
            pieces.push(trimmed);
        }
    }
    pieces.join(" ")
}

fn center_distance(bounds: &Rect, width: f64, height: f64) -> f64 {
    let cx = bounds.x + bounds.w / 2.0;
    let cy = bounds.y + bounds.h / 2.0;
    let dx = (cx - width / 2.0) / width.max(1.0);
    let dy = (cy - height / 2.0) / height.max(1.0);
    (dx * dx + dy * dy).sqrt()
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
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

#[cfg(test)]
fn semantic_trigger(
    previous: Option<&SemanticFingerprint>,
    current: &SemanticFingerprint,
) -> Option<&'static str> {
    let previous = previous?;

    if previous.app_name != current.app_name {
        return Some("app_switch");
    }

    if previous.window_name != current.window_name {
        return Some("window_focus");
    }

    if previous.browser_url != current.browser_url
        || previous.document_path != current.document_path
    {
        return Some("navigation");
    }

    if previous.text_hash != current.text_hash {
        return Some("accessibility_change");
    }

    None
}

fn should_skip_dedup(
    dedupe: bool,
    previous_image_hash: Option<&str>,
    current_image_hash: &str,
    previous_content_hash: Option<&str>,
    current_content_hash: Option<&str>,
) -> bool {
    if !dedupe {
        return false;
    }

    let same_image = previous_image_hash.is_some_and(|prev| prev == current_image_hash);
    let same_content = match (previous_content_hash, current_content_hash) {
        (Some(previous), Some(current)) => previous == current,
        (None, None) => true,
        _ => false,
    };

    same_image && same_content
}

fn capture_status_snapshot(
    app: &AppHandle,
    state: &State<CaptureState>,
) -> Result<CaptureStatus, String> {
    capture_status_snapshot_inner(app, &state.inner)
}

fn capture_status_snapshot_inner(
    app: &AppHandle,
    runtime_state: &Arc<Mutex<CaptureRuntime>>,
) -> Result<CaptureStatus, String> {
    let paths = capture_paths(app)?;
    let _ = ensure_db(app);
    let conn = open_db(app).ok();

    let runtime = runtime_state
        .lock()
        .map_err(|_| "capture runtime lock poisoned".to_string())?;
    let running = runtime.running;
    let started_at = runtime.started_at;
    let last_error = runtime.last_error.clone();
    let runtime_latest_frame = runtime.last_frame.clone();
    let active_session_id = runtime.active_session_id.clone();
    let runtime_last_export = runtime.last_export.clone();
    let skipped_samples = runtime.skipped_samples;
    let last_skipped_at = runtime.last_skipped_at;
    drop(runtime);

    let active_session = conn.as_ref().and_then(|conn| {
        active_session_id
            .as_deref()
            .and_then(|id| load_capture_session(conn, id).ok())
    });
    let latest_session = conn
        .as_ref()
        .and_then(|conn| load_latest_session(conn).ok().flatten());
    let scoped_session_id = active_session
        .as_ref()
        .or(latest_session.as_ref())
        .map(|session| session.id.as_str());
    let counts = conn
        .as_ref()
        .and_then(|conn| {
            scoped_session_id
                .map(|session_id| session_counts(conn, session_id).ok())
                .flatten()
        })
        .unwrap_or_default();
    let session_count = conn
        .as_ref()
        .and_then(|conn| row_count(conn, "capture_sessions").ok())
        .unwrap_or(0);

    let latest_frame = conn.as_ref().and_then(|conn| {
        conn.query_row(
            &format!(
                "SELECT {}
             FROM frames
             WHERE (?1 IS NULL OR session_id = ?1)
             ORDER BY captured_at DESC
             LIMIT 1",
                FRAME_COLUMNS
            ),
            params![scoped_session_id],
            frame_from_row,
        )
        .optional()
        .ok()
        .flatten()
    });

    Ok(CaptureStatus {
        running,
        frame_count: counts.frames,
        event_count: counts.events,
        transition_count: counts.transitions,
        content_unit_count: counts.content_units,
        session_count,
        active_session,
        latest_session,
        last_export: runtime_last_export,
        started_at,
        last_error,
        latest_frame: latest_frame.or(runtime_latest_frame),
        skipped_samples,
        last_skipped_at,
        data_dir: paths.root_dir.to_string_lossy().to_string(),
        database_path: paths.db_path.to_string_lossy().to_string(),
        screenshot_tool: Path::new("/usr/sbin/screencapture").exists(),
        accessibility_tool: Path::new("/usr/bin/osascript").exists(),
        ocr_tool: Path::new("/usr/bin/swiftc").exists()
            || Path::new("/usr/bin/swift").exists()
            || command_in_path("tesseract"),
    })
}

fn latest_frames(
    conn: &Connection,
    limit: u32,
    session_id: Option<&str>,
) -> Result<Vec<SearchResult>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {},
                    substr(coalesce(full_text, ''), 1, 260) AS snippet
             FROM frames
             WHERE (?2 IS NULL OR session_id = ?2)
             ORDER BY captured_at DESC
             LIMIT ?1",
            FRAME_COLUMNS
        ))
        .map_err(to_string)?;

    let rows = stmt
        .query_map(params![limit, session_id], |row| {
            Ok(SearchResult {
                frame: frame_from_row(row)?,
                snippet: row.get(32)?,
                rank: 0.0,
            })
        })
        .map_err(to_string)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_frames_since(
    conn: &Connection,
    since: i64,
    session_id: Option<&str>,
) -> Result<Vec<CaptureFrame>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {}
             FROM frames
             WHERE captured_at >= ?1
               AND (?2 IS NULL OR session_id = ?2)
             ORDER BY captured_at DESC
             LIMIT 120",
            FRAME_COLUMNS
        ))
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![since, session_id], frame_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_ui_events(
    conn: &Connection,
    since: i64,
    session_id: Option<&str>,
) -> Result<Vec<UiEventSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, ts_ms, event_type, app_name, window_title, key_category, payload_json
             FROM ui_events
             WHERE ts_ms >= ?1
               AND (?2 IS NULL OR session_id = ?2)
             ORDER BY ts_ms DESC
             LIMIT 240",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![since, session_id], ui_event_summary_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_events_for_frame(
    conn: &Connection,
    frame: &CaptureFrame,
) -> Result<Vec<UiEventSummary>, String> {
    let mut ids = Vec::new();

    if let Some(trigger_id) = frame.capture_trigger_id.as_deref() {
        let caused_by = conn
            .query_row(
                "SELECT caused_by_event_ids FROM capture_triggers WHERE id = ?1",
                params![trigger_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(to_string)?;
        if let Some(caused_by) = caused_by {
            ids.extend(serde_json::from_str::<Vec<String>>(&caused_by).unwrap_or_default());
        }
    }

    let frame_key = frame.id.to_string();
    let mut stmt = conn
        .prepare(
            "SELECT primary_event_id
             FROM event_transitions
             WHERE (pre_frame_id = ?1 OR post_frame_id = ?1)
               AND primary_event_id IS NOT NULL",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_key], |row| row.get::<_, String>(0))
        .map_err(to_string)?;
    for row in rows {
        let id = row.map_err(to_string)?;
        if !ids.iter().any(|existing| existing == &id) {
            ids.push(id);
        }
    }

    let mut events = Vec::new();
    for id in ids.into_iter().take(24) {
        if let Some(event) = conn
            .query_row(
                "SELECT id, ts_ms, event_type, app_name, window_title, key_category, payload_json
                 FROM ui_events
                 WHERE id = ?1",
                params![id],
                ui_event_summary_from_row,
            )
            .optional()
            .map_err(to_string)?
        {
            events.push(event);
        }
    }
    Ok(events)
}

fn ui_event_summary_from_row(row: &Row<'_>) -> rusqlite::Result<UiEventSummary> {
    Ok(UiEventSummary {
        id: row.get(0)?,
        ts_ms: row.get(1)?,
        event_type: row.get(2)?,
        app_name: row.get(3)?,
        window_title: row.get(4)?,
        key_category: row.get(5)?,
        payload_json: row.get(6)?,
    })
}

fn query_capture_triggers(
    conn: &Connection,
    since: i64,
    session_id: Option<&str>,
) -> Result<Vec<CaptureTriggerSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, ts_ms, trigger_type, caused_by_event_ids, pre_frame_id,
                    post_frame_id, status
             FROM capture_triggers
             WHERE ts_ms >= ?1
               AND (?2 IS NULL OR session_id = ?2)
             ORDER BY ts_ms DESC
             LIMIT 160",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![since, session_id], |row| {
            Ok(CaptureTriggerSummary {
                id: row.get(0)?,
                ts_ms: row.get(1)?,
                trigger_type: row.get(2)?,
                caused_by_event_ids: row.get(3)?,
                pre_frame_id: row.get(4)?,
                post_frame_id: row.get(5)?,
                status: row.get(6)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_transitions_since(
    conn: &Connection,
    since: i64,
    session_id: Option<&str>,
) -> Result<Vec<TransitionSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, trigger_id, primary_event_id, pre_frame_id, post_frame_id,
                    ts_start_ms, ts_end_ms, transition_type, confidence, summary
             FROM event_transitions
             WHERE ts_start_ms >= ?1
               AND (?2 IS NULL OR session_id = ?2)
             ORDER BY ts_start_ms DESC
             LIMIT 160",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![since, session_id], transition_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_transition_by_id(
    conn: &Connection,
    transition_id: &str,
) -> Result<Option<TransitionSummary>, String> {
    conn.query_row(
        "SELECT id, trigger_id, primary_event_id, pre_frame_id, post_frame_id,
                ts_start_ms, ts_end_ms, transition_type, confidence, summary
         FROM event_transitions
         WHERE id = ?1",
        params![transition_id],
        transition_from_row,
    )
    .optional()
    .map_err(to_string)
}

fn query_transitions_for_frame(
    conn: &Connection,
    frame_id: &str,
) -> Result<Vec<TransitionSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, trigger_id, primary_event_id, pre_frame_id, post_frame_id,
                    ts_start_ms, ts_end_ms, transition_type, confidence, summary
             FROM event_transitions
             WHERE pre_frame_id = ?1 OR post_frame_id = ?1
             ORDER BY ts_start_ms DESC
             LIMIT 40",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], transition_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn transition_from_row(row: &Row<'_>) -> rusqlite::Result<TransitionSummary> {
    Ok(TransitionSummary {
        id: row.get(0)?,
        trigger_id: row.get(1)?,
        primary_event_id: row.get(2)?,
        pre_frame_id: row.get(3)?,
        post_frame_id: row.get(4)?,
        ts_start_ms: row.get(5)?,
        ts_end_ms: row.get(6)?,
        transition_type: row.get(7)?,
        confidence: row.get(8)?,
        summary: row.get(9)?,
    })
}

fn query_ax_nodes(conn: &Connection, frame_id: &str) -> Result<Vec<AxNodeSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, parent_id, role,
                    trim(coalesce(title, '') || ' ' || coalesce(value, '') || ' ' ||
                         coalesce(description, '') || ' ' || coalesce(selected_text, '')) AS text,
                    focused, bounds_x, bounds_y, bounds_w, bounds_h, depth
             FROM ax_nodes
             WHERE frame_id = ?1
             ORDER BY depth ASC, id ASC
             LIMIT 450",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(AxNodeSummary {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                role: row.get(2)?,
                text: row.get(3)?,
                focused: row.get::<_, Option<i64>>(4)?.map(|v| v == 1),
                bounds_x: row.get(5)?,
                bounds_y: row.get(6)?,
                bounds_w: row.get(7)?,
                bounds_h: row.get(8)?,
                depth: row.get(9)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_ocr_spans(conn: &Connection, frame_id: &str) -> Result<Vec<OcrSpanSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, engine, text, confidence, bounds_x, bounds_y, bounds_w, bounds_h
             FROM ocr_spans
             WHERE frame_id = ?1
             ORDER BY line_index ASC
             LIMIT 500",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(OcrSpanSummary {
                id: row.get(0)?,
                engine: row.get(1)?,
                text: row.get(2)?,
                confidence: row.get(3)?,
                bounds_x: row.get(4)?,
                bounds_y: row.get(5)?,
                bounds_w: row.get(6)?,
                bounds_h: row.get(7)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_content_units_for_frame(
    conn: &Connection,
    frame_id: &str,
) -> Result<Vec<ContentUnitSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, source, unit_type, semantic_role, text, bounds_x, bounds_y,
                    bounds_w, bounds_h, confidence
             FROM content_units
             WHERE frame_id = ?1
             ORDER BY confidence DESC, created_at_ms DESC
             LIMIT 220",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], content_unit_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn content_unit_from_row(row: &Row<'_>) -> rusqlite::Result<ContentUnitSummary> {
    Ok(ContentUnitSummary {
        id: row.get(0)?,
        source: row.get(1)?,
        unit_type: row.get(2)?,
        semantic_role: row.get(3)?,
        text: row.get(4)?,
        bounds_x: row.get(5)?,
        bounds_y: row.get(6)?,
        bounds_w: row.get(7)?,
        bounds_h: row.get(8)?,
        confidence: row.get(9)?,
    })
}

fn query_app_contexts(conn: &Connection, frame_id: &str) -> Result<Vec<AppContextSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, adapter_id, object_type, title, url, file_path, selected_text,
                    focused_object, confidence, metadata_json
             FROM app_contexts
             WHERE frame_id = ?1
             ORDER BY confidence DESC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(AppContextSummary {
                id: row.get(0)?,
                adapter_id: row.get(1)?,
                object_type: row.get(2)?,
                title: row.get(3)?,
                url: row.get(4)?,
                file_path: row.get(5)?,
                selected_text: row.get(6)?,
                focused_object: row.get(7)?,
                confidence: row.get(8)?,
                metadata_json: row.get(9)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_sensitive_regions(
    conn: &Connection,
    frame_id: &str,
) -> Result<Vec<SensitiveRegionSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, region_type, bounds_x, bounds_y, bounds_w, bounds_h,
                    source, confidence, action_taken
             FROM sensitive_regions
             WHERE frame_id = ?1",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(SensitiveRegionSummary {
                id: row.get(0)?,
                region_type: row.get(1)?,
                bounds_x: row.get(2)?,
                bounds_y: row.get(3)?,
                bounds_w: row.get(4)?,
                bounds_h: row.get(5)?,
                source: row.get(6)?,
                confidence: row.get(7)?,
                action_taken: row.get(8)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_windows_for_frame(
    conn: &Connection,
    frame_id: &str,
) -> Result<Vec<WindowSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT w.cg_window_id, w.owner_name, w.bundle_id, w.window_title,
                    w.is_active, w.bounds_x, w.bounds_y, w.bounds_w, w.bounds_h
             FROM windows w
             JOIN window_snapshots s ON s.id = w.window_snapshot_id
             WHERE s.frame_id = ?1
             ORDER BY w.is_active DESC, w.layer ASC
             LIMIT 80",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(WindowSummary {
                cg_window_id: row.get(0)?,
                owner_name: row.get(1)?,
                bundle_id: row.get(2)?,
                window_title: row.get(3)?,
                is_active: row.get::<_, i64>(4)? == 1,
                bounds_x: row.get(5)?,
                bounds_y: row.get(6)?,
                bounds_w: row.get(7)?,
                bounds_h: row.get(8)?,
            })
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn build_verification_signals(
    frame: &CaptureFrame,
    ax_nodes: &[AxNodeSummary],
    ocr_spans: &[OcrSpanSummary],
    content_units: &[ContentUnitSummary],
    app_contexts: &[AppContextSummary],
    sensitive_regions: &[SensitiveRegionSummary],
    windows: &[WindowSummary],
    transitions: &[TransitionSummary],
    events: &[UiEventSummary],
) -> VerificationSignals {
    let screenshot_present = Path::new(&frame.snapshot_path).exists();
    let has_ax = !ax_nodes.is_empty();
    let has_ocr = !ocr_spans.is_empty();
    let has_content_units = !content_units.is_empty();
    let has_app_context = !app_contexts.is_empty();
    let has_window_graph = !windows.is_empty();
    let has_transition = !transitions.is_empty();
    let has_event_provenance = !events.is_empty();
    let has_sensitive_regions = !sensitive_regions.is_empty();

    let mut missing_signals = Vec::new();
    if !screenshot_present {
        missing_signals.push("screenshot file missing".to_string());
    }
    if !has_ax {
        missing_signals.push("no accessibility nodes".to_string());
    }
    if !has_ocr && frame.text_source.as_deref() != Some("accessibility") {
        missing_signals.push("no OCR spans".to_string());
    }
    if !has_content_units {
        missing_signals.push("no content units".to_string());
    }
    if !has_app_context {
        missing_signals.push("no app context adapter output".to_string());
    }
    if !has_window_graph {
        missing_signals.push("no window graph".to_string());
    }
    let eventless_trigger =
        frame.capture_trigger == "manual" || frame.capture_trigger == "session_start";
    if !has_transition && !eventless_trigger {
        missing_signals.push("no linked transition".to_string());
    }
    if !has_event_provenance && !eventless_trigger {
        missing_signals.push("no raw event provenance".to_string());
    }

    let mut trust_score = 0.0_f64;
    if screenshot_present {
        trust_score += 0.22;
    }
    if has_ax || has_ocr {
        trust_score += 0.18;
    }
    if has_content_units {
        trust_score += 0.2;
    }
    if has_app_context {
        trust_score += 0.12;
    }
    if has_window_graph {
        trust_score += 0.1;
    }
    if has_transition || eventless_trigger {
        trust_score += 0.1;
    }
    if has_event_provenance || eventless_trigger {
        trust_score += 0.08;
    }

    let trust_label = if trust_score >= 0.78 {
        "complete"
    } else if trust_score >= 0.46 {
        "partial"
    } else {
        "thin"
    }
    .to_string();

    VerificationSignals {
        screenshot_present,
        has_ax,
        has_ocr,
        has_content_units,
        has_app_context,
        has_window_graph,
        has_transition,
        has_event_provenance,
        has_sensitive_regions,
        ax_node_count: ax_nodes.len(),
        ocr_span_count: ocr_spans.len(),
        content_unit_count: content_units.len(),
        app_context_count: app_contexts.len(),
        window_count: windows.len(),
        transition_count: transitions.len(),
        event_count: events.len(),
        missing_signals,
        trust_label,
        trust_score,
    }
}

fn validate_frame_consistency_inner(
    conn: &Connection,
    frame: &CaptureFrame,
) -> Result<FrameConsistencyReport, String> {
    ensure_frame_quality_warnings_table(conn)?;
    let frame_key = frame.id.to_string();
    conn.execute(
        "DELETE FROM frame_quality_warnings WHERE frame_id = ?1",
        params![frame_key],
    )
    .map_err(to_string)?;

    let app_contexts = query_app_contexts(conn, &frame.id.to_string())?;
    let windows = query_windows_for_frame(conn, &frame.id.to_string()).unwrap_or_default();
    let content_units =
        query_content_units_for_frame(conn, &frame.id.to_string()).unwrap_or_default();
    let mut warnings = Vec::new();
    let text = frame.full_text.as_deref().unwrap_or("").to_lowercase();
    let app_name = frame.app_name.as_deref().unwrap_or("").to_lowercase();
    let window_name = frame.window_name.as_deref().unwrap_or("").to_lowercase();
    let url = frame.browser_url.as_deref().unwrap_or("").to_lowercase();
    let context_object = app_contexts
        .first()
        .map(|context| context.object_type.as_str());

    if frame.active_window_crop_path.is_some() && frame.window_id.is_none() {
        warnings.push(make_frame_quality_warning(
            &frame_key,
            "active_window_id_missing",
            "medium",
            "active-window crop exists but frame has no active window id",
            serde_json::json!({ "active_window_crop_path": frame.active_window_crop_path }),
        ));
    }
    if is_browser_app(&app_name) && frame.browser_url.is_none() {
        warnings.push(make_frame_quality_warning(
            &frame_key,
            "browser_url_missing",
            "medium",
            "browser-like app did not provide a browser_url",
            serde_json::json!({ "app_name": frame.app_name, "window_name": frame.window_name }),
        ));
    }
    if let Some(active) = windows.iter().find(|window| window.is_active) {
        let active_app = active.owner_name.as_deref().unwrap_or("").to_lowercase();
        if !active_app.is_empty()
            && !app_name.is_empty()
            && !active_app.contains(&app_name)
            && !app_name.contains(&active_app)
        {
            warnings.push(make_frame_quality_warning(
                &frame_key,
                "active_app_mismatch",
                "high",
                "window graph active app differs from frame app metadata",
                serde_json::json!({ "frame_app": frame.app_name, "window_owner": active.owner_name }),
            ));
        }
    }
    if text.contains("chatgpt") && context_object == Some("unknown") {
        warnings.push(make_frame_quality_warning(
            &frame_key,
            "chat_surface_unknown_adapter",
            "medium",
            "visible text suggests ChatGPT but app_context is unknown",
            serde_json::json!({ "object_type": context_object }),
        ));
    }
    if (url.contains("chatgpt") || window_name.contains("chatgpt"))
        && context_object != Some("chat_conversation")
    {
        warnings.push(make_frame_quality_warning(
            &frame_key,
            "chat_adapter_mismatch",
            "medium",
            "metadata suggests a chat conversation but adapter did not classify it that way",
            serde_json::json!({ "url": frame.browser_url, "window_name": frame.window_name, "object_type": context_object }),
        ));
    }
    let chromeish_units = content_units
        .iter()
        .filter(|unit| {
            unit.semantic_role.as_deref().is_some_and(|role| {
                matches!(
                    role,
                    "browser_chrome" | "toolbar" | "app_sidebar" | "system_menu"
                )
            })
        })
        .count();
    if !content_units.is_empty() && chromeish_units as f64 / content_units.len() as f64 > 0.55 {
        warnings.push(make_frame_quality_warning(
            &frame_key,
            "browser_chrome_dominated_text",
            "medium",
            "frame text is dominated by browser/app chrome content units",
            serde_json::json!({ "chromeish_units": chromeish_units, "content_units": content_units.len() }),
        ));
    }

    for warning in &warnings {
        conn.execute(
            "INSERT OR REPLACE INTO frame_quality_warnings (
                id, frame_id, warning_type, severity, message, evidence_json, created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                warning.id,
                warning.frame_id,
                warning.warning_type,
                warning.severity,
                warning.message,
                warning.evidence_json,
                warning.created_at_ms,
            ],
        )
        .map_err(to_string)?;
    }
    let adjustment = warnings
        .iter()
        .map(|warning| match warning.severity.as_str() {
            "high" => 0.18,
            "medium" => 0.1,
            _ => 0.04,
        })
        .sum::<f64>()
        .min(0.32);
    Ok(FrameConsistencyReport {
        frame_id: frame_key,
        warnings,
        confidence_adjustment: -adjustment,
    })
}

fn make_frame_quality_warning(
    frame_id: &str,
    warning_type: &str,
    severity: &str,
    message: &str,
    evidence: Value,
) -> FrameQualityWarning {
    FrameQualityWarning {
        id: next_id("quality"),
        frame_id: frame_id.to_string(),
        warning_type: warning_type.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
        evidence_json: evidence.to_string(),
        created_at_ms: now_millis(),
    }
}

fn query_frame_quality_warnings(
    conn: &Connection,
    frame_id: &str,
) -> Result<Vec<FrameQualityWarning>, String> {
    ensure_frame_quality_warnings_table(conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, frame_id, warning_type, severity, message, evidence_json, created_at_ms
             FROM frame_quality_warnings
             WHERE frame_id = ?1
             ORDER BY created_at_ms ASC, id ASC",
        )
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![frame_id], |row| {
            Ok(FrameQualityWarning {
                id: row.get(0)?,
                frame_id: row.get(1)?,
                warning_type: row.get(2)?,
                severity: row.get(3)?,
                message: row.get(4)?,
                evidence_json: row.get(5)?,
                created_at_ms: row.get(6)?,
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
        capture_provider: row.get(15)?,
        scope: row.get(16)?,
        display_id: row.get(17)?,
        window_id: row.get(18)?,
        app_pid: row.get(19)?,
        app_bundle_id: row.get(20)?,
        screen_scale: row.get(21)?,
        pixel_width: row.get(22)?,
        pixel_height: row.get(23)?,
        full_screenshot_path: row.get(24)?,
        active_window_crop_path: row.get(25)?,
        active_element_crop_path: row.get(26)?,
        phash: row.get(27)?,
        privacy_status: row.get(28)?,
        capture_trigger_id: row.get(29)?,
        previous_frame_id: row.get(30)?,
        session_id: row.get(31)?,
    })
}

fn create_capture_session(app: &AppHandle) -> Result<CaptureSession, String> {
    let conn = open_db(app)?;
    let started_at = now_millis();
    conn.execute(
        "UPDATE capture_sessions
         SET status = 'interrupted',
             stopped_at_ms = COALESCE(stopped_at_ms, ?1)
         WHERE status = 'running'",
        params![started_at],
    )
    .map_err(to_string)?;

    let id = next_id("session");
    let sequence = next_session_sequence(&conn)?;
    conn.execute(
        "INSERT INTO capture_sessions (
            id, sequence, started_at_ms, status, created_at_ms
         ) VALUES (?1, ?2, ?3, 'running', ?3)",
        params![id, sequence, started_at],
    )
    .map_err(to_string)?;
    load_capture_session(&conn, &id)
}

fn finish_capture_session(
    app: &AppHandle,
    session_id: &str,
) -> Result<(CaptureSession, SessionExportSummary), String> {
    let conn = open_db(app)?;
    let stopped_at = now_millis();
    conn.execute(
        "UPDATE capture_sessions
         SET status = 'stopped',
             stopped_at_ms = COALESCE(stopped_at_ms, ?2)
         WHERE id = ?1",
        params![session_id, stopped_at],
    )
    .map_err(to_string)?;
    refresh_session_counts(&conn, session_id)?;

    let session = load_capture_session(&conn, session_id)?;
    let planned_export_path = session_output_dir(&project_output_root()?, session.sequence)?;
    conn.execute(
        "UPDATE capture_sessions
         SET export_path = ?2
         WHERE id = ?1",
        params![
            session_id,
            planned_export_path.to_string_lossy().to_string()
        ],
    )
    .map_err(to_string)?;
    let session = load_capture_session(&conn, session_id)?;
    let export = write_session_export(app, &conn, &session)?;

    Ok((load_capture_session(&conn, session_id)?, export))
}

fn next_session_sequence(conn: &Connection) -> Result<i64, String> {
    let current = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) FROM capture_sessions",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(to_string)?;
    Ok(current + 1)
}

fn latest_session_id(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT id
         FROM capture_sessions
         ORDER BY started_at_ms DESC
         LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(to_string)
}

fn load_latest_session(conn: &Connection) -> Result<Option<CaptureSession>, String> {
    let id = latest_session_id(conn)?;
    id.map(|id| load_capture_session(conn, &id)).transpose()
}

fn load_capture_session(conn: &Connection, session_id: &str) -> Result<CaptureSession, String> {
    let mut session = conn
        .query_row(
            "SELECT id, sequence, started_at_ms, stopped_at_ms, status, export_path
             FROM capture_sessions
             WHERE id = ?1",
            params![session_id],
            |row| {
                Ok(CaptureSession {
                    id: row.get(0)?,
                    sequence: row.get(1)?,
                    started_at: row.get(2)?,
                    stopped_at: row.get(3)?,
                    status: row.get(4)?,
                    export_path: row.get(5)?,
                    counts: SessionCounts::default(),
                })
            },
        )
        .map_err(to_string)?;
    session.counts = session_counts(conn, session_id)?;
    Ok(session)
}

fn refresh_session_counts(conn: &Connection, session_id: &str) -> Result<SessionCounts, String> {
    let counts = session_counts(conn, session_id)?;
    conn.execute(
        "UPDATE capture_sessions
         SET frame_count = ?2,
             event_count = ?3,
             transition_count = ?4,
             content_unit_count = ?5
         WHERE id = ?1",
        params![
            session_id,
            counts.frames,
            counts.events,
            counts.transitions,
            counts.content_units,
        ],
    )
    .map_err(to_string)?;
    Ok(counts)
}

fn session_counts(conn: &Connection, session_id: &str) -> Result<SessionCounts, String> {
    Ok(SessionCounts {
        frames: session_count_query(
            conn,
            "SELECT COUNT(*) FROM frames WHERE session_id = ?1",
            session_id,
        )?,
        events: session_count_query(
            conn,
            "SELECT COUNT(*) FROM ui_events WHERE session_id = ?1",
            session_id,
        )?,
        triggers: session_count_query(
            conn,
            "SELECT COUNT(*) FROM capture_triggers WHERE session_id = ?1",
            session_id,
        )?,
        transitions: session_count_query(
            conn,
            "SELECT COUNT(*) FROM event_transitions WHERE session_id = ?1",
            session_id,
        )?,
        content_units: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM content_units
             WHERE frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        ax_nodes: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM ax_nodes
             WHERE frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        ocr_text_rows: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM ocr_text
             WHERE frame_id IN (SELECT id FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        ocr_spans: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM ocr_spans
             WHERE frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        app_contexts: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM app_contexts
             WHERE frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        window_snapshots: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM window_snapshots
             WHERE frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        windows: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM windows
             WHERE window_snapshot_id IN (
               SELECT id
               FROM window_snapshots
               WHERE frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)
             )",
            session_id,
        )?,
        frame_diffs: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM frame_diffs
             WHERE session_id = ?1
                OR to_frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        clipboard_events: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM clipboard_events
             WHERE session_id = ?1
                OR source_frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        typing_bursts: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM typing_bursts
             WHERE session_id = ?1
                OR pre_frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)
                OR post_frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
        presence_samples: session_count_query(
            conn,
            "SELECT COUNT(*) FROM presence_samples WHERE session_id = ?1",
            session_id,
        )?,
        sensitive_regions: session_count_query(
            conn,
            "SELECT COUNT(*)
             FROM sensitive_regions
             WHERE frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)",
            session_id,
        )?,
    })
}

fn session_count_query(conn: &Connection, sql: &str, session_id: &str) -> Result<i64, String> {
    conn.query_row(sql, params![session_id], |row| row.get(0))
        .map_err(to_string)
}

#[derive(Debug, Clone, Serialize)]
struct ExportWarning {
    scope: String,
    code: String,
    message: String,
    path: Option<String>,
    frame_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
struct ExportedImageArtifact {
    label: String,
    source_path: String,
    original_path: Option<String>,
    png_path: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct ExportStats {
    file_count: i64,
    byte_size: i64,
}

fn write_session_export(
    _app: &AppHandle,
    conn: &Connection,
    session: &CaptureSession,
) -> Result<SessionExportSummary, String> {
    write_session_export_to_root(conn, session, &project_output_root()?)
}

fn write_session_export_to_root(
    conn: &Connection,
    session: &CaptureSession,
    output_root: &Path,
) -> Result<SessionExportSummary, String> {
    let generated_at = now_millis();
    let folder_name = session_folder_name(session.sequence);
    let final_dir = session_output_dir(output_root, session.sequence)?;
    let tmp_dir = output_root.join(format!(".{}.tmp-{}", folder_name, generated_at));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).map_err(to_string)?;
    }
    fs::create_dir_all(output_root).map_err(to_string)?;
    fs::create_dir_all(&tmp_dir).map_err(to_string)?;

    let mut warnings = Vec::new();
    let raw_dir = tmp_dir.join("raw");
    let raw_tables_dir = raw_dir.join("tables");
    let timeline_dir = tmp_dir.join("timeline");
    let frames_dir = tmp_dir.join("frames");
    fs::create_dir_all(&raw_tables_dir).map_err(to_string)?;
    fs::create_dir_all(&timeline_dir).map_err(to_string)?;
    fs::create_dir_all(&frames_dir).map_err(to_string)?;

    copy_database_snapshot(conn, &raw_dir.join("smalltalk-capture.sqlite"))?;
    write_schema_dump(conn, &raw_dir.join("schema.sql"))?;
    let table_manifest = export_all_tables(conn, &raw_tables_dir, &mut warnings)?;
    let timeline_manifest = export_timeline(conn, session, &timeline_dir)?;
    let frames = frames_for_session(conn, &session.id)?;
    let mut frame_manifests = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let manifest = export_frame_folder(conn, frame, index + 1, &frames_dir, &mut warnings)?;
        frame_manifests.push(manifest);
    }

    write_json_pretty(
        &tmp_dir.join("export_warnings.json"),
        &Value::Array(
            warnings
                .iter()
                .map(serde_json::to_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(to_string)?,
        ),
    )?;

    let session_json_path = tmp_dir.join("session.json");
    let mut stats = ExportStats {
        file_count: 0,
        byte_size: 0,
    };
    for _ in 0..2 {
        write_session_manifest(
            &session_json_path,
            session,
            generated_at,
            &folder_name,
            &final_dir,
            &table_manifest,
            &timeline_manifest,
            &frame_manifests,
            &warnings,
            stats,
        )?;
        stats = directory_stats(&tmp_dir)?;
    }

    if final_dir.exists() {
        fs::remove_dir_all(&final_dir).map_err(to_string)?;
    }
    fs::rename(&tmp_dir, &final_dir).map_err(to_string)?;
    let stats = directory_stats(&final_dir)?;

    Ok(SessionExportSummary {
        session_id: session.id.clone(),
        session_sequence: session.sequence,
        generated_at,
        kind: "folder".to_string(),
        folder_name,
        path: final_dir.to_string_lossy().to_string(),
        byte_size: stats.byte_size,
        file_count: stats.file_count,
        warning_count: warnings.len() as i64,
        counts: session.counts.clone(),
    })
}

fn write_session_manifest(
    path: &Path,
    session: &CaptureSession,
    generated_at: i64,
    folder_name: &str,
    final_dir: &Path,
    table_manifest: &[Value],
    timeline_manifest: &Value,
    frame_manifests: &[Value],
    warnings: &[ExportWarning],
    stats: ExportStats,
) -> Result<(), String> {
    write_json_pretty(
        path,
        &serde_json::json!({
            "schema": "smalltalk.capture_session.folder.v1",
            "generatedAtMs": generated_at,
            "session": session,
            "counts": session.counts.clone(),
            "output": {
                "kind": "folder",
                "folderName": folder_name,
                "path": final_dir.to_string_lossy(),
                "fileCount": stats.file_count,
                "byteSize": stats.byte_size,
                "warningCount": warnings.len(),
            },
            "notes": [
                "raw/tables contains every SQLite table dumped at export time.",
                "raw/smalltalk-capture.sqlite is a SQLite snapshot created with VACUUM INTO.",
                "Each frame folder copies original image artifacts and creates PNG copies when possible."
            ],
            "manifest": {
                "rawDatabase": "raw/smalltalk-capture.sqlite",
                "schema": "raw/schema.sql",
                "rawTables": table_manifest,
                "timeline": timeline_manifest,
                "frames": frame_manifests,
                "warnings": "export_warnings.json",
            },
            "warnings": warnings,
        }),
    )
}

fn query_json_rows_for_session(
    conn: &Connection,
    sql: &str,
    session_id: &str,
) -> Result<Vec<Value>, String> {
    query_json_rows_with_params(conn, sql, &[&session_id])
}

fn query_json_rows_with_params(
    conn: &Connection,
    sql: &str,
    values: &[&dyn ToSql],
) -> Result<Vec<Value>, String> {
    let mut stmt = conn.prepare(sql).map_err(to_string)?;
    let column_names = stmt
        .column_names()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let rows = stmt
        .query_map(values, |row| {
            let mut object = serde_json::Map::new();
            for (index, column_name) in column_names.iter().enumerate() {
                object.insert(column_name.clone(), sql_value_to_json(row.get_ref(index)?));
            }
            Ok(Value::Object(object))
        })
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn query_json_rows(conn: &Connection, sql: &str) -> Result<Vec<Value>, String> {
    query_json_rows_with_params(conn, sql, &[])
}

fn export_all_tables(
    conn: &Connection,
    tables_dir: &Path,
    warnings: &mut Vec<ExportWarning>,
) -> Result<Vec<Value>, String> {
    let tables = table_names_for_export(conn)?;
    let mut manifest = Vec::new();
    for table in tables {
        let stem = safe_file_stem(&table);
        let sql = format!("SELECT * FROM {}", quote_identifier(&table));
        let rows = match query_json_rows(conn, &sql) {
            Ok(rows) => rows,
            Err(error) => {
                push_export_warning(
                    warnings,
                    "raw_tables",
                    "table_export_failed",
                    format!("failed to export table {}: {}", table, error),
                    None,
                    None,
                );
                Vec::new()
            }
        };
        write_json_rows_file(&tables_dir.join(format!("{}.json", stem)), &rows)?;
        write_ndjson_rows_file(&tables_dir.join(format!("{}.ndjson", stem)), &rows)?;
        manifest.push(serde_json::json!({
            "table": table,
            "rowCount": rows.len(),
            "json": format!("raw/tables/{}.json", stem),
            "ndjson": format!("raw/tables/{}.ndjson", stem),
        }));
    }
    Ok(manifest)
}

fn export_timeline(
    conn: &Connection,
    session: &CaptureSession,
    timeline_dir: &Path,
) -> Result<Value, String> {
    let entries = [
        (
            "frames",
            "SELECT * FROM frames WHERE session_id = ?1 ORDER BY captured_at ASC, id ASC",
        ),
        (
            "ui_events",
            "SELECT * FROM ui_events WHERE session_id = ?1 ORDER BY ts_ms ASC, id ASC",
        ),
        (
            "capture_triggers",
            "SELECT * FROM capture_triggers WHERE session_id = ?1 ORDER BY ts_ms ASC, id ASC",
        ),
        (
            "event_transitions",
            "SELECT * FROM event_transitions WHERE session_id = ?1 ORDER BY ts_start_ms ASC, id ASC",
        ),
        (
            "frame_diffs",
            "SELECT * FROM frame_diffs
             WHERE session_id = ?1
                OR to_frame_id IN (SELECT CAST(id AS TEXT) FROM frames WHERE session_id = ?1)
             ORDER BY ts_ms ASC, id ASC",
        ),
    ];
    let mut manifest = serde_json::Map::new();
    for (name, sql) in entries {
        let rows = query_json_rows_for_session(conn, sql, &session.id)?;
        write_json_rows_file(&timeline_dir.join(format!("{}.json", name)), &rows)?;
        write_ndjson_rows_file(&timeline_dir.join(format!("{}.ndjson", name)), &rows)?;
        manifest.insert(
            name.to_string(),
            serde_json::json!({
                "rowCount": rows.len(),
                "json": format!("timeline/{}.json", name),
                "ndjson": format!("timeline/{}.ndjson", name),
            }),
        );
    }
    Ok(Value::Object(manifest))
}

fn export_frame_folder(
    conn: &Connection,
    frame: &CaptureFrame,
    index: usize,
    frames_dir: &Path,
    warnings: &mut Vec<ExportWarning>,
) -> Result<Value, String> {
    let frame_folder = format!("frame-{:06}", index);
    let frame_dir = frames_dir.join(&frame_folder);
    let text_dir = frame_dir.join("text");
    let ocr_dir = frame_dir.join("ocr");
    let accessibility_dir = frame_dir.join("accessibility");
    let content_dir = frame_dir.join("content");
    let context_dir = frame_dir.join("context");
    let windows_dir = frame_dir.join("windows");
    let events_dir = frame_dir.join("events");
    let privacy_dir = frame_dir.join("privacy");
    let images_dir = frame_dir.join("images");
    for dir in [
        &text_dir,
        &ocr_dir,
        &accessibility_dir,
        &content_dir,
        &context_dir,
        &windows_dir,
        &events_dir,
        &privacy_dir,
        &images_dir,
    ] {
        fs::create_dir_all(dir).map_err(to_string)?;
    }

    let frame_id = frame.id;
    let frame_key = frame.id.to_string();
    let frame_rows =
        query_json_rows_with_params(conn, "SELECT * FROM frames WHERE id = ?1", &[&frame_id])?;
    let ocr_text_rows = query_json_rows_with_params(
        conn,
        "SELECT * FROM ocr_text WHERE frame_id = ?1 ORDER BY id ASC",
        &[&frame_id],
    )?;
    let ax_nodes = query_json_rows_with_params(
        conn,
        "SELECT * FROM ax_nodes WHERE frame_id = ?1 ORDER BY depth ASC, id ASC",
        &[&frame_key],
    )?;
    let ocr_spans = query_json_rows_with_params(
        conn,
        "SELECT * FROM ocr_spans WHERE frame_id = ?1 ORDER BY line_index ASC, id ASC",
        &[&frame_key],
    )?;
    let content_units = query_json_rows_with_params(
        conn,
        "SELECT * FROM content_units WHERE frame_id = ?1 ORDER BY confidence DESC, id ASC",
        &[&frame_key],
    )?;
    let app_contexts = query_json_rows_with_params(
        conn,
        "SELECT * FROM app_contexts WHERE frame_id = ?1 ORDER BY confidence DESC, id ASC",
        &[&frame_key],
    )?;
    let sensitive_regions = query_json_rows_with_params(
        conn,
        "SELECT * FROM sensitive_regions WHERE frame_id = ?1 ORDER BY id ASC",
        &[&frame_key],
    )?;
    let window_snapshots = query_json_rows_with_params(
        conn,
        "SELECT * FROM window_snapshots WHERE frame_id = ?1 ORDER BY ts_ms ASC, id ASC",
        &[&frame_key],
    )?;
    let windows = query_json_rows_with_params(
        conn,
        "SELECT w.*
         FROM windows w
         JOIN window_snapshots s ON s.id = w.window_snapshot_id
         WHERE s.frame_id = ?1
         ORDER BY s.ts_ms ASC, w.is_active DESC, w.id ASC",
        &[&frame_key],
    )?;
    let capture_triggers = query_json_rows_with_params(
        conn,
        "SELECT * FROM capture_triggers
         WHERE pre_frame_id = ?1 OR post_frame_id = ?1 OR id = ?2
         ORDER BY ts_ms ASC, id ASC",
        &[&frame_key, &frame.capture_trigger_id],
    )?;
    let event_transitions = query_json_rows_with_params(
        conn,
        "SELECT * FROM event_transitions
         WHERE pre_frame_id = ?1 OR post_frame_id = ?1 OR trigger_id = ?2
         ORDER BY ts_start_ms ASC, id ASC",
        &[&frame_key, &frame.capture_trigger_id],
    )?;
    let frame_diffs = query_json_rows_with_params(
        conn,
        "SELECT * FROM frame_diffs
         WHERE from_frame_id = ?1 OR to_frame_id = ?1
         ORDER BY ts_ms ASC, id ASC",
        &[&frame_key],
    )?;
    let ui_events = query_full_events_for_frame(conn, &capture_triggers, &event_transitions)?;

    write_json_rows_file(&frame_dir.join("frame_row.json"), &frame_rows)?;
    write_json_rows_file(&ocr_dir.join("ocr_text_rows.json"), &ocr_text_rows)?;
    write_json_rows_file(&ocr_dir.join("ocr_spans.json"), &ocr_spans)?;
    write_json_rows_file(&accessibility_dir.join("ax_nodes.json"), &ax_nodes)?;
    write_json_rows_file(&content_dir.join("content_units.json"), &content_units)?;
    write_json_rows_file(&context_dir.join("app_contexts.json"), &app_contexts)?;
    write_json_rows_file(
        &privacy_dir.join("sensitive_regions.json"),
        &sensitive_regions,
    )?;
    write_json_rows_file(
        &windows_dir.join("window_snapshots.json"),
        &window_snapshots,
    )?;
    write_json_rows_file(&windows_dir.join("windows.json"), &windows)?;
    write_json_rows_file(&events_dir.join("ui_events.json"), &ui_events)?;
    write_json_rows_file(&events_dir.join("capture_triggers.json"), &capture_triggers)?;
    write_json_rows_file(
        &events_dir.join("event_transitions.json"),
        &event_transitions,
    )?;
    write_json_rows_file(&events_dir.join("frame_diffs.json"), &frame_diffs)?;

    if let Some(text) = frame.full_text.as_deref() {
        write_text_file(&text_dir.join("full_text.txt"), text)?;
    }
    if let Some(text) = frame.accessibility_text.as_deref() {
        write_text_file(&text_dir.join("accessibility_text.txt"), text)?;
    }
    if let Some(tree) = frame.accessibility_tree_json.as_deref() {
        write_json_pretty(
            &text_dir.join("accessibility_tree.json"),
            &json_string_or_value(tree),
        )?;
    }
    write_text_file(
        &ocr_dir.join("ocr_text.txt"),
        &joined_text_field(&ocr_text_rows, "text"),
    )?;
    write_json_pretty(
        &ocr_dir.join("ocr_text_json.json"),
        &Value::Array(
            ocr_text_rows
                .iter()
                .map(|row| {
                    row.get("text_json")
                        .and_then(Value::as_str)
                        .map(json_string_or_value)
                        .unwrap_or(Value::Null)
                })
                .collect(),
        ),
    )?;

    let images = export_frame_images(frame, index, &images_dir, warnings)?;
    let frame_warning_count = warnings
        .iter()
        .filter(|warning| warning.frame_id == Some(frame.id))
        .count();
    let manifest = serde_json::json!({
        "schema": "smalltalk.capture_frame.folder.v1",
        "frameIndex": index,
        "frameId": frame.id,
        "sessionId": frame.session_id,
        "folder": format!("frames/{}", frame_folder),
        "frameRow": "frame_row.json",
        "images": images,
        "text": {
            "fullText": "text/full_text.txt",
            "accessibilityText": "text/accessibility_text.txt",
            "accessibilityTree": "text/accessibility_tree.json",
            "textSource": frame.text_source,
        },
        "ocr": {
            "rows": "ocr/ocr_text_rows.json",
            "plainText": "ocr/ocr_text.txt",
            "json": "ocr/ocr_text_json.json",
            "spans": "ocr/ocr_spans.json",
        },
        "evidence": {
            "accessibility": "accessibility/ax_nodes.json",
            "contentUnits": "content/content_units.json",
            "appContexts": "context/app_contexts.json",
            "sensitiveRegions": "privacy/sensitive_regions.json",
            "windowSnapshots": "windows/window_snapshots.json",
            "windows": "windows/windows.json",
            "uiEvents": "events/ui_events.json",
            "captureTriggers": "events/capture_triggers.json",
            "eventTransitions": "events/event_transitions.json",
            "frameDiffs": "events/frame_diffs.json",
        },
        "rowCounts": {
            "ocrTextRows": ocr_text_rows.len(),
            "ocrSpans": ocr_spans.len(),
            "axNodes": ax_nodes.len(),
            "contentUnits": content_units.len(),
            "appContexts": app_contexts.len(),
            "sensitiveRegions": sensitive_regions.len(),
            "windowSnapshots": window_snapshots.len(),
            "windows": windows.len(),
            "uiEvents": ui_events.len(),
            "captureTriggers": capture_triggers.len(),
            "eventTransitions": event_transitions.len(),
            "frameDiffs": frame_diffs.len(),
        },
        "warningCount": frame_warning_count,
    });
    write_json_pretty(&frame_dir.join("frame.json"), &manifest)?;
    Ok(manifest)
}

fn export_frame_images(
    frame: &CaptureFrame,
    index: usize,
    images_dir: &Path,
    warnings: &mut Vec<ExportWarning>,
) -> Result<Vec<ExportedImageArtifact>, String> {
    let candidates = [
        ("snapshot", Some(frame.snapshot_path.clone()), true),
        ("full_screenshot", frame.full_screenshot_path.clone(), true),
        (
            "active_window_crop",
            frame.active_window_crop_path.clone(),
            false,
        ),
        (
            "active_element_crop",
            frame.active_element_crop_path.clone(),
            false,
        ),
    ];
    let mut artifacts = Vec::new();
    for (label, source, required) in candidates {
        match source {
            Some(source) if !source.trim().is_empty() => {
                artifacts.push(copy_image_artifact(
                    frame.id, index, label, &source, images_dir, warnings,
                )?);
            }
            _ if required => {
                push_export_warning(
                    warnings,
                    "frame_images",
                    "missing_image_path",
                    format!("frame {} has no {} path", frame.id, label),
                    None,
                    Some(frame.id),
                );
            }
            _ => {}
        }
    }
    Ok(artifacts)
}

fn copy_image_artifact(
    frame_id: i64,
    index: usize,
    label: &str,
    source: &str,
    images_dir: &Path,
    warnings: &mut Vec<ExportWarning>,
) -> Result<ExportedImageArtifact, String> {
    let source_path = Path::new(source);
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("jpg")
        .to_ascii_lowercase();
    let original_path = images_dir.join(format!("frame-{:06}-{}.{}", index, label, extension));
    let png_path = images_dir.join(format!("frame-{:06}-{}.png", index, label));
    let mut artifact = ExportedImageArtifact {
        label: label.to_string(),
        source_path: source.to_string(),
        original_path: None,
        png_path: None,
    };

    if !source_path.exists() {
        push_export_warning(
            warnings,
            "frame_images",
            "image_source_missing",
            format!("source image for {} does not exist", label),
            Some(source.to_string()),
            Some(frame_id),
        );
        return Ok(artifact);
    }

    match fs::copy(source_path, &original_path) {
        Ok(_) => artifact.original_path = Some(original_path.to_string_lossy().to_string()),
        Err(error) => {
            push_export_warning(
                warnings,
                "frame_images",
                "image_copy_failed",
                format!("failed to copy {}: {}", label, error),
                Some(source.to_string()),
                Some(frame_id),
            );
            return Ok(artifact);
        }
    }

    if extension == "png" {
        if original_path != png_path {
            fs::copy(&original_path, &png_path).map_err(to_string)?;
        }
        artifact.png_path = Some(png_path.to_string_lossy().to_string());
        return Ok(artifact);
    }

    match convert_image_to_png(&original_path, &png_path) {
        Ok(()) => artifact.png_path = Some(png_path.to_string_lossy().to_string()),
        Err(error) => push_export_warning(
            warnings,
            "frame_images",
            "png_conversion_failed",
            format!("failed to create PNG for {}: {}", label, error),
            Some(original_path.to_string_lossy().to_string()),
            Some(frame_id),
        ),
    }
    Ok(artifact)
}

fn convert_image_to_png(source: &Path, output: &Path) -> Result<(), String> {
    let output_result = Command::new("/usr/bin/sips")
        .arg("-s")
        .arg("format")
        .arg("png")
        .arg(source)
        .arg("--out")
        .arg(output)
        .output()
        .map_err(|error| format!("sips failed to start: {}", error))?;
    if output_result.status.success() && output.exists() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output_result.stderr)
            .trim()
            .to_string();
        Err(if stderr.is_empty() {
            "sips did not produce a PNG".to_string()
        } else {
            stderr
        })
    }
}

fn query_full_events_for_frame(
    conn: &Connection,
    triggers: &[Value],
    transitions: &[Value],
) -> Result<Vec<Value>, String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for trigger in triggers {
        if let Some(raw) = trigger.get("caused_by_event_ids").and_then(Value::as_str) {
            for id in serde_json::from_str::<Vec<String>>(raw).unwrap_or_default() {
                if seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
        }
    }
    for transition in transitions {
        if let Some(id) = transition.get("primary_event_id").and_then(Value::as_str) {
            if seen.insert(id.to_string()) {
                ids.push(id.to_string());
            }
        }
    }
    let mut rows = Vec::new();
    for id in ids {
        rows.extend(query_json_rows_with_params(
            conn,
            "SELECT * FROM ui_events WHERE id = ?1",
            &[&id],
        )?);
    }
    Ok(rows)
}

fn frames_for_session(conn: &Connection, session_id: &str) -> Result<Vec<CaptureFrame>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {}
             FROM frames
             WHERE session_id = ?1
             ORDER BY captured_at ASC, id ASC",
            FRAME_COLUMNS
        ))
        .map_err(to_string)?;
    let rows = stmt
        .query_map(params![session_id], frame_from_row)
        .map_err(to_string)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(to_string)
}

fn table_names_for_export(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name
             FROM sqlite_schema
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
             ORDER BY name ASC",
        )
        .map_err(to_string)?;
    let mut tables = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    let sqlite_sequence = conn
        .query_row(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name = 'sqlite_sequence'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_string)?;
    if let Some(name) = sqlite_sequence {
        tables.push(name);
    }
    Ok(tables)
}

fn write_schema_dump(conn: &Connection, path: &Path) -> Result<(), String> {
    let rows = query_json_rows(
        conn,
        "SELECT type, name, tbl_name, sql
         FROM sqlite_schema
         WHERE sql IS NOT NULL
         ORDER BY type ASC, name ASC",
    )?;
    let mut output = String::new();
    for row in rows {
        if let Some(sql) = row.get("sql").and_then(Value::as_str) {
            output.push_str(sql);
            output.push_str(";\n\n");
        }
    }
    write_text_file(path, &output)
}

fn copy_database_snapshot(conn: &Connection, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(to_string)?;
    }
    if path.exists() {
        fs::remove_file(path).map_err(to_string)?;
    }
    let escaped = path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM main INTO '{}';", escaped))
        .map_err(to_string)
}

fn write_json_rows_file(path: &Path, rows: &[Value]) -> Result<(), String> {
    write_json_pretty(path, &Value::Array(rows.to_vec()))
}

fn write_ndjson_rows_file(path: &Path, rows: &[Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(to_string)?;
    }
    let mut file = fs::File::create(path).map_err(to_string)?;
    for row in rows {
        serde_json::to_writer(&mut file, row).map_err(to_string)?;
        file.write_all(b"\n").map_err(to_string)?;
    }
    Ok(())
}

fn write_json_pretty(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(to_string)?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(to_string)?;
    fs::write(path, bytes).map_err(to_string)
}

fn write_text_file(path: &Path, value: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(to_string)?;
    }
    fs::write(path, value).map_err(to_string)
}

fn directory_stats(path: &Path) -> Result<ExportStats, String> {
    let mut stats = ExportStats {
        file_count: 0,
        byte_size: 0,
    };
    collect_directory_stats(path, &mut stats)?;
    Ok(stats)
}

fn collect_directory_stats(path: &Path, stats: &mut ExportStats) -> Result<(), String> {
    for entry in fs::read_dir(path).map_err(to_string)? {
        let entry = entry.map_err(to_string)?;
        let metadata = entry.metadata().map_err(to_string)?;
        if metadata.is_dir() {
            collect_directory_stats(&entry.path(), stats)?;
        } else if metadata.is_file() {
            stats.file_count += 1;
            stats.byte_size += metadata.len() as i64;
        }
    }
    Ok(())
}

fn project_output_root() -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .ok_or_else(|| "could not resolve project root from CARGO_MANIFEST_DIR".to_string())?;
    Ok(project_root.join("output"))
}

fn session_folder_name(sequence: i64) -> String {
    format!("session-{:03}", sequence.max(1))
}

fn session_output_dir(output_root: &Path, sequence: i64) -> Result<PathBuf, String> {
    Ok(output_root.join(session_folder_name(sequence)))
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn safe_file_stem(value: &str) -> String {
    let stem = sanitize_id(value);
    if stem.is_empty() {
        "table".to_string()
    } else {
        stem
    }
}

fn joined_text_field(rows: &[Value], field: &str) -> String {
    rows.iter()
        .filter_map(|row| row.get(field).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn json_string_or_value(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))
}

fn push_export_warning(
    warnings: &mut Vec<ExportWarning>,
    scope: &str,
    code: &str,
    message: String,
    path: Option<String>,
    frame_id: Option<i64>,
) {
    warnings.push(ExportWarning {
        scope: scope.to_string(),
        code: code.to_string(),
        message,
        path,
        frame_id,
    });
}

fn sql_value_to_json(value: SqlValueRef<'_>) -> Value {
    match value {
        SqlValueRef::Null => Value::Null,
        SqlValueRef::Integer(value) => Value::Number(value.into()),
        SqlValueRef::Real(value) => serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        SqlValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).to_string()),
        SqlValueRef::Blob(value) => Value::String(base64_encode(value)),
    }
}

fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("frame id number must be an integer"))
            .map(Some),
        Some(Value::String(value)) if value.trim().is_empty() => Ok(None),
        Some(Value::String(value)) => value
            .trim()
            .parse::<i64>()
            .map(Some)
            .map_err(|_| serde::de::Error::custom("frame id string must contain an integer")),
        _ => Err(serde::de::Error::custom(
            "frame id must be null, an integer, or an integer string",
        )),
    }
}

fn ensure_db(app: &AppHandle) -> Result<(), String> {
    let conn = open_db(app)?;
    init_db(&conn)
}

fn clear_capture_store(app: &AppHandle) -> Result<(), String> {
    let paths = capture_paths(app)?;
    fs::create_dir_all(&paths.root_dir).map_err(to_string)?;
    replace_capture_db(&paths)?;
    clear_snapshot_dir(&paths.snapshot_dir)?;
    Ok(())
}

#[cfg(test)]
fn clear_capture_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        BEGIN IMMEDIATE;
        DELETE FROM episode_edges;
        DELETE FROM episode_nodes;
        DELETE FROM episodes;
        DELETE FROM embeddings;
        DELETE FROM ai_export_audit;
        DELETE FROM frame_quality_warnings;
        DELETE FROM sensitive_regions;
        DELETE FROM exclusion_rules;
        DELETE FROM presence_samples;
        DELETE FROM typing_bursts;
        DELETE FROM clipboard_events;
        DELETE FROM app_contexts;
        DELETE FROM frame_diffs;
        DELETE FROM content_units;
        DELETE FROM ocr_spans;
        DELETE FROM ax_nodes;
        DELETE FROM windows;
        DELETE FROM window_snapshots;
        DELETE FROM event_transitions;
        DELETE FROM capture_triggers;
        DELETE FROM ui_events;
        DELETE FROM ocr_text;
        DELETE FROM frames;
        DELETE FROM frames_fts;
        DELETE FROM capture_sessions;
        DELETE FROM sqlite_sequence WHERE name IN ('frames', 'ocr_text');
        COMMIT;
        ",
    )
    .map_err(to_string)
}

fn ensure_capture_db_empty(conn: &Connection) -> Result<(), String> {
    let frames = row_count(conn, "frames")?;
    let ocr = row_count(conn, "ocr_text")?;
    let fts = row_count(conn, "frames_fts")?;
    if frames == 0 && ocr == 0 && fts == 0 {
        Ok(())
    } else {
        Err(format!(
            "capture store was not fully cleared: frames={}, ocr_text={}, frames_fts={}",
            frames, ocr, fts
        ))
    }
}

fn row_count(conn: &Connection, table: &str) -> Result<i64, String> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
        row.get(0)
    })
    .map_err(to_string)
}

fn replace_capture_db(paths: &CapturePaths) -> Result<(), String> {
    drop_capture_db_files(&paths.db_path)?;
    let conn = Connection::open(&paths.db_path).map_err(to_string)?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(to_string)?;
    init_db(&conn)?;
    ensure_capture_db_empty(&conn)
}

fn drop_capture_db_files(db_path: &Path) -> Result<(), String> {
    for path in capture_db_files(db_path) {
        if path.exists() {
            fs::remove_file(&path).map_err(to_string)?;
        }
    }
    Ok(())
}

fn capture_db_files(db_path: &Path) -> Vec<PathBuf> {
    let db = db_path.to_string_lossy();
    vec![
        db_path.to_path_buf(),
        PathBuf::from(format!("{}-wal", db)),
        PathBuf::from(format!("{}-shm", db)),
    ]
}

fn clear_snapshot_dir(snapshot_dir: &Path) -> Result<(), String> {
    if snapshot_dir.exists() {
        fs::remove_dir_all(snapshot_dir).map_err(to_string)?;
    }
    fs::create_dir_all(snapshot_dir).map_err(to_string)
}

fn open_db(app: &AppHandle) -> Result<Connection, String> {
    let paths = capture_paths(app)?;
    fs::create_dir_all(&paths.root_dir).map_err(to_string)?;
    let conn = Connection::open(&paths.db_path).map_err(to_string)?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(to_string)?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    init_db(&conn)?;
    Ok(conn)
}

fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS frames (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          session_id TEXT,
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
        CREATE INDEX IF NOT EXISTS idx_frames_session ON frames(session_id, captured_at DESC);
        CREATE INDEX IF NOT EXISTS idx_ocr_text_frame_id ON ocr_text(frame_id);

        CREATE TABLE IF NOT EXISTS capture_sessions (
          id TEXT PRIMARY KEY,
          sequence INTEGER NOT NULL DEFAULT 0,
          started_at_ms INTEGER NOT NULL,
          stopped_at_ms INTEGER,
          status TEXT NOT NULL,
          export_path TEXT,
          frame_count INTEGER NOT NULL DEFAULT 0,
          event_count INTEGER NOT NULL DEFAULT 0,
          transition_count INTEGER NOT NULL DEFAULT 0,
          content_unit_count INTEGER NOT NULL DEFAULT 0,
          summary_json TEXT,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_capture_sessions_started
          ON capture_sessions(started_at_ms DESC);

        CREATE TABLE IF NOT EXISTS ui_events (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          ts_ms INTEGER NOT NULL,
          event_type TEXT NOT NULL,
          app_pid INTEGER,
          app_bundle_id TEXT,
          app_name TEXT,
          window_id INTEGER,
          window_title TEXT,
          x REAL,
          y REAL,
          button TEXT,
          scroll_dx REAL,
          scroll_dy REAL,
          key_category TEXT,
          modifier_flags TEXT,
          is_repeat INTEGER,
          payload_json TEXT,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_ui_events_session_ts ON ui_events(session_id, ts_ms);
        CREATE INDEX IF NOT EXISTS idx_ui_events_ts ON ui_events(ts_ms);
        CREATE INDEX IF NOT EXISTS idx_ui_events_type_ts ON ui_events(event_type, ts_ms);

        CREATE TABLE IF NOT EXISTS capture_triggers (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          ts_ms INTEGER NOT NULL,
          trigger_type TEXT NOT NULL,
          caused_by_event_ids TEXT NOT NULL,
          settle_delay_ms INTEGER NOT NULL,
          rate_limited INTEGER DEFAULT 0,
          dedupe_policy TEXT NOT NULL,
          pre_frame_id TEXT,
          post_frame_id TEXT,
          status TEXT NOT NULL,
          error TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_capture_triggers_session_ts
          ON capture_triggers(session_id, ts_ms);

        CREATE TABLE IF NOT EXISTS event_transitions (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          trigger_id TEXT NOT NULL,
          primary_event_id TEXT,
          pre_frame_id TEXT,
          post_frame_id TEXT,
          ts_start_ms INTEGER NOT NULL,
          ts_end_ms INTEGER NOT NULL,
          transition_type TEXT,
          confidence REAL,
          summary TEXT,
          changed_region_json TEXT,
          FOREIGN KEY(trigger_id) REFERENCES capture_triggers(id)
        );
        CREATE INDEX IF NOT EXISTS idx_event_transitions_session_ts
          ON event_transitions(session_id, ts_start_ms);

        CREATE TABLE IF NOT EXISTS window_snapshots (
          id TEXT PRIMARY KEY,
          frame_id TEXT,
          ts_ms INTEGER NOT NULL,
          active_window_id INTEGER,
          active_app_pid INTEGER,
          active_app_bundle_id TEXT,
          screen_count INTEGER,
          raw_json TEXT
        );

        CREATE TABLE IF NOT EXISTS windows (
          id TEXT PRIMARY KEY,
          window_snapshot_id TEXT NOT NULL,
          cg_window_id INTEGER,
          owner_pid INTEGER,
          owner_name TEXT,
          bundle_id TEXT,
          window_title TEXT,
          layer INTEGER,
          alpha REAL,
          is_onscreen INTEGER,
          is_active INTEGER,
          bounds_x REAL,
          bounds_y REAL,
          bounds_w REAL,
          bounds_h REAL,
          workspace INTEGER,
          raw_json TEXT,
          FOREIGN KEY(window_snapshot_id) REFERENCES window_snapshots(id)
        );

        CREATE TABLE IF NOT EXISTS ax_nodes (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          parent_id TEXT,
          app_pid INTEGER,
          window_id INTEGER,
          role TEXT,
          subrole TEXT,
          role_description TEXT,
          title TEXT,
          value TEXT,
          description TEXT,
          help TEXT,
          identifier TEXT,
          document TEXT,
          url TEXT,
          selected_text TEXT,
          selected_text_range_json TEXT,
          visible_character_range_json TEXT,
          number_of_characters INTEGER,
          focused INTEGER,
          enabled INTEGER,
          selected INTEGER,
          bounds_x REAL,
          bounds_y REAL,
          bounds_w REAL,
          bounds_h REAL,
          actions_json TEXT,
          children_count INTEGER,
          depth INTEGER,
          raw_json TEXT,
          FOREIGN KEY(frame_id) REFERENCES frames(id)
        );
        CREATE INDEX IF NOT EXISTS idx_ax_nodes_frame ON ax_nodes(frame_id);
        CREATE INDEX IF NOT EXISTS idx_ax_nodes_bounds ON ax_nodes(frame_id, bounds_x, bounds_y);

        CREATE TABLE IF NOT EXISTS ocr_spans (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          engine TEXT NOT NULL,
          text TEXT NOT NULL,
          confidence REAL,
          lang TEXT,
          block_index INTEGER,
          line_index INTEGER,
          word_index INTEGER,
          bounds_x REAL NOT NULL,
          bounds_y REAL NOT NULL,
          bounds_w REAL NOT NULL,
          bounds_h REAL NOT NULL,
          normalized_bounds_json TEXT,
          raw_json TEXT,
          FOREIGN KEY(frame_id) REFERENCES frames(id)
        );
        CREATE INDEX IF NOT EXISTS idx_ocr_spans_frame ON ocr_spans(frame_id);

        CREATE TABLE IF NOT EXISTS content_units (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          window_id INTEGER,
          source TEXT NOT NULL,
          unit_type TEXT NOT NULL,
          text TEXT,
          text_hash TEXT,
          semantic_role TEXT,
          ax_node_id TEXT,
          ocr_span_ids TEXT,
          adapter_object_id TEXT,
          bounds_x REAL,
          bounds_y REAL,
          bounds_w REAL,
          bounds_h REAL,
          crop_path TEXT,
          visible_ratio REAL,
          center_distance REAL,
          confidence REAL,
          created_at_ms INTEGER NOT NULL,
          raw_json TEXT,
          FOREIGN KEY(frame_id) REFERENCES frames(id)
        );
        CREATE INDEX IF NOT EXISTS idx_content_units_frame ON content_units(frame_id);
        CREATE INDEX IF NOT EXISTS idx_content_units_text_hash ON content_units(text_hash);

        CREATE TABLE IF NOT EXISTS frame_diffs (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          from_frame_id TEXT NOT NULL,
          to_frame_id TEXT NOT NULL,
          ts_ms INTEGER NOT NULL,
          same_app INTEGER,
          same_window INTEGER,
          visual_phash_distance REAL,
          changed_region_json TEXT,
          added_text_hashes TEXT,
          removed_text_hashes TEXT,
          stable_text_hashes TEXT,
          ax_tree_delta_json TEXT,
          ocr_delta_json TEXT,
          diff_type TEXT,
          confidence REAL,
          summary TEXT,
          FOREIGN KEY(from_frame_id) REFERENCES frames(id),
          FOREIGN KEY(to_frame_id) REFERENCES frames(id)
        );

        CREATE TABLE IF NOT EXISTS app_contexts (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          adapter_id TEXT NOT NULL,
          object_type TEXT NOT NULL,
          primary_id TEXT,
          title TEXT,
          url TEXT,
          file_path TEXT,
          repo_path TEXT,
          line_number INTEGER,
          selected_text TEXT,
          focused_object TEXT,
          confidence REAL,
          metadata_json TEXT,
          FOREIGN KEY(frame_id) REFERENCES frames(id)
        );

        CREATE TABLE IF NOT EXISTS clipboard_events (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          ts_ms INTEGER NOT NULL,
          change_count INTEGER NOT NULL,
          content_type TEXT NOT NULL,
          text_hash TEXT,
          redacted_preview TEXT,
          byte_size INTEGER,
          source_frame_id TEXT,
          source_content_unit_id TEXT,
          target_frame_id TEXT,
          pasted_within_ms INTEGER,
          metadata_json TEXT
        );

        CREATE TABLE IF NOT EXISTS typing_bursts (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          started_at_ms INTEGER NOT NULL,
          ended_at_ms INTEGER NOT NULL,
          app_pid INTEGER,
          app_bundle_id TEXT,
          app_name TEXT,
          window_id INTEGER,
          window_title TEXT,
          focused_ax_node_id TEXT,
          focused_content_unit_id TEXT,
          char_count INTEGER DEFAULT 0,
          backspace_count INTEGER DEFAULT 0,
          enter_count INTEGER DEFAULT 0,
          paste_count INTEGER DEFAULT 0,
          shortcut_count INTEGER DEFAULT 0,
          committed INTEGER DEFAULT 0,
          commit_signal TEXT,
          raw_text_captured INTEGER DEFAULT 0,
          text_hash TEXT,
          redacted_preview TEXT,
          pre_frame_id TEXT,
          post_frame_id TEXT
        );

        CREATE TABLE IF NOT EXISTS presence_samples (
          id TEXT PRIMARY KEY,
          session_id TEXT,
          ts_ms INTEGER NOT NULL,
          idle_seconds REAL,
          display_asleep INTEGER,
          screen_locked INTEGER,
          active_input_recently INTEGER,
          cursor_moved_recently INTEGER,
          frontmost_app_bundle_id TEXT
        );

        CREATE TABLE IF NOT EXISTS exclusion_rules (
          id TEXT PRIMARY KEY,
          rule_type TEXT NOT NULL,
          pattern TEXT NOT NULL,
          action TEXT NOT NULL,
          enabled INTEGER DEFAULT 1,
          created_at_ms INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sensitive_regions (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          region_type TEXT NOT NULL,
          bounds_x REAL,
          bounds_y REAL,
          bounds_w REAL,
          bounds_h REAL,
          source TEXT NOT NULL,
          confidence REAL,
          action_taken TEXT,
          metadata_json TEXT
        );

        CREATE TABLE IF NOT EXISTS frame_quality_warnings (
          id TEXT PRIMARY KEY,
          frame_id TEXT NOT NULL,
          warning_type TEXT NOT NULL,
          severity TEXT NOT NULL,
          message TEXT NOT NULL,
          evidence_json TEXT NOT NULL,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_frame_quality_warnings_frame
          ON frame_quality_warnings(frame_id);

        CREATE TABLE IF NOT EXISTS ai_export_audit (
          id TEXT PRIMARY KEY,
          created_at_ms INTEGER NOT NULL,
          export_type TEXT NOT NULL,
          lookback_start_ms INTEGER,
          lookback_end_ms INTEGER,
          input_frame_count INTEGER NOT NULL,
          exported_frame_count INTEGER NOT NULL,
          excluded_frame_count INTEGER NOT NULL,
          masked_image_count INTEGER NOT NULL,
          redacted_text_count INTEGER NOT NULL,
          warnings_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS embeddings (
          id TEXT PRIMARY KEY,
          object_type TEXT NOT NULL,
          object_id TEXT NOT NULL,
          model TEXT NOT NULL,
          dims INTEGER NOT NULL,
          vector BLOB NOT NULL,
          text_hash TEXT,
          created_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_embeddings_object ON embeddings(object_type, object_id);

        CREATE TABLE IF NOT EXISTS episodes (
          id TEXT PRIMARY KEY,
          started_at_ms INTEGER NOT NULL,
          ended_at_ms INTEGER,
          status TEXT NOT NULL,
          primary_goal TEXT,
          current_surface_type TEXT,
          confidence REAL,
          summary TEXT,
          metadata_json TEXT
        );

        CREATE TABLE IF NOT EXISTS episode_nodes (
          id TEXT PRIMARY KEY,
          episode_id TEXT NOT NULL,
          node_type TEXT NOT NULL,
          object_id TEXT NOT NULL,
          role TEXT,
          ts_ms INTEGER NOT NULL,
          confidence REAL,
          FOREIGN KEY(episode_id) REFERENCES episodes(id)
        );

        CREATE TABLE IF NOT EXISTS episode_edges (
          id TEXT PRIMARY KEY,
          episode_id TEXT NOT NULL,
          from_node_id TEXT NOT NULL,
          to_node_id TEXT NOT NULL,
          edge_type TEXT NOT NULL,
          evidence_json TEXT,
          confidence REAL,
          FOREIGN KEY(episode_id) REFERENCES episodes(id)
        );
        ",
    )
    .map_err(to_string)?;

    ensure_frame_columns(conn)?;
    ensure_session_columns(conn)?;
    ensure_default_exclusion_rules(conn)?;
    Ok(())
}

fn ensure_frame_columns(conn: &Connection) -> Result<(), String> {
    let columns = table_columns(conn, "frames")?;
    let additions = [
        ("session_id", "TEXT"),
        ("capture_provider", "TEXT"),
        ("scope", "TEXT"),
        ("display_id", "TEXT"),
        ("window_id", "INTEGER"),
        ("app_pid", "INTEGER"),
        ("app_bundle_id", "TEXT"),
        ("screen_scale", "REAL"),
        ("pixel_width", "INTEGER"),
        ("pixel_height", "INTEGER"),
        ("full_screenshot_path", "TEXT"),
        ("active_window_crop_path", "TEXT"),
        ("active_element_crop_path", "TEXT"),
        ("phash", "TEXT"),
        ("privacy_status", "TEXT DEFAULT 'normal'"),
        ("capture_trigger_id", "TEXT"),
        ("previous_frame_id", "TEXT"),
    ];

    for (name, definition) in additions {
        if !columns.contains(name) {
            conn.execute(
                &format!("ALTER TABLE frames ADD COLUMN {} {}", name, definition),
                [],
            )
            .map_err(to_string)?;
        }
    }

    Ok(())
}

fn ensure_session_columns(conn: &Connection) -> Result<(), String> {
    ensure_table_column(
        conn,
        "capture_sessions",
        "sequence",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    backfill_session_sequences(conn)?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_capture_sessions_sequence
         ON capture_sessions(sequence)",
        [],
    )
    .map_err(to_string)?;

    let tables = [
        "ui_events",
        "capture_triggers",
        "event_transitions",
        "frame_diffs",
        "clipboard_events",
        "typing_bursts",
        "presence_samples",
    ];
    for table in tables {
        ensure_table_column(conn, table, "session_id", "TEXT")?;
    }
    Ok(())
}

fn backfill_session_sequences(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT id
             FROM capture_sessions
             WHERE sequence IS NULL OR sequence <= 0
             ORDER BY started_at_ms ASC, created_at_ms ASC, id ASC",
        )
        .map_err(to_string)?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    drop(stmt);

    let mut next = next_session_sequence(conn)?;
    for id in ids {
        conn.execute(
            "UPDATE capture_sessions SET sequence = ?2 WHERE id = ?1",
            params![id, next],
        )
        .map_err(to_string)?;
        next += 1;
    }
    Ok(())
}

fn ensure_table_column(
    conn: &Connection,
    table: &str,
    name: &str,
    definition: &str,
) -> Result<(), String> {
    let columns = table_columns(conn, table)?;
    if !columns.contains(name) {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {} {}", table, name, definition),
            [],
        )
        .map_err(to_string)?;
    }
    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(to_string)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(to_string)?;

    let mut columns = HashSet::new();
    for row in rows {
        columns.insert(row.map_err(to_string)?);
    }
    Ok(columns)
}

fn ensure_default_exclusion_rules(conn: &Connection) -> Result<(), String> {
    let existing: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM exclusion_rules WHERE id LIKE 'default-%'",
            [],
            |row| row.get(0),
        )
        .map_err(to_string)?;
    if existing > 0 {
        return Ok(());
    }

    let created = now_millis();
    let defaults = [
        (
            "default-password-managers",
            "app_bundle",
            "1Password|Bitwarden|LastPass|Dashlane|KeePass",
            "skip_capture",
        ),
        (
            "default-private-auth",
            "window_title_regex",
            "password|passcode|verification code|authentication|sign in",
            "store_redacted",
        ),
        (
            "default-sensitive-sites",
            "url_regex",
            "bank|checkout|payment|health|medical|1password|bitwarden",
            "store_redacted",
        ),
        (
            "default-api-secrets",
            "content_regex",
            "sk-[A-Za-z0-9]|api[_-]?key|token|secret",
            "never_send_to_ai",
        ),
    ];

    for (id, rule_type, pattern, action) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO exclusion_rules (
                id, rule_type, pattern, action, enabled, created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            params![id, rule_type, pattern, action, created],
        )
        .map_err(to_string)?;
    }
    Ok(())
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

fn capture_window_screenshot(window_id: i64, path: &Path) -> Result<(), String> {
    let output = Command::new("/usr/sbin/screencapture")
        .arg("-x")
        .arg("-t")
        .arg("jpg")
        .arg("-l")
        .arg(window_id.to_string())
        .arg(path)
        .output()
        .map_err(|error| format!("window screencapture failed to start: {}", error))?;

    if output.status.success() && path.exists() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "active-window crop failed".to_string()
        } else {
            stderr
        })
    }
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(i64, i64)> {
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }

    let mut index = 2;
    while index + 9 < bytes.len() {
        if bytes[index] != 0xff {
            index += 1;
            continue;
        }
        while index < bytes.len() && bytes[index] == 0xff {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let marker = bytes[index];
        index += 1;
        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        if index + 2 > bytes.len() {
            return None;
        }
        let length = u16::from_be_bytes([bytes[index], bytes[index + 1]]) as usize;
        if length < 2 || index + length > bytes.len() {
            return None;
        }
        let is_sof = matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        );
        if is_sof && index + 7 < bytes.len() {
            let height = u16::from_be_bytes([bytes[index + 3], bytes[index + 4]]) as i64;
            let width = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]) as i64;
            return Some((width, height));
        }
        index += length;
    }

    None
}

fn collect_window_snapshot(paths: &CapturePaths) -> Result<WindowSnapshotPayload, String> {
    if !cfg!(target_os = "macos") {
        return Err("window graph capture is only implemented for macOS".to_string());
    }

    let helper_path = ensure_swift_helper(
        paths,
        "window_snapshot",
        WINDOW_SNAPSHOT_SWIFT,
        &["AppKit", "CoreGraphics", "ApplicationServices"],
    )?;
    let output = Command::new(helper_path)
        .output()
        .map_err(|error| format!("window snapshot helper failed to start: {}", error))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "window snapshot helper failed".to_string()
        } else {
            stderr
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).map_err(to_string)
}

fn privacy_decision(
    app: &AppHandle,
    context: &AccessibilityContext,
) -> Result<PrivacyDecision, String> {
    let conn = open_db(app)?;
    let mut decision = PrivacyDecision {
        skip_capture: false,
        status: "normal".to_string(),
        regions: Vec::new(),
    };

    let haystack = format!(
        "{}\n{}\n{}\n{}",
        context.app_name.as_deref().unwrap_or(""),
        context.app_bundle_id.as_deref().unwrap_or(""),
        context.window_name.as_deref().unwrap_or(""),
        context.browser_url.as_deref().unwrap_or("")
    )
    .to_lowercase();

    let mut stmt = conn
        .prepare("SELECT rule_type, pattern, action FROM exclusion_rules WHERE enabled = 1")
        .map_err(to_string)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(to_string)?;

    for row in rows {
        let (rule_type, pattern, action) = row.map_err(to_string)?;
        let matched = match rule_type.as_str() {
            "app_bundle" => pattern_match(
                context
                    .app_bundle_id
                    .as_deref()
                    .or(context.app_name.as_deref()),
                &pattern,
            ),
            "window_title_regex" => pattern_match(context.window_name.as_deref(), &pattern),
            "url_regex" => pattern_match(context.browser_url.as_deref(), &pattern),
            "content_regex" => pattern_match(Some(&context.text), &pattern),
            _ => pattern_match(Some(&haystack), &pattern),
        };
        if matched {
            if action == "skip_capture" {
                decision.skip_capture = true;
                decision.status = "skipped_sensitive".to_string();
            } else {
                decision.status = "redacted".to_string();
            }
            decision.regions.push(DetectedSensitiveRegion {
                region_type: if action == "skip_capture" {
                    "excluded_app".to_string()
                } else {
                    "unknown_sensitive".to_string()
                },
                bounds: None,
                source: "user_rule".to_string(),
                confidence: 0.9,
                action_taken: action,
                metadata_json: Some(
                    serde_json::json!({ "rule_type": rule_type, "pattern": pattern }).to_string(),
                ),
            });
        }
    }

    let sensitive_patterns = [
        ("credit_card", "credit card"),
        ("api_key", "api key"),
        ("token", "token"),
        ("password_field", "password"),
        ("banking", "bank"),
        ("health", "health"),
    ];
    for (region_type, needle) in sensitive_patterns {
        if haystack.contains(needle) || context.text.to_lowercase().contains(needle) {
            if decision.status == "normal" {
                decision.status = "redacted".to_string();
            }
            decision.regions.push(DetectedSensitiveRegion {
                region_type: region_type.to_string(),
                bounds: None,
                source: "regex".to_string(),
                confidence: 0.62,
                action_taken: "redacted_text".to_string(),
                metadata_json: None,
            });
        }
    }

    Ok(decision)
}

fn pattern_match(value: Option<&str>, pattern: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    let value = value.to_lowercase();
    pattern
        .split('|')
        .map(|part| part.trim().to_lowercase())
        .filter(|part| !part.is_empty())
        .any(|part| value.contains(&part))
}

fn collect_accessibility_context(paths: &CapturePaths) -> AccessibilityContext {
    match collect_accessibility_context_native(paths) {
        Ok(context) if context_has_accessibility_signal(&context) => return context,
        Ok(context) if context.error.is_none() => return context,
        Ok(native_context) => {
            let mut fallback = collect_accessibility_context_applescript();
            if !context_has_accessibility_signal(&fallback) {
                fallback.error = native_context.error.or(fallback.error);
            }
            fallback
        }
        Err(native_error) => {
            let mut fallback = collect_accessibility_context_applescript();
            if !context_has_accessibility_signal(&fallback) {
                fallback.error = Some(match fallback.error {
                    Some(fallback_error) => format!(
                        "native accessibility: {}; applescript: {}",
                        native_error, fallback_error
                    ),
                    None => native_error,
                });
            }
            fallback
        }
    }
}

fn collect_accessibility_context_native(
    paths: &CapturePaths,
) -> Result<AccessibilityContext, String> {
    if !cfg!(target_os = "macos") {
        return Err("native accessibility capture is only implemented for macOS".to_string());
    }

    let helper_path = ensure_swift_helper(
        paths,
        "accessibility_snapshot",
        ACCESSIBILITY_SNAPSHOT_SWIFT,
        &["ApplicationServices", "AppKit"],
    )?;
    let output = Command::new(helper_path)
        .output()
        .map_err(|error| format!("accessibility helper failed to start: {}", error))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut context = parse_accessibility_output(&stdout);
    if output.status.success() {
        Ok(context)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if context.error.is_none() {
            context.error = Some(if stderr.is_empty() {
                "accessibility helper failed".to_string()
            } else {
                stderr
            });
        }
        Ok(context)
    }
}

fn collect_accessibility_context_applescript() -> AccessibilityContext {
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

fn context_has_accessibility_signal(context: &AccessibilityContext) -> bool {
    context.app_name.is_some()
        || context.window_name.is_some()
        || context.browser_url.is_some()
        || context.document_path.is_some()
        || !context.text.trim().is_empty()
        || !context.nodes.is_empty()
}

fn parse_accessibility_output(stdout: &str) -> AccessibilityContext {
    let mut context = AccessibilityContext::default();
    let mut text_parts = Vec::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(4, '\t');
        match parts.next() {
            Some("APP") => context.app_name = non_empty(parts.next().unwrap_or("").to_string()),
            Some("APP_PID") => {
                context.app_pid = parts.next().and_then(|value| value.parse::<i64>().ok());
            }
            Some("APP_BUNDLE_ID") => {
                context.app_bundle_id = non_empty(parts.next().unwrap_or("").to_string())
            }
            Some("WINDOW") => {
                context.window_name = non_empty(parts.next().unwrap_or("").to_string())
            }
            Some("WINDOW_ID") => {
                context.window_id = parts.next().and_then(|value| value.parse::<i64>().ok());
            }
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
            Some("ERROR") => {
                context.error = non_empty(parts.next().unwrap_or("").to_string());
            }
            Some("NODE_JSON") => {
                let json = parts.next().unwrap_or("");
                if let Ok(mut node) = serde_json::from_str::<AccessibilityNode>(json) {
                    if node.text.trim().is_empty() {
                        node.text = node_text(&node);
                    }
                    if !node.text.trim().is_empty() {
                        text_parts.push(node.text.clone());
                    }
                    context.nodes.push(node);
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
                    context.nodes.push(AccessibilityNode {
                        role,
                        text,
                        depth,
                        ..AccessibilityNode::default()
                    });
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

fn ensure_swift_helper(
    paths: &CapturePaths,
    name: &str,
    source: &str,
    frameworks: &[&str],
) -> Result<PathBuf, String> {
    fs::create_dir_all(&paths.helper_dir).map_err(to_string)?;
    let source_path = paths.helper_dir.join(format!("{}.swift", name));
    let helper_path = paths.helper_dir.join(name);

    let should_write = fs::read_to_string(&source_path)
        .map(|existing| existing != source)
        .unwrap_or(true);
    if should_write {
        fs::write(&source_path, source).map_err(to_string)?;
        let _ = fs::remove_file(&helper_path);
    }

    if !helper_path.exists() {
        let mut command = Command::new("/usr/bin/swiftc");
        command.arg("-O").arg(&source_path);
        for framework in frameworks {
            command.arg("-framework").arg(framework);
        }
        command.arg("-o").arg(&helper_path);

        let output = command
            .output()
            .map_err(|error| format!("swiftc failed to start: {}", error))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("swiftc failed to build {}", name)
            } else {
                stderr
            });
        }
    }

    Ok(helper_path)
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

fn stop_runtime(state: &CaptureState) -> Result<(), String> {
    let handle = {
        let mut runtime = lock_runtime(state)?;
        if let Some(stop_signal) = runtime.stop_signal.take() {
            stop_signal.store(true, Ordering::Relaxed);
        }
        runtime.running = false;
        runtime.started_at = None;
        runtime.worker.take()
    };

    if let Some(handle) = handle {
        handle
            .join()
            .map_err(|_| "capture worker panicked while stopping".to_string())?;
    }

    Ok(())
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

fn update_error_and_island(app: &AppHandle, state: &Arc<Mutex<CaptureRuntime>>, error: String) {
    update_error(state, error);
    if let Ok(status) = capture_status_snapshot_inner(app, state) {
        crate::session_island::update_session_island_from_status(
            &status,
            crate::session_island::SessionIslandState::Error,
        );
    }
}

fn update_skip(state: &Arc<Mutex<CaptureRuntime>>) {
    if let Ok(mut runtime) = state.lock() {
        runtime.skipped_samples += 1;
        runtime.last_skipped_at = Some(now_millis());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(
        app: &str,
        window: &str,
        url: Option<&str>,
        text_hash: Option<&str>,
    ) -> SemanticFingerprint {
        SemanticFingerprint {
            app_name: Some(app.to_string()),
            window_name: Some(window.to_string()),
            browser_url: url.map(str::to_string),
            document_path: None,
            text_hash: text_hash.map(str::to_string),
        }
    }

    #[test]
    fn dedupe_requires_image_and_content_to_match() {
        assert!(should_skip_dedup(
            true,
            Some("img-a"),
            "img-a",
            Some("txt-a"),
            Some("txt-a")
        ));
        assert!(!should_skip_dedup(
            true,
            Some("img-a"),
            "img-b",
            Some("txt-a"),
            Some("txt-a")
        ));
        assert!(!should_skip_dedup(
            true,
            Some("img-a"),
            "img-a",
            Some("txt-a"),
            Some("txt-b")
        ));
        assert!(!should_skip_dedup(
            false,
            Some("img-a"),
            "img-a",
            Some("txt-a"),
            Some("txt-a")
        ));
    }

    #[test]
    fn semantic_trigger_classifies_meaningful_context_changes() {
        let base = fingerprint(
            "Chrome",
            "Docs",
            Some("https://example.com/a"),
            Some("text-a"),
        );

        assert_eq!(
            semantic_trigger(
                Some(&base),
                &fingerprint("Arc", "Docs", Some("https://example.com/a"), Some("text-a"))
            ),
            Some("app_switch")
        );
        assert_eq!(
            semantic_trigger(
                Some(&base),
                &fingerprint(
                    "Chrome",
                    "Other",
                    Some("https://example.com/a"),
                    Some("text-a")
                )
            ),
            Some("window_focus")
        );
        assert_eq!(
            semantic_trigger(
                Some(&base),
                &fingerprint(
                    "Chrome",
                    "Docs",
                    Some("https://example.com/b"),
                    Some("text-a")
                )
            ),
            Some("navigation")
        );
        assert_eq!(
            semantic_trigger(
                Some(&base),
                &fingerprint(
                    "Chrome",
                    "Docs",
                    Some("https://example.com/a"),
                    Some("text-b")
                )
            ),
            Some("accessibility_change")
        );
        assert_eq!(semantic_trigger(Some(&base), &base), None);
    }

    #[test]
    fn stop_runtime_signals_and_joins_worker_before_returning() {
        let state = CaptureState::default();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let worker_started = Arc::new(AtomicBool::new(false));
        let worker_finished = Arc::new(AtomicBool::new(false));
        let thread_stop = stop_signal.clone();
        let thread_started = worker_started.clone();
        let thread_finished = worker_finished.clone();
        let worker = thread::spawn(move || {
            thread_started.store(true, Ordering::SeqCst);
            while !thread_stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1));
            }
            thread_finished.store(true, Ordering::SeqCst);
        });

        {
            let mut runtime = lock_runtime(&state).unwrap();
            runtime.running = true;
            runtime.started_at = Some(1);
            runtime.stop_signal = Some(stop_signal);
            runtime.worker = Some(worker);
        }

        while !worker_started.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
        }

        stop_runtime(&state).unwrap();

        assert!(worker_finished.load(Ordering::SeqCst));
        let runtime = lock_runtime(&state).unwrap();
        assert!(!runtime.running);
        assert!(runtime.started_at.is_none());
        assert!(runtime.stop_signal.is_none());
        assert!(runtime.worker.is_none());
    }

    fn insert_test_frame(conn: &Connection, snapshot_path: &str) -> i64 {
        conn.execute(
            "INSERT INTO frames (
                captured_at, snapshot_path, app_name, window_name, browser_url,
                document_path, focused, capture_trigger, text_source,
                accessibility_text, accessibility_tree_json, full_text,
                content_hash, image_hash, created_at
             ) VALUES (1, ?1, 'App', 'Window', NULL, NULL, 1, 'manual',
                       'accessibility', 'hello', NULL, 'hello searchable', 'h1', 'i1', 1)",
            params![snapshot_path],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn insert_numbered_test_session(conn: &Connection) -> CaptureSession {
        let sequence = next_session_sequence(conn).unwrap();
        let id = format!("session-test-{}", sequence);
        let started_at = now_millis();
        conn.execute(
            "INSERT INTO capture_sessions (
                id, sequence, started_at_ms, stopped_at_ms, status, created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, 'stopped', ?3)",
            params![id, sequence, started_at, started_at + 1],
        )
        .unwrap();
        load_capture_session(conn, &id).unwrap()
    }

    fn insert_export_test_frame(conn: &Connection, session_id: &str, image_path: &str) -> i64 {
        conn.execute(
            INSERT_FRAME_SQL,
            params![
                10_i64,
                image_path,
                Some("App"),
                Some("Window"),
                Some("https://example.com"),
                Option::<String>::None,
                true,
                "manual",
                "hybrid",
                Some("accessibility hello"),
                Some(r#"[{"role":"AXWindow","text":"hello"}]"#),
                Some("accessibility hello\nocr hello"),
                Some("content-hash"),
                Some("image-hash"),
                10_i64,
                "screencapture_cli",
                "active_display",
                Some("main"),
                Option::<i64>::None,
                Some(123_i64),
                Some("com.example.App"),
                1.0_f64,
                Some(1_i64),
                Some(1_i64),
                image_path,
                Option::<String>::None,
                Option::<String>::None,
                Some("phash"),
                "normal",
                Some("trigger-1"),
                Option::<String>::None,
                Some(session_id),
            ],
        )
        .unwrap();
        let frame_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO ocr_text (frame_id, text, text_json, ocr_engine)
             VALUES (?1, 'ocr hello', '[{\"text\":\"ocr hello\"}]', 'test')",
            params![frame_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ocr_spans (
                id, frame_id, engine, text, bounds_x, bounds_y, bounds_w, bounds_h
             ) VALUES ('ocr-1', ?1, 'test', 'ocr hello', 0.0, 0.0, 1.0, 1.0)",
            params![frame_id.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ax_nodes (
                id, frame_id, role, title, focused, bounds_x, bounds_y, bounds_w,
                bounds_h, depth, raw_json
             ) VALUES ('ax-1', ?1, 'AXWindow', 'hello', 1, 0.0, 0.0, 1.0, 1.0, 0, '{}')",
            params![frame_id.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO content_units (
                id, frame_id, source, unit_type, text, text_hash, confidence, created_at_ms
             ) VALUES ('unit-1', ?1, 'ocr', 'text', 'ocr hello', 'h', 0.8, 10)",
            params![frame_id.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO app_contexts (
                id, frame_id, adapter_id, object_type, title, confidence
             ) VALUES ('ctx-1', ?1, 'test', 'browser_tab', 'Window', 0.9)",
            params![frame_id.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ui_events (
                id, session_id, ts_ms, event_type, created_at_ms
             ) VALUES ('evt-1', ?1, 10, 'click', 10)",
            params![session_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO capture_triggers (
                id, session_id, ts_ms, trigger_type, caused_by_event_ids,
                settle_delay_ms, dedupe_policy, post_frame_id, status
             ) VALUES ('trigger-1', ?1, 10, 'manual', '[\"evt-1\"]', 0, 'manual', ?2, 'captured')",
            params![session_id, frame_id.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_transitions (
                id, session_id, trigger_id, primary_event_id, post_frame_id,
                ts_start_ms, ts_end_ms, transition_type
             ) VALUES ('tx-1', ?1, 'trigger-1', 'evt-1', ?2, 10, 11, 'manual')",
            params![session_id, frame_id.to_string()],
        )
        .unwrap();
        frame_id
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_resume_test_frame(
        conn: &Connection,
        session_id: &str,
        image_path: &str,
        captured_at: i64,
        app_name: &str,
        bundle_id: &str,
        window_name: &str,
        browser_url: &str,
        object_type: &str,
        text: &str,
        phash: &str,
        previous_frame_id: Option<i64>,
    ) -> i64 {
        let trigger_id = format!("trigger-{}", captured_at);
        conn.execute(
            INSERT_FRAME_SQL,
            params![
                captured_at,
                image_path,
                Some(app_name),
                Some(window_name),
                Some(browser_url),
                Option::<String>::None,
                true,
                "navigation",
                "hybrid",
                Some(text),
                Some(r#"[{"role":"AXWindow","text":"resume"}]"#),
                Some(text),
                Some(stable_hash_bytes(text.as_bytes())),
                Some(phash),
                captured_at,
                "screencapture_cli",
                "active_window",
                Some("main"),
                Some(42_i64),
                Some(123_i64),
                Some(bundle_id),
                1.0_f64,
                Some(1_i64),
                Some(1_i64),
                image_path,
                Option::<String>::None,
                Option::<String>::None,
                Some(phash),
                "normal",
                Some(trigger_id.clone()),
                previous_frame_id.map(|id| id.to_string()),
                Some(session_id),
            ],
        )
        .unwrap();
        let frame_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO content_units (
                id, frame_id, source, unit_type, text, text_hash, semantic_role,
                confidence, created_at_ms
             ) VALUES (?1, ?2, 'test', 'paragraph', ?3, ?4, 'main_content', 0.92, ?5)",
            params![
                format!("unit-{}", frame_id),
                frame_id.to_string(),
                text,
                stable_hash_bytes(text.as_bytes()),
                captured_at,
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO app_contexts (
                id, frame_id, adapter_id, object_type, primary_id, title, url,
                confidence, metadata_json
             ) VALUES (?1, ?2, 'test_adapter', ?3, ?4, ?5, ?4, 0.95, '{}')",
            params![
                format!("ctx-{}", frame_id),
                frame_id.to_string(),
                object_type,
                browser_url,
                window_name,
            ],
        )
        .unwrap();
        frame_id
    }

    fn insert_resume_test_transition(
        conn: &Connection,
        session_id: &str,
        id: &str,
        pre_frame_id: i64,
        post_frame_id: i64,
        ts: i64,
        transition_type: &str,
    ) {
        let event_id = format!("event-{}", id);
        conn.execute(
            "INSERT INTO ui_events (
                id, session_id, ts_ms, event_type, app_name, created_at_ms
             ) VALUES (?1, ?2, ?3, 'click', 'test', ?3)",
            params![event_id, session_id, ts],
        )
        .unwrap();
        let trigger_id = format!("trigger-{}", id);
        conn.execute(
            "INSERT INTO capture_triggers (
                id, session_id, ts_ms, trigger_type, caused_by_event_ids,
                settle_delay_ms, dedupe_policy, pre_frame_id, post_frame_id, status
             ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 'test', ?6, ?7, 'captured')",
            params![
                trigger_id,
                session_id,
                ts,
                transition_type,
                serde_json::json!([event_id.clone()]).to_string(),
                pre_frame_id.to_string(),
                post_frame_id.to_string(),
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_transitions (
                id, session_id, trigger_id, primary_event_id, pre_frame_id, post_frame_id,
                ts_start_ms, ts_end_ms, transition_type, confidence, summary
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0.6, 'test transition')",
            params![
                id,
                session_id,
                format!("trigger-{}", id),
                event_id,
                pre_frame_id.to_string(),
                post_frame_id.to_string(),
                ts,
                ts + 1,
                transition_type,
            ],
        )
        .unwrap();
    }

    fn tiny_png() -> &'static [u8] {
        &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4,
            0, 9, 251, 3, 253, 160, 117, 240, 210, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ]
    }

    #[test]
    fn production_frame_insert_matches_schema_column_count() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        conn.execute(
            INSERT_FRAME_SQL,
            params![
                1_i64,
                "/tmp/smalltalk-frame.jpg",
                Some("App"),
                Some("Window"),
                Option::<String>::None,
                Option::<String>::None,
                true,
                "manual",
                "accessibility",
                Some("visible text"),
                Option::<String>::None,
                Some("visible text searchable"),
                Some("content-hash"),
                Some("image-hash"),
                1_i64,
                "screencapture_cli",
                "active_display",
                Some("main"),
                Option::<i64>::None,
                Some(123_i64),
                Some("com.example.App"),
                1.0_f64,
                Some(1440_i64),
                Some(900_i64),
                "/tmp/smalltalk-frame.jpg",
                Option::<String>::None,
                Option::<String>::None,
                Some("phash"),
                "normal",
                Some("trigger-1"),
                Option::<String>::None,
                Some("session-1"),
            ],
        )
        .unwrap();

        let frames: i64 = conn
            .query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0))
            .unwrap();
        assert_eq!(frames, 1);
    }

    #[test]
    fn text_redaction_removes_common_secrets_and_contact_data() {
        let redacted = redact_text_for_ai(
            "email person@example.com with sk-abc123SECRET456 or call +1-415-555-1212",
        );
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("[REDACTED_SECRET]"));
        assert!(redacted.contains("[REDACTED_PHONE]"));
        assert!(!redacted.contains("person@example.com"));
        assert!(!redacted.contains("sk-abc123SECRET456"));
    }

    #[test]
    fn safe_ai_export_excludes_never_send_frames_completely() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let output_root =
            std::env::temp_dir().join(format!("smalltalk-safe-export-exclude-{}", now_millis()));
        let image_path = output_root.join("source.png");
        fs::create_dir_all(&output_root).unwrap();
        fs::write(&image_path, tiny_png()).unwrap();

        let session = insert_numbered_test_session(&conn);
        let frame_id = insert_export_test_frame(&conn, &session.id, &image_path.to_string_lossy());
        conn.execute(
            "INSERT INTO sensitive_regions (
                id, frame_id, region_type, source, confidence, action_taken
             ) VALUES ('region-never', ?1, 'api_key', 'test', 1.0, 'never_send_to_ai')",
            params![frame_id.to_string()],
        )
        .unwrap();

        let bundle = build_safe_ai_export_from_conn(
            &conn,
            &output_root,
            SafeAiExportInput {
                lookback_minutes: None,
                range_ms: Some(1_000),
                current_frame_id: Some(frame_id),
                include_images: Some(true),
                max_frames: Some(10),
                export_type: Some("test".to_string()),
            },
        )
        .unwrap();

        assert_eq!(bundle.input_frame_count, 1);
        assert_eq!(bundle.exported_frame_count, 0);
        assert_eq!(bundle.excluded_frame_count, 1);
        assert!(bundle
            .warnings
            .iter()
            .any(|warning| warning.contains("never_send_to_ai")));
        let audits: i64 = conn
            .query_row("SELECT COUNT(*) FROM ai_export_audit", [], |row| row.get(0))
            .unwrap();
        assert_eq!(audits, 1);
        fs::remove_dir_all(output_root).unwrap();
    }

    #[test]
    fn safe_ai_export_derives_safe_image_for_redacted_frame() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let output_root =
            std::env::temp_dir().join(format!("smalltalk-safe-export-mask-{}", now_millis()));
        let source_root = output_root.join("source");
        fs::create_dir_all(&source_root).unwrap();
        let image_path = source_root.join("frame.png");
        fs::write(&image_path, tiny_png()).unwrap();

        let session = insert_numbered_test_session(&conn);
        let frame_id = insert_export_test_frame(&conn, &session.id, &image_path.to_string_lossy());
        conn.execute(
            "UPDATE frames SET privacy_status = 'redacted' WHERE id = ?1",
            params![frame_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sensitive_regions (
                id, frame_id, region_type, bounds_x, bounds_y, bounds_w, bounds_h,
                source, confidence, action_taken
             ) VALUES ('region-mask', ?1, 'password_field', 0.0, 0.0, 1.0, 1.0,
                       'test', 1.0, 'store_redacted')",
            params![frame_id.to_string()],
        )
        .unwrap();

        let bundle = build_safe_ai_export_from_conn(
            &conn,
            &output_root,
            SafeAiExportInput {
                lookback_minutes: None,
                range_ms: Some(1_000),
                current_frame_id: Some(frame_id),
                include_images: Some(true),
                max_frames: Some(10),
                export_type: Some("test".to_string()),
            },
        )
        .unwrap();

        assert_eq!(bundle.exported_frame_count, 1);
        assert_eq!(bundle.masked_image_count, 1);
        let safe_path = bundle.frames[0].image_path_safe.as_ref().unwrap();
        assert_ne!(safe_path, &image_path.to_string_lossy().to_string());
        assert!(Path::new(safe_path).exists());
        assert_eq!(bundle.frames[0].privacy_status, "redacted");
        fs::remove_dir_all(output_root).unwrap();
    }

    #[test]
    fn safe_ai_export_uses_derived_image_for_normal_frame() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let output_root =
            std::env::temp_dir().join(format!("smalltalk-safe-export-normal-{}", now_millis()));
        let source_root = output_root.join("source");
        fs::create_dir_all(&source_root).unwrap();
        let image_path = source_root.join("frame.png");
        fs::write(&image_path, tiny_png()).unwrap();

        let session = insert_numbered_test_session(&conn);
        let frame_id = insert_export_test_frame(&conn, &session.id, &image_path.to_string_lossy());
        let bundle = build_safe_ai_export_from_conn(
            &conn,
            &output_root,
            SafeAiExportInput {
                lookback_minutes: None,
                range_ms: Some(1_000),
                current_frame_id: Some(frame_id),
                include_images: Some(true),
                max_frames: Some(10),
                export_type: Some("test".to_string()),
            },
        )
        .unwrap();

        let safe_path = bundle.frames[0].image_path_safe.as_ref().unwrap();
        assert_ne!(safe_path, &image_path.to_string_lossy().to_string());
        assert!(safe_path.contains("ai-export"));
        assert!(safe_path.ends_with("safe_full_screenshot.png"));
        assert!(Path::new(safe_path).exists());
        fs::remove_dir_all(output_root).unwrap();
    }

    #[test]
    fn synthetic_twenty_minute_loop_produces_resume_card() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let output_root =
            std::env::temp_dir().join(format!("smalltalk-resume-card-e2e-{}", now_millis()));
        let source_root = output_root.join("source");
        fs::create_dir_all(&source_root).unwrap();
        let image_path = source_root.join("frame.png");
        fs::write(&image_path, tiny_png()).unwrap();

        let session = insert_numbered_test_session(&conn);
        let origin_text = "Smalltalk native resume engine return task detection transition graph. The user is refining the trustworthy resume cue and needs a branch-aware answer about where to continue.";
        let branch_text = "Return task detection transition graph paper verification branch. This article supports using previous surface similarity and keyframes to identify resume targets.";
        let returned_text = "Smalltalk native resume engine return task detection transition graph. Continue implementing the trustworthy resume cue from the native return classifier.";

        let frame1 = insert_resume_test_frame(
            &conn,
            &session.id,
            &image_path.to_string_lossy(),
            1_000,
            "ChatGPT",
            "com.openai.chat",
            "Smalltalk native resume engine",
            "https://chatgpt.com/c/native-resume",
            "chat_conversation",
            origin_text,
            "aaaabbbbcccc1111",
            None,
        );
        let frame2 = insert_resume_test_frame(
            &conn,
            &session.id,
            &image_path.to_string_lossy(),
            6 * 60_000,
            "Chrome",
            "com.google.Chrome",
            "Return task detection paper",
            "https://example.com/return-task-detection",
            "browser_tab",
            branch_text,
            "ffffeeee11112222",
            Some(frame1),
        );
        let frame3 = insert_resume_test_frame(
            &conn,
            &session.id,
            &image_path.to_string_lossy(),
            12 * 60_000,
            "ChatGPT",
            "com.openai.chat",
            "Smalltalk native resume engine",
            "https://chatgpt.com/c/native-resume",
            "chat_conversation",
            returned_text,
            "aaaabbbbcccc1111",
            Some(frame2),
        );
        insert_resume_test_transition(
            &conn,
            &session.id,
            "branch",
            frame1,
            frame2,
            6 * 60_000,
            "navigation",
        );
        insert_resume_test_transition(
            &conn,
            &session.id,
            "return",
            frame2,
            frame3,
            12 * 60_000,
            "navigation",
        );

        let dossier = get_native_storyboard_dossier_from_conn(
            &conn,
            &output_root,
            Some(NativeStoryboardInput {
                lookback_minutes: Some(20),
                max_keyframes: Some(10),
                current_frame_id: Some(frame3),
                include_images: Some(true),
            }),
        )
        .unwrap();
        let card = get_native_resume_card_from_conn(
            &conn,
            &output_root,
            Some(NativeResumeInput {
                lookback_minutes: Some(20),
                max_keyframes: Some(10),
                current_frame_id: Some(frame3),
            }),
        )
        .unwrap();

        assert!(dossier.keyframes.len() <= 12);
        assert!(dossier
            .transitions
            .iter()
            .any(|transition| transition.transition_type == "verification_branch"));
        assert!(dossier
            .transitions
            .iter()
            .any(|transition| transition.transition_type == "returning_to_previous_task"));
        assert_eq!(card.continue_from.frame_id, Some(frame3.to_string()));
        assert!(card.focus_now.contains("ChatGPT"));
        assert!(card.what_was_i_doing.contains("ChatGPT"));
        assert!(card
            .what_was_i_reading
            .as_deref()
            .unwrap_or("")
            .contains("resume"));
        assert!(card.next_action.contains("Resume"));
        assert!(card.evidence_frame_ids.contains(&frame1.to_string()));
        assert!(card.evidence_frame_ids.contains(&frame3.to_string()));
        assert!(card.evidence_transition_ids.contains(&"return".to_string()));
        for keyframe in &dossier.keyframes {
            if let Some(path) = keyframe.image_path_safe.as_deref() {
                assert_ne!(path, image_path.to_string_lossy());
                assert!(Path::new(path).exists());
            }
        }
        fs::remove_dir_all(output_root).unwrap();
    }

    #[test]
    fn live_capture_store_native_resume_card_smoke_when_env_set() {
        let Ok(live_db_path) = std::env::var("SMALLTALK_LIVE_DB_PATH") else {
            return;
        };
        let source = PathBuf::from(live_db_path);
        if !source.exists() {
            return;
        }
        let root = std::env::temp_dir().join(format!("smalltalk-live-card-{}", now_millis()));
        fs::create_dir_all(&root).unwrap();
        let db_copy = root.join("smalltalk-capture.sqlite");
        fs::copy(&source, &db_copy).unwrap();
        let wal = PathBuf::from(format!("{}-wal", source.to_string_lossy()));
        if wal.exists() {
            let _ = fs::copy(&wal, root.join("smalltalk-capture.sqlite-wal"));
        }
        let shm = PathBuf::from(format!("{}-shm", source.to_string_lossy()));
        if shm.exists() {
            let _ = fs::copy(&shm, root.join("smalltalk-capture.sqlite-shm"));
        }

        let conn = Connection::open(&db_copy).unwrap();
        init_db(&conn).unwrap();
        let latest_frame_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM frames ORDER BY captured_at DESC, id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        let Some(latest_frame_id) = latest_frame_id else {
            fs::remove_dir_all(root).unwrap();
            return;
        };

        let card = get_native_resume_card_from_conn(
            &conn,
            &root.join("safe-ai-exports"),
            Some(NativeResumeInput {
                lookback_minutes: Some(20),
                max_keyframes: Some(10),
                current_frame_id: Some(latest_frame_id),
            }),
        )
        .unwrap();

        assert!(!card.what_was_i_doing.trim().is_empty());
        assert!(!card.focus_now.trim().is_empty());
        assert!(!card.next_action.trim().is_empty());
        assert!(!card.evidence_frame_ids.is_empty());
        assert!(card.confidence >= 0.0 && card.confidence <= 1.0);
        fs::remove_dir_all(root).unwrap();
    }

    fn safe_frame_for_transition(
        id: &str,
        ts: i64,
        app: &str,
        url: &str,
        text: &str,
    ) -> SafeAiFrame {
        SafeAiFrame {
            frame_id: id.to_string(),
            captured_at_ms: ts,
            app_name: Some(app.to_string()),
            app_bundle_id: Some(format!("com.example.{}", sanitize_id(app))),
            window_name: Some(app.to_string()),
            window_id: Some(42),
            browser_url: Some(url.to_string()),
            document_path: None,
            phash: Some(format!("phash-{}", sanitize_id(url))),
            app_context_id: Some(format!("ctx-{}", id)),
            app_context_object_type: Some("chat_conversation".to_string()),
            image_path_safe: None,
            active_window_crop_path_safe: None,
            top_content_units: vec![CompactContentUnit {
                id: format!("unit-{}", id),
                source: "test".to_string(),
                unit_type: "text".to_string(),
                semantic_role: Some("main_content".to_string()),
                text: text.to_string(),
                confidence: Some(0.9),
            }],
            text_source: Some("hybrid".to_string()),
            text: Some(text.to_string()),
            evidence_strength: 0.8,
            privacy_status: "normal".to_string(),
            warnings: Vec::new(),
        }
    }

    fn transition_for_test(
        id: &str,
        pre: &str,
        post: &str,
        ts: i64,
        base_type: &str,
    ) -> TransitionSummary {
        TransitionSummary {
            id: id.to_string(),
            trigger_id: format!("trigger-{}", id),
            primary_event_id: Some(format!("event-{}", id)),
            pre_frame_id: Some(pre.to_string()),
            post_frame_id: Some(post.to_string()),
            ts_start_ms: ts,
            ts_end_ms: ts + 1,
            transition_type: Some(base_type.to_string()),
            confidence: Some(0.5),
            summary: None,
        }
    }

    #[test]
    fn classifier_marks_research_branch_then_return_to_previous_task() {
        let frames = vec![
            safe_frame_for_transition(
                "1",
                100,
                "ChatGPT",
                "https://chatgpt.com/c/native-resume",
                "native resume engine return task detection transition graph",
            ),
            safe_frame_for_transition(
                "2",
                200,
                "Chrome",
                "https://example.com/paper",
                "return task detection transition graph paper verification",
            ),
            safe_frame_for_transition(
                "3",
                300,
                "ChatGPT",
                "https://chatgpt.com/c/native-resume",
                "native resume engine return task detection transition graph",
            ),
        ];
        let transitions = vec![
            transition_for_test("t2", "2", "3", 300, "navigation"),
            transition_for_test("t1", "1", "2", 200, "navigation"),
        ];

        let classified = classify_safe_episode_transitions(&frames, &transitions);

        assert_eq!(classified[0].transition_type, "returning_to_previous_task");
        assert_eq!(classified[1].transition_type, "verification_branch");
        assert!(classified[0].return_score.unwrap() >= 0.72);
        assert!(classified[0].evidence_frame_ids.contains(&"1".to_string()));
        assert!(classified[0]
            .evidence_event_ids
            .contains(&"event-t2".to_string()));
    }

    #[test]
    fn classifier_marks_unrelated_media_as_background_media() {
        let frames = vec![
            safe_frame_for_transition(
                "1",
                100,
                "ChatGPT",
                "https://chatgpt.com/c/native-resume",
                "native resume engine transition graph",
            ),
            safe_frame_for_transition(
                "2",
                200,
                "Chrome",
                "https://youtube.com/watch?v=test",
                "playlist music video",
            ),
        ];
        let transitions = vec![transition_for_test("t1", "1", "2", 200, "navigation")];

        let classified = classify_safe_episode_transitions(&frames, &transitions);

        assert_eq!(classified[0].transition_type, "background_media");
    }

    #[test]
    fn phash_similarity_contributes_to_return_score() {
        let mut first = safe_frame_for_transition(
            "1",
            100,
            "Chrome",
            "https://example.com/a",
            "shared visible task anchors",
        );
        let mut second = safe_frame_for_transition(
            "2",
            200,
            "Chrome",
            "https://example.com/b",
            "shared visible task anchors",
        );
        first.phash = Some("abcdef123456".to_string());
        second.phash = Some("abcdef123456".to_string());

        assert_eq!(phash_similarity(&first, &second), 1.0);
        assert!(return_score(&first, &second) > 0.1);
    }

    #[test]
    fn native_inputs_accept_string_or_numeric_current_frame_id() {
        let from_string: NativeResumeInput =
            serde_json::from_value(serde_json::json!({ "current_frame_id": "42" })).unwrap();
        let from_number: NativeResumeInput =
            serde_json::from_value(serde_json::json!({ "current_frame_id": 43 })).unwrap();

        assert_eq!(from_string.current_frame_id, Some(42));
        assert_eq!(from_number.current_frame_id, Some(43));
    }

    #[test]
    fn app_context_adapters_classify_common_resume_surfaces() {
        assert_eq!(
            classify_app_context("Google Chrome", Some("https://chatgpt.com/c/abc")).1,
            "chat_conversation"
        );
        assert_eq!(
            classify_app_context("Arc", Some("https://www.notion.so/page")).1,
            "notes_doc"
        );
        assert_eq!(
            classify_app_context("Safari", Some("https://youtube.com/watch?v=1")).1,
            "media"
        );
        assert_eq!(classify_app_context("Cursor", None).1, "code_editor");
        assert_eq!(classify_app_context("Warp", None).1, "terminal");
    }

    #[test]
    fn frame_consistency_warning_persists_for_browser_without_url() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn.execute(
            INSERT_FRAME_SQL,
            params![
                1_i64,
                "/tmp/smalltalk-frame.jpg",
                Some("Google Chrome"),
                Some("Untitled"),
                Option::<String>::None,
                Option::<String>::None,
                true,
                "manual",
                "accessibility",
                Some("visible text"),
                Option::<String>::None,
                Some("visible text searchable"),
                Some("content-hash"),
                Some("image-hash"),
                1_i64,
                "screencapture_cli",
                "active_display",
                Some("main"),
                Option::<i64>::None,
                Some(123_i64),
                Some("com.google.Chrome"),
                1.0_f64,
                Some(1440_i64),
                Some(900_i64),
                "/tmp/smalltalk-frame.jpg",
                Option::<String>::None,
                Option::<String>::None,
                Some("phash"),
                "normal",
                Some("trigger-1"),
                Option::<String>::None,
                Some("session-1"),
            ],
        )
        .unwrap();
        let frame_id = conn.last_insert_rowid();
        let frame = conn
            .query_row(
                &format!("SELECT {} FROM frames WHERE id = ?1", FRAME_COLUMNS),
                params![frame_id],
                frame_from_row,
            )
            .unwrap();
        let report = validate_frame_consistency_inner(&conn, &frame).unwrap();
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.warning_type == "browser_url_missing"));
        let persisted = query_frame_quality_warnings(&conn, &frame_id.to_string()).unwrap();
        assert_eq!(persisted.len(), report.warnings.len());
    }

    #[test]
    fn clear_db_removes_frames_ocr_fts_and_resets_ids() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let frame_id = insert_test_frame(&conn, "/tmp/a.jpg");
        conn.execute(
            "INSERT INTO ocr_text (frame_id, text, text_json, ocr_engine)
             VALUES (?1, 'hello', '[]', 'test')",
            params![frame_id],
        )
        .unwrap();

        clear_capture_db(&conn).unwrap();

        let frames: i64 = conn
            .query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0))
            .unwrap();
        let ocr: i64 = conn
            .query_row("SELECT COUNT(*) FROM ocr_text", [], |row| row.get(0))
            .unwrap();
        let fts: i64 = conn
            .query_row("SELECT COUNT(*) FROM frames_fts", [], |row| row.get(0))
            .unwrap();

        assert_eq!(frames, 0);
        assert_eq!(ocr, 0);
        assert_eq!(fts, 0);

        let next_frame_id = insert_test_frame(&conn, "/tmp/b.jpg");
        conn.execute(
            "INSERT INTO ocr_text (frame_id, text, text_json, ocr_engine)
             VALUES (?1, 'fresh', '[]', 'test')",
            params![next_frame_id],
        )
        .unwrap();
        let next_ocr_id: i64 = conn
            .query_row(
                "SELECT id FROM ocr_text WHERE frame_id = ?1",
                params![next_frame_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(next_frame_id, 1);
        assert_eq!(next_ocr_id, 1);
    }

    #[test]
    fn clear_snapshot_dir_recreates_empty_directory() {
        let root = std::env::temp_dir().join(format!("smalltalk-clear-snapshots-{}", now_millis()));
        let day_dir = root.join("day-1");
        fs::create_dir_all(&day_dir).unwrap();
        fs::write(day_dir.join("frame.jpg"), b"test").unwrap();

        clear_snapshot_dir(&root).unwrap();

        assert!(root.exists());
        assert!(fs::read_dir(&root).unwrap().next().is_none());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn session_numbering_creates_ordered_folder_names() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        let first = insert_numbered_test_session(&conn);
        let second = insert_numbered_test_session(&conn);

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(session_folder_name(first.sequence), "session-001");
        assert_eq!(session_folder_name(second.sequence), "session-002");
    }

    #[test]
    fn folder_export_writes_raw_tables_images_and_sqlite_snapshot() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let source_root =
            std::env::temp_dir().join(format!("smalltalk-export-source-{}", now_millis()));
        let output_root =
            std::env::temp_dir().join(format!("smalltalk-export-output-{}", now_millis()));
        fs::create_dir_all(&source_root).unwrap();
        let image_path = source_root.join("frame.png");
        fs::write(&image_path, tiny_png()).unwrap();

        let mut session = insert_numbered_test_session(&conn);
        insert_export_test_frame(&conn, &session.id, &image_path.to_string_lossy());
        refresh_session_counts(&conn, &session.id).unwrap();
        session = load_capture_session(&conn, &session.id).unwrap();

        let summary = write_session_export_to_root(&conn, &session, &output_root).unwrap();
        let session_dir = PathBuf::from(&summary.path);
        assert_eq!(summary.kind, "folder");
        assert_eq!(summary.folder_name, "session-001");
        assert_eq!(summary.warning_count, 0);
        assert!(session_dir.join("session.json").exists());
        assert!(session_dir.join("export_warnings.json").exists());
        assert!(session_dir.join("raw/smalltalk-capture.sqlite").exists());
        assert!(session_dir.join("raw/schema.sql").exists());
        assert!(session_dir.join("timeline/frames.json").exists());
        assert!(session_dir
            .join("frames/frame-000001/images/frame-000001-snapshot.png")
            .exists());
        assert!(session_dir
            .join("frames/frame-000001/ocr/ocr_text_rows.json")
            .exists());
        assert!(session_dir
            .join("frames/frame-000001/accessibility/ax_nodes.json")
            .exists());
        assert!(session_dir
            .join("frames/frame-000001/events/ui_events.json")
            .exists());

        for table in table_names_for_export(&conn).unwrap() {
            let stem = safe_file_stem(&table);
            assert!(session_dir
                .join(format!("raw/tables/{}.json", stem))
                .exists());
            assert!(session_dir
                .join(format!("raw/tables/{}.ndjson", stem))
                .exists());
        }

        let snapshot = Connection::open(session_dir.join("raw/smalltalk-capture.sqlite")).unwrap();
        let frames: i64 = snapshot
            .query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0))
            .unwrap();
        assert_eq!(frames, 1);

        fs::remove_dir_all(source_root).unwrap();
        fs::remove_dir_all(output_root).unwrap();
    }

    #[test]
    fn folder_export_records_missing_image_warnings() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let output_root =
            std::env::temp_dir().join(format!("smalltalk-export-missing-output-{}", now_millis()));
        let mut session = insert_numbered_test_session(&conn);
        insert_export_test_frame(&conn, &session.id, "/tmp/smalltalk-missing-frame.png");
        refresh_session_counts(&conn, &session.id).unwrap();
        session = load_capture_session(&conn, &session.id).unwrap();

        let summary = write_session_export_to_root(&conn, &session, &output_root).unwrap();
        assert!(summary.warning_count > 0);
        let warnings =
            fs::read_to_string(PathBuf::from(&summary.path).join("export_warnings.json")).unwrap();
        assert!(warnings.contains("image_source_missing"));

        fs::remove_dir_all(output_root).unwrap();
    }
}
