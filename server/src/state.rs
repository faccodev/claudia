use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex as TokioMutex;
use serde::{Deserialize, Serialize};
use anyhow::Result;

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub session_store: SessionStore,
    pub process_registry: ProcessRegistry,
    pub claude_dir: PathBuf,
    pub data_dir: PathBuf,
}

// Session store for authenticated sessions
pub struct SessionStore {
    sessions: TokioMutex<HashMap<String, SessionData>>,
}

#[derive(Clone, Debug)]
pub struct SessionData {
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// Process registry (simplified version)
pub struct ProcessRegistry {
    processes: Arc<TokioMutex<HashMap<i64, ProcessInfo>>>,
    next_id: Arc<TokioMutex<i64>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub run_id: i64,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub project_path: String,
    pub task: String,
    pub model: String,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: TokioMutex::new(HashMap::new()),
        }
    }

    pub async fn create_session(&self, user_id: &str) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(24);

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), SessionData {
            user_id: user_id.to_string(),
            created_at: now,
            expires_at,
        });

        session_id
    }

    pub async fn validate_session(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).map(|data| {
            // Check if expired
            if data.expires_at > Utc::now() {
                data.user_id.clone()
            } else {
                String::new()
            }
        }).filter(|user| !user.is_empty())
    }

    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id);
    }
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(TokioMutex::new(HashMap::new())),
            next_id: Arc::new(TokioMutex::new(1000000)),
        }
    }

    pub async fn generate_id(&self) -> i64 {
        let mut next_id = self.next_id.lock().await;
        let id = *next_id;
        *next_id += 1;
        id
    }

    pub async fn register_process(&self, run_id: i64, pid: u32, project_path: String, task: String, model: String) {
        let mut processes = self.processes.lock().await;
        processes.insert(run_id, ProcessInfo {
            run_id,
            pid,
            started_at: chrono::Utc::now(),
            project_path,
            task,
            model,
        });
    }

    pub async fn get_live_output(&self, _run_id: i64) -> String {
        // TODO: Implement live output buffer
        String::new()
    }

    pub async fn cleanup(&self) {
        let mut processes = self.processes.lock().await;
        processes.retain(|_, p| {
            // Keep if less than 1 hour old
            (chrono::Utc::now() - p.started_at).num_hours() < 1
        });
    }
}

impl AppState {
    pub async fn new() -> Result<Self> {
        // Initialize directories
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claudia");

        let claude_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude");

        // Ensure directories exist
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&claude_dir)?;

        // Initialize database
        let db_path = data_dir.join("agents.db");
        let conn = Connection::open(&db_path)?;

        // Run migrations
        init_database(&conn)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            session_store: SessionStore::new(),
            process_registry: ProcessRegistry::new(),
            claude_dir,
            data_dir,
        })
    }
}

fn init_database(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS agents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            icon TEXT NOT NULL DEFAULT '🤖',
            system_prompt TEXT NOT NULL DEFAULT '',
            default_task TEXT,
            model TEXT NOT NULL DEFAULT 'sonnet',
            enable_file_read INTEGER NOT NULL DEFAULT 1,
            enable_file_write INTEGER NOT NULL DEFAULT 1,
            enable_network INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS agent_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id INTEGER NOT NULL,
            agent_name TEXT NOT NULL,
            agent_icon TEXT NOT NULL DEFAULT '🤖',
            task TEXT NOT NULL,
            model TEXT NOT NULL DEFAULT 'sonnet',
            project_path TEXT NOT NULL,
            session_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            pid INTEGER,
            process_started_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            completed_at TEXT
        );

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "
    )?;

    Ok(())
}
