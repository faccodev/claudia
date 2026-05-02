use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

// ============== Auth Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
}

// ============== Project Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub path: String,
    pub sessions: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub project_path: String,
    pub todo_data: Option<serde_json::Value>,
    pub created_at: u64,
    pub first_message: Option<String>,
    pub message_timestamp: Option<String>,
}

// ============== Agent Models ==============

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: Option<i64>,
    pub name: String,
    pub icon: String,
    pub system_prompt: String,
    pub default_task: Option<String>,
    pub model: String,
    pub enable_file_read: bool,
    pub enable_file_write: bool,
    pub enable_network: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub icon: String,
    pub system_prompt: String,
    pub default_task: Option<String>,
    pub model: Option<String>,
    pub enable_file_read: Option<bool>,
    pub enable_file_write: Option<bool>,
    pub enable_network: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: String,
    pub icon: String,
    pub system_prompt: String,
    pub default_task: Option<String>,
    pub model: Option<String>,
    pub enable_file_read: Option<bool>,
    pub enable_file_write: Option<bool>,
    pub enable_network: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentExport {
    pub version: u32,
    pub exported_at: String,
    pub agent: AgentData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentData {
    pub name: String,
    pub icon: String,
    pub system_prompt: String,
    pub default_task: Option<String>,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentRun {
    pub id: Option<i64>,
    pub agent_id: i64,
    pub agent_name: String,
    pub agent_icon: String,
    pub task: String,
    pub model: String,
    pub project_path: String,
    pub session_id: String,
    pub status: String,
    pub pid: Option<u32>,
    pub process_started_at: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentRunMetrics {
    pub duration_ms: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub message_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentRunWithMetrics {
    #[serde(flatten)]
    pub run: AgentRun,
    pub metrics: Option<AgentRunMetrics>,
    pub output: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteAgentRequest {
    pub project_path: String,
    pub task: String,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteRunResponse {
    pub run_id: i64,
}

// ============== Claude Code Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteClaudeCodeRequest {
    pub project_path: String,
    pub prompt: String,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumeClaudeCodeRequest {
    pub project_path: String,
    pub session_id: String,
    pub prompt: String,
    pub model: Option<String>,
}

// ============== Settings Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeSettings {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeVersionStatus {
    pub is_installed: bool,
    pub version: Option<String>,
    pub output: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeInstallation {
    pub path: String,
    pub version: Option<String>,
    pub source: String,
}

// ============== CLAUDE.md Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeMdFile {
    pub path: String,
    pub relative_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FindClaudeMdFilesRequest {
    pub project_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadClaudeMdFileRequest {
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveClaudeMdFileRequest {
    pub file_path: String,
    pub content: String,
}

// ============== Directory Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilesRequest {
    pub base_path: String,
    pub query: String,
}

// ============== Checkpoint Models ==============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub session_id: String,
    pub project_id: String,
    pub message_index: usize,
    pub timestamp: DateTime<Utc>,
    pub description: Option<String>,
    pub parent_checkpoint_id: Option<String>,
    pub metadata: CheckpointMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub total_tokens: u64,
    pub model_used: String,
    pub user_prompt: String,
    pub file_changes: usize,
    pub snapshot_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckpointResult {
    pub checkpoint: Checkpoint,
    pub files_processed: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheckpointRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub message_index: Option<usize>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreCheckpointRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForkFromCheckpointRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub new_session_id: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineNode {
    pub checkpoint: Checkpoint,
    pub children: Vec<TimelineNode>,
    pub file_snapshot_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionTimeline {
    pub session_id: String,
    pub root_node: Option<TimelineNode>,
    pub current_checkpoint_id: Option<String>,
    pub auto_checkpoint_enabled: bool,
    pub checkpoint_strategy: String,
    pub total_checkpoints: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckpointDiff {
    pub from_checkpoint_id: String,
    pub to_checkpoint_id: String,
    pub modified_files: Vec<FileDiff>,
    pub added_files: Vec<PathBuf>,
    pub deleted_files: Vec<PathBuf>,
    pub token_delta: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: PathBuf,
    pub additions: usize,
    pub deletions: usize,
    pub diff_content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckpointDiffRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCheckpointSettingsRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub auto_checkpoint_enabled: bool,
    pub checkpoint_strategy: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackCheckpointMessageRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub message: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackSessionMessagesRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub messages: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckpointSettings {
    pub auto_checkpoint_enabled: bool,
    pub checkpoint_strategy: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupOldCheckpointsRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub keep_count: usize,
}

// ============== MCP Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPServer {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddMCPServerRequest {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub url: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddMCPServerResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPDeleteResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddJsonRequest {
    pub name: String,
    pub json_config: String,
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportFromClaudeDesktopResult {
    pub success: bool,
    pub imported: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPProjectConfig {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveMCPProjectConfigRequest {
    pub project_path: String,
    pub config: serde_json::Value,
}

// ============== Usage Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_cost: f64,
    pub total_tokens: i64,
    pub total_requests: i64,
    pub by_model: std::collections::HashMap<String, ModelUsage>,
    pub by_project: std::collections::HashMap<String, ProjectUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectUsage {
    pub project_id: String,
    pub project_path: String,
    pub sessions: i64,
    pub total_cost: f64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageEntry {
    pub timestamp: String,
    pub project_id: String,
    pub session_id: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
    pub cache_hits: i64,
    pub cache_creations: i64,
}

// ============== GitHub Models ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubAgentFile {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub download_url: String,
}

// ============== Generic Responses ==============

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiMessage {
    pub message: String,
}

impl ApiMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
