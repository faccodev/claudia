use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use crate::state::AppState;
use crate::api::models::*;

/// Expand tilde in path to user's home directory
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

// ============== Auth Handlers ==============

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    use crate::auth::AuthService;

    let db = state.db.lock().unwrap();

    // Find user
    let user: Option<(i64, String, String)> = db
        .query_row(
            "SELECT id, username, password_hash FROM users WHERE username = ?",
            [&req.username],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();

    let auth = AuthService::new(&crate::auth::get_jwt_secret());

    match user {
        Some((id, username, password_hash)) => {
            if auth.verify_password(&req.password, &password_hash).unwrap_or(false) {
                let token = auth.create_token(&id.to_string(), 24).unwrap_or_default();
                (StatusCode::OK, Json(AuthResponse {
                    token,
                    user: UserInfo { id, username },
                }))
            } else {
                (StatusCode::UNAUTHORIZED, Json(AuthResponse {
                    token: String::new(),
                    user: UserInfo { id: 0, username: String::new() },
                }))
            }
        }
        None => (StatusCode::UNAUTHORIZED, Json(AuthResponse {
            token: String::new(),
            user: UserInfo { id: 0, username: String::new() },
        })),
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    use crate::auth::AuthService;

    let auth = AuthService::new(&crate::auth::get_jwt_secret());
    let password_hash = match auth.hash_password(&req.password) {
        Ok(hash) => hash,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(AuthResponse {
            token: String::new(),
            user: UserInfo { id: 0, username: String::new() },
        })),
    };

    let db = state.db.lock().unwrap();

    match db.execute(
        "INSERT INTO users (username, password_hash) VALUES (?, ?)",
        [&req.username, &password_hash],
    ) {
        Ok(_) => {
            let user_id = db.last_insert_rowid();
            drop(db);
            let token = auth.create_token(&user_id.to_string(), 24).unwrap_or_default();
            (StatusCode::CREATED, Json(AuthResponse {
                token,
                user: UserInfo { id: user_id, username: req.username },
            }))
        }
        Err(_) => (StatusCode::CONFLICT, Json(AuthResponse {
            token: String::new(),
            user: UserInfo { id: 0, username: String::new() },
        })),
    }
}

pub async fn logout(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // For JWT, logout is handled client-side by removing the token
    (StatusCode::OK, Json(ApiMessage::new("Logged out successfully")))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let user = db
        .query_row("SELECT id, username FROM users LIMIT 1", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .ok()
        .unwrap_or((1i64, "default".to_string()));

    (StatusCode::OK, Json(ApiResponse::success(UserInfo { id: user.0, username: user.1 })))
}

// ============== Project Handlers ==============

pub async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let projects_dir = state.claude_dir.join("projects");

    if !projects_dir.exists() {
        return (StatusCode::OK, Json(ApiResponse::<Vec<Project>>::success(Vec::new())));
    }

    let mut projects = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let project_id = entry.file_name().to_string_lossy().to_string();
                let project_path = decode_project_path(&project_id);
                let sessions = get_project_sessions_list(&projects_dir.join(&project_id));
                let created_at = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.created().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                projects.push(Project {
                    id: project_id,
                    path: project_path,
                    sessions,
                    created_at,
                });
            }
        }
    }

    // Sort by creation date (newest first)
    projects.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    (StatusCode::OK, Json(ApiResponse::success(projects)))
}

pub async fn get_project_sessions(
    Path(project_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let sessions_dir = state.claude_dir.join("projects").join(&project_id);

    if !sessions_dir.exists() {
        return (StatusCode::NOT_FOUND, Json(ApiResponse::<Vec<String>>::error("Project not found".to_string())));
    }

    let sessions = get_project_sessions_list(&sessions_dir);

    (StatusCode::OK, Json(ApiResponse::<Vec<String>>::success(sessions)))
}

pub async fn load_session_history(
    Path((project_id, session_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session_file = state
        .claude_dir
        .join("projects")
        .join(&project_id)
        .join(format!("{}.jsonl", session_id));

    if !session_file.exists() {
        return (StatusCode::NOT_FOUND, Json(ApiResponse::<String>::error("Session not found".to_string())));
    }

    match std::fs::read_to_string(&session_file) {
        Ok(content) => (StatusCode::OK, Json(ApiResponse::<String>::success(content))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error("Failed to read session".to_string()))),
    }
}

// ============== Directory Handlers ==============

pub async fn list_directory_contents(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let path = params.get("path").map(|p| p.as_str()).unwrap_or(".");
    let path = expand_tilde(path);

    match std::fs::read_dir(&path) {
        Ok(entries) => {
            let mut files: Vec<FileEntry> = Vec::new();
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files except .claude
                    if name.starts_with('.') && name != ".claude" {
                        continue;
                    }

                    files.push(FileEntry {
                        name: name.clone(),
                        path: entry.path().to_string_lossy().to_string(),
                        is_directory: metadata.is_dir(),
                        size: metadata.len(),
                        modified: metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    });
                }
            }

            // Sort: directories first, then by name
            files.sort_by(|a, b| {
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });

            (StatusCode::OK, Json(ApiResponse::<Vec<FileEntry>>::success(files)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Vec<FileEntry>>::error(e.to_string()))),
    }
}

pub async fn search_files(
    Json(req): Json<SearchFilesRequest>,
) -> impl IntoResponse {
    let base_path = std::path::Path::new(&req.base_path);
    let query = req.query.to_lowercase();
    let mut results = Vec::new();
    let mut visited = 0;

    search_files_recursive(base_path, &query, &mut results, &mut visited, 5, 50);

    (StatusCode::OK, Json(ApiResponse::<Vec<FileEntry>>::success(results)))
}

fn search_files_recursive(
    dir: &std::path::Path,
    query: &str,
    results: &mut Vec<FileEntry>,
    visited: &mut usize,
    max_depth: usize,
    max_results: usize,
) {
    if *visited >= 1000 || results.len() >= max_results {
        return;
    }
    *visited += 1;

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if results.len() >= max_results {
            return;
        }

        let name = entry.file_name().to_string_lossy().to_lowercase();
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Skip excluded directories
        let skip_dirs = ["node_modules", "target", ".git", "dist", "build", ".next", "__pycache__"];
        if metadata.is_dir() {
            if skip_dirs.iter().any(|d| name.contains(d)) || name.starts_with('.') {
                continue;
            }

            if results.len() < max_results {
                search_files_recursive(&path, query, results, visited, max_depth - 1, max_results);
            }
        } else if name.contains(query) {
            results.push(FileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                is_directory: false,
                size: metadata.len(),
                modified: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            });
        }
    }
}

// ============== Claude Code Handlers ==============

pub async fn execute_claude_code(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteClaudeCodeRequest>,
) -> impl IntoResponse {
    // Spawn claude process
    let session_id = spawn_claude_process(
        &state,
        &req.project_path,
        &req.prompt,
        req.model.as_deref().unwrap_or("sonnet"),
        false,
        None,
    )
    .await;

    match session_id {
        Ok(id) => (StatusCode::OK, Json(ApiResponse::<String>::success(id))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(e.to_string()))),
    }
}

pub async fn continue_claude_code(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteClaudeCodeRequest>,
) -> impl IntoResponse {
    let session_id = spawn_claude_process(
        &state,
        &req.project_path,
        &req.prompt,
        req.model.as_deref().unwrap_or("sonnet"),
        true,
        None,
    )
    .await;

    match session_id {
        Ok(id) => (StatusCode::OK, Json(ApiResponse::<String>::success(id))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(e.to_string()))),
    }
}

pub async fn resume_claude_code(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResumeClaudeCodeRequest>,
) -> impl IntoResponse {
    let session_id = spawn_claude_process(
        &state,
        &req.project_path,
        &req.prompt,
        req.model.as_deref().unwrap_or("sonnet"),
        false,
        Some(&req.session_id),
    )
    .await;

    match session_id {
        Ok(id) => (StatusCode::OK, Json(ApiResponse::<String>::success(id))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(e.to_string()))),
    }
}

pub async fn cancel_claude_execution(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Implement cancellation using process registry
    (StatusCode::OK, Json(ApiMessage::new("Cancellation request sent")))
}

pub async fn list_running_claude_sessions(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Return from process registry
    (StatusCode::OK, Json(Vec::<()>::new()))
}

pub async fn get_claude_session_output(
    Path(session_id): Path<String>,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Get from process registry
    (StatusCode::OK, Json("".to_string()))
}

// ============== Settings Handlers ==============

pub async fn get_claude_settings(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let settings_path = state.claude_dir.join("settings.json");
    let settings = if settings_path.exists() {
        std::fs::read_to_string(&settings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    (StatusCode::OK, Json(settings))
}

pub async fn save_claude_settings(
    State(state): State<Arc<AppState>>,
    Json(settings): Json<serde_json::Value>,
) -> impl IntoResponse {
    let settings_path = state.claude_dir.join("settings.json");

    match serde_json::to_string_pretty(&settings) {
        Ok(content) => match std::fs::write(&settings_path, content) {
            Ok(_) => (StatusCode::OK, Json(ApiMessage::new("Settings saved"))),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiMessage::new(format!("Failed to save settings: {}", e)))),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiMessage::new(format!("Failed to serialize settings: {}", e)))),
    }
}

pub async fn get_system_prompt(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_md = state.claude_dir.join("CLAUDE.md");
    let content = std::fs::read_to_string(&claude_md).unwrap_or_default();
    (StatusCode::OK, Json(ApiResponse::<String>::success(content)))
}

pub async fn save_system_prompt(
    State(state): State<Arc<AppState>>,
    Json(content): Json<String>,
) -> impl IntoResponse {
    let claude_md = state.claude_dir.join("CLAUDE.md");
    match std::fs::write(&claude_md, content) {
        Ok(_) => (StatusCode::OK, Json(ApiMessage::new("System prompt saved"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiMessage::new(format!("Failed to save system prompt: {}", e)))),
    }
}

pub async fn check_claude_version(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let output = std::process::Command::new(&claude_path)
        .arg("--version")
        .output();

    let (is_installed, version, output_str) = match output {
        Ok(o) if o.status.success() => {
            let v = String::from_utf8_lossy(&o.stdout).to_string();
            (true, Some(v.trim().to_string()), v)
        }
        Ok(o) => (false, None, String::from_utf8_lossy(&o.stderr).to_string()),
        Err(e) => (false, None, e.to_string()),
    };

    (StatusCode::OK, Json(ClaudeVersionStatus {
        is_installed,
        version,
        output: output_str,
    }))
}

pub async fn get_claude_binary_path(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());
    (StatusCode::OK, Json(path))
}

pub async fn set_claude_binary_path(
    State(state): State<Arc<AppState>>,
    Json(path): Json<String>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES ('claude_binary_path', ?, datetime('now'))",
        [&path],
    ).ok();
    (StatusCode::OK, Json(ApiMessage::new("Claude binary path updated")))
}

pub async fn list_claude_installations(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let installations = discover_claude_installations(&state).await;
    (StatusCode::OK, Json(installations))
}

// ============== CLAUDE.md Handlers ==============

pub async fn find_claude_md_files(
    Json(req): Json<FindClaudeMdFilesRequest>,
) -> impl IntoResponse {
    let base_path = std::path::Path::new(&req.project_path);
    let mut files = Vec::new();

    find_claude_md_recursive(base_path, &mut files, 0);

    (StatusCode::OK, Json(ApiResponse::<Vec<ClaudeMdFile>>::success(files)))
}

pub async fn read_claude_md_file(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let path = params.get("path").map(|p| p.as_str()).unwrap_or("");

    match std::fs::read_to_string(path) {
        Ok(content) => (StatusCode::OK, Json(ApiResponse::<String>::success(content))),
        Err(e) => (StatusCode::NOT_FOUND, Json(ApiResponse::<String>::error(e.to_string()))),
    }
}

pub async fn save_claude_md_file(
    Json(req): Json<SaveClaudeMdFileRequest>,
) -> impl IntoResponse {
    match std::fs::write(&req.file_path, &req.content) {
        Ok(_) => (StatusCode::OK, Json(ApiMessage::new("File saved"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiMessage::new(format!("Failed to save file: {}", e)))),
    }
}

// ============== Agent Handlers ==============

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let mut stmt = match db.prepare(
        "SELECT id, name, icon, system_prompt, default_task, model, enable_file_read, enable_file_write, enable_network, created_at, updated_at FROM agents ORDER BY name"
    ) {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::<Agent>::new())),
    };

    let agents = stmt.query_map([], |row| {
        Ok(Agent {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            icon: row.get(2)?,
            system_prompt: row.get(3)?,
            default_task: row.get(4)?,
            model: row.get(5)?,
            enable_file_read: row.get::<_, i32>(6)? != 0,
            enable_file_write: row.get::<_, i32>(7)? != 0,
            enable_network: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default();

    (StatusCode::OK, Json(agents))
}

pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();

    let result = db.execute(
        "INSERT INTO agents (name, icon, system_prompt, default_task, model, enable_file_read, enable_file_write, enable_network) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            &req.name,
            &req.icon,
            &req.system_prompt,
            &req.default_task,
            req.model.as_deref().unwrap_or("sonnet"),
            req.enable_file_read.unwrap_or(true) as i32,
            req.enable_file_write.unwrap_or(true) as i32,
            req.enable_network.unwrap_or(false) as i32,
        ],
    );

    match result {
        Ok(_) => {
            let id = db.last_insert_rowid();
            drop(db);
            let agent = get_agent_by_id(&state, id).await;
            (StatusCode::CREATED, Json(ApiResponse::success(agent)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Agent>::error(e.to_string()))),
    }
}

pub async fn get_agent(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = get_agent_by_id(&state, id).await;
    if agent.id.is_some() {
        (StatusCode::OK, Json(ApiResponse::success(agent)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<Agent>::error("Agent not found".to_string())))
    }
}

pub async fn update_agent(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateAgentRequest>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();

    let result = db.execute(
        "UPDATE agents SET name = ?, icon = ?, system_prompt = ?, default_task = ?, model = ?, enable_file_read = ?, enable_file_write = ?, enable_network = ?, updated_at = datetime('now') WHERE id = ?",
        rusqlite::params![
            &req.name,
            &req.icon,
            &req.system_prompt,
            &req.default_task,
            req.model.as_deref().unwrap_or("sonnet"),
            req.enable_file_read.unwrap_or(true) as i32,
            req.enable_file_write.unwrap_or(true) as i32,
            req.enable_network.unwrap_or(false) as i32,
            id,
        ],
    );

    match result {
        Ok(_) => {
            drop(db);
            let agent = get_agent_by_id(&state, id).await;
            (StatusCode::OK, Json(ApiResponse::success(agent)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Agent>::error(e.to_string()))),
    }
}

pub async fn delete_agent(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();

    match db.execute("DELETE FROM agents WHERE id = ?", [id]) {
        Ok(_) => (StatusCode::OK, Json(ApiMessage::new("Agent deleted"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiMessage::new(format!("Failed to delete agent: {}", e)))),
    }
}

pub async fn export_agent(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = get_agent_by_id(&state, id).await;
    if agent.id.is_none() {
        return (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentExport>::error("Agent not found".to_string())));
    }

    let export = AgentExport {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        agent: AgentData {
            name: agent.name.clone(),
            icon: agent.icon.clone(),
            system_prompt: agent.system_prompt.clone(),
            default_task: agent.default_task.clone(),
            model: agent.model.clone(),
        },
    };

    (StatusCode::OK, Json(ApiResponse::<String>::success(serde_json::to_string(&export).unwrap_or_default())))
}

pub async fn import_agent(
    State(state): State<Arc<AppState>>,
    Json(json_data): Json<String>,
) -> impl IntoResponse {
    let import: AgentExport = match serde_json::from_str(&json_data) {
        Ok(i) => i,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ApiResponse::<Agent>::error(format!("Invalid JSON: {}", e)))),
    };

    let db = state.db.lock().unwrap();
    let name = if import.agent.name.contains("(Imported)") {
        import.agent.name.clone()
    } else {
        format!("{} (Imported)", import.agent.name)
    };

    let result = db.execute(
        "INSERT INTO agents (name, icon, system_prompt, default_task, model) VALUES (?, ?, ?, ?, ?)",
        rusqlite::params![&name, &import.agent.icon, &import.agent.system_prompt, &import.agent.default_task, &import.agent.model],
    );

    match result {
        Ok(_) => {
            let id = db.last_insert_rowid();
            drop(db);
            let agent = get_agent_by_id(&state, id).await;
            (StatusCode::CREATED, Json(ApiResponse::success(agent)))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Agent>::error(e.to_string()))),
    }
}

// ============== Agent Execution Handlers ==============

pub async fn execute_agent(
    Path(agent_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteAgentRequest>,
) -> impl IntoResponse {
    let agent = get_agent_by_id(&state, agent_id).await;
    if agent.id.is_none() {
        return (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentRun>::error("Agent not found".to_string())));
    }

    // Create agent run record
    let session_id = uuid::Uuid::new_v4().to_string();
    let db = state.db.lock().unwrap();

    let result = db.execute(
        "INSERT INTO agent_runs (agent_id, agent_name, agent_icon, task, model, project_path, session_id, status) VALUES (?, ?, ?, ?, ?, ?, ?, 'running')",
        rusqlite::params![agent_id, &agent.name, &agent.icon, &req.task, req.model.as_deref().unwrap_or(&agent.model), &req.project_path, &session_id],
    );

    drop(db);

    let run_id = match result {
        Ok(_) => {
            let db = state.db.lock().unwrap();
            db.last_insert_rowid()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<i64>::error(e.to_string()))),
    };

    // Spawn the agent process
    let state_clone = state.clone();
    tokio::spawn(async move {
        spawn_agent_process(state_clone, run_id, agent_id, agent, req.project_path, req.task, req.model).await;
    });

    (StatusCode::OK, Json(ApiResponse::<ExecuteRunResponse>::success(ExecuteRunResponse { run_id })))
}

pub async fn list_agent_runs(
    Path(agent_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let mut stmt = match db.prepare(
        "SELECT id, agent_id, agent_name, agent_icon, task, model, project_path, session_id, status, pid, process_started_at, created_at, completed_at FROM agent_runs WHERE agent_id = ? ORDER BY created_at DESC LIMIT 50"
    ) {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::<AgentRun>::new())),
    };

    let runs = stmt.query_map([agent_id], |row| {
        Ok(AgentRun {
            id: Some(row.get(0)?),
            agent_id: row.get(1)?,
            agent_name: row.get(2)?,
            agent_icon: row.get(3)?,
            task: row.get(4)?,
            model: row.get(5)?,
            project_path: row.get(6)?,
            session_id: row.get(7)?,
            status: row.get(8)?,
            pid: row.get(9)?,
            process_started_at: row.get(10)?,
            created_at: row.get(11)?,
            completed_at: row.get(12)?,
        })
    }).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default();

    (StatusCode::OK, Json(runs))
}

pub async fn get_agent_run(
    Path(run_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let run: Option<AgentRun> = db.query_row(
        "SELECT id, agent_id, agent_name, agent_icon, task, model, project_path, session_id, status, pid, process_started_at, created_at, completed_at FROM agent_runs WHERE id = ?",
        [run_id],
        |row| Ok(AgentRun {
            id: Some(row.get(0)?),
            agent_id: row.get(1)?,
            agent_name: row.get(2)?,
            agent_icon: row.get(3)?,
            task: row.get(4)?,
            model: row.get(5)?,
            project_path: row.get(6)?,
            session_id: row.get(7)?,
            status: row.get(8)?,
            pid: row.get(9)?,
            process_started_at: row.get(10)?,
            created_at: row.get(11)?,
            completed_at: row.get(12)?,
        })
    ).ok();

    match run {
        Some(r) => (StatusCode::OK, Json(ApiResponse::success(r))),
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentRun>::error("Run not found".to_string()))),
    }
}

pub async fn get_agent_run_with_metrics(
    Path(run_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let run: Option<AgentRun> = db.query_row(
        "SELECT id, agent_id, agent_name, agent_icon, task, model, project_path, session_id, status, pid, process_started_at, created_at, completed_at FROM agent_runs WHERE id = ?",
        [run_id],
        |row| Ok(AgentRun {
            id: Some(row.get(0)?),
            agent_id: row.get(1)?,
            agent_name: row.get(2)?,
            agent_icon: row.get(3)?,
            task: row.get(4)?,
            model: row.get(5)?,
            project_path: row.get(6)?,
            session_id: row.get(7)?,
            status: row.get(8)?,
            pid: row.get(9)?,
            process_started_at: row.get(10)?,
            created_at: row.get(11)?,
            completed_at: row.get(12)?,
        })
    ).ok();

    match run {
        Some(r) => {
            let metrics = calculate_run_metrics(&state, run_id).await;
            (StatusCode::OK, Json(ApiResponse::success(AgentRunWithMetrics { run: r, metrics, output: None })))
        }
        None => (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentRunWithMetrics>::error("Run not found".to_string()))),
    }
}

pub async fn list_running_sessions(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let mut stmt = match db.prepare(
        "SELECT id, agent_id, agent_name, agent_icon, task, model, project_path, session_id, status, pid, process_started_at, created_at, completed_at FROM agent_runs WHERE status = 'running' ORDER BY created_at DESC"
    ) {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::<AgentRun>::new())),
    };

    let runs = stmt.query_map([], |row| {
        Ok(AgentRun {
            id: Some(row.get(0)?),
            agent_id: row.get(1)?,
            agent_name: row.get(2)?,
            agent_icon: row.get(3)?,
            task: row.get(4)?,
            model: row.get(5)?,
            project_path: row.get(6)?,
            session_id: row.get(7)?,
            status: row.get(8)?,
            pid: row.get(9)?,
            process_started_at: row.get(10)?,
            created_at: row.get(11)?,
            completed_at: row.get(12)?,
        })
    }).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default();

    (StatusCode::OK, Json(runs))
}

pub async fn kill_agent_session(
    Path(run_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Get PID and kill
    let db = state.db.lock().unwrap();
    let pid: Option<u32> = db.query_row(
        "SELECT pid FROM agent_runs WHERE id = ?",
        [run_id],
        |row| row.get::<_, Option<u32>>(0).ok().flatten(),
    ).ok().flatten();

    drop(db);

    if let Some(pid) = pid {
        #[cfg(unix)]
        {
            std::process::Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .output()
                .ok();
        }
        #[cfg(windows)]
        {
            std::process::Command::new("taskkill")
                .arg("/F")
                .arg("/PID")
                .arg(pid.to_string())
                .output()
                .ok();
        }
    }

    let db = state.db.lock().unwrap();
    db.execute("UPDATE agent_runs SET status = 'cancelled', completed_at = datetime('now') WHERE id = ?", [run_id]).ok();

    (StatusCode::OK, Json(true))
}

pub async fn get_session_status(
    Path(run_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let status: Option<String> = db.query_row(
        "SELECT status FROM agent_runs WHERE id = ?",
        [run_id],
        |row| row.get::<_, String>(0),
    ).ok();

    (StatusCode::OK, Json(ApiResponse::success(status.unwrap_or_else(|| "unknown".to_string()))))
}

pub async fn get_session_output(
    Path(run_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let (session_id, status): (Option<String>, Option<String>) = db.query_row(
        "SELECT session_id, status FROM agent_runs WHERE id = ?",
        [run_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).ok().unwrap_or((None, None));

    if status.as_ref().map(|s| s == "running").unwrap_or(false) {
        // Get from live output buffer
        let output = state.process_registry.get_live_output(run_id).await;
        return (StatusCode::OK, Json(output));
    }

    // Read from JSONL file
    if let Some(session_id) = session_id {
        let jsonl_path = state.claude_dir.join("projects").join("_runs").join(&session_id);
        let content = std::fs::read_to_string(&jsonl_path).unwrap_or_default();
        return (StatusCode::OK, Json(ApiResponse::<String>::success(content)));
    }

    (StatusCode::OK, Json(ApiResponse::<String>::success(String::new())))
}

pub async fn get_live_session_output(
    Path(run_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let output = state.process_registry.get_live_output(run_id).await;
    (StatusCode::OK, Json(ApiResponse::success(output)))
}

pub async fn cleanup_finished_processes(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    let count = db.execute("UPDATE agent_runs SET status = 'completed', completed_at = datetime('now') WHERE status = 'running' AND pid IS NOT NULL AND NOT EXISTS (SELECT 1 FROM active_processes WHERE pid = agent_runs.pid)", []).unwrap_or(0);
    (StatusCode::OK, Json(ApiResponse::success(count)))
}

// ============== GitHub Handlers ==============

pub async fn fetch_github_agents() -> impl IntoResponse {
    let url = "https://api.github.com/repos/faccodev/claudia/contents/cc_agents";
    let client = reqwest::Client::new();

    match client.get(url).header("User-Agent", "claudia-server").send().await {
        Ok(response) => {
            match response.json::<Vec<GitHubAgentFile>>().await {
                Ok(files) => (StatusCode::OK, Json(ApiResponse::success(files))),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Vec<GitHubAgentFile>>::error("Failed to parse response".to_string()))),
            }
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Vec<GitHubAgentFile>>::error("Request failed".to_string()))),
    }
}

pub async fn fetch_github_agent_content(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let url = params.get("url").map(|p| p.as_str()).unwrap_or("");
    let client = reqwest::Client::new();

    match client.get(url).header("User-Agent", "claudia-server").send().await {
        Ok(response) => {
            match response.text().await {
                Ok(content) => (StatusCode::OK, Json(ApiResponse::<String>::success(content))),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error("Failed to read response".to_string()))),
            }
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error("Request failed".to_string()))),
    }
}

pub async fn import_agent_from_github(
    State(state): State<Arc<AppState>>,
    Json(url): Json<String>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();

    let content = match client.get(&url).header("User-Agent", "claudia-server").send().await {
        Ok(response) => response.text().await.ok(),
        Err(_) => None,
    };

    if let Some(content) = content {
        if let Ok(export) = serde_json::from_str::<AgentExport>(&content) {
            // Import as regular agent
            let db = state.db.lock().unwrap();
            let name = format!("{} (Imported)", export.agent.name);

            let result = db.execute(
                "INSERT INTO agents (name, icon, system_prompt, default_task, model) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![&name, &export.agent.icon, &export.agent.system_prompt, &export.agent.default_task, &export.agent.model],
            );

            if result.is_ok() {
                let id = db.last_insert_rowid();
                drop(db);
                let agent = get_agent_by_id(&state, id).await;
                return (StatusCode::CREATED, Json(ApiResponse::success(agent)));
            }
        }
    }

    (StatusCode::BAD_REQUEST, Json(ApiResponse::<Agent>::error("Failed to import agent".to_string())))
}

// ============== Checkpoint Handlers ==============

pub async fn create_checkpoint(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCheckpointRequest>,
) -> impl IntoResponse {
    // TODO: Implement full checkpoint logic
    let checkpoint_id = uuid::Uuid::new_v4().to_string();
    let checkpoint = Checkpoint {
        id: checkpoint_id.clone(),
        session_id: req.session_id,
        project_id: req.project_id,
        message_index: req.message_index.unwrap_or(0),
        timestamp: chrono::Utc::now(),
        description: req.description,
        parent_checkpoint_id: None,
        metadata: CheckpointMetadata {
            total_tokens: 0,
            model_used: "sonnet".to_string(),
            user_prompt: String::new(),
            file_changes: 0,
            snapshot_size: 0,
        },
    };

    (StatusCode::CREATED, Json(CheckpointResult {
        checkpoint,
        files_processed: 0,
        warnings: vec![],
    }))
}

pub async fn restore_checkpoint(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RestoreCheckpointRequest>,
) -> impl IntoResponse {
    // TODO: Implement restore logic
    (StatusCode::OK, Json(CheckpointResult {
        checkpoint: Checkpoint {
            id: String::new(),
            session_id: req.session_id,
            project_id: req.project_id,
            message_index: 0,
            timestamp: chrono::Utc::now(),
            description: None,
            parent_checkpoint_id: None,
            metadata: CheckpointMetadata {
                total_tokens: 0,
                model_used: "sonnet".to_string(),
                user_prompt: String::new(),
                file_changes: 0,
                snapshot_size: 0,
            },
        },
        files_processed: 0,
        warnings: vec![],
    }))
}

pub async fn list_checkpoints(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let session_id = params.get("session_id").map(|p| p.as_str()).unwrap_or("");
    let project_id = params.get("project_id").map(|p| p.as_str()).unwrap_or("");

    // TODO: Read from checkpoint storage
    (StatusCode::OK, Json(Vec::<Checkpoint>::new()))
}

pub async fn fork_from_checkpoint(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForkFromCheckpointRequest>,
) -> impl IntoResponse {
    // TODO: Implement fork logic
    (StatusCode::OK, Json(CheckpointResult {
        checkpoint: Checkpoint {
            id: String::new(),
            session_id: req.session_id,
            project_id: req.project_id,
            message_index: 0,
            timestamp: chrono::Utc::now(),
            description: req.description,
            parent_checkpoint_id: None,
            metadata: CheckpointMetadata {
                total_tokens: 0,
                model_used: "sonnet".to_string(),
                user_prompt: String::new(),
                file_changes: 0,
                snapshot_size: 0,
            },
        },
        files_processed: 0,
        warnings: vec![],
    }))
}

pub async fn get_session_timeline(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let session_id = params.get("session_id").map(|p| p.as_str()).unwrap_or("");
    let project_id = params.get("project_id").map(|p| p.as_str()).unwrap_or("");

    (StatusCode::OK, Json(SessionTimeline {
        session_id: session_id.to_string(),
        root_node: None,
        current_checkpoint_id: None,
        auto_checkpoint_enabled: false,
        checkpoint_strategy: "smart".to_string(),
        total_checkpoints: 0,
    }))
}

pub async fn update_checkpoint_settings(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateCheckpointSettingsRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(ApiMessage::new("Checkpoint settings updated")))
}

pub async fn get_checkpoint_diff(
    Path((from, to)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(CheckpointDiff {
        from_checkpoint_id: from,
        to_checkpoint_id: to,
        modified_files: vec![],
        added_files: vec![],
        deleted_files: vec![],
        token_delta: 0,
    }))
}

pub async fn track_checkpoint_message(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<TrackCheckpointMessageRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(ApiMessage::new("Message tracked")))
}

pub async fn track_session_messages(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<TrackSessionMessagesRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(ApiMessage::new("Messages tracked")))
}

pub async fn check_auto_checkpoint(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<TrackCheckpointMessageRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(false))
}

pub async fn cleanup_old_checkpoints(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<CleanupOldCheckpointsRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(0))
}

pub async fn get_checkpoint_settings(
    State(_state): State<Arc<AppState>>,
    Query(_params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(CheckpointSettings {
        auto_checkpoint_enabled: false,
        checkpoint_strategy: "smart".to_string(),
    }))
}

pub async fn clear_checkpoint_manager(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let session_id = params.get("session_id").map(|p| p.as_str()).unwrap_or("");
    (StatusCode::OK, Json(ApiMessage::new(format!("Checkpoint manager cleared for {}", session_id))))
}

pub async fn get_checkpoint_state_stats(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "active_managers": 0, "sessions": [] })))
}

// ============== MCP Handlers ==============

pub async fn mcp_add(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddMCPServerRequest>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let mut cmd = std::process::Command::new(&claude_path);
    cmd.arg("mcp").arg("add").arg("-s").arg(req.scope.as_deref().unwrap_or("user"));

    if req.transport == "sse" {
        cmd.arg("--transport").arg("sse");
    }

    if let Some(ref command) = req.command {
        cmd.arg("--").arg(command);
        if let Some(ref args) = req.args {
            for arg in args {
                cmd.arg(arg);
            }
        }
    }

    let output = cmd.output();

    match output {
        Ok(o) if o.status.success() => {
            (StatusCode::OK, Json(AddMCPServerResult {
                success: true,
                message: "Server added successfully".to_string(),
            }))
        }
        Ok(o) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(AddMCPServerResult {
                success: false,
                message: String::from_utf8_lossy(&o.stderr).to_string(),
            }))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(AddMCPServerResult {
                success: false,
                message: e.to_string(),
            }))
        }
    }
}

pub async fn mcp_list(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let output = std::process::Command::new(&claude_path)
        .arg("mcp")
        .arg("list")
        .output();

    match output {
        Ok(o) => {
            let lines: Vec<String> = String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|s| s.to_string())
                .collect();
            (StatusCode::OK, Json(lines))
        }
        Err(_) => (StatusCode::OK, Json(Vec::<String>::new())),
    }
}

pub async fn mcp_get(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let output = std::process::Command::new(&claude_path)
        .arg("mcp")
        .arg("get")
        .arg(&name)
        .output();

    match output {
        Ok(o) => (StatusCode::OK, Json(String::from_utf8_lossy(&o.stdout).to_string())),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())),
    }
}

pub async fn mcp_remove(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let output = std::process::Command::new(&claude_path)
        .arg("mcp")
        .arg("remove")
        .arg(&name)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            (StatusCode::OK, Json(MCPDeleteResult {
                success: true,
                message: "Server removed".to_string(),
            }))
        }
        Ok(o) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(MCPDeleteResult {
                success: false,
                message: String::from_utf8_lossy(&o.stderr).to_string(),
            }))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(MCPDeleteResult {
                success: false,
                message: e.to_string(),
            }))
        }
    }
}

pub async fn mcp_add_json(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddJsonRequest>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let mut cmd = std::process::Command::new(&claude_path);
    cmd.arg("mcp").arg("add-json").arg(&req.name).arg(&req.json_config);

    if let Some(ref scope) = req.scope {
        cmd.arg("-s").arg(scope);
    }

    let output = cmd.output();

    match output {
        Ok(o) if o.status.success() => {
            (StatusCode::OK, Json(AddMCPServerResult {
                success: true,
                message: "Server added successfully".to_string(),
            }))
        }
        Ok(o) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(AddMCPServerResult {
                success: false,
                message: String::from_utf8_lossy(&o.stderr).to_string(),
            }))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(AddMCPServerResult {
                success: false,
                message: e.to_string(),
            }))
        }
    }
}

pub async fn mcp_add_from_claude_desktop(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Read Claude Desktop config and import
    (StatusCode::OK, Json(ImportFromClaudeDesktopResult {
        success: true,
        imported: vec![],
        message: "No servers imported".to_string(),
    }))
}

pub async fn mcp_serve(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let output = std::process::Command::new(&claude_path)
        .arg("mcp")
        .arg("serve")
        .output();

    match output {
        Ok(o) => (StatusCode::OK, Json(String::from_utf8_lossy(&o.stdout).to_string())),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())),
    }
}

pub async fn mcp_test_connection(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let output = std::process::Command::new(&claude_path)
        .arg("mcp")
        .arg("test")
        .arg(&name)
        .output();

    match output {
        Ok(o) => (StatusCode::OK, Json(String::from_utf8_lossy(&o.stdout).to_string())),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())),
    }
}

pub async fn mcp_reset_project_choices(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());

    let output = std::process::Command::new(&claude_path)
        .arg("mcp")
        .arg("reset-project-choices")
        .output();

    match output {
        Ok(o) => (StatusCode::OK, Json(String::from_utf8_lossy(&o.stdout).to_string())),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(e.to_string())),
    }
}

pub async fn mcp_get_server_status(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::Map::<String, serde_json::Value>::new()))
}

pub async fn mcp_read_project_config(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let project_path = params.get("project_path").map(|p| p.as_str()).unwrap_or(".");
    let config_path = std::path::Path::new(project_path).join(".mcp.json");

    if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => return (StatusCode::OK, Json(config)),
                Err(_) => {}
            }
            Err(_) => {}
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::Value::Object(serde_json::Map::new()))))
}

pub async fn mcp_save_project_config(
    Json(req): Json<SaveMCPProjectConfigRequest>,
) -> impl IntoResponse {
    let config_path = std::path::Path::new(&req.project_path).join(".mcp.json");

    match serde_json::to_string_pretty(&req.config) {
        Ok(content) => match std::fs::write(&config_path, content) {
            Ok(_) => (StatusCode::OK, Json(ApiMessage::new("Config saved"))),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiMessage::new(format!("Failed to save config: {}", e)))),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiMessage::new(format!("Failed to serialize config: {}", e)))),
    }
}

// ============== Usage Handlers ==============

pub async fn get_usage_stats(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let since = params.get("since").and_then(|s| s.parse::<i64>().ok());
    let stats = calculate_usage_stats(&state, since).await;
    (StatusCode::OK, Json(stats))
}

pub async fn get_usage_by_date_range(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let start = params.get("start_date").map(|s| s.as_str()).unwrap_or("");
    let end = params.get("end_date").map(|s| s.as_str()).unwrap_or("");
    let stats = calculate_usage_stats(&state, None).await;
    (StatusCode::OK, Json(stats))
}

pub async fn get_usage_details(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(100);
    let entries = calculate_usage_details(&state, limit).await;
    (StatusCode::OK, Json(entries))
}

pub async fn get_session_stats(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let since = params.get("since").and_then(|s| s.parse::<i64>().ok());
    let stats = calculate_session_stats(&state, since).await;
    (StatusCode::OK, Json(stats))
}

// ============== Helper Functions ==============

fn decode_project_path(encoded: &str) -> String {
    // Simple URL decode
    encoded.replace("%2F", "/").replace("%20", " ")
}

fn get_project_sessions_list(project_dir: &std::path::Path) -> Vec<String> {
    let mut sessions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(project_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".jsonl") {
                sessions.push(name.trim_end_matches(".jsonl").to_string());
            }
        }
    }

    sessions
}

async fn get_agent_by_id(state: &Arc<AppState>, id: i64) -> Agent {
    let db = state.db.lock().unwrap();
    db.query_row(
        "SELECT id, name, icon, system_prompt, default_task, model, enable_file_read, enable_file_write, enable_network, created_at, updated_at FROM agents WHERE id = ?",
        [id],
        |row| Ok(Agent {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            icon: row.get(2)?,
            system_prompt: row.get(3)?,
            default_task: row.get(4)?,
            model: row.get(5)?,
            enable_file_read: row.get::<_, i32>(6)? != 0,
            enable_file_write: row.get::<_, i32>(7)? != 0,
            enable_network: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    ).ok().unwrap_or(Agent {
        id: None,
        name: String::new(),
        icon: "🤖".to_string(),
        system_prompt: String::new(),
        default_task: None,
        model: "sonnet".to_string(),
        enable_file_read: true,
        enable_file_write: true,
        enable_network: false,
        created_at: String::new(),
        updated_at: String::new(),
    })
}

async fn find_claude_binary(state: &Arc<AppState>) -> Option<String> {
    // Check stored path
    {
        let db = state.db.lock().unwrap();
        if let Ok(path) = db.query_row(
            "SELECT value FROM app_settings WHERE key = 'claude_binary_path'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
        }
    }

    // Discover installations
    let installations = discover_claude_installations(state).await;
    installations.into_iter().next().map(|i| i.path)
}

async fn discover_claude_installations(state: &Arc<AppState>) -> Vec<ClaudeInstallation> {
    let mut installations = Vec::new();

    // Try 'which'
    if let Ok(output) = std::process::Command::new("which").arg("claude").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                installations.push(ClaudeInstallation {
                    path,
                    version: None,
                    source: "which".to_string(),
                });
            }
        }
    }

    installations
}

fn find_claude_md_recursive(dir: &std::path::Path, files: &mut Vec<ClaudeMdFile>, depth: usize) {
    if depth > 5 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if name == "node_modules" || name == "target" || name == ".git" {
                continue;
            }
            find_claude_md_recursive(&path, files, depth + 1);
        } else if name == "CLAUDE.md" {
            files.push(ClaudeMdFile {
                path: path.to_string_lossy().to_string(),
                relative_path: path.strip_prefix(dir).map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            });
        }
    }
}

async fn spawn_claude_process(
    state: &Arc<AppState>,
    project_path: &str,
    prompt: &str,
    model: &str,
    _continue: bool,
    _session_id: Option<&str>,
) -> Result<String, String> {
    let claude_path = find_claude_binary(state).await.unwrap_or_else(|| "claude".to_string());
    let session_id = uuid::Uuid::new_v4().to_string();

    let mut cmd = std::process::Command::new(&claude_path);
    cmd.arg("-p").arg(prompt)
       .arg("--model").arg(model)
       .arg("--output-format").arg("stream-json")
       .arg("--dangerously-skip-permissions")
       .current_dir(project_path);

    // Inherit PATH
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }

    // Spawn process (we don't wait for it)
    match cmd.spawn() {
        Ok(_child) => Ok(session_id),
        Err(e) => Err(e.to_string()),
    }
}

async fn spawn_agent_process(
    state: Arc<AppState>,
    run_id: i64,
    agent_id: i64,
    agent: Agent,
    project_path: String,
    task: String,
    model: Option<String>,
) {
    let claude_path = find_claude_binary(&state).await.unwrap_or_else(|| "claude".to_string());
    let session_id = uuid::Uuid::new_v4().to_string();
    let model = model.unwrap_or(agent.model);

    let mut cmd = std::process::Command::new(&claude_path);
    cmd.arg("-p").arg(&task)
       .arg("--system-prompt").arg(&agent.system_prompt)
       .arg("--model").arg(&model)
       .arg("--output-format").arg("stream-json")
       .arg("--dangerously-skip-permissions")
       .current_dir(&project_path);

    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }

    let pid = match cmd.spawn() {
        Ok(child) => Some(child.id()),
        Err(_) => None,
    };

    // Update run record
    {
        let db = state.db.lock().unwrap();
        db.execute(
            "UPDATE agent_runs SET pid = ?, process_started_at = datetime('now') WHERE id = ?",
            rusqlite::params![pid, run_id],
        ).ok();
    }

    // Store process info
    state.process_registry.register_process(run_id, pid.unwrap_or(0), project_path.clone(), task.clone(), model.clone()).await;
}

async fn calculate_run_metrics(state: &Arc<AppState>, run_id: i64) -> Option<AgentRunMetrics> {
    None
}

async fn calculate_usage_stats(state: &Arc<AppState>, _since: Option<i64>) -> UsageStats {
    UsageStats {
        total_cost: 0.0,
        total_tokens: 0,
        total_requests: 0,
        by_model: std::collections::HashMap::new(),
        by_project: std::collections::HashMap::new(),
    }
}

async fn calculate_usage_details(state: &Arc<AppState>, _limit: usize) -> Vec<UsageEntry> {
    Vec::new()
}

async fn calculate_session_stats(state: &Arc<AppState>, _since: Option<i64>) -> Vec<ProjectUsage> {
    Vec::new()
}
