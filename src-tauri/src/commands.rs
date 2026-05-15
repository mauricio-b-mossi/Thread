#![allow(non_snake_case)]

use crate::persistence::{self, Session as DbSession, Task as DbTask, TaskEvent};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Position, State, WebviewUrl,
    WebviewWindowBuilder,
};

type CommandResult<T> = Result<T, CommandError>;

const TASK_KIND_PICKUP: &str = "pickup";
const TASK_KIND_LONG_TERM: &str = "long_term";

const TASK_STATUS_PICKUP: &str = "pickup";
const TASK_STATUS_BACKLOG: &str = "backlog";
const TASK_STATUS_ACTIVE: &str = "active";
const TASK_STATUS_COMPLETED: &str = "completed";
const TASK_STATUS_ARCHIVED: &str = "archived";

const END_REASON_COMPLETED: &str = "completed";
const END_REASON_STOPPED: &str = "stopped";
const END_REASON_SWITCHED: &str = "switched";
const END_REASON_APP_CLOSED: &str = "app_closed";
const END_REASON_DISCARDED: &str = "discarded";

const RECOVERY_ACTION_RESUME: &str = "resume";
const RECOVERY_ACTION_STOP: &str = "stop";
const RECOVERY_ACTION_DISCARD: &str = "discard";

const SETTING_STARTUP_LAUNCH: &str = "startup.launch";
const SETTING_STARTUP_TODAY_ON_STARTUP: &str = "startup.today_on_startup";
const SETTING_DONUT_LAP_DURATION_SECONDS: &str = "donut.lap_duration_seconds";
const SETTING_THEME: &str = "theme";
const SETTING_FLOATING_WINDOW_ALWAYS_ON_TOP: &str = "floating_window.always_on_top";
const SETTING_FLOATING_WINDOW_POSITION: &str = "floating_window.position";
const SETTING_LONG_TERM_STOP_REQUIREMENTS: &str = "long_term.stop_requirements";
const SETTING_TODAY_RETURN_BEHAVIOR: &str = "today.return_behavior";
const EXPORT_DIR_NAME: &str = "exports";
const EXPORT_FILE_EXTENSION: &str = ".sqlite3";
const FLOATING_WINDOW_WIDTH: f64 = 280.0;
const FLOATING_WINDOW_HEIGHT: f64 = 140.0;
const FLOATING_WINDOW_MARGIN: i32 = 24;
const TODAY_ROUTE: &str = "today";
const SETTINGS_ROUTE: &str = "settings";
const STOP_CURRENT_TASK_ROUTE: &str = "stop-current-task";
const DEFAULT_FLOATING_WINDOW_POSITION: FloatingWindowPosition =
    FloatingWindowPosition { x: 24, y: 24 };

#[derive(Debug, Default)]
pub struct PendingSessionRecovery(pub Mutex<Option<String>>);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    fn validation(message: impl Into<String>) -> Self {
        Self {
            code: "validation".to_string(),
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found".to_string(),
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "conflict".to_string(),
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal".to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub status: String,
    pub priority: i64,
    pub pickup_date: Option<String>,
    pub next_action: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub task_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<i64>,
    pub end_reason: Option<String>,
    pub progress_note: Option<String>,
    pub next_action: Option<String>,
    pub lap_duration_seconds: i64,
    pub recovered_from_crash: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSession {
    pub session: Session,
    pub task: Task,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TodayPayload {
    pub active_session: Option<ActiveSession>,
    pub pickup: Vec<Task>,
    pub backlog: Vec<Task>,
    pub recent_threads: Vec<RecentThread>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentThread {
    pub task: Task,
    pub session: Session,
    pub last_worked_at: String,
    pub progress_note: Option<String>,
    pub next_action: Option<String>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub task: Task,
    pub sessions: Vec<Session>,
    pub total_duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FloatingWindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub launch_on_startup: bool,
    pub today_on_startup: bool,
    pub donut_lap_duration_seconds: i64,
    pub theme: String,
    pub floating_window_always_on_top: bool,
    pub floating_window_position: FloatingWindowPosition,
    pub long_term_stop_requirements: String,
    pub today_return_behavior: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub title: String,
    pub description: Option<String>,
    pub kind: String,
    pub status: Option<String>,
    pub priority: Option<i64>,
    pub pickup_date: Option<String>,
    pub next_action: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i64>,
    pub pickup_date: Option<String>,
    pub next_action: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveTaskInput {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListRecentThreadsInput {
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskDetailInput {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionInput {
    pub task_id: String,
    pub lap_duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSessionInput {
    pub session_id: Option<String>,
    pub progress_note: Option<String>,
    pub next_action: Option<String>,
    pub confirm_long_term_completion: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StopSessionInput {
    pub session_id: Option<String>,
    pub progress_note: Option<String>,
    pub next_action: Option<String>,
    pub destination_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SwitchTaskInput {
    pub task_id: String,
    pub progress_note: Option<String>,
    pub next_action: Option<String>,
    pub destination_status: Option<String>,
    pub lap_duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveSessionRecoveryInput {
    pub action: String,
    pub session_id: Option<String>,
    pub progress_note: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveSessionRecoveryResult {
    pub active_session: Option<ActiveSession>,
    pub session: Option<Session>,
    pub task: Option<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetPendingSessionRecoveryResult {
    pub active_session: Option<ActiveSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub launch_on_startup: Option<bool>,
    pub today_on_startup: Option<bool>,
    pub donut_lap_duration_seconds: Option<i64>,
    pub theme: Option<String>,
    pub floating_window_always_on_top: Option<bool>,
    pub floating_window_position: Option<FloatingWindowPosition>,
    pub long_term_stop_requirements: Option<String>,
    pub today_return_behavior: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveFloatingWindowPositionInput {
    pub position: FloatingWindowPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportDatabaseInput {
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportDatabaseResult {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenDataFolderResult {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResetFloatingWindowPositionResult {
    pub position: FloatingWindowPosition,
}

#[tauri::command]
pub fn createTask(app: AppHandle, input: CreateTaskInput) -> CommandResult<Task> {
    let conn = open_connection(&app)?;
    create_task(&conn, input)
}

#[tauri::command]
pub fn updateTask(app: AppHandle, input: UpdateTaskInput) -> CommandResult<Task> {
    let conn = open_connection(&app)?;
    update_task(&conn, input)
}

#[tauri::command]
pub fn archiveTask(app: AppHandle, input: ArchiveTaskInput) -> CommandResult<Task> {
    let conn = open_connection(&app)?;
    archive_task(&conn, input)
}

#[tauri::command]
pub fn listToday(app: AppHandle) -> CommandResult<TodayPayload> {
    let conn = open_connection(&app)?;
    list_today(&conn)
}

#[tauri::command]
pub fn listBacklog(app: AppHandle) -> CommandResult<Vec<Task>> {
    let conn = open_connection(&app)?;
    list_backlog(&conn)
}

#[tauri::command]
pub fn listRecentThreads(
    app: AppHandle,
    input: Option<ListRecentThreadsInput>,
) -> CommandResult<Vec<RecentThread>> {
    let conn = open_connection(&app)?;
    let limit = input.and_then(|value| value.limit).unwrap_or(10);
    list_recent_threads(&conn, limit)
}

#[tauri::command]
pub fn getTaskDetail(app: AppHandle, input: GetTaskDetailInput) -> CommandResult<TaskDetail> {
    let conn = open_connection(&app)?;
    get_task_detail(&conn, input)
}

#[tauri::command]
pub async fn startSession(
    app: AppHandle,
    input: StartSessionInput,
) -> CommandResult<ActiveSession> {
    let conn = open_connection(&app)?;
    let active_session = start_session(&conn, input)?;
    open_floating_session_window(&app)?;
    Ok(active_session)
}

#[tauri::command]
pub fn getActiveSession(app: AppHandle) -> CommandResult<Option<ActiveSession>> {
    let conn = open_connection(&app)?;
    get_active_session(&conn)
}

#[tauri::command]
pub fn getPendingSessionRecovery(
    app: AppHandle,
    pending_recovery: State<PendingSessionRecovery>,
) -> CommandResult<GetPendingSessionRecoveryResult> {
    let conn = open_connection(&app)?;
    get_pending_session_recovery(&conn, &pending_recovery)
}

#[tauri::command]
pub fn completeSession(
    app: AppHandle,
    input: CompleteSessionInput,
) -> CommandResult<ActiveSession> {
    let conn = open_connection(&app)?;
    complete_session(&conn, input)
}

#[tauri::command]
pub fn stopSession(app: AppHandle, input: StopSessionInput) -> CommandResult<ActiveSession> {
    let conn = open_connection(&app)?;
    stop_session(&conn, input, END_REASON_STOPPED)
}

#[tauri::command]
pub async fn switchTask(app: AppHandle, input: SwitchTaskInput) -> CommandResult<ActiveSession> {
    let conn = open_connection(&app)?;
    let active_session = switch_task(&conn, input)?;
    open_floating_session_window(&app)?;
    Ok(active_session)
}

#[tauri::command]
pub fn resolveSessionRecovery(
    app: AppHandle,
    input: ResolveSessionRecoveryInput,
    pending_recovery: State<PendingSessionRecovery>,
) -> CommandResult<ResolveSessionRecoveryResult> {
    let conn = open_connection(&app)?;
    let result = resolve_session_recovery(&conn, input)?;
    clear_pending_session_recovery(&pending_recovery)?;
    if result.active_session.is_some() {
        open_floating_session_window(&app)?;
    }
    Ok(result)
}

#[tauri::command]
pub fn getSettings(app: AppHandle) -> CommandResult<Settings> {
    let conn = open_connection(&app)?;
    get_settings(&conn)
}

#[tauri::command]
pub fn updateSettings(app: AppHandle, input: UpdateSettingsInput) -> CommandResult<Settings> {
    let conn = open_connection(&app)?;
    update_settings_for_app(&app, &conn, input)
}

#[tauri::command]
pub fn saveFloatingWindowPosition(
    app: AppHandle,
    input: SaveFloatingWindowPositionInput,
) -> CommandResult<FloatingWindowPosition> {
    let conn = open_connection(&app)?;
    save_floating_window_position(&conn, input.position)
}

#[tauri::command]
pub fn exportDatabase(
    app: AppHandle,
    input: Option<ExportDatabaseInput>,
) -> CommandResult<ExportDatabaseResult> {
    let source_path = persistence::app_database_path(&app).map_err(to_internal_error)?;
    let export_dir = app
        .path()
        .app_data_dir()
        .map_err(to_internal_error)?
        .join(EXPORT_DIR_NAME);
    export_database_to_app_dir(
        &source_path,
        &export_dir,
        input.and_then(|value| value.file_name),
    )
}

#[tauri::command]
pub fn openDataFolder(app: AppHandle) -> CommandResult<OpenDataFolderResult> {
    let data_dir = app.path().app_data_dir().map_err(to_internal_error)?;
    fs::create_dir_all(&data_dir).map_err(to_internal_error)?;
    open_folder(&data_dir)?;

    Ok(OpenDataFolderResult {
        path: data_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn openTodayWindow(app: AppHandle) -> CommandResult<()> {
    open_today_window(&app)
}

#[tauri::command]
pub fn openSettingsWindow(app: AppHandle) -> CommandResult<()> {
    open_settings_window(&app)
}

#[tauri::command]
pub fn showFloatingTask(app: AppHandle) -> CommandResult<()> {
    show_floating_task(&app)
}

#[tauri::command]
pub fn stopCurrentTask(app: AppHandle) -> CommandResult<Option<ActiveSession>> {
    stop_current_task(&app)
}

#[tauri::command]
pub fn resetFloatingWindowPosition(
    app: AppHandle,
) -> CommandResult<ResetFloatingWindowPositionResult> {
    let conn = open_connection(&app)?;
    let position = save_floating_window_position(&conn, DEFAULT_FLOATING_WINDOW_POSITION)?;
    if let Some(window) = app.get_webview_window("floating") {
        window
            .set_position(Position::Physical(PhysicalPosition::new(
                position.x, position.y,
            )))
            .map_err(to_internal_error)?;
    }
    Ok(ResetFloatingWindowPositionResult { position })
}

#[tauri::command]
pub fn quitApp(app: AppHandle) -> CommandResult<()> {
    quit_app(&app)
}

fn open_connection(app: &AppHandle) -> CommandResult<Connection> {
    persistence::open_app_database(app).map_err(to_internal_error)
}

fn open_floating_session_window(app: &AppHandle) -> CommandResult<()> {
    let settings = get_settings(&open_connection(app)?)?;
    let position = floating_window_start_position(app, &settings)?;

    if let Some(window) = app.get_webview_window("floating") {
        window.set_always_on_top(true).map_err(to_internal_error)?;
        window
            .set_position(Position::Physical(PhysicalPosition::new(
                position.x, position.y,
            )))
            .map_err(to_internal_error)?;
        window.show().map_err(to_internal_error)?;
        window.set_focus().map_err(to_internal_error)?;
        window
            .emit("session-changed", ())
            .map_err(to_internal_error)?;
    } else {
        let logical_x = f64::from(position.x) / primary_monitor_scale_factor(app)?;
        let logical_y = f64::from(position.y) / primary_monitor_scale_factor(app)?;
        let window = WebviewWindowBuilder::new(
            app,
            "floating",
            WebviewUrl::App("index.html#/floating".into()),
        )
        .title("Thread")
        .inner_size(FLOATING_WINDOW_WIDTH, FLOATING_WINDOW_HEIGHT)
        .min_inner_size(FLOATING_WINDOW_WIDTH, 120.0)
        .max_inner_size(FLOATING_WINDOW_WIDTH, 160.0)
        .position(logical_x, logical_y)
        .resizable(false)
        .decorations(false)
        .shadow(true)
        .always_on_top(true)
        .build()
        .map_err(to_internal_error)?;
        window
            .set_position(Position::Physical(PhysicalPosition::new(
                position.x, position.y,
            )))
            .map_err(to_internal_error)?;
    }

    if let Some(today) = app.get_webview_window("today") {
        today.hide().map_err(to_internal_error)?;
    }

    Ok(())
}

fn floating_window_start_position(
    app: &AppHandle,
    settings: &Settings,
) -> CommandResult<FloatingWindowPosition> {
    let default_position = FloatingWindowPosition {
        x: FLOATING_WINDOW_MARGIN,
        y: FLOATING_WINDOW_MARGIN,
    };

    if settings.floating_window_position != default_position {
        return Ok(settings.floating_window_position.clone());
    }

    let Some(monitor) = app.primary_monitor().map_err(to_internal_error)? else {
        return Ok(default_position);
    };

    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor();
    let width = (FLOATING_WINDOW_WIDTH * scale_factor).round() as i32;
    let x = work_area.position.x + work_area.size.width as i32 - width - FLOATING_WINDOW_MARGIN;
    let y = work_area.position.y + FLOATING_WINDOW_MARGIN;

    Ok(FloatingWindowPosition {
        x: x.max(work_area.position.x),
        y: y.max(work_area.position.y),
    })
}

fn primary_monitor_scale_factor(app: &AppHandle) -> CommandResult<f64> {
    Ok(app
        .primary_monitor()
        .map_err(to_internal_error)?
        .map(|monitor| monitor.scale_factor())
        .unwrap_or(1.0))
}

pub fn open_today_window(app: &AppHandle) -> CommandResult<()> {
    open_today_window_at(app, TODAY_ROUTE).map(|_| ())
}

fn open_today_window_at(app: &AppHandle, route: &str) -> CommandResult<TodayWindowState> {
    if let Some(today) = app.get_webview_window("today") {
        today.show().map_err(to_internal_error)?;
        today.set_focus().map_err(to_internal_error)?;
        if route != TODAY_ROUTE {
            let _ = today.eval(format!("window.location.hash = '#/{route}';"));
        }
        today
            .emit("session-changed", ())
            .map_err(to_internal_error)?;
        hide_floating_window(app)?;
        return Ok(TodayWindowState::Existing);
    } else {
        WebviewWindowBuilder::new(
            app,
            "today",
            WebviewUrl::App(format!("index.html#/{route}").into()),
        )
        .title("Thread")
        .inner_size(720.0, 560.0)
        .min_inner_size(360.0, 420.0)
        .resizable(true)
        .build()
        .map_err(to_internal_error)?;
    }

    hide_floating_window(app)?;

    Ok(TodayWindowState::Created)
}

fn hide_floating_window(app: &AppHandle) -> CommandResult<()> {
    if let Some(floating) = app.get_webview_window("floating") {
        floating.hide().map_err(to_internal_error)?;
    }

    Ok(())
}

pub fn open_settings_window(app: &AppHandle) -> CommandResult<()> {
    if matches!(
        open_today_window_at(app, SETTINGS_ROUTE)?,
        TodayWindowState::Existing
    ) {
        let Some(today) = app.get_webview_window("today") else {
            return Ok(());
        };
        today.emit("open-settings", ()).map_err(to_internal_error)?;
    }
    Ok(())
}

pub fn show_floating_task(app: &AppHandle) -> CommandResult<()> {
    if open_connection(app)?
        .query_row(
            "SELECT 1 FROM sessions WHERE ended_at IS NULL LIMIT 1",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(to_internal_error)?
        .is_some()
    {
        open_floating_session_window(app)
    } else {
        open_today_window(app)
    }
}

pub fn stop_current_task(app: &AppHandle) -> CommandResult<Option<ActiveSession>> {
    let conn = open_connection(app)?;
    match stop_current_task_for_tray(&conn)? {
        CurrentTaskStop::NoActiveSession => Ok(None),
        CurrentTaskStop::RequiresUi => {
            if matches!(
                open_today_window_at(app, STOP_CURRENT_TASK_ROUTE)?,
                TodayWindowState::Existing
            ) {
                app.emit("stop-current-task-requested", ())
                    .map_err(to_internal_error)?;
            }
            Ok(None)
        }
        CurrentTaskStop::Stopped(stopped) => {
            app.emit("session-changed", ()).map_err(to_internal_error)?;
            Ok(Some(stopped))
        }
    }
}

pub fn quit_app(app: &AppHandle) -> CommandResult<()> {
    let conn = open_connection(app)?;
    preserve_unfinished_session_for_recovery(&conn)?;
    app.exit(0);
    Ok(())
}

enum CurrentTaskStop {
    NoActiveSession,
    RequiresUi,
    Stopped(ActiveSession),
}

enum TodayWindowState {
    Existing,
    Created,
}

fn stop_current_task_for_tray(conn: &Connection) -> CommandResult<CurrentTaskStop> {
    let Some(active_session) = get_active_session(conn)? else {
        return Ok(CurrentTaskStop::NoActiveSession);
    };

    if active_session.task.kind == TASK_KIND_LONG_TERM {
        return Ok(CurrentTaskStop::RequiresUi);
    }

    let stopped = stop_session(
        conn,
        StopSessionInput {
            session_id: Some(active_session.session.id),
            progress_note: None,
            next_action: None,
            destination_status: None,
        },
        END_REASON_STOPPED,
    )?;

    Ok(CurrentTaskStop::Stopped(stopped))
}

fn preserve_unfinished_session_for_recovery(conn: &Connection) -> CommandResult<()> {
    let _ = get_active_session(conn)?;
    Ok(())
}

fn create_task(conn: &Connection, input: CreateTaskInput) -> CommandResult<Task> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    let title = validate_required_text("title", input.title)?;
    let description = input.description.unwrap_or_default();
    let kind = validate_task_kind(&input.kind)?.to_string();
    let status = match input.status {
        Some(status) => validate_initial_status(&status)?.to_string(),
        None => default_status_for_kind(&kind).to_string(),
    };
    let next_action = normalize_optional_text(input.next_action);
    let pickup_date = normalize_optional_text(input.pickup_date);
    let priority = input.priority.unwrap_or(0);
    let sort_order = match input.sort_order {
        Some(sort_order) => sort_order,
        None => next_sort_order(&tx, &status)?,
    };
    let now = utc_now(&tx)?;

    let task = DbTask {
        id: new_id(&tx, "task")?,
        title,
        description,
        kind,
        status,
        priority,
        pickup_date,
        next_action,
        sort_order,
        created_at: now.clone(),
        updated_at: now.clone(),
        completed_at: None,
        archived_at: None,
    };

    persistence::insert_task(&tx, &task).map_err(to_internal_error)?;
    insert_task_event(
        &tx,
        &task.id,
        None,
        "task_created",
        None,
        Some(&task.status),
        None,
    )?;
    tx.commit().map_err(to_internal_error)?;

    Ok(Task::from(task))
}

fn update_task(conn: &Connection, input: UpdateTaskInput) -> CommandResult<Task> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    let mut task = require_task(&tx, &input.id)?;
    let previous_status = task.status.clone();

    if let Some(title) = input.title {
        task.title = validate_required_text("title", title)?;
    }

    if let Some(description) = input.description {
        task.description = description;
    }

    if let Some(kind) = input.kind {
        task.kind = validate_task_kind(&kind)?.to_string();
    }

    if let Some(status) = input.status {
        let next_status = validate_task_status(&status)?.to_string();
        if next_status == TASK_STATUS_ACTIVE {
            return Err(CommandError::validation(
                "Use startSession to make a task active.",
            ));
        }

        if next_status != previous_status && task_has_active_session(&tx, &task.id)? {
            return Err(CommandError::conflict(
                "Cannot change the status of a task with an active session; use completeSession, stopSession, or switchTask.",
            ));
        }

        task.status = next_status;
    }

    if let Some(priority) = input.priority {
        task.priority = priority;
    }

    if let Some(pickup_date) = input.pickup_date {
        task.pickup_date = normalize_optional_text(Some(pickup_date));
    }

    if let Some(next_action) = input.next_action {
        task.next_action = normalize_optional_text(Some(next_action));
    }

    if let Some(sort_order) = input.sort_order {
        task.sort_order = sort_order;
    }

    let now = utc_now(&tx)?;
    apply_terminal_task_timestamps(&mut task, &previous_status, &now);
    task.updated_at = now;

    persistence::update_task(&tx, &task).map_err(to_internal_error)?;

    if previous_status != task.status {
        insert_task_event(
            &tx,
            &task.id,
            None,
            "status_changed",
            Some(&previous_status),
            Some(&task.status),
            None,
        )?;
    }
    tx.commit().map_err(to_internal_error)?;

    Ok(Task::from(task))
}

fn archive_task(conn: &Connection, input: ArchiveTaskInput) -> CommandResult<Task> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    let mut task = require_task(&tx, &input.task_id)?;

    if task.status == TASK_STATUS_ACTIVE || task_has_active_session(&tx, &task.id)? {
        return Err(CommandError::conflict(
            "Cannot archive a task while it has an active session.",
        ));
    }

    let previous_status = task.status.clone();
    let now = utc_now(&tx)?;
    task.status = TASK_STATUS_ARCHIVED.to_string();
    task.archived_at = Some(now.clone());
    task.updated_at = now;

    persistence::update_task(&tx, &task).map_err(to_internal_error)?;
    insert_task_event(
        &tx,
        &task.id,
        None,
        "status_changed",
        Some(&previous_status),
        Some(TASK_STATUS_ARCHIVED),
        None,
    )?;
    tx.commit().map_err(to_internal_error)?;

    Ok(Task::from(task))
}

fn list_today(conn: &Connection) -> CommandResult<TodayPayload> {
    Ok(TodayPayload {
        active_session: get_active_session(conn)?,
        pickup: list_today_pickup_tasks(conn, 100)?,
        backlog: list_today_backlog_tasks(conn, 6)?,
        recent_threads: list_recent_threads(conn, 5)?,
    })
}

fn list_backlog(conn: &Connection) -> CommandResult<Vec<Task>> {
    list_tasks_by_status(conn, TASK_STATUS_BACKLOG, 200)
}

fn list_recent_threads(conn: &Connection, limit: i64) -> CommandResult<Vec<RecentThread>> {
    let limit = validate_limit(limit)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                s.id, s.task_id, s.started_at, s.ended_at, s.duration_seconds,
                s.end_reason, s.progress_note, s.next_action, s.lap_duration_seconds,
                s.recovered_from_crash, s.created_at, s.updated_at,
                t.id, t.title, t.description, t.kind, t.status, t.priority,
                t.pickup_date, t.next_action, t.sort_order, t.created_at,
                t.updated_at, t.completed_at, t.archived_at
            FROM sessions s
            JOIN tasks t ON t.id = s.task_id
            WHERE s.ended_at IS NOT NULL
                AND COALESCE(s.end_reason, '') != ?1
            ORDER BY s.ended_at DESC, s.started_at DESC
            LIMIT ?2
            "#,
        )
        .map_err(to_internal_error)?;

    let rows = stmt
        .query_map(params![END_REASON_DISCARDED, limit], |row| {
            let session = db_session_from_row(row, 0)?;
            let task = db_task_from_row(row, 12)?;
            let last_worked_at = session
                .ended_at
                .clone()
                .unwrap_or_else(|| session.started_at.clone());
            let progress_note = session.progress_note.clone();
            let next_action = session
                .next_action
                .clone()
                .or_else(|| task.next_action.clone());
            let duration_seconds = session.duration_seconds;

            Ok(RecentThread {
                task: Task::from(task),
                session: Session::from(session),
                last_worked_at,
                progress_note,
                next_action,
                duration_seconds,
            })
        })
        .map_err(to_internal_error)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(to_internal_error)
}

fn get_task_detail(conn: &Connection, input: GetTaskDetailInput) -> CommandResult<TaskDetail> {
    let task = require_task(conn, &input.task_id)?;
    let sessions = list_task_sessions(conn, &task.id)?;
    let total_duration_seconds = sessions
        .iter()
        .filter(|session| session.end_reason.as_deref() != Some(END_REASON_DISCARDED))
        .filter_map(|session| session.duration_seconds)
        .sum();

    Ok(TaskDetail {
        task: Task::from(task),
        sessions: sessions.into_iter().map(Session::from).collect(),
        total_duration_seconds,
    })
}

fn start_session(conn: &Connection, input: StartSessionInput) -> CommandResult<ActiveSession> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    if get_active_session(&tx)?.is_some() {
        return Err(CommandError::conflict(
            "A session is already active; use switchTask to change tasks.",
        ));
    }

    let active_session = start_session_without_active_check(&tx, input)?;
    tx.commit().map_err(to_internal_error)?;
    Ok(active_session)
}

fn start_session_without_active_check(
    conn: &Connection,
    input: StartSessionInput,
) -> CommandResult<ActiveSession> {
    let mut task = require_task(conn, &input.task_id)?;
    validate_startable_task(&task)?;
    let lap_duration_seconds = match input.lap_duration_seconds {
        Some(value) => validate_lap_duration_seconds(value)?,
        None => get_settings(conn)?.donut_lap_duration_seconds,
    };
    let now = utc_now(conn)?;
    let previous_status = task.status.clone();

    let session = DbSession {
        id: new_id(conn, "session")?,
        task_id: task.id.clone(),
        started_at: now.clone(),
        ended_at: None,
        duration_seconds: None,
        end_reason: None,
        progress_note: None,
        next_action: None,
        lap_duration_seconds,
        recovered_from_crash: false,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    insert_active_session(conn, &session)?;

    task.status = TASK_STATUS_ACTIVE.to_string();
    task.updated_at = now;
    persistence::update_task(conn, &task).map_err(to_internal_error)?;
    insert_task_event(
        conn,
        &task.id,
        Some(&session.id),
        "session_started",
        Some(&previous_status),
        Some(TASK_STATUS_ACTIVE),
        None,
    )?;

    Ok(ActiveSession {
        session: Session::from(session),
        task: Task::from(task),
    })
}

fn get_active_session(conn: &Connection) -> CommandResult<Option<ActiveSession>> {
    let rows = active_session_rows(conn)?;

    match rows.len() {
        0 => Ok(None),
        1 => {
            let (session, task) = rows.into_iter().next().expect("one active session row");
            Ok(Some(ActiveSession {
                session: Session::from(session),
                task: Task::from(task),
            }))
        }
        _ => Err(CommandError::conflict(
            "Multiple active sessions were found; resolve recovery before continuing.",
        )),
    }
}

pub fn pending_recovery_from_startup(conn: &Connection) -> CommandResult<PendingSessionRecovery> {
    let session_id = get_active_session(conn)?.map(|active| active.session.id);
    Ok(PendingSessionRecovery(Mutex::new(session_id)))
}

pub fn pending_recovery_session_id(
    pending_recovery: &PendingSessionRecovery,
) -> CommandResult<Option<String>> {
    Ok(pending_recovery
        .0
        .lock()
        .map_err(|_| CommandError::internal("Session recovery state could not be read."))?
        .clone())
}

pub fn settings_show_today_on_startup(conn: &Connection) -> CommandResult<bool> {
    parse_bool_setting(conn, SETTING_STARTUP_TODAY_ON_STARTUP)
}

fn get_pending_session_recovery(
    conn: &Connection,
    pending_recovery: &PendingSessionRecovery,
) -> CommandResult<GetPendingSessionRecoveryResult> {
    let pending_session_id = pending_recovery
        .0
        .lock()
        .map_err(|_| CommandError::internal("Session recovery state could not be read."))?
        .clone();

    let Some(pending_session_id) = pending_session_id else {
        return Ok(GetPendingSessionRecoveryResult {
            active_session: None,
        });
    };

    let active_session = match get_active_session(conn)? {
        Some(active_session) if active_session.session.id == pending_session_id => {
            Some(active_session)
        }
        _ => {
            clear_pending_session_recovery(pending_recovery)?;
            None
        }
    };

    Ok(GetPendingSessionRecoveryResult { active_session })
}

fn clear_pending_session_recovery(pending_recovery: &PendingSessionRecovery) -> CommandResult<()> {
    *pending_recovery
        .0
        .lock()
        .map_err(|_| CommandError::internal("Session recovery state could not be updated."))? =
        None;
    Ok(())
}

fn complete_session(
    conn: &Connection,
    input: CompleteSessionInput,
) -> CommandResult<ActiveSession> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    let active_session = require_active_session(&tx, input.session_id.as_deref())?;
    validate_completion_confirmation(
        &active_session.1,
        input.confirm_long_term_completion.unwrap_or(false),
    )?;
    let ended = end_active_session(
        &tx,
        active_session,
        END_REASON_COMPLETED,
        input.progress_note,
        input.next_action,
        Some(TASK_STATUS_COMPLETED.to_string()),
    )?;
    tx.commit().map_err(to_internal_error)?;
    Ok(ended)
}

fn stop_session(
    conn: &Connection,
    input: StopSessionInput,
    end_reason: &str,
) -> CommandResult<ActiveSession> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    let active_session = require_active_session(&tx, input.session_id.as_deref())?;
    let progress_note = normalize_optional_text(input.progress_note);
    let next_action = normalize_optional_text(input.next_action);
    validate_long_term_stop_requirements(&active_session.1, &progress_note, &next_action)?;
    let destination = validate_stop_destination(
        input.destination_status.as_deref(),
        &active_session.1,
        end_reason,
    )?;

    let ended = end_active_session(
        &tx,
        active_session,
        end_reason,
        progress_note,
        next_action,
        Some(destination),
    )?;
    tx.commit().map_err(to_internal_error)?;
    Ok(ended)
}

fn switch_task(conn: &Connection, input: SwitchTaskInput) -> CommandResult<ActiveSession> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    let active_session = require_active_session(&tx, None)?;

    if active_session.1.id == input.task_id {
        return Err(CommandError::validation(
            "The requested task already has the active session.",
        ));
    }

    let target_task = require_task(&tx, &input.task_id)?;
    validate_startable_task(&target_task)?;
    let lap_duration_seconds = match input.lap_duration_seconds {
        Some(value) => validate_lap_duration_seconds(value)?,
        None => get_settings(&tx)?.donut_lap_duration_seconds,
    };

    let progress_note = normalize_optional_text(input.progress_note);
    let next_action = normalize_optional_text(input.next_action);
    validate_long_term_stop_requirements(&active_session.1, &progress_note, &next_action)?;
    let destination = validate_stop_destination(
        input.destination_status.as_deref(),
        &active_session.1,
        END_REASON_SWITCHED,
    )?;
    end_active_session(
        &tx,
        active_session,
        END_REASON_SWITCHED,
        progress_note,
        next_action,
        Some(destination),
    )?;

    let started = start_session_without_active_check(
        &tx,
        StartSessionInput {
            task_id: input.task_id,
            lap_duration_seconds: Some(lap_duration_seconds),
        },
    )?;
    tx.commit().map_err(to_internal_error)?;

    Ok(started)
}

fn resolve_session_recovery(
    conn: &Connection,
    input: ResolveSessionRecoveryInput,
) -> CommandResult<ResolveSessionRecoveryResult> {
    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    let active_session = require_active_session(&tx, input.session_id.as_deref())?;

    let result = match input.action.as_str() {
        RECOVERY_ACTION_RESUME => {
            let mut session = active_session.0;
            session.recovered_from_crash = true;
            session.updated_at = utc_now(&tx)?;
            persistence::update_session(&tx, &session).map_err(to_internal_error)?;

            Ok(ResolveSessionRecoveryResult {
                active_session: Some(ActiveSession {
                    session: Session::from(session),
                    task: Task::from(active_session.1),
                }),
                session: None,
                task: None,
            })
        }
        RECOVERY_ACTION_STOP => {
            let stopped = end_active_session(
                &tx,
                active_session,
                END_REASON_APP_CLOSED,
                normalize_optional_text(input.progress_note),
                normalize_optional_text(input.next_action),
                None,
            )?;

            Ok(ResolveSessionRecoveryResult {
                active_session: None,
                session: Some(stopped.session),
                task: Some(stopped.task),
            })
        }
        RECOVERY_ACTION_DISCARD => {
            let stopped = end_active_session(
                &tx,
                active_session,
                END_REASON_DISCARDED,
                normalize_optional_text(input.progress_note),
                normalize_optional_text(input.next_action),
                None,
            )?;

            Ok(ResolveSessionRecoveryResult {
                active_session: None,
                session: Some(stopped.session),
                task: Some(stopped.task),
            })
        }
        _ => Err(CommandError::validation(format!(
            "Invalid recovery action '{action}'. Expected resume, stop, or discard.",
            action = input.action
        ))),
    }?;
    tx.commit().map_err(to_internal_error)?;
    Ok(result)
}

fn get_settings(conn: &Connection) -> CommandResult<Settings> {
    Ok(Settings {
        launch_on_startup: parse_bool_setting(conn, SETTING_STARTUP_LAUNCH)?,
        today_on_startup: parse_bool_setting(conn, SETTING_STARTUP_TODAY_ON_STARTUP)?,
        donut_lap_duration_seconds: validate_lap_duration_seconds(parse_i64_setting(
            conn,
            SETTING_DONUT_LAP_DURATION_SECONDS,
        )?)?,
        theme: required_setting(conn, SETTING_THEME)?,
        floating_window_always_on_top: parse_bool_setting(
            conn,
            SETTING_FLOATING_WINDOW_ALWAYS_ON_TOP,
        )?,
        floating_window_position: parse_position_setting(conn)?,
        long_term_stop_requirements: required_setting(conn, SETTING_LONG_TERM_STOP_REQUIREMENTS)?,
        today_return_behavior: required_setting(conn, SETTING_TODAY_RETURN_BEHAVIOR)?,
    })
}

fn update_settings_for_app(
    app: &AppHandle,
    conn: &Connection,
    input: UpdateSettingsInput,
) -> CommandResult<Settings> {
    validate_settings_input(&input)?;
    let startup_registration = input.launch_on_startup;
    if let Some(enabled) = startup_registration {
        set_startup_registration(app, enabled)?;
    }

    update_settings(conn, input)
}

fn update_settings(conn: &Connection, input: UpdateSettingsInput) -> CommandResult<Settings> {
    validate_settings_input(&input)?;
    let donut_lap_duration_seconds = input
        .donut_lap_duration_seconds
        .map(validate_lap_duration_seconds)
        .transpose()?;
    let theme = input
        .theme
        .map(|value| validate_required_text("theme", value))
        .transpose()?;
    if let Some(position) = &input.floating_window_position {
        validate_position(position)?;
    }
    let long_term_stop_requirements = input
        .long_term_stop_requirements
        .map(|value| validate_required_text("longTermStopRequirements", value))
        .transpose()?;
    let today_return_behavior = input
        .today_return_behavior
        .map(|value| validate_required_text("todayReturnBehavior", value))
        .transpose()?;

    let tx = conn.unchecked_transaction().map_err(to_internal_error)?;
    if let Some(value) = input.launch_on_startup {
        set_bool_setting(&tx, SETTING_STARTUP_LAUNCH, value)?;
    }

    if let Some(value) = input.today_on_startup {
        set_bool_setting(&tx, SETTING_STARTUP_TODAY_ON_STARTUP, value)?;
    }

    if let Some(value) = donut_lap_duration_seconds {
        persistence::set_setting(&tx, SETTING_DONUT_LAP_DURATION_SECONDS, &value.to_string())
            .map_err(to_internal_error)?;
    }

    if let Some(value) = theme {
        persistence::set_setting(&tx, SETTING_THEME, &value).map_err(to_internal_error)?;
    }

    if let Some(value) = input.floating_window_always_on_top {
        set_bool_setting(&tx, SETTING_FLOATING_WINDOW_ALWAYS_ON_TOP, value)?;
    }

    if let Some(value) = input.floating_window_position {
        save_floating_window_position(&tx, value)?;
    }

    if let Some(value) = long_term_stop_requirements {
        persistence::set_setting(&tx, SETTING_LONG_TERM_STOP_REQUIREMENTS, &value)
            .map_err(to_internal_error)?;
    }

    if let Some(value) = today_return_behavior {
        persistence::set_setting(&tx, SETTING_TODAY_RETURN_BEHAVIOR, &value)
            .map_err(to_internal_error)?;
    }

    let settings = get_settings(&tx)?;
    tx.commit().map_err(to_internal_error)?;
    Ok(settings)
}

fn validate_settings_input(input: &UpdateSettingsInput) -> CommandResult<()> {
    if let Some(value) = input.donut_lap_duration_seconds {
        validate_lap_duration_seconds(value)?;
    }

    if let Some(value) = &input.theme {
        validate_required_text("theme", value.clone())?;
    }

    if let Some(position) = &input.floating_window_position {
        validate_position(position)?;
    }

    if let Some(value) = &input.long_term_stop_requirements {
        validate_required_text("longTermStopRequirements", value.clone())?;
    }

    if let Some(value) = &input.today_return_behavior {
        validate_required_text("todayReturnBehavior", value.clone())?;
    }

    Ok(())
}

fn save_floating_window_position(
    conn: &Connection,
    position: FloatingWindowPosition,
) -> CommandResult<FloatingWindowPosition> {
    validate_position(&position)?;
    let value = serde_json::to_string(&position).map_err(to_internal_error)?;
    persistence::set_setting(conn, SETTING_FLOATING_WINDOW_POSITION, &value)
        .map_err(to_internal_error)?;
    Ok(position)
}

fn end_active_session(
    conn: &Connection,
    active_session: (DbSession, DbTask),
    end_reason: &str,
    progress_note: Option<String>,
    next_action: Option<String>,
    destination_status: Option<String>,
) -> CommandResult<ActiveSession> {
    validate_end_reason(end_reason)?;

    let (mut session, mut task) = active_session;
    let previous_status = task.status.clone();
    let now = utc_now(conn)?;
    let duration_seconds = if end_reason == END_REASON_DISCARDED {
        None
    } else {
        Some(session_duration_seconds(conn, &session.started_at, &now)?)
    };

    session.ended_at = Some(now.clone());
    session.duration_seconds = duration_seconds;
    session.end_reason = Some(end_reason.to_string());
    if let Some(progress_note) = normalize_optional_text(progress_note) {
        session.progress_note = Some(progress_note);
    }
    if let Some(next_action) = normalize_optional_text(next_action) {
        session.next_action = Some(next_action);
    }
    session.updated_at = now.clone();
    persistence::update_session(conn, &session).map_err(to_internal_error)?;

    let destination_status =
        destination_status.unwrap_or_else(|| default_stopped_status_for_task(&task).to_string());
    validate_task_status(&destination_status)?;
    if destination_status == TASK_STATUS_ACTIVE {
        return Err(CommandError::validation(
            "A stopped or completed session cannot leave its task active.",
        ));
    }

    task.status = destination_status;
    if let Some(next_action) = session.next_action.clone() {
        task.next_action = Some(next_action);
    }
    apply_terminal_task_timestamps(&mut task, &previous_status, &now);
    task.updated_at = now;
    persistence::update_task(conn, &task).map_err(to_internal_error)?;
    insert_task_event(
        conn,
        &task.id,
        Some(&session.id),
        "session_ended",
        Some(&previous_status),
        Some(&task.status),
        session.progress_note.as_deref(),
    )?;

    Ok(ActiveSession {
        session: Session::from(session),
        task: Task::from(task),
    })
}

fn list_tasks_by_status(conn: &Connection, status: &str, limit: i64) -> CommandResult<Vec<Task>> {
    validate_task_status(status)?;
    let limit = validate_limit(limit)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                id, title, description, kind, status, priority, pickup_date,
                next_action, sort_order, created_at, updated_at, completed_at,
                archived_at
            FROM tasks
            WHERE status = ?1
            ORDER BY sort_order ASC, priority DESC, updated_at DESC
            LIMIT ?2
            "#,
        )
        .map_err(to_internal_error)?;

    let rows = stmt
        .query_map(params![status, limit], |row| db_task_from_row(row, 0))
        .map_err(to_internal_error)?;

    rows.map(|row| row.map(Task::from))
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_internal_error)
}

fn list_today_pickup_tasks(conn: &Connection, limit: i64) -> CommandResult<Vec<Task>> {
    let local_today = local_today_date(conn)?;
    list_today_pickup_tasks_for_date(conn, limit, &local_today)
}

fn local_today_date(conn: &Connection) -> CommandResult<String> {
    conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))
        .map_err(to_internal_error)
}

fn list_today_pickup_tasks_for_date(
    conn: &Connection,
    limit: i64,
    local_today: &str,
) -> CommandResult<Vec<Task>> {
    let limit = validate_limit(limit)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                id, title, description, kind, status, priority, pickup_date,
                next_action, sort_order, created_at, updated_at, completed_at,
                archived_at
            FROM tasks
            WHERE status = ?1
            ORDER BY
                CASE
                    WHEN pickup_date IS NOT NULL AND pickup_date <= ?2 THEN 0
                    ELSE 1
                END ASC,
                CASE
                    WHEN pickup_date IS NOT NULL AND pickup_date <= ?2 THEN pickup_date
                    ELSE NULL
                END ASC,
                priority DESC,
                updated_at DESC,
                sort_order ASC
            LIMIT ?3
            "#,
        )
        .map_err(to_internal_error)?;

    let rows = stmt
        .query_map(params![TASK_STATUS_PICKUP, local_today, limit], |row| {
            db_task_from_row(row, 0)
        })
        .map_err(to_internal_error)?;

    rows.map(|row| row.map(Task::from))
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_internal_error)
}

fn list_today_backlog_tasks(conn: &Connection, limit: i64) -> CommandResult<Vec<Task>> {
    let limit = validate_limit(limit)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                id, title, description, kind, status, priority, pickup_date,
                next_action, sort_order, created_at, updated_at, completed_at,
                archived_at
            FROM tasks
            WHERE status = ?1 AND kind = ?2
            ORDER BY priority DESC, updated_at DESC, sort_order ASC
            LIMIT ?3
            "#,
        )
        .map_err(to_internal_error)?;

    let rows = stmt
        .query_map(
            params![TASK_STATUS_BACKLOG, TASK_KIND_LONG_TERM, limit],
            |row| db_task_from_row(row, 0),
        )
        .map_err(to_internal_error)?;

    rows.map(|row| row.map(Task::from))
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_internal_error)
}

fn list_task_sessions(conn: &Connection, task_id: &str) -> CommandResult<Vec<DbSession>> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                id, task_id, started_at, ended_at, duration_seconds, end_reason,
                progress_note, next_action, lap_duration_seconds,
                recovered_from_crash, created_at, updated_at
            FROM sessions
            WHERE task_id = ?1
            ORDER BY COALESCE(ended_at, started_at) DESC, started_at DESC
            "#,
        )
        .map_err(to_internal_error)?;

    let rows = stmt
        .query_map(params![task_id], |row| db_session_from_row(row, 0))
        .map_err(to_internal_error)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(to_internal_error)
}

fn active_session_rows(conn: &Connection) -> CommandResult<Vec<(DbSession, DbTask)>> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                s.id, s.task_id, s.started_at, s.ended_at, s.duration_seconds,
                s.end_reason, s.progress_note, s.next_action, s.lap_duration_seconds,
                s.recovered_from_crash, s.created_at, s.updated_at,
                t.id, t.title, t.description, t.kind, t.status, t.priority,
                t.pickup_date, t.next_action, t.sort_order, t.created_at,
                t.updated_at, t.completed_at, t.archived_at
            FROM sessions s
            JOIN tasks t ON t.id = s.task_id
            WHERE s.ended_at IS NULL
            ORDER BY s.started_at DESC
            LIMIT 2
            "#,
        )
        .map_err(to_internal_error)?;

    let rows = stmt
        .query_map([], |row| {
            Ok((db_session_from_row(row, 0)?, db_task_from_row(row, 12)?))
        })
        .map_err(to_internal_error)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(to_internal_error)
}

fn require_active_session(
    conn: &Connection,
    session_id: Option<&str>,
) -> CommandResult<(DbSession, DbTask)> {
    let active_sessions = active_session_rows(conn)?;
    let active_session = match active_sessions.len() {
        0 => {
            return Err(CommandError::validation(
                "No active session is available for this operation.",
            ))
        }
        1 => active_sessions
            .into_iter()
            .next()
            .expect("one active session row"),
        _ => {
            return Err(CommandError::conflict(
                "Multiple active sessions were found; resolve recovery before continuing.",
            ))
        }
    };

    if let Some(expected_session_id) = session_id {
        if active_session.0.id != expected_session_id {
            return Err(CommandError::validation(format!(
                "Session '{expected_session_id}' is not the active session."
            )));
        }
    }

    Ok(active_session)
}

fn require_task(conn: &Connection, task_id: &str) -> CommandResult<DbTask> {
    persistence::get_task(conn, task_id)
        .map_err(to_internal_error)?
        .ok_or_else(|| CommandError::not_found(format!("Task '{task_id}' was not found.")))
}

fn task_has_active_session(conn: &Connection, task_id: &str) -> CommandResult<bool> {
    conn.query_row(
        "SELECT 1 FROM sessions WHERE task_id = ?1 AND ended_at IS NULL LIMIT 1",
        params![task_id],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(to_internal_error)
}

fn insert_active_session(conn: &Connection, session: &DbSession) -> CommandResult<()> {
    persistence::insert_session(conn, session).map_err(|error| {
        let message = error.to_string();
        if message.contains("idx_sessions_one_active")
            || message.contains("UNIQUE constraint failed")
        {
            CommandError::conflict("A session is already active; use switchTask to change tasks.")
        } else {
            to_internal_error(error)
        }
    })
}

fn next_sort_order(conn: &Connection, status: &str) -> CommandResult<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 10 FROM tasks WHERE status = ?1",
        params![status],
        |row| row.get(0),
    )
    .map_err(to_internal_error)
}

fn new_id(conn: &Connection, prefix: &str) -> CommandResult<String> {
    let random: String = conn
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(to_internal_error)?;
    Ok(format!("{prefix}-{random}"))
}

fn utc_now(conn: &Connection) -> CommandResult<String> {
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })
    .map_err(to_internal_error)
}

fn session_duration_seconds(
    conn: &Connection,
    started_at: &str,
    ended_at: &str,
) -> CommandResult<i64> {
    conn.query_row(
        "SELECT MAX(0, CAST(strftime('%s', ?2) AS INTEGER) - CAST(strftime('%s', ?1) AS INTEGER))",
        params![started_at, ended_at],
        |row| row.get(0),
    )
    .map_err(to_internal_error)
}

fn insert_task_event(
    conn: &Connection,
    task_id: &str,
    session_id: Option<&str>,
    event_type: &str,
    from_status: Option<&str>,
    to_status: Option<&str>,
    note: Option<&str>,
) -> CommandResult<()> {
    let now = utc_now(conn)?;
    let event = TaskEvent {
        id: new_id(conn, "event")?,
        task_id: task_id.to_string(),
        session_id: session_id.map(ToString::to_string),
        event_type: event_type.to_string(),
        from_status: from_status.map(ToString::to_string),
        to_status: to_status.map(ToString::to_string),
        note: note.map(ToString::to_string),
        created_at: now,
    };

    persistence::insert_task_event(conn, &event).map_err(to_internal_error)
}

fn validate_task_kind(kind: &str) -> CommandResult<&str> {
    match kind {
        TASK_KIND_PICKUP | TASK_KIND_LONG_TERM => Ok(kind),
        _ => Err(CommandError::validation(format!(
            "Invalid task kind '{kind}'. Expected pickup or long_term."
        ))),
    }
}

fn validate_task_status(status: &str) -> CommandResult<&str> {
    match status {
        TASK_STATUS_PICKUP
        | TASK_STATUS_BACKLOG
        | TASK_STATUS_ACTIVE
        | TASK_STATUS_COMPLETED
        | TASK_STATUS_ARCHIVED => Ok(status),
        _ => Err(CommandError::validation(format!(
            "Invalid task status '{status}'. Expected pickup, backlog, active, completed, or archived."
        ))),
    }
}

fn validate_initial_status(status: &str) -> CommandResult<&str> {
    validate_task_status(status)?;
    match status {
        TASK_STATUS_PICKUP | TASK_STATUS_BACKLOG => Ok(status),
        _ => Err(CommandError::validation(
            "New tasks must start in pickup or backlog status.",
        )),
    }
}

fn validate_end_reason(end_reason: &str) -> CommandResult<&str> {
    match end_reason {
        END_REASON_COMPLETED
        | END_REASON_STOPPED
        | END_REASON_SWITCHED
        | END_REASON_APP_CLOSED
        | END_REASON_DISCARDED => Ok(end_reason),
        _ => Err(CommandError::validation(format!(
            "Invalid end reason '{end_reason}'."
        ))),
    }
}

fn validate_stop_destination(
    destination_status: Option<&str>,
    task: &DbTask,
    end_reason: &str,
) -> CommandResult<String> {
    let destination = destination_status
        .unwrap_or_else(|| default_stopped_status_for_task(task))
        .to_string();
    validate_task_status(&destination)?;

    if destination == TASK_STATUS_ACTIVE {
        return Err(CommandError::validation(
            "Stop destinations must be pickup, backlog, completed, or archived.",
        ));
    }

    if task.kind == TASK_KIND_LONG_TERM
        && matches!(end_reason, END_REASON_STOPPED | END_REASON_SWITCHED)
        && !matches!(
            destination.as_str(),
            TASK_STATUS_BACKLOG | TASK_STATUS_PICKUP
        )
    {
        return Err(CommandError::validation(
            "Long-term tasks can only stop to backlog or pickup.",
        ));
    }

    Ok(destination)
}

fn validate_long_term_stop_requirements(
    task: &DbTask,
    progress_note: &Option<String>,
    next_action: &Option<String>,
) -> CommandResult<()> {
    if task.kind != TASK_KIND_LONG_TERM {
        return Ok(());
    }

    if progress_note.is_none() {
        return Err(CommandError::validation(
            "Stopping a long-term task requires a progress note.",
        ));
    }

    if next_action.is_none() {
        return Err(CommandError::validation(
            "Stopping a long-term task requires a next action.",
        ));
    }

    Ok(())
}

fn validate_completion_confirmation(task: &DbTask, confirmed: bool) -> CommandResult<()> {
    if task.kind == TASK_KIND_LONG_TERM && !confirmed {
        return Err(CommandError::validation(
            "Completing a long-term task requires explicit confirmation.",
        ));
    }

    Ok(())
}

fn validate_startable_task(task: &DbTask) -> CommandResult<()> {
    validate_task_kind(&task.kind)?;
    validate_task_status(&task.status)?;

    match task.status.as_str() {
        TASK_STATUS_PICKUP | TASK_STATUS_BACKLOG => Ok(()),
        TASK_STATUS_ACTIVE => Err(CommandError::validation("Task is already active.")),
        TASK_STATUS_COMPLETED => Err(CommandError::validation(
            "Completed tasks cannot start a new session.",
        )),
        TASK_STATUS_ARCHIVED => Err(CommandError::validation(
            "Archived tasks cannot start a new session.",
        )),
        _ => Err(CommandError::validation("Task has an invalid status.")),
    }
}

fn validate_lap_duration_seconds(value: i64) -> CommandResult<i64> {
    if (10..=600).contains(&value) {
        Ok(value)
    } else {
        Err(CommandError::validation(
            "Donut lap duration must be between 10 and 600 seconds.",
        ))
    }
}

fn validate_limit(value: i64) -> CommandResult<i64> {
    if (1..=200).contains(&value) {
        Ok(value)
    } else {
        Err(CommandError::validation(
            "List limit must be between 1 and 200.",
        ))
    }
}

fn validate_position(position: &FloatingWindowPosition) -> CommandResult<()> {
    if (-100_000..=100_000).contains(&position.x) && (-100_000..=100_000).contains(&position.y) {
        Ok(())
    } else {
        Err(CommandError::validation(
            "Floating window position is outside the allowed range.",
        ))
    }
}

fn validate_required_text(field_name: &str, value: String) -> CommandResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(CommandError::validation(format!(
            "{field_name} is required."
        )))
    } else {
        Ok(value)
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

fn default_status_for_kind(kind: &str) -> &'static str {
    if kind == TASK_KIND_LONG_TERM {
        TASK_STATUS_BACKLOG
    } else {
        TASK_STATUS_PICKUP
    }
}

fn default_stopped_status_for_task(task: &DbTask) -> &'static str {
    if task.kind == TASK_KIND_LONG_TERM {
        TASK_STATUS_BACKLOG
    } else {
        TASK_STATUS_PICKUP
    }
}

fn apply_terminal_task_timestamps(task: &mut DbTask, previous_status: &str, now: &str) {
    if task.status == TASK_STATUS_COMPLETED && previous_status != TASK_STATUS_COMPLETED {
        task.completed_at = Some(now.to_string());
    }

    if task.status == TASK_STATUS_ARCHIVED && previous_status != TASK_STATUS_ARCHIVED {
        task.archived_at = Some(now.to_string());
    }
}

fn required_setting(conn: &Connection, key: &str) -> CommandResult<String> {
    persistence::get_setting(conn, key)
        .map_err(to_internal_error)?
        .ok_or_else(|| CommandError::validation(format!("Missing required setting '{key}'.")))
}

fn parse_bool_setting(conn: &Connection, key: &str) -> CommandResult<bool> {
    let value = required_setting(conn, key)?;
    match value.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(CommandError::validation(format!(
            "Setting '{key}' must be true or false."
        ))),
    }
}

fn parse_i64_setting(conn: &Connection, key: &str) -> CommandResult<i64> {
    let value = required_setting(conn, key)?;
    value
        .parse::<i64>()
        .map_err(|_| CommandError::validation(format!("Setting '{key}' must be an integer value.")))
}

fn parse_position_setting(conn: &Connection) -> CommandResult<FloatingWindowPosition> {
    let value = required_setting(conn, SETTING_FLOATING_WINDOW_POSITION)?;
    let position = serde_json::from_str::<FloatingWindowPosition>(&value).map_err(|_| {
        CommandError::validation("Floating window position setting must be a JSON object.")
    })?;
    validate_position(&position)?;
    Ok(position)
}

fn set_bool_setting(conn: &Connection, key: &str, value: bool) -> CommandResult<()> {
    persistence::set_setting(conn, key, if value { "true" } else { "false" })
        .map_err(to_internal_error)
}

fn set_startup_registration(_app: &AppHandle, enabled: bool) -> CommandResult<()> {
    #[cfg(target_os = "windows")]
    {
        let executable = std::env::current_exe().map_err(to_internal_error)?;
        set_windows_startup_registration("Thread", &executable, enabled)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = _app;
        let _ = enabled;
        Err(CommandError::validation(
            "Launch on startup is only supported on Windows.",
        ))
    }
}

#[cfg(target_os = "windows")]
fn set_windows_startup_registration(
    name: &str,
    executable: &Path,
    enabled: bool,
) -> CommandResult<()> {
    let status = if enabled {
        Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                name,
                "/t",
                "REG_SZ",
                "/d",
            ])
            .arg(format!("\"{}\"", executable.display()))
            .arg("/f")
            .status()
            .map_err(to_internal_error)?
    } else {
        Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                name,
                "/f",
            ])
            .status()
            .map_err(to_internal_error)?
    };

    if status.success() {
        Ok(())
    } else {
        Err(CommandError::internal(format!(
            "Windows startup registration failed with status {status}."
        )))
    }
}

fn db_task_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<DbTask> {
    Ok(DbTask {
        id: row.get(offset)?,
        title: row.get(offset + 1)?,
        description: row.get(offset + 2)?,
        kind: row.get(offset + 3)?,
        status: row.get(offset + 4)?,
        priority: row.get(offset + 5)?,
        pickup_date: row.get(offset + 6)?,
        next_action: row.get(offset + 7)?,
        sort_order: row.get(offset + 8)?,
        created_at: row.get(offset + 9)?,
        updated_at: row.get(offset + 10)?,
        completed_at: row.get(offset + 11)?,
        archived_at: row.get(offset + 12)?,
    })
}

fn db_session_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<DbSession> {
    let recovered_from_crash: i64 = row.get(offset + 9)?;

    Ok(DbSession {
        id: row.get(offset)?,
        task_id: row.get(offset + 1)?,
        started_at: row.get(offset + 2)?,
        ended_at: row.get(offset + 3)?,
        duration_seconds: row.get(offset + 4)?,
        end_reason: row.get(offset + 5)?,
        progress_note: row.get(offset + 6)?,
        next_action: row.get(offset + 7)?,
        lap_duration_seconds: row.get(offset + 8)?,
        recovered_from_crash: recovered_from_crash != 0,
        created_at: row.get(offset + 10)?,
        updated_at: row.get(offset + 11)?,
    })
}

fn export_database_to_app_dir(
    source_path: &Path,
    export_dir: &Path,
    file_name: Option<String>,
) -> CommandResult<ExportDatabaseResult> {
    let file_name = match file_name {
        Some(file_name) => validate_export_file_name(file_name)?,
        None => default_export_file_name()?,
    };

    fs::create_dir_all(export_dir).map_err(to_internal_error)?;
    let canonical_source = fs::canonicalize(source_path).map_err(to_internal_error)?;
    let canonical_export_dir = fs::canonicalize(export_dir).map_err(to_internal_error)?;
    let destination_path = canonical_export_dir.join(file_name);

    if !destination_path.starts_with(&canonical_export_dir) {
        return Err(CommandError::validation(
            "Export file must stay inside the app export directory.",
        ));
    }

    let destination_parent = destination_path
        .parent()
        .ok_or_else(|| CommandError::internal("Export path has no parent directory."))?;
    if fs::canonicalize(destination_parent).map_err(to_internal_error)? != canonical_export_dir {
        return Err(CommandError::validation(
            "Export file must stay inside the app export directory.",
        ));
    }

    if destination_path.exists() {
        return Err(CommandError::validation(
            "Export file already exists; choose a new file name.",
        ));
    }

    let mut source = fs::File::open(&canonical_source).map_err(to_internal_error)?;
    let mut destination = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination_path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                CommandError::validation("Export file already exists; choose a new file name.")
            } else {
                to_internal_error(error)
            }
        })?;
    io::copy(&mut source, &mut destination).map_err(to_internal_error)?;

    Ok(ExportDatabaseResult {
        path: destination_path.to_string_lossy().to_string(),
    })
}

fn validate_export_file_name(file_name: String) -> CommandResult<String> {
    let file_name = validate_required_text("fileName", file_name)?;
    if file_name == "." || file_name == ".." {
        return Err(CommandError::validation(
            "Export file name must be a file name, not a path.",
        ));
    }

    if file_name.contains('/') || file_name.contains('\\') || file_name.contains(':') {
        return Err(CommandError::validation(
            "Export file name must be a file name, not a path.",
        ));
    }

    let path = Path::new(&file_name);
    if path.file_name().and_then(|value| value.to_str()) != Some(file_name.as_str()) {
        return Err(CommandError::validation(
            "Export file name must be a file name, not a path.",
        ));
    }

    if !file_name.ends_with(EXPORT_FILE_EXTENSION) {
        return Err(CommandError::validation(format!(
            "Export file name must end with {EXPORT_FILE_EXTENSION}."
        )));
    }

    Ok(file_name)
}

fn default_export_file_name() -> CommandResult<String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(to_internal_error)?
        .as_millis();
    Ok(format!("thread-export-{timestamp}{EXPORT_FILE_EXTENSION}"))
}

fn open_folder(path: &PathBuf) -> CommandResult<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(path.as_os_str());
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path.as_os_str());
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path.as_os_str());
        command
    };

    command.spawn().map_err(to_internal_error)?;
    Ok(())
}

fn to_internal_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::internal(error.to_string())
}

impl From<DbTask> for Task {
    fn from(task: DbTask) -> Self {
        Self {
            id: task.id,
            title: task.title,
            description: task.description,
            kind: task.kind,
            status: task.status,
            priority: task.priority,
            pickup_date: task.pickup_date,
            next_action: task.next_action,
            sort_order: task.sort_order,
            created_at: task.created_at,
            updated_at: task.updated_at,
            completed_at: task.completed_at,
            archived_at: task.archived_at,
        }
    }
}

impl From<DbSession> for Session {
    fn from(session: DbSession) -> Self {
        Self {
            id: session.id,
            task_id: session.task_id,
            started_at: session.started_at,
            ended_at: session.ended_at,
            duration_seconds: session.duration_seconds,
            end_reason: session.end_reason,
            progress_note: session.progress_note,
            next_action: session.next_action,
            lap_duration_seconds: session.lap_duration_seconds,
            recovered_from_crash: session.recovered_from_crash,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        persistence::initialize_database(&mut conn).expect("initialize database");
        conn
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}"))
    }

    fn create_input(kind: &str) -> CreateTaskInput {
        CreateTaskInput {
            title: "Write a command test".to_string(),
            description: Some("Cover command validation".to_string()),
            kind: kind.to_string(),
            status: None,
            priority: Some(1),
            pickup_date: None,
            next_action: Some("Run cargo test".to_string()),
            sort_order: None,
        }
    }

    fn task_fixture(
        id: &str,
        title: &str,
        kind: &str,
        status: &str,
        priority: i64,
        pickup_date: Option<&str>,
        updated_at: &str,
        sort_order: i64,
    ) -> DbTask {
        DbTask {
            id: id.to_string(),
            title: title.to_string(),
            description: String::new(),
            kind: kind.to_string(),
            status: status.to_string(),
            priority,
            pickup_date: pickup_date.map(str::to_string),
            next_action: Some(format!("Next action for {title}")),
            sort_order,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: updated_at.to_string(),
            completed_at: None,
            archived_at: None,
        }
    }

    fn session_fixture(id: &str, task_id: &str, started_at: &str, ended_at: &str) -> DbSession {
        DbSession {
            id: id.to_string(),
            task_id: task_id.to_string(),
            started_at: started_at.to_string(),
            ended_at: Some(ended_at.to_string()),
            duration_seconds: Some(900),
            end_reason: Some(END_REASON_STOPPED.to_string()),
            progress_note: Some(format!("Progress for {task_id}")),
            next_action: Some(format!("Resume {task_id}")),
            lap_duration_seconds: 60,
            recovered_from_crash: false,
            created_at: started_at.to_string(),
            updated_at: ended_at.to_string(),
        }
    }

    #[test]
    fn create_task_validates_and_returns_predictable_shape() {
        let conn = in_memory_db();

        let pickup = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create pickup");
        assert_eq!(pickup.kind, TASK_KIND_PICKUP);
        assert_eq!(pickup.status, TASK_STATUS_PICKUP);
        assert_eq!(pickup.title, "Write a command test");
        assert!(pickup.id.starts_with("task-"));

        let long_term =
            create_task(&conn, create_input(TASK_KIND_LONG_TERM)).expect("create long-term task");
        assert_eq!(long_term.kind, TASK_KIND_LONG_TERM);
        assert_eq!(long_term.status, TASK_STATUS_BACKLOG);
    }

    #[test]
    fn create_task_rejects_invalid_kind_and_initial_status() {
        let conn = in_memory_db();

        let invalid_kind = create_task(&conn, create_input("today")).expect_err("invalid kind");
        assert_eq!(invalid_kind.code, "validation");
        assert!(invalid_kind.message.contains("Invalid task kind"));

        let mut input = create_input(TASK_KIND_PICKUP);
        input.status = Some(TASK_STATUS_ACTIVE.to_string());
        let invalid_status = create_task(&conn, input).expect_err("invalid initial status");
        assert_eq!(invalid_status.code, "validation");
        assert!(invalid_status.message.contains("New tasks must start"));
    }

    #[test]
    fn list_today_orders_active_pickup_recent_threads_and_backlog_preview() {
        let conn = in_memory_db();
        let active_task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: active_task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start active session");

        for task in [
            task_fixture(
                "pickup-due",
                "Due pickup",
                TASK_KIND_PICKUP,
                TASK_STATUS_PICKUP,
                0,
                Some("1900-01-01"),
                "2026-01-01T00:00:00.000Z",
                30,
            ),
            task_fixture(
                "pickup-high",
                "High priority pickup",
                TASK_KIND_PICKUP,
                TASK_STATUS_PICKUP,
                5,
                None,
                "2026-02-01T00:00:00.000Z",
                20,
            ),
            task_fixture(
                "pickup-recent",
                "Recently touched pickup",
                TASK_KIND_PICKUP,
                TASK_STATUS_PICKUP,
                1,
                None,
                "2026-03-01T00:00:00.000Z",
                10,
            ),
        ] {
            persistence::insert_task(&conn, &task).expect("insert pickup fixture");
        }

        for task in [
            task_fixture(
                "recent-older-task",
                "Older recent thread",
                TASK_KIND_PICKUP,
                TASK_STATUS_COMPLETED,
                0,
                None,
                "2026-04-01T00:00:00.000Z",
                10,
            ),
            task_fixture(
                "recent-newer-task",
                "Newer recent thread",
                TASK_KIND_PICKUP,
                TASK_STATUS_COMPLETED,
                0,
                None,
                "2026-04-02T00:00:00.000Z",
                20,
            ),
        ] {
            persistence::insert_task(&conn, &task).expect("insert recent task fixture");
        }
        persistence::insert_session(
            &conn,
            &session_fixture(
                "session-older",
                "recent-older-task",
                "2026-05-07T10:00:00.000Z",
                "2026-05-07T10:15:00.000Z",
            ),
        )
        .expect("insert older session");
        persistence::insert_session(
            &conn,
            &session_fixture(
                "session-newer",
                "recent-newer-task",
                "2026-05-08T10:00:00.000Z",
                "2026-05-08T10:15:00.000Z",
            ),
        )
        .expect("insert newer session");

        persistence::insert_task(
            &conn,
            &task_fixture(
                "backlog-pickup",
                "Pickup parked in backlog",
                TASK_KIND_PICKUP,
                TASK_STATUS_BACKLOG,
                99,
                None,
                "2026-05-01T00:00:00.000Z",
                5,
            ),
        )
        .expect("insert pickup backlog fixture");

        persistence::insert_task(
            &conn,
            &task_fixture(
                "backlog-high",
                "High priority backlog",
                TASK_KIND_LONG_TERM,
                TASK_STATUS_BACKLOG,
                9,
                None,
                "2026-01-01T00:00:00.000Z",
                10,
            ),
        )
        .expect("insert high priority backlog");

        for index in 0..7 {
            persistence::insert_task(
                &conn,
                &task_fixture(
                    &format!("backlog-recent-{index}"),
                    &format!("Recent backlog {index}"),
                    TASK_KIND_LONG_TERM,
                    TASK_STATUS_BACKLOG,
                    0,
                    None,
                    &format!("2026-02-0{}T00:00:00.000Z", index + 1),
                    index + 20,
                ),
            )
            .expect("insert backlog fixture");
        }

        let today = list_today(&conn).expect("list today");

        assert_eq!(
            today
                .active_session
                .as_ref()
                .expect("active work is first-class in today payload")
                .task
                .id,
            active_task.id
        );
        assert_eq!(
            today
                .pickup
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["pickup-due", "pickup-high", "pickup-recent"]
        );
        assert_eq!(
            today
                .recent_threads
                .iter()
                .map(|thread| thread.task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["recent-newer-task", "recent-older-task"]
        );
        assert_eq!(today.backlog.len(), 6);
        assert_eq!(today.backlog[0].id, "backlog-high");
        assert_eq!(today.backlog[1].id, "backlog-recent-6");
        assert!(!today.backlog.iter().any(|task| task.id == "backlog-pickup"));
        assert!(!today
            .backlog
            .iter()
            .any(|task| task.id == "backlog-recent-0"));
    }

    #[test]
    fn list_today_pickup_due_group_uses_local_today_date() {
        let conn = in_memory_db();
        for task in [
            task_fixture(
                "pickup-local-today",
                "Local today pickup",
                TASK_KIND_PICKUP,
                TASK_STATUS_PICKUP,
                0,
                Some("2026-05-09"),
                "2026-05-01T00:00:00.000Z",
                10,
            ),
            task_fixture(
                "pickup-next-local-day",
                "Next local day pickup",
                TASK_KIND_PICKUP,
                TASK_STATUS_PICKUP,
                99,
                Some("2026-05-10"),
                "2026-05-02T00:00:00.000Z",
                20,
            ),
            task_fixture(
                "pickup-undated",
                "Undated pickup",
                TASK_KIND_PICKUP,
                TASK_STATUS_PICKUP,
                1,
                None,
                "2026-05-03T00:00:00.000Z",
                30,
            ),
        ] {
            persistence::insert_task(&conn, &task).expect("insert pickup fixture");
        }

        let may_ninth = list_today_pickup_tasks_for_date(&conn, 10, "2026-05-09")
            .expect("list pickups for local May 9");
        assert_eq!(may_ninth[0].id, "pickup-local-today");
        assert_eq!(may_ninth[1].id, "pickup-next-local-day");

        let may_tenth = list_today_pickup_tasks_for_date(&conn, 10, "2026-05-10")
            .expect("list pickups for local May 10");
        assert_eq!(
            may_tenth
                .iter()
                .take(2)
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["pickup-local-today", "pickup-next-local-day"]
        );
    }

    #[test]
    fn list_recent_threads_orders_by_last_worked_and_includes_completed_sessions() {
        let conn = in_memory_db();
        for task in [
            task_fixture(
                "recent-stopped-task",
                "Stopped task",
                TASK_KIND_PICKUP,
                TASK_STATUS_PICKUP,
                0,
                None,
                "2026-05-01T00:00:00.000Z",
                10,
            ),
            task_fixture(
                "recent-completed-task",
                "Completed task",
                TASK_KIND_PICKUP,
                TASK_STATUS_COMPLETED,
                0,
                None,
                "2026-05-01T00:00:00.000Z",
                20,
            ),
        ] {
            persistence::insert_task(&conn, &task).expect("insert task fixture");
        }

        persistence::insert_session(
            &conn,
            &session_fixture(
                "session-stopped",
                "recent-stopped-task",
                "2026-05-10T10:00:00.000Z",
                "2026-05-10T10:15:00.000Z",
            ),
        )
        .expect("insert stopped session");
        let mut completed = session_fixture(
            "session-completed",
            "recent-completed-task",
            "2026-05-10T11:00:00.000Z",
            "2026-05-10T11:15:00.000Z",
        );
        completed.end_reason = Some(END_REASON_COMPLETED.to_string());
        persistence::insert_session(&conn, &completed).expect("insert completed session");

        let threads = list_recent_threads(&conn, 10).expect("list recent threads");

        assert_eq!(
            threads
                .iter()
                .map(|thread| thread.session.id.as_str())
                .collect::<Vec<_>>(),
            vec!["session-completed", "session-stopped"]
        );
        assert_eq!(
            threads[0].session.end_reason.as_deref(),
            Some(END_REASON_COMPLETED)
        );
        assert_eq!(threads[0].duration_seconds, Some(900));
    }

    #[test]
    fn task_detail_excludes_discarded_sessions_from_total_but_marks_history() {
        let conn = in_memory_db();
        let mut task = task_fixture(
            "detail-task",
            "Detail task",
            TASK_KIND_LONG_TERM,
            TASK_STATUS_BACKLOG,
            0,
            None,
            "2026-05-01T00:00:00.000Z",
            10,
        );
        task.description = "Inspect recoverable history".to_string();
        persistence::insert_task(&conn, &task).expect("insert detail task");

        persistence::insert_session(
            &conn,
            &session_fixture(
                "session-counted",
                "detail-task",
                "2026-05-10T10:00:00.000Z",
                "2026-05-10T10:15:00.000Z",
            ),
        )
        .expect("insert counted session");
        let mut discarded = session_fixture(
            "session-discarded",
            "detail-task",
            "2026-05-10T11:00:00.000Z",
            "2026-05-10T11:15:00.000Z",
        );
        discarded.end_reason = Some(END_REASON_DISCARDED.to_string());
        discarded.progress_note = Some("Accidental startup".to_string());
        persistence::insert_session(&conn, &discarded).expect("insert discarded session");

        let detail = get_task_detail(
            &conn,
            GetTaskDetailInput {
                task_id: "detail-task".to_string(),
            },
        )
        .expect("read task detail");

        assert_eq!(detail.task.title, "Detail task");
        assert_eq!(detail.task.description, "Inspect recoverable history");
        assert_eq!(detail.total_duration_seconds, 900);
        assert_eq!(
            detail
                .sessions
                .iter()
                .map(|session| session.id.as_str())
                .collect::<Vec<_>>(),
            vec!["session-discarded", "session-counted"]
        );
        assert_eq!(
            detail.sessions[0].end_reason.as_deref(),
            Some(END_REASON_DISCARDED)
        );
    }

    #[test]
    fn update_task_rejects_invalid_status() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");

        let error = update_task(
            &conn,
            UpdateTaskInput {
                id: task.id,
                status: Some("done".to_string()),
                ..UpdateTaskInput::default()
            },
        )
        .expect_err("invalid status");

        assert_eq!(error.code, "validation");
        assert!(error.message.contains("Invalid task status"));
    }

    #[test]
    fn update_task_rejects_status_change_for_task_with_active_session() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start session");

        let error = update_task(
            &conn,
            UpdateTaskInput {
                id: task.id.clone(),
                status: Some(TASK_STATUS_COMPLETED.to_string()),
                ..UpdateTaskInput::default()
            },
        )
        .expect_err("active session status change");

        assert_eq!(error.code, "conflict");
        assert!(error.message.contains("active session"));

        let active_session = get_active_session(&conn)
            .expect("read active session")
            .expect("active session remains open");
        assert_eq!(active_session.task.id, task.id);
        assert_eq!(active_session.task.status, TASK_STATUS_ACTIVE);
    }

    #[test]
    fn pickup_start_creates_only_active_session_and_hides_from_pickup() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");

        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start pickup session");

        assert_eq!(active.task.status, TASK_STATUS_ACTIVE);
        assert_eq!(active.session.ended_at, None);
        assert_eq!(active.session.duration_seconds, None);

        let today = list_today(&conn).expect("list today");
        assert_eq!(
            today.active_session.expect("active session").session.id,
            active.session.id
        );
        assert!(!today.pickup.iter().any(|pickup| pickup.id == task.id));

        let second_task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("second task");
        let error = start_session(
            &conn,
            StartSessionInput {
                task_id: second_task.id,
                lap_duration_seconds: Some(60),
            },
        )
        .expect_err("second active session rejected");
        assert_eq!(error.code, "conflict");
        assert!(error.message.contains("switchTask"));
    }

    #[test]
    fn completing_pickup_ends_session_and_removes_from_pickup() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start pickup session");

        let completed = complete_session(
            &conn,
            CompleteSessionInput {
                session_id: Some(active.session.id.clone()),
                progress_note: Some("Finished it".to_string()),
                next_action: None,
                confirm_long_term_completion: None,
            },
        )
        .expect("complete pickup session");

        assert_eq!(completed.task.status, TASK_STATUS_COMPLETED);
        assert!(completed.task.completed_at.is_some());
        assert_eq!(
            completed.session.end_reason.as_deref(),
            Some(END_REASON_COMPLETED)
        );
        assert!(completed.session.ended_at.is_some());
        assert!(completed.session.duration_seconds.is_some());
        assert!(get_active_session(&conn)
            .expect("read active session")
            .is_none());
        assert!(!list_today(&conn)
            .expect("list today")
            .pickup
            .iter()
            .any(|pickup| pickup.id == task.id));
    }

    #[test]
    fn stopping_pickup_defaults_back_to_pickup_with_optional_notes() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start pickup session");

        let stopped = stop_session(
            &conn,
            StopSessionInput {
                session_id: Some(active.session.id),
                progress_note: None,
                next_action: Some("Resume the draft".to_string()),
                destination_status: None,
            },
            END_REASON_STOPPED,
        )
        .expect("stop pickup session");

        assert_eq!(stopped.task.status, TASK_STATUS_PICKUP);
        assert_eq!(
            stopped.task.next_action.as_deref(),
            Some("Resume the draft")
        );
        assert_eq!(
            stopped.session.end_reason.as_deref(),
            Some(END_REASON_STOPPED)
        );
        assert!(stopped.session.duration_seconds.is_some());
        assert!(list_today(&conn)
            .expect("list today")
            .pickup
            .iter()
            .any(|pickup| pickup.id == task.id));
    }

    #[test]
    fn stopping_pickup_can_return_to_backlog() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start pickup session");

        let stopped = stop_session(
            &conn,
            StopSessionInput {
                session_id: None,
                progress_note: Some("Paused after triage".to_string()),
                next_action: Some("Pick this up later".to_string()),
                destination_status: Some(TASK_STATUS_BACKLOG.to_string()),
            },
            END_REASON_STOPPED,
        )
        .expect("stop pickup to backlog");

        assert_eq!(stopped.task.status, TASK_STATUS_BACKLOG);
        assert!(!list_today(&conn)
            .expect("list today")
            .pickup
            .iter()
            .any(|pickup| pickup.id == task.id));
    }

    #[test]
    fn tray_stop_stops_pickup_without_extra_input() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start pickup session");

        match stop_current_task_for_tray(&conn).expect("tray stop pickup") {
            CurrentTaskStop::Stopped(stopped) => {
                assert_eq!(stopped.task.id, task.id);
                assert_eq!(
                    stopped.session.end_reason.as_deref(),
                    Some(END_REASON_STOPPED)
                );
            }
            CurrentTaskStop::NoActiveSession | CurrentTaskStop::RequiresUi => {
                panic!("pickup tray stop should end the session")
            }
        }

        assert!(get_active_session(&conn)
            .expect("read active session")
            .is_none());
    }

    #[test]
    fn tray_stop_routes_long_term_to_ui_for_required_fields() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_LONG_TERM)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start long-term session");

        match stop_current_task_for_tray(&conn).expect("tray stop long-term") {
            CurrentTaskStop::RequiresUi => {}
            CurrentTaskStop::NoActiveSession | CurrentTaskStop::Stopped(_) => {
                panic!("long-term tray stop should require UI input")
            }
        }

        assert_eq!(
            get_active_session(&conn)
                .expect("read active session")
                .expect("still active")
                .task
                .id,
            task.id
        );
    }

    #[test]
    fn quit_preservation_leaves_active_session_for_startup_recovery() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id,
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start pickup session");

        preserve_unfinished_session_for_recovery(&conn).expect("preserve active session");
        let pending =
            pending_recovery_from_startup(&conn).expect("read pending recovery after quit");

        assert_eq!(
            pending_recovery_session_id(&pending).expect("read pending session id"),
            Some(active.session.id)
        );
    }

    #[test]
    fn switch_task_ends_current_session_and_starts_target() {
        let conn = in_memory_db();
        let first = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("first task");
        let second = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("second task");
        let original = start_session(
            &conn,
            StartSessionInput {
                task_id: first.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start first session");

        let switched = switch_task(
            &conn,
            SwitchTaskInput {
                task_id: second.id.clone(),
                progress_note: Some("Paused first".to_string()),
                next_action: Some("Return to first".to_string()),
                destination_status: None,
                lap_duration_seconds: Some(60),
            },
        )
        .expect("switch task");

        assert_eq!(switched.task.id, second.id);
        assert_eq!(switched.task.status, TASK_STATUS_ACTIVE);
        assert_eq!(
            require_task(&conn, &first.id).expect("read first").status,
            TASK_STATUS_PICKUP
        );
        let ended_original = persistence::get_session(&conn, &original.session.id)
            .expect("read original")
            .expect("original session");
        assert_eq!(
            ended_original.end_reason.as_deref(),
            Some(END_REASON_SWITCHED)
        );
        assert!(ended_original.duration_seconds.is_some());
        assert_eq!(
            get_active_session(&conn)
                .expect("read active")
                .expect("active after switch")
                .session
                .id,
            switched.session.id
        );
    }

    #[test]
    fn switch_task_rejects_long_term_without_required_notes() {
        let conn = in_memory_db();
        let first = create_task(&conn, create_input(TASK_KIND_LONG_TERM)).expect("first task");
        let second = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("second task");
        let original = start_session(
            &conn,
            StartSessionInput {
                task_id: first.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start long-term session");

        let error = switch_task(
            &conn,
            SwitchTaskInput {
                task_id: second.id.clone(),
                progress_note: Some("Mapped the next work".to_string()),
                next_action: None,
                destination_status: None,
                lap_duration_seconds: Some(60),
            },
        )
        .expect_err("long-term switch requires notes");

        assert_eq!(error.code, "validation");
        assert!(error.message.contains("next action"));
        let active = get_active_session(&conn)
            .expect("read active")
            .expect("session remains active");
        assert_eq!(active.session.id, original.session.id);
        assert_eq!(active.task.id, first.id);
        assert_eq!(
            require_task(&conn, &second.id).expect("read target").status,
            TASK_STATUS_PICKUP
        );
    }

    #[test]
    fn recovery_resume_marks_session_and_keeps_task_active() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start session");

        let result = resolve_session_recovery(
            &conn,
            ResolveSessionRecoveryInput {
                action: RECOVERY_ACTION_RESUME.to_string(),
                session_id: Some(active.session.id.clone()),
                progress_note: None,
                next_action: None,
            },
        )
        .expect("resume session");

        let resumed = result.active_session.expect("active session returned");
        assert_eq!(resumed.session.id, active.session.id);
        assert!(resumed.session.recovered_from_crash);
        assert_eq!(resumed.session.ended_at, None);
        assert_eq!(resumed.task.status, TASK_STATUS_ACTIVE);
    }

    #[test]
    fn pending_recovery_only_reports_session_present_at_startup() {
        let conn = in_memory_db();
        let pending_before_session =
            pending_recovery_from_startup(&conn).expect("read startup recovery");
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start session after startup");

        assert!(get_pending_session_recovery(&conn, &pending_before_session)
            .expect("read pending recovery")
            .active_session
            .is_none());

        let pending_with_session =
            pending_recovery_from_startup(&conn).expect("read startup recovery");
        assert_eq!(
            get_pending_session_recovery(&conn, &pending_with_session)
                .expect("read pending recovery")
                .active_session
                .expect("startup active session")
                .session
                .id,
            active.session.id
        );

        clear_pending_session_recovery(&pending_with_session).expect("clear pending recovery");
        assert!(get_pending_session_recovery(&conn, &pending_with_session)
            .expect("read cleared pending recovery")
            .active_session
            .is_none());
    }

    #[test]
    fn recovery_stop_uses_app_closed_and_updates_task() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start session");

        let result = resolve_session_recovery(
            &conn,
            ResolveSessionRecoveryInput {
                action: RECOVERY_ACTION_STOP.to_string(),
                session_id: Some(active.session.id),
                progress_note: Some("Recovered after restart".to_string()),
                next_action: Some("Continue the draft".to_string()),
            },
        )
        .expect("stop recovered session");
        let session = result.session.expect("stopped session returned");
        let task = result.task.expect("updated task returned");

        assert_eq!(session.end_reason.as_deref(), Some(END_REASON_APP_CLOSED));
        assert_eq!(
            session.progress_note.as_deref(),
            Some("Recovered after restart")
        );
        assert_eq!(task.status, TASK_STATUS_PICKUP);
        assert_eq!(task.next_action.as_deref(), Some("Continue the draft"));
        assert!(get_active_session(&conn).expect("read active").is_none());
    }

    #[test]
    fn recovery_discard_excludes_session_from_recent_threads() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start session");

        let result = resolve_session_recovery(
            &conn,
            ResolveSessionRecoveryInput {
                action: RECOVERY_ACTION_DISCARD.to_string(),
                session_id: Some(active.session.id.clone()),
                progress_note: Some("Ignore accidental open".to_string()),
                next_action: None,
            },
        )
        .expect("discard recovered session");
        let session = result.session.expect("discarded session returned");
        let task = result.task.expect("updated task returned");

        assert_eq!(session.end_reason.as_deref(), Some(END_REASON_DISCARDED));
        assert_eq!(session.duration_seconds, None);
        assert_eq!(task.status, TASK_STATUS_PICKUP);
        assert_eq!(
            require_task(&conn, &active.task.id)
                .expect("read task after discard")
                .status,
            TASK_STATUS_PICKUP
        );
        assert!(get_active_session(&conn).expect("read active").is_none());
        assert!(!list_recent_threads(&conn, 10)
            .expect("list recent threads")
            .iter()
            .any(|thread| thread.session.id == active.session.id));
    }

    #[test]
    fn long_term_stop_requires_progress_note() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_LONG_TERM)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start long-term session");

        let error = stop_session(
            &conn,
            StopSessionInput {
                session_id: None,
                progress_note: None,
                next_action: Some("Plan the next slice".to_string()),
                destination_status: None,
            },
            END_REASON_STOPPED,
        )
        .expect_err("progress note required");

        assert_eq!(error.code, "validation");
        assert!(error.message.contains("progress note"));
        assert_eq!(
            get_active_session(&conn)
                .expect("read active")
                .expect("still active")
                .task
                .id,
            task.id
        );
    }

    #[test]
    fn long_term_stop_requires_next_action() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_LONG_TERM)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start long-term session");

        let error = stop_session(
            &conn,
            StopSessionInput {
                session_id: None,
                progress_note: Some("Mapped the next phase".to_string()),
                next_action: None,
                destination_status: None,
            },
            END_REASON_STOPPED,
        )
        .expect_err("next action required");

        assert_eq!(error.code, "validation");
        assert!(error.message.contains("next action"));
        assert_eq!(
            get_active_session(&conn)
                .expect("read active")
                .expect("still active")
                .task
                .id,
            task.id
        );
    }

    #[test]
    fn long_term_stop_returns_to_backlog_or_pickup_only() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_LONG_TERM)).expect("create task");
        start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start long-term session");

        let error = stop_session(
            &conn,
            StopSessionInput {
                session_id: None,
                progress_note: Some("Made progress".to_string()),
                next_action: Some("Continue tomorrow".to_string()),
                destination_status: Some(TASK_STATUS_COMPLETED.to_string()),
            },
            END_REASON_STOPPED,
        )
        .expect_err("invalid long-term stop destination");
        assert_eq!(error.code, "validation");
        assert!(error.message.contains("backlog or pickup"));

        let stopped = stop_session(
            &conn,
            StopSessionInput {
                session_id: None,
                progress_note: Some("Made progress".to_string()),
                next_action: Some("Continue tomorrow".to_string()),
                destination_status: None,
            },
            END_REASON_STOPPED,
        )
        .expect("stop to backlog");

        assert_eq!(stopped.task.status, TASK_STATUS_BACKLOG);
    }

    #[test]
    fn completing_long_term_requires_explicit_confirmation() {
        let conn = in_memory_db();
        let task = create_task(&conn, create_input(TASK_KIND_LONG_TERM)).expect("create task");
        let active = start_session(
            &conn,
            StartSessionInput {
                task_id: task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start long-term session");

        let error = complete_session(
            &conn,
            CompleteSessionInput {
                session_id: Some(active.session.id.clone()),
                progress_note: Some("All phases finished".to_string()),
                next_action: None,
                confirm_long_term_completion: None,
            },
        )
        .expect_err("confirmation required");
        assert_eq!(error.code, "validation");
        assert!(error.message.contains("explicit confirmation"));

        let completed = complete_session(
            &conn,
            CompleteSessionInput {
                session_id: Some(active.session.id),
                progress_note: Some("All phases finished".to_string()),
                next_action: None,
                confirm_long_term_completion: Some(true),
            },
        )
        .expect("complete long-term task");

        assert_eq!(completed.task.status, TASK_STATUS_COMPLETED);
        assert!(completed.task.completed_at.is_some());
    }

    #[test]
    fn switch_task_rejects_invalid_lap_duration_without_ending_active_session() {
        let conn = in_memory_db();
        let active_task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("active task");
        let target_task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("target task");
        let original_session = start_session(
            &conn,
            StartSessionInput {
                task_id: active_task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start session");

        let error = switch_task(
            &conn,
            SwitchTaskInput {
                task_id: target_task.id.clone(),
                progress_note: None,
                next_action: None,
                destination_status: None,
                lap_duration_seconds: Some(5),
            },
        )
        .expect_err("invalid lap duration");

        assert_eq!(error.code, "validation");
        assert!(error.message.contains("Donut lap duration"));

        let active_session = get_active_session(&conn)
            .expect("read active session")
            .expect("active session remains open");
        assert_eq!(active_session.session.id, original_session.session.id);
        assert_eq!(active_session.task.id, active_task.id);
        assert_eq!(active_session.task.status, TASK_STATUS_ACTIVE);
        assert_eq!(
            require_task(&conn, &target_task.id)
                .expect("read target task")
                .status,
            TASK_STATUS_PICKUP
        );
    }

    #[test]
    fn database_rejects_multiple_open_sessions() {
        let conn = in_memory_db();
        let active_task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("active task");
        let second_task = create_task(&conn, create_input(TASK_KIND_PICKUP)).expect("second task");
        let original_session = start_session(
            &conn,
            StartSessionInput {
                task_id: active_task.id.clone(),
                lap_duration_seconds: Some(60),
            },
        )
        .expect("start session");
        let now = utc_now(&conn).expect("read database time");
        let second_session = DbSession {
            id: "session-second-open".to_string(),
            task_id: second_task.id,
            started_at: now.clone(),
            ended_at: None,
            duration_seconds: None,
            end_reason: None,
            progress_note: None,
            next_action: None,
            lap_duration_seconds: 60,
            recovered_from_crash: false,
            created_at: now.clone(),
            updated_at: now,
        };

        let error = persistence::insert_session(&conn, &second_session)
            .expect_err("single active session invariant");

        assert!(error.to_string().contains("idx_sessions_one_active"));
        assert_eq!(
            get_active_session(&conn)
                .expect("read active session")
                .expect("active session remains")
                .session
                .id,
            original_session.session.id
        );
    }

    #[test]
    fn update_settings_validates_all_fields_before_writing() {
        let conn = in_memory_db();

        let error = update_settings(
            &conn,
            UpdateSettingsInput {
                today_on_startup: Some(false),
                donut_lap_duration_seconds: Some(5),
                ..UpdateSettingsInput::default()
            },
        )
        .expect_err("invalid settings update");

        assert_eq!(error.code, "validation");
        assert!(error.message.contains("Donut lap duration"));
        assert!(get_settings(&conn).expect("read settings").today_on_startup);
    }

    #[test]
    fn update_settings_persists_startup_display_lap_theme_and_position() {
        let conn = in_memory_db();

        let settings = update_settings(
            &conn,
            UpdateSettingsInput {
                launch_on_startup: Some(true),
                today_on_startup: Some(false),
                donut_lap_duration_seconds: Some(120),
                theme: Some("notion_light".to_string()),
                floating_window_position: Some(FloatingWindowPosition { x: 320, y: 180 }),
                ..UpdateSettingsInput::default()
            },
        )
        .expect("update settings");

        assert!(settings.launch_on_startup);
        assert!(!settings.today_on_startup);
        assert_eq!(settings.donut_lap_duration_seconds, 120);
        assert_eq!(settings.theme, "notion_light");
        assert_eq!(
            settings.floating_window_position,
            FloatingWindowPosition { x: 320, y: 180 }
        );
        assert_eq!(
            get_settings(&conn).expect("read persisted settings"),
            settings
        );
    }

    #[test]
    fn export_database_writes_inside_app_export_dir_and_rejects_overwrite() {
        let dir = unique_temp_dir("thread-command-export");
        fs::create_dir_all(&dir).expect("create temp dir");
        let source_path = dir.join(persistence::DATABASE_FILE_NAME);
        fs::write(&source_path, b"database bytes").expect("write source database");
        let export_dir = dir.join(EXPORT_DIR_NAME);

        let result = export_database_to_app_dir(
            &source_path,
            &export_dir,
            Some("backup.sqlite3".to_string()),
        )
        .expect("export database");
        let destination_path = PathBuf::from(result.path);

        assert!(destination_path
            .starts_with(fs::canonicalize(&export_dir).expect("canonicalize export dir")));
        assert_eq!(
            fs::read(&destination_path).expect("read exported database"),
            b"database bytes"
        );

        let error = export_database_to_app_dir(
            &source_path,
            &export_dir,
            Some("backup.sqlite3".to_string()),
        )
        .expect_err("reject overwrite");
        assert_eq!(error.code, "validation");
        assert!(error.message.contains("already exists"));

        fs::remove_dir_all(dir).expect("remove temp dir");
    }

    #[test]
    fn export_database_rejects_renderer_supplied_paths() {
        let error = export_database_to_app_dir(
            Path::new("thread.sqlite3"),
            Path::new("exports"),
            Some("../backup.sqlite3".to_string()),
        )
        .expect_err("reject path traversal");

        assert_eq!(error.code, "validation");
        assert!(error.message.contains("file name"));
    }

    #[test]
    fn get_settings_returns_typed_defaults() {
        let conn = in_memory_db();

        let settings = get_settings(&conn).expect("read settings");

        assert!(!settings.launch_on_startup);
        assert!(settings.today_on_startup);
        assert_eq!(settings.donut_lap_duration_seconds, 60);
        assert_eq!(settings.theme, "notion_light");
        assert!(settings.floating_window_always_on_top);
        assert_eq!(
            settings.floating_window_position,
            FloatingWindowPosition { x: 24, y: 24 }
        );
        assert_eq!(
            settings.long_term_stop_requirements,
            "require_progress_note_and_next_action"
        );
        assert_eq!(
            settings.today_return_behavior,
            "return_to_today_after_session"
        );
    }
}
