use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub task_id: i64,
    pub start_time: i64,
    pub end_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithStats {
    #[serde(flatten)]
    pub task: Task,
    pub total_seconds: i64,
    pub active_session: Option<Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    pub task: Task,
    pub sessions: Vec<Session>,
    pub total_seconds: i64,
    pub active_session: Option<Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEntry {
    pub task_id: i64,
    pub task_name: String,
    pub total_seconds: i64,
}
