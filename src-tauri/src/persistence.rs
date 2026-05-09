use rusqlite::{params, Connection, OptionalExtension, Row};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::Manager;

pub const DATABASE_FILE_NAME: &str = "thread.sqlite3";

pub type DbResult<T> = Result<T, Box<dyn std::error::Error>>;

pub const DEFAULT_SETTINGS: &[(&str, &str)] = &[
    ("startup.launch", "false"),
    ("startup.today_on_startup", "true"),
    ("donut.lap_duration_seconds", "60"),
    ("theme", "notion_light"),
    ("floating_window.always_on_top", "true"),
    ("floating_window.position", r#"{"x":24,"y":24}"#),
    (
        "long_term.stop_requirements",
        "require_progress_note_and_next_action",
    ),
    ("today.return_behavior", "return_to_today_after_session"),
];

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_core_persistence_tables",
        sql: r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            priority INTEGER NOT NULL,
            pickup_date TEXT,
            next_action TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            archived_at TEXT
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            task_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            duration_seconds INTEGER,
            end_reason TEXT,
            progress_note TEXT,
            next_action TEXT,
            lap_duration_seconds INTEGER NOT NULL DEFAULT 60,
            recovered_from_crash INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS task_events (
            id TEXT PRIMARY KEY NOT NULL,
            task_id TEXT NOT NULL,
            session_id TEXT,
            event_type TEXT NOT NULL,
            from_status TEXT,
            to_status TEXT,
            note TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_status_sort_order
            ON tasks(status, sort_order);
        CREATE INDEX IF NOT EXISTS idx_sessions_task_id_started_at
            ON sessions(task_id, started_at);
        CREATE INDEX IF NOT EXISTS idx_task_events_task_id_created_at
            ON task_events(task_id, created_at);
    "#,
    },
    Migration {
        version: 2,
        name: "enforce_single_active_session",
        sql: r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_one_active
            ON sessions((1))
            WHERE ended_at IS NULL;
    "#,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEvent {
    pub id: String,
    pub task_id: String,
    pub session_id: Option<String>,
    pub event_type: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

pub fn app_database_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> DbResult<PathBuf> {
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir.join(DATABASE_FILE_NAME))
}

pub fn open_app_database<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> DbResult<Connection> {
    open_database_at(app_database_path(app)?)
}

pub fn open_database_at(path: impl AsRef<Path>) -> DbResult<Connection> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut conn = Connection::open(path)?;
    initialize_database(&mut conn)?;
    Ok(conn)
}

pub fn initialize_database(conn: &mut Connection) -> DbResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    run_migrations(conn)?;
    insert_default_settings(conn)?;
    Ok(())
}

pub fn run_migrations(conn: &mut Connection) -> DbResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );
    "#,
    )?;

    let tx = conn.transaction()?;

    for migration in MIGRATIONS {
        let already_applied = tx
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1",
                params![migration.version],
                |_| Ok(()),
            )
            .optional()?
            .is_some();

        if already_applied {
            continue;
        }

        tx.execute_batch(migration.sql)?;
        tx.execute(
            r#"
            INSERT INTO schema_migrations (version, name, applied_at)
            VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            "#,
            params![migration.version, migration.name],
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn insert_default_settings(conn: &Connection) -> DbResult<()> {
    for (key, value) in DEFAULT_SETTINGS {
        conn.execute(
            r#"
            INSERT OR IGNORE INTO settings (key, value, updated_at)
            VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            "#,
            params![key, value],
        )?;
    }

    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> DbResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> DbResult<()> {
    conn.execute(
        r#"
        INSERT INTO settings (key, value, updated_at)
        VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
        params![key, value],
    )?;

    Ok(())
}

pub fn insert_task(conn: &Connection, task: &Task) -> DbResult<()> {
    conn.execute(
        r#"
        INSERT INTO tasks (
            id, title, description, kind, status, priority, pickup_date,
            next_action, sort_order, created_at, updated_at, completed_at,
            archived_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            &task.id,
            &task.title,
            &task.description,
            &task.kind,
            &task.status,
            task.priority,
            task.pickup_date.as_deref(),
            task.next_action.as_deref(),
            task.sort_order,
            &task.created_at,
            &task.updated_at,
            task.completed_at.as_deref(),
            task.archived_at.as_deref(),
        ],
    )?;

    Ok(())
}

pub fn get_task(conn: &Connection, id: &str) -> DbResult<Option<Task>> {
    Ok(conn
        .query_row(
            r#"
            SELECT
                id, title, description, kind, status, priority, pickup_date,
                next_action, sort_order, created_at, updated_at, completed_at,
                archived_at
            FROM tasks
            WHERE id = ?1
            "#,
            params![id],
            map_task,
        )
        .optional()?)
}

pub fn update_task(conn: &Connection, task: &Task) -> DbResult<()> {
    conn.execute(
        r#"
        UPDATE tasks
        SET
            title = ?2,
            description = ?3,
            kind = ?4,
            status = ?5,
            priority = ?6,
            pickup_date = ?7,
            next_action = ?8,
            sort_order = ?9,
            created_at = ?10,
            updated_at = ?11,
            completed_at = ?12,
            archived_at = ?13
        WHERE id = ?1
        "#,
        params![
            &task.id,
            &task.title,
            &task.description,
            &task.kind,
            &task.status,
            task.priority,
            task.pickup_date.as_deref(),
            task.next_action.as_deref(),
            task.sort_order,
            &task.created_at,
            &task.updated_at,
            task.completed_at.as_deref(),
            task.archived_at.as_deref(),
        ],
    )?;

    Ok(())
}

pub fn delete_task(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn insert_session(conn: &Connection, session: &Session) -> DbResult<()> {
    conn.execute(
        r#"
        INSERT INTO sessions (
            id, task_id, started_at, ended_at, duration_seconds, end_reason,
            progress_note, next_action, lap_duration_seconds,
            recovered_from_crash, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params![
            &session.id,
            &session.task_id,
            &session.started_at,
            session.ended_at.as_deref(),
            session.duration_seconds,
            session.end_reason.as_deref(),
            session.progress_note.as_deref(),
            session.next_action.as_deref(),
            session.lap_duration_seconds,
            bool_as_i64(session.recovered_from_crash),
            &session.created_at,
            &session.updated_at,
        ],
    )?;

    Ok(())
}

pub fn get_session(conn: &Connection, id: &str) -> DbResult<Option<Session>> {
    Ok(conn
        .query_row(
            r#"
            SELECT
                id, task_id, started_at, ended_at, duration_seconds, end_reason,
                progress_note, next_action, lap_duration_seconds,
                recovered_from_crash, created_at, updated_at
            FROM sessions
            WHERE id = ?1
            "#,
            params![id],
            map_session,
        )
        .optional()?)
}

pub fn update_session(conn: &Connection, session: &Session) -> DbResult<()> {
    conn.execute(
        r#"
        UPDATE sessions
        SET
            task_id = ?2,
            started_at = ?3,
            ended_at = ?4,
            duration_seconds = ?5,
            end_reason = ?6,
            progress_note = ?7,
            next_action = ?8,
            lap_duration_seconds = ?9,
            recovered_from_crash = ?10,
            created_at = ?11,
            updated_at = ?12
        WHERE id = ?1
        "#,
        params![
            &session.id,
            &session.task_id,
            &session.started_at,
            session.ended_at.as_deref(),
            session.duration_seconds,
            session.end_reason.as_deref(),
            session.progress_note.as_deref(),
            session.next_action.as_deref(),
            session.lap_duration_seconds,
            bool_as_i64(session.recovered_from_crash),
            &session.created_at,
            &session.updated_at,
        ],
    )?;

    Ok(())
}

pub fn delete_session(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn insert_task_event(conn: &Connection, event: &TaskEvent) -> DbResult<()> {
    conn.execute(
        r#"
        INSERT INTO task_events (
            id, task_id, session_id, event_type, from_status, to_status, note,
            created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            &event.id,
            &event.task_id,
            event.session_id.as_deref(),
            &event.event_type,
            event.from_status.as_deref(),
            event.to_status.as_deref(),
            event.note.as_deref(),
            &event.created_at,
        ],
    )?;

    Ok(())
}

pub fn list_task_events(conn: &Connection, task_id: &str) -> DbResult<Vec<TaskEvent>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            id, task_id, session_id, event_type, from_status, to_status, note,
            created_at
        FROM task_events
        WHERE task_id = ?1
        ORDER BY created_at ASC
        "#,
    )?;

    let events = stmt
        .query_map(params![task_id], map_task_event)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(events)
}

fn map_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        kind: row.get(3)?,
        status: row.get(4)?,
        priority: row.get(5)?,
        pickup_date: row.get(6)?,
        next_action: row.get(7)?,
        sort_order: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        completed_at: row.get(11)?,
        archived_at: row.get(12)?,
    })
}

fn map_session(row: &Row<'_>) -> rusqlite::Result<Session> {
    let recovered_from_crash: i64 = row.get(9)?;

    Ok(Session {
        id: row.get(0)?,
        task_id: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        duration_seconds: row.get(4)?,
        end_reason: row.get(5)?,
        progress_note: row.get(6)?,
        next_action: row.get(7)?,
        lap_duration_seconds: row.get(8)?,
        recovered_from_crash: recovered_from_crash != 0,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn map_task_event(row: &Row<'_>) -> rusqlite::Result<TaskEvent> {
    Ok(TaskEvent {
        id: row.get(0)?,
        task_id: row.get(1)?,
        session_id: row.get(2)?,
        event_type: row.get(3)?,
        from_status: row.get(4)?,
        to_status: row.get(5)?,
        note: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn bool_as_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    const NOW: &str = "2026-05-09T12:00:00Z";

    fn in_memory_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory sqlite database");
        initialize_database(&mut conn).expect("initialize database");
        conn
    }

    fn sample_task() -> Task {
        Task {
            id: "task-1".to_string(),
            title: "Draft persistence layer".to_string(),
            description: "Create migrations and CRUD helpers".to_string(),
            kind: "today".to_string(),
            status: "planned".to_string(),
            priority: 2,
            pickup_date: Some("2026-05-09".to_string()),
            next_action: Some("Write Rust tests".to_string()),
            sort_order: 10,
            created_at: NOW.to_string(),
            updated_at: NOW.to_string(),
            completed_at: None,
            archived_at: None,
        }
    }

    fn sample_session() -> Session {
        Session {
            id: "session-1".to_string(),
            task_id: "task-1".to_string(),
            started_at: "2026-05-09T12:10:00Z".to_string(),
            ended_at: None,
            duration_seconds: None,
            end_reason: None,
            progress_note: Some("Started schema work".to_string()),
            next_action: Some("Add CRUD coverage".to_string()),
            lap_duration_seconds: 60,
            recovered_from_crash: false,
            created_at: NOW.to_string(),
            updated_at: NOW.to_string(),
        }
    }

    #[test]
    fn open_database_at_creates_and_reopens_database_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("thread-db-test-{unique}"));
        let db_path = dir.join(DATABASE_FILE_NAME);

        let conn = open_database_at(&db_path).expect("open database at temp path");
        assert!(db_path.exists());
        assert_eq!(
            get_setting(&conn, "donut.lap_duration_seconds").expect("read default setting"),
            Some("60".to_string())
        );
        drop(conn);

        let conn = open_database_at(&db_path).expect("reopen existing database");
        assert_eq!(
            get_setting(&conn, "theme").expect("read default theme"),
            Some("notion_light".to_string())
        );
        drop(conn);

        fs::remove_dir_all(dir).expect("remove temp database directory");
    }

    #[test]
    fn migrations_create_expected_tables_and_default_settings() {
        let mut conn = Connection::open_in_memory().expect("open in-memory sqlite database");

        initialize_database(&mut conn).expect("first migration run");
        initialize_database(&mut conn).expect("second migration run is idempotent");

        let mut stmt = conn
            .prepare(
                r#"
                SELECT name
                FROM sqlite_master
                WHERE type = 'table'
                    AND name IN ('tasks', 'sessions', 'task_events', 'settings')
                "#,
            )
            .expect("prepare table lookup");
        let tables = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query tables")
            .collect::<Result<HashSet<_>, _>>()
            .expect("collect tables");

        assert_eq!(
            tables,
            HashSet::from([
                "tasks".to_string(),
                "sessions".to_string(),
                "task_events".to_string(),
                "settings".to_string(),
            ])
        );

        for (key, expected_value) in DEFAULT_SETTINGS {
            assert_eq!(
                get_setting(&conn, key).expect("read default setting"),
                Some((*expected_value).to_string())
            );
        }

        set_setting(&conn, "theme", "custom").expect("override setting");
        insert_default_settings(&conn).expect("reinsert defaults");
        assert_eq!(
            get_setting(&conn, "theme").expect("read overridden setting"),
            Some("custom".to_string())
        );

        let setting_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .expect("count settings");
        assert_eq!(setting_count, DEFAULT_SETTINGS.len() as i64);
    }

    #[test]
    fn task_session_and_event_rows_round_trip() {
        let conn = in_memory_db();
        let mut task = sample_task();

        insert_task(&conn, &task).expect("insert task");
        assert_eq!(
            get_task(&conn, "task-1").expect("read task"),
            Some(task.clone())
        );

        task.status = "active".to_string();
        task.next_action = Some("Run cargo test".to_string());
        task.updated_at = "2026-05-09T12:15:00Z".to_string();
        update_task(&conn, &task).expect("update task");
        assert_eq!(
            get_task(&conn, "task-1").expect("read updated task"),
            Some(task.clone())
        );

        let mut session = sample_session();
        insert_session(&conn, &session).expect("insert session");
        assert_eq!(
            get_session(&conn, "session-1").expect("read session"),
            Some(session.clone())
        );

        session.ended_at = Some("2026-05-09T12:30:00Z".to_string());
        session.duration_seconds = Some(1200);
        session.end_reason = Some("completed".to_string());
        session.progress_note = Some("CRUD tests passed locally".to_string());
        session.updated_at = "2026-05-09T12:30:00Z".to_string();
        update_session(&conn, &session).expect("update session");
        assert_eq!(
            get_session(&conn, "session-1").expect("read updated session"),
            Some(session.clone())
        );

        let event = TaskEvent {
            id: "event-1".to_string(),
            task_id: task.id.clone(),
            session_id: Some(session.id.clone()),
            event_type: "status_changed".to_string(),
            from_status: Some("planned".to_string()),
            to_status: Some("active".to_string()),
            note: Some("Started work".to_string()),
            created_at: "2026-05-09T12:16:00Z".to_string(),
        };

        insert_task_event(&conn, &event).expect("insert task event");
        assert_eq!(
            list_task_events(&conn, "task-1").expect("list task events"),
            vec![event]
        );

        delete_session(&conn, "session-1").expect("delete session");
        assert_eq!(
            get_session(&conn, "session-1").expect("read deleted session"),
            None
        );

        delete_task(&conn, "task-1").expect("delete task");
        assert_eq!(get_task(&conn, "task-1").expect("read deleted task"), None);
    }
}
