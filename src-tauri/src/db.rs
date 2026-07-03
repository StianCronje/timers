use crate::models::{ReportEntry, Session, Task, TaskDetail, TaskWithStats};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Db {
    conn: Mutex<Connection>,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("not found")]
    NotFound,
}

impl serde::Serialize for DbError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type DbResult<T> = Result<T, DbError>;

impl Db {
    pub fn open(path: &Path) -> DbResult<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Db {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> DbResult<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                start_time INTEGER NOT NULL,
                end_time INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_task ON sessions(task_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(end_time) WHERE end_time IS NULL;
            CREATE INDEX IF NOT EXISTS idx_sessions_range ON sessions(start_time);
            "#,
        )?;
        Ok(())
    }

    fn now() -> i64 {
        Utc::now().timestamp()
    }

    fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            created_at: row.get(3)?,
            archived: row.get::<_, i64>(4)? != 0,
        })
    }

    fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
        Ok(Session {
            id: row.get(0)?,
            task_id: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
        })
    }

    pub fn create_task(&self, name: &str, description: Option<&str>) -> DbResult<Task> {
        let conn = self.conn.lock();
        let now = Self::now();
        conn.execute(
            "INSERT INTO tasks (name, description, created_at, archived) VALUES (?1, ?2, ?3, 0)",
            params![name, description, now],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Task {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            created_at: now,
            archived: false,
        })
    }

    pub fn update_task(
        &self,
        id: i64,
        name: &str,
        description: Option<&str>,
    ) -> DbResult<Task> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE tasks SET name = ?1, description = ?2 WHERE id = ?3",
            params![name, description, id],
        )?;
        if changed == 0 {
            return Err(DbError::NotFound);
        }
        let task = conn
            .query_row(
                "SELECT id, name, description, created_at, archived FROM tasks WHERE id = ?1",
                params![id],
                Self::row_to_task,
            )
            .optional()?
            .ok_or(DbError::NotFound)?;
        Ok(task)
    }

    pub fn set_archived(&self, id: i64, archived: bool) -> DbResult<()> {
        let mut conn = self.conn.lock();
        let v: i64 = if archived { 1 } else { 0 };
        let now = Self::now();
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE tasks SET archived = ?1 WHERE id = ?2",
            params![v, id],
        )?;
        if changed == 0 {
            return Err(DbError::NotFound);
        }
        if archived {
            tx.execute(
                "UPDATE sessions SET end_time = ?1 WHERE task_id = ?2 AND end_time IS NULL",
                params![now, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> DbResult<()> {
        let conn = self.conn.lock();
        let changed = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub fn list_tasks(&self, include_archived: bool) -> DbResult<Vec<TaskWithStats>> {
        let conn = self.conn.lock();
        let sql = if include_archived {
            "SELECT id, name, description, created_at, archived FROM tasks ORDER BY archived ASC, created_at DESC"
        } else {
            "SELECT id, name, description, created_at, archived FROM tasks WHERE archived = 0 ORDER BY created_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let tasks: Vec<Task> = stmt
            .query_map([], Self::row_to_task)?
            .collect::<Result<_, _>>()?;
        drop(stmt);

        let now = Self::now();
        let mut out = Vec::with_capacity(tasks.len());
        for task in tasks {
            let total_seconds = Self::total_seconds_for_task(&conn, task.id, now)?;
            let active_session = Self::active_session_for_task(&conn, task.id)?;
            out.push(TaskWithStats {
                task,
                total_seconds,
                active_session,
            });
        }
        Ok(out)
    }

    pub fn get_task_detail(&self, id: i64) -> DbResult<TaskDetail> {
        let conn = self.conn.lock();
        let task = conn
            .query_row(
                "SELECT id, name, description, created_at, archived FROM tasks WHERE id = ?1",
                params![id],
                Self::row_to_task,
            )
            .optional()?
            .ok_or(DbError::NotFound)?;

        let mut stmt = conn.prepare(
            "SELECT id, task_id, start_time, end_time FROM sessions WHERE task_id = ?1 ORDER BY start_time DESC",
        )?;
        let sessions: Vec<Session> = stmt
            .query_map(params![id], Self::row_to_session)?
            .collect::<Result<_, _>>()?;
        drop(stmt);

        let now = Self::now();
        let total_seconds = Self::total_seconds_for_task(&conn, id, now)?;
        let active_session = sessions.iter().find(|s| s.end_time.is_none()).cloned();

        Ok(TaskDetail {
            task,
            sessions,
            total_seconds,
            active_session,
        })
    }

    fn total_seconds_for_task(
        conn: &Connection,
        task_id: i64,
        now: i64,
    ) -> DbResult<i64> {
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(COALESCE(end_time, ?1) - start_time), 0) FROM sessions WHERE task_id = ?2",
            params![now, task_id],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    fn active_session_for_task(
        conn: &Connection,
        task_id: i64,
    ) -> DbResult<Option<Session>> {
        let s = conn
            .query_row(
                "SELECT id, task_id, start_time, end_time FROM sessions WHERE task_id = ?1 AND end_time IS NULL LIMIT 1",
                params![task_id],
                Self::row_to_session,
            )
            .optional()?;
        Ok(s)
    }

    pub fn get_active_session(&self) -> DbResult<Option<(Task, Session)>> {
        let conn = self.conn.lock();
        let row = conn
            .query_row(
                "SELECT s.id, s.task_id, s.start_time, s.end_time, t.id, t.name, t.description, t.created_at, t.archived
                 FROM sessions s JOIN tasks t ON t.id = s.task_id
                 WHERE s.end_time IS NULL ORDER BY s.start_time DESC LIMIT 1",
                [],
                |row| {
                    let session = Session {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        start_time: row.get(2)?,
                        end_time: row.get(3)?,
                    };
                    let task = Task {
                        id: row.get(4)?,
                        name: row.get(5)?,
                        description: row.get(6)?,
                        created_at: row.get(7)?,
                        archived: row.get::<_, i64>(8)? != 0,
                    };
                    Ok((task, session))
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Start a timer for a task. Stops any other active timer first.
    pub fn start_timer(&self, task_id: i64) -> DbResult<Session> {
        let mut conn = self.conn.lock();
        let now = Self::now();
        let tx = conn.transaction()?;

        // Check the task exists.
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(DbError::NotFound);
        }

        // If there's already an active timer for this task, return it.
        let existing: Option<Session> = tx
            .query_row(
                "SELECT id, task_id, start_time, end_time FROM sessions WHERE task_id = ?1 AND end_time IS NULL LIMIT 1",
                params![task_id],
                Self::row_to_session,
            )
            .optional()?;
        if let Some(s) = existing {
            tx.commit()?;
            return Ok(s);
        }

        // Stop any other active sessions.
        tx.execute(
            "UPDATE sessions SET end_time = ?1 WHERE end_time IS NULL",
            params![now],
        )?;

        tx.execute(
            "INSERT INTO sessions (task_id, start_time, end_time) VALUES (?1, ?2, NULL)",
            params![task_id, now],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(Session {
            id,
            task_id,
            start_time: now,
            end_time: None,
        })
    }

    pub fn stop_active_timer(&self) -> DbResult<Option<Session>> {
        let mut conn = self.conn.lock();
        let now = Self::now();
        let tx = conn.transaction()?;
        let active: Option<Session> = tx
            .query_row(
                "SELECT id, task_id, start_time, end_time FROM sessions WHERE end_time IS NULL ORDER BY start_time DESC LIMIT 1",
                [],
                Self::row_to_session,
            )
            .optional()?;
        let Some(mut session) = active else {
            tx.commit()?;
            return Ok(None);
        };
        tx.execute(
            "UPDATE sessions SET end_time = ?1 WHERE id = ?2",
            params![now, session.id],
        )?;
        tx.commit()?;
        session.end_time = Some(now);
        Ok(Some(session))
    }

    pub fn report(&self, range_start: i64, range_end: i64) -> DbResult<Vec<ReportEntry>> {
        let conn = self.conn.lock();
        let now = Self::now();
        let mut stmt = conn.prepare(
            r#"
            SELECT
                t.id,
                t.name,
                COALESCE(SUM(
                    MIN(COALESCE(s.end_time, ?3), ?2) - MAX(s.start_time, ?1)
                ), 0) AS total
            FROM tasks t
            JOIN sessions s ON s.task_id = t.id
            WHERE
                s.start_time < ?2
                AND COALESCE(s.end_time, ?3) > ?1
            GROUP BY t.id, t.name
            HAVING total > 0
            ORDER BY total DESC
            "#,
        )?;
        let rows: Vec<ReportEntry> = stmt
            .query_map(params![range_start, range_end, now], |row| {
                Ok(ReportEntry {
                    task_id: row.get(0)?,
                    task_name: row.get(1)?,
                    total_seconds: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}
