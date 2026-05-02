pub mod models;
pub mod handlers;
pub mod routes;
pub mod ws;

use std::sync::Arc;
use axum::{Router};
use tower_http::cors::CorsLayer;

pub use handlers::*;
pub use models::*;

pub fn routes(state: Arc<crate::state::AppState>) -> Router {
    let cors = CorsLayer::permissive();

    Router::new()
        // Auth routes
        .route("/auth/login", axum::routing::post(login))
        .route("/auth/logout", axum::routing::post(logout))
        .route("/auth/register", axum::routing::post(register))
        .route("/auth/me", axum::routing::get(me))

        // Project routes
        .route("/projects", axum::routing::get(list_projects))
        .route("/projects/:id/sessions", axum::routing::get(get_project_sessions))
        .route("/projects/:id/sessions/:session_id/history", axum::routing::get(load_session_history))

        // Directory routes
        .route("/directories", axum::routing::get(list_directory_contents))
        .route("/search", axum::routing::get(search_files))

        // Claude Code routes
        .route("/claude/execute", axum::routing::post(execute_claude_code))
        .route("/claude/continue", axum::routing::post(continue_claude_code))
        .route("/claude/resume", axum::routing::post(resume_claude_code))
        .route("/claude/cancel", axum::routing::post(cancel_claude_execution))
        .route("/claude/sessions", axum::routing::get(list_running_claude_sessions))
        .route("/claude/output/:session_id", axum::routing::get(get_claude_session_output))

        // Settings routes
        .route("/settings", axum::routing::get(get_claude_settings))
        .route("/settings", axum::routing::put(save_claude_settings))
        .route("/settings/system-prompt", axum::routing::get(get_system_prompt))
        .route("/settings/system-prompt", axum::routing::put(save_system_prompt))
        .route("/settings/claude-version", axum::routing::get(check_claude_version))
        .route("/settings/claude-path", axum::routing::get(get_claude_binary_path))
        .route("/settings/claude-path", axum::routing::put(set_claude_binary_path))
        .route("/settings/claude-installations", axum::routing::get(list_claude_installations))

        // CLAUDE.md routes
        .route("/claude-md/find", axum::routing::post(find_claude_md_files))
        .route("/claude-md/read", axum::routing::get(read_claude_md_file))
        .route("/claude-md/save", axum::routing::post(save_claude_md_file))

        // Agent routes
        .route("/agents", axum::routing::get(list_agents))
        .route("/agents", axum::routing::post(create_agent))
        .route("/agents/:id", axum::routing::get(get_agent))
        .route("/agents/:id", axum::routing::put(update_agent))
        .route("/agents/:id", axum::routing::delete(delete_agent))
        .route("/agents/:id/export", axum::routing::get(export_agent))
        .route("/agents/import", axum::routing::post(import_agent))

        // Agent execution routes
        .route("/agents/:id/execute", axum::routing::post(execute_agent))
        .route("/agents/:id/runs", axum::routing::get(list_agent_runs))
        .route("/runs/:id", axum::routing::get(get_agent_run))
        .route("/runs/:id/realtime", axum::routing::get(get_agent_run_with_metrics))
        .route("/runs", axum::routing::get(list_running_sessions))
        .route("/runs/:id/kill", axum::routing::post(kill_agent_session))
        .route("/runs/:id/status", axum::routing::get(get_session_status))
        .route("/runs/:id/output", axum::routing::get(get_session_output))
        .route("/runs/:id/live-output", axum::routing::get(get_live_session_output))
        .route("/runs/cleanup", axum::routing::post(cleanup_finished_processes))

        // GitHub agents
        .route("/github-agents", axum::routing::get(fetch_github_agents))
        .route("/github-agents/content", axum::routing::get(fetch_github_agent_content))
        .route("/github-agents/import", axum::routing::post(import_agent_from_github))

        // Checkpoint routes
        .route("/checkpoints", axum::routing::post(create_checkpoint))
        .route("/checkpoints/:checkpoint_id/restore", axum::routing::post(restore_checkpoint))
        .route("/checkpoints/list", axum::routing::get(list_checkpoints))
        .route("/checkpoints/:checkpoint_id/fork", axum::routing::post(fork_from_checkpoint))
        .route("/checkpoints/timeline", axum::routing::get(get_session_timeline))
        .route("/checkpoints/settings", axum::routing::put(update_checkpoint_settings))
        .route("/checkpoints/:from/diff/:to", axum::routing::get(get_checkpoint_diff))
        .route("/checkpoints/track", axum::routing::post(track_checkpoint_message))
        .route("/checkpoints/track/batch", axum::routing::post(track_session_messages))
        .route("/checkpoints/auto-check", axum::routing::post(check_auto_checkpoint))
        .route("/checkpoints/cleanup", axum::routing::post(cleanup_old_checkpoints))
        .route("/checkpoints/settings/get", axum::routing::get(get_checkpoint_settings))
        .route("/checkpoints/clear", axum::routing::post(clear_checkpoint_manager))
        .route("/checkpoints/stats", axum::routing::get(get_checkpoint_state_stats))

        // MCP routes
        .route("/mcp/add", axum::routing::post(mcp_add))
        .route("/mcp/list", axum::routing::get(mcp_list))
        .route("/mcp/:name", axum::routing::get(mcp_get))
        .route("/mcp/:name", axum::routing::delete(mcp_remove))
        .route("/mcp/add-json", axum::routing::post(mcp_add_json))
        .route("/mcp/from-claude-desktop", axum::routing::post(mcp_add_from_claude_desktop))
        .route("/mcp/serve", axum::routing::post(mcp_serve))
        .route("/mcp/test/:name", axum::routing::get(mcp_test_connection))
        .route("/mcp/reset-choices", axum::routing::post(mcp_reset_project_choices))
        .route("/mcp/status", axum::routing::get(mcp_get_server_status))
        .route("/mcp/project-config", axum::routing::get(mcp_read_project_config))
        .route("/mcp/project-config", axum::routing::post(mcp_save_project_config))

        // Usage routes
        .route("/usage/stats", axum::routing::get(get_usage_stats))
        .route("/usage/by-date-range", axum::routing::get(get_usage_by_date_range))
        .route("/usage/details", axum::routing::get(get_usage_details))
        .route("/usage/sessions", axum::routing::get(get_session_stats))

        .layer(cors)
        .with_state(state)
}
