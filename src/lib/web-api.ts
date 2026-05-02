/**
 * Web API client for Claudia Server
 * This is a browser-compatible version of the API that uses fetch instead of Tauri invoke
 */

const API_BASE = import.meta.env.VITE_API_URL || '';

// Type definitions (same as desktop)
export type ProcessType =
  | { AgentRun: { agent_id: number; agent_name: string } }
  | { ClaudeSession: { session_id: string } };

export interface ProcessInfo {
  run_id: number;
  process_type: ProcessType;
  pid: number;
  started_at: string;
  project_path: string;
  task: string;
  model: string;
}

export interface Project {
  id: string;
  path: string;
  sessions: string[];
  created_at: number;
}

export interface Session {
  id: string;
  project_id: string;
  project_path: string;
  todo_data?: any;
  created_at: number;
  first_message?: string;
  message_timestamp?: string;
}

export interface ClaudeSettings {
  [key: string]: any;
}

export interface ClaudeVersionStatus {
  is_installed: boolean;
  version?: string;
  output: string;
}

export interface ClaudeMdFile {
  relative_path: string;
  absolute_path: string;
  size: number;
  modified: number;
}

export interface FileEntry {
  name: string;
  path: string;
  is_directory: boolean;
  size: number;
  extension?: string;
}

export interface ClaudeInstallation {
  path: string;
  version?: string;
  source: string;
}

export interface Agent {
  id?: number;
  name: string;
  icon: string;
  system_prompt: string;
  default_task?: string;
  model: string;
  created_at: string;
  updated_at: string;
}

export interface AgentExport {
  version: number;
  exported_at: string;
  agent: {
    name: string;
    icon: string;
    system_prompt: string;
    default_task?: string;
    model: string;
  };
}

export interface GitHubAgentFile {
  name: string;
  path: string;
  download_url: string;
  size: number;
  sha: string;
}

export interface AgentRun {
  id?: number;
  agent_id: number;
  agent_name: string;
  agent_icon: string;
  task: string;
  model: string;
  project_path: string;
  session_id: string;
  status: string;
  pid?: number;
  process_started_at?: string;
  created_at: string;
  completed_at?: string;
}

export interface AgentRunMetrics {
  duration_ms?: number;
  total_tokens?: number;
  cost_usd?: number;
  message_count?: number;
}

export interface AgentRunWithMetrics extends AgentRun {
  metrics?: AgentRunMetrics;
  output?: string;
}

export interface UsageEntry {
  project: string;
  timestamp: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_write_tokens: number;
  cache_read_tokens: number;
  cost: number;
}

export interface ModelUsage {
  model: string;
  total_cost: number;
  total_tokens: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  session_count: number;
}

export interface DailyUsage {
  date: string;
  total_cost: number;
  total_tokens: number;
  models_used: string[];
}

export interface ProjectUsage {
  project_path: string;
  project_name: string;
  total_cost: number;
  total_tokens: number;
  session_count: number;
  last_used: string;
}

export interface UsageStats {
  total_cost: number;
  total_tokens: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_creation_tokens: number;
  total_cache_read_tokens: number;
  total_sessions: number;
  by_model: ModelUsage[];
  by_date: DailyUsage[];
  by_project: ProjectUsage[];
}

export interface Checkpoint {
  id: string;
  sessionId: string;
  projectId: string;
  messageIndex: number;
  timestamp: string;
  description?: string;
  parentCheckpointId?: string;
  metadata: CheckpointMetadata;
}

export interface CheckpointMetadata {
  totalTokens: number;
  modelUsed: string;
  userPrompt: string;
  fileChanges: number;
  snapshotSize: number;
}

export interface FileSnapshot {
  checkpointId: string;
  filePath: string;
  content: string;
  hash: string;
  isDeleted: boolean;
  permissions?: number;
  size: number;
}

export interface TimelineNode {
  checkpoint: Checkpoint;
  children: TimelineNode[];
  fileSnapshotIds: string[];
}

export interface SessionTimeline {
  sessionId: string;
  rootNode?: TimelineNode;
  currentCheckpointId?: string;
  autoCheckpointEnabled: boolean;
  checkpointStrategy: CheckpointStrategy;
  totalCheckpoints: number;
}

export type CheckpointStrategy = 'manual' | 'per_prompt' | 'per_tool_use' | 'smart';

export interface CheckpointResult {
  checkpoint: Checkpoint;
  filesProcessed: number;
  warnings: string[];
}

export interface CheckpointDiff {
  fromCheckpointId: string;
  toCheckpointId: string;
  modifiedFiles: FileDiff[];
  addedFiles: string[];
  deletedFiles: string[];
  tokenDelta: number;
}

export interface FileDiff {
  path: string;
  additions: number;
  deletions: number;
  diffContent?: string;
}

export interface MCPServer {
  name: string;
  transport: string;
  command?: string;
  args: string[];
  env: Record<string, string>;
  url?: string;
  scope: string;
  is_active: boolean;
  status: ServerStatus;
}

export interface ServerStatus {
  running: boolean;
  error?: string;
  last_checked?: number;
}

export interface MCPProjectConfig {
  mcpServers: Record<string, MCPServerConfig>;
}

export interface MCPServerConfig {
  command: string;
  args: string[];
  env: Record<string, string>;
}

export interface AddServerResult {
  success: boolean;
  message: string;
  server_name?: string;
}

export interface ImportResult {
  imported_count: number;
  failed_count: number;
  servers: ImportServerResult[];
}

export interface ImportServerResult {
  name: string;
  success: boolean;
  error?: string;
}

// Auth types
export interface AuthResponse {
  token: string;
  user: UserInfo;
}

export interface UserInfo {
  id: number;
  username: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  password: string;
}

// WebSocket connection
let ws: WebSocket | null = null;
let wsListeners: Map<string, Set<(data: any) => void>> = new Map();

export function connectWebSocket(onMessage: (data: any) => void) {
  const wsUrl = API_BASE.replace(/^http/, 'ws') + '/ws';
  ws = new WebSocket(wsUrl);

  ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      onMessage(data);

      // Notify registered listeners
      const listeners = wsListeners.get(data.type);
      if (listeners) {
        listeners.forEach((callback) => callback(data));
      }
    } catch (e) {
      console.error('Failed to parse WebSocket message:', e);
    }
  };

  ws.onerror = (error) => {
    console.error('WebSocket error:', error);
  };

  ws.onclose = () => {
    console.log('WebSocket disconnected');
    // Attempt to reconnect after 5 seconds
    setTimeout(() => {
      if (!ws || ws.readyState === WebSocket.CLOSED) {
        connectWebSocket(onMessage);
      }
    }, 5000);
  };
}

export function disconnectWebSocket() {
  if (ws) {
    ws.close();
    ws = null;
  }
}

export function subscribeToRun(runId: number, callback: (data: any) => void) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({
      type: 'subscribe',
      run_id: runId,
    }));
  }

  const key = `run-${runId}`;
  if (!wsListeners.has(key)) {
    wsListeners.set(key, new Set());
  }
  wsListeners.get(key)!.add(callback);
}

export function unsubscribeFromRun(runId: number, callback: (data: any) => void) {
  const key = `run-${runId}`;
  const listeners = wsListeners.get(key);
  if (listeners) {
    listeners.delete(callback);
  }
}

// Helper function for making API requests
async function request<T>(
  method: string,
  endpoint: string,
  body?: any,
  queryParams?: Record<string, string>
): Promise<T> {
  let url = `${API_BASE}/api${endpoint}`;

  if (queryParams) {
    const params = new URLSearchParams(queryParams);
    url += `?${params.toString()}`;
  }

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  // Add auth token if available
  const token = localStorage.getItem('claudia_token');
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const options: RequestInit = {
    method,
    headers,
  };

  if (body !== undefined) {
    options.body = JSON.stringify(body);
  }

  const response = await fetch(url, options);

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Unknown error' }));
    throw new Error(error.error || error.message || 'Request failed');
  }

  // Handle different response types
  const text = await response.text();
  if (!text) return undefined as T;

  try {
    return JSON.parse(text);
  } catch {
    return text as T;
  }
}

// Main API client
export const webApi = {
  // Auth
  async login(username: string, password: string): Promise<AuthResponse> {
    return request<AuthResponse>('POST', '/auth/login', { username, password });
  },

  async register(username: string, password: string): Promise<AuthResponse> {
    return request<AuthResponse>('POST', '/auth/register', { username, password });
  },

  async logout(): Promise<void> {
    await request('POST', '/auth/logout');
    localStorage.removeItem('claudia_token');
  },

  async me(): Promise<UserInfo> {
    return request<UserInfo>('GET', '/auth/me');
  },

  // Projects
  async listProjects(): Promise<Project[]> {
    return request<Project[]>('GET', '/projects');
  },

  async getProjectSessions(projectId: string): Promise<string[]> {
    return request<string[]>('GET', `/projects/${projectId}/sessions`);
  },

  async loadSessionHistory(sessionId: string, projectId: string): Promise<any[]> {
    const content = await request<string>('GET', `/projects/${projectId}/sessions/${sessionId}/history`);
    return content.split('\n').filter(line => line.trim()).map(line => JSON.parse(line));
  },

  // Directories
  async listDirectoryContents(directoryPath: string): Promise<FileEntry[]> {
    return request<FileEntry[]>('GET', '/directories', undefined, { path: directoryPath });
  },

  async searchFiles(basePath: string, query: string): Promise<FileEntry[]> {
    return request<FileEntry[]>('GET', '/search', { base_path: basePath, query });
  },

  // Claude Code
  async executeClaudeCode(projectPath: string, prompt: string, model: string): Promise<void> {
    await request('POST', '/claude/execute', { project_path: projectPath, prompt, model });
  },

  async continueClaudeCode(projectPath: string, prompt: string, model: string): Promise<void> {
    await request('POST', '/claude/continue', { project_path: projectPath, prompt, model });
  },

  async resumeClaudeCode(projectPath: string, sessionId: string, prompt: string, model: string): Promise<void> {
    await request('POST', '/claude/resume', { project_path: projectPath, session_id: sessionId, prompt, model });
  },

  async cancelClaudeExecution(): Promise<void> {
    await request('POST', '/claude/cancel');
  },

  async listRunningClaudeSessions(): Promise<any[]> {
    return request('GET', '/claude/sessions');
  },

  async getClaudeSessionOutput(sessionId: string): Promise<string> {
    return request('GET', `/claude/output/${sessionId}`);
  },

  // Settings
  async getClaudeSettings(): Promise<ClaudeSettings> {
    return request('GET', '/settings');
  },

  async saveClaudeSettings(settings: ClaudeSettings): Promise<void> {
    await request('PUT', '/settings', settings);
  },

  async getSystemPrompt(): Promise<string> {
    return request('GET', '/settings/system-prompt');
  },

  async saveSystemPrompt(content: string): Promise<void> {
    await request('PUT', '/settings/system-prompt', content);
  },

  async checkClaudeVersion(): Promise<ClaudeVersionStatus> {
    return request('GET', '/settings/claude-version');
  },

  async getClaudeBinaryPath(): Promise<string | null> {
    return request('GET', '/settings/claude-path');
  },

  async setClaudeBinaryPath(path: string): Promise<void> {
    await request('PUT', '/settings/claude-path', path);
  },

  async listClaudeInstallations(): Promise<ClaudeInstallation[]> {
    return request('GET', '/settings/claude-installations');
  },

  // CLAUDE.md
  async findClaudeMdFiles(projectPath: string): Promise<ClaudeMdFile[]> {
    return request('POST', '/claude-md/find', { project_path: projectPath });
  },

  async readClaudeMdFile(filePath: string): Promise<string> {
    return request('GET', '/claude-md/read', undefined, { path: filePath });
  },

  async saveClaudeMdFile(filePath: string, content: string): Promise<void> {
    await request('POST', '/claude-md/save', { file_path: filePath, content });
  },

  // Agents
  async listAgents(): Promise<Agent[]> {
    return request('GET', '/agents');
  },

  async createAgent(
    name: string,
    icon: string,
    system_prompt: string,
    default_task?: string,
    model?: string,
    enable_file_read?: boolean,
    enable_file_write?: boolean,
    enable_network?: boolean
  ): Promise<Agent> {
    return request('POST', '/agents', {
      name,
      icon,
      system_prompt,
      default_task,
      model,
      enable_file_read,
      enable_file_write,
      enable_network,
    });
  },

  async updateAgent(
    id: number,
    name: string,
    icon: string,
    system_prompt: string,
    default_task?: string,
    model?: string
  ): Promise<Agent> {
    return request('PUT', `/agents/${id}`, { name, icon, system_prompt, default_task, model });
  },

  async deleteAgent(id: number): Promise<void> {
    await request('DELETE', `/agents/${id}`);
  },

  async getAgent(id: number): Promise<Agent> {
    return request('GET', `/agents/${id}`);
  },

  async exportAgent(id: number): Promise<string> {
    return request('GET', `/agents/${id}/export`);
  },

  async importAgent(jsonData: string): Promise<Agent> {
    return request('POST', '/agents/import', jsonData);
  },

  // Agent Execution
  async executeAgent(agentId: number, projectPath: string, task: string, model?: string): Promise<{ run_id: number }> {
    return request('POST', `/agents/${agentId}/execute`, { project_path: projectPath, task, model });
  },

  async listAgentRuns(agentId?: number): Promise<AgentRunWithMetrics[]> {
    const endpoint = agentId ? `/agents/${agentId}/runs` : '/runs';
    return request('GET', endpoint);
  },

  async getAgentRun(id: number): Promise<AgentRunWithMetrics> {
    return request('GET', `/runs/${id}`);
  },

  async getAgentRunWithRealTimeMetrics(id: number): Promise<AgentRunWithMetrics> {
    return request('GET', `/runs/${id}/realtime`);
  },

  async listRunningSessions(): Promise<AgentRun[]> {
    return request('GET', '/runs');
  },

  async killAgentSession(runId: number): Promise<void> {
    await request('POST', `/runs/${runId}/kill`);
  },

  async getSessionStatus(runId: number): Promise<string> {
    return request('GET', `/runs/${runId}/status`);
  },

  async getSessionOutput(runId: number): Promise<string> {
    return request('GET', `/runs/${runId}/output`);
  },

  async getLiveSessionOutput(runId: number): Promise<string> {
    return request('GET', `/runs/${runId}/live-output`);
  },

  async cleanupFinishedProcesses(): Promise<number> {
    return request('POST', '/runs/cleanup');
  },

  // GitHub Agents
  async fetchGitHubAgents(): Promise<GitHubAgentFile[]> {
    return request('GET', '/github-agents');
  },

  async fetchGitHubAgentContent(downloadUrl: string): Promise<AgentExport> {
    return request('GET', '/github-agents/content', undefined, { url: downloadUrl });
  },

  async importAgentFromGitHub(downloadUrl: string): Promise<Agent> {
    return request('POST', '/github-agents/import', downloadUrl);
  },

  // Checkpoints
  async createCheckpoint(
    sessionId: string,
    projectId: string,
    projectPath: string,
    messageIndex?: number,
    description?: string
  ): Promise<CheckpointResult> {
    return request('POST', '/checkpoints', {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
      message_index: messageIndex,
      description,
    });
  },

  async restoreCheckpoint(
    checkpointId: string,
    sessionId: string,
    projectId: string,
    projectPath: string
  ): Promise<CheckpointResult> {
    return request('POST', `/checkpoints/${checkpointId}/restore`, {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
    });
  },

  async listCheckpoints(sessionId: string, projectId: string, projectPath: string): Promise<Checkpoint[]> {
    return request('GET', '/checkpoints/list', undefined, { session_id: sessionId, project_id: projectId, project_path: projectPath });
  },

  async forkFromCheckpoint(
    checkpointId: string,
    sessionId: string,
    projectId: string,
    projectPath: string,
    newSessionId: string,
    description?: string
  ): Promise<CheckpointResult> {
    return request('POST', `/checkpoints/${checkpointId}/fork`, {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
      new_session_id: newSessionId,
      description,
    });
  },

  async getSessionTimeline(sessionId: string, projectId: string, projectPath: string): Promise<SessionTimeline> {
    return request('GET', '/checkpoints/timeline', undefined, { session_id: sessionId, project_id: projectId, project_path: projectPath });
  },

  async updateCheckpointSettings(
    sessionId: string,
    projectId: string,
    projectPath: string,
    autoCheckpointEnabled: boolean,
    checkpointStrategy: CheckpointStrategy
  ): Promise<void> {
    await request('PUT', '/checkpoints/settings', {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
      auto_checkpoint_enabled: autoCheckpointEnabled,
      checkpoint_strategy: checkpointStrategy,
    });
  },

  async getCheckpointDiff(
    fromCheckpointId: string,
    toCheckpointId: string,
    sessionId: string,
    projectId: string
  ): Promise<CheckpointDiff> {
    return request('GET', `/checkpoints/${fromCheckpointId}/diff/${toCheckpointId}`, undefined, { session_id: sessionId, project_id: projectId });
  },

  async trackCheckpointMessage(
    sessionId: string,
    projectId: string,
    projectPath: string,
    message: any
  ): Promise<void> {
    await request('POST', '/checkpoints/track', {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
      message,
    });
  },

  async trackSessionMessages(
    sessionId: string,
    projectId: string,
    projectPath: string,
    messages: any[]
  ): Promise<void> {
    await request('POST', '/checkpoints/track/batch', {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
      messages,
    });
  },

  async checkAutoCheckpoint(
    sessionId: string,
    projectId: string,
    projectPath: string,
    message: any
  ): Promise<boolean> {
    return request('POST', '/checkpoints/auto-check', {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
      message,
    });
  },

  async cleanupOldCheckpoints(
    sessionId: string,
    projectId: string,
    projectPath: string,
    keepCount: number
  ): Promise<number> {
    return request('POST', '/checkpoints/cleanup', {
      session_id: sessionId,
      project_id: projectId,
      project_path: projectPath,
      keep_count: keepCount,
    });
  },

  async getCheckpointSettings(sessionId: string, projectId: string, projectPath: string): Promise<any> {
    return request('GET', '/checkpoints/settings/get', undefined, { session_id: sessionId, project_id: projectId, project_path: projectPath });
  },

  async clearCheckpointManager(sessionId: string): Promise<void> {
    await request('POST', '/checkpoints/clear', undefined, { session_id: sessionId });
  },

  // MCP
  async mcpAdd(
    name: string,
    transport: string,
    command?: string,
    args?: string[],
    env?: Record<string, string>,
    url?: string,
    scope?: string
  ): Promise<AddServerResult> {
    return request('POST', '/mcp/add', {
      name,
      transport,
      command,
      args,
      env,
      url,
      scope,
    });
  },

  async mcpList(): Promise<any[]> {
    return request('GET', '/mcp/list');
  },

  async mcpGet(name: string): Promise<any> {
    return request('GET', `/mcp/${name}`);
  },

  async mcpRemove(name: string): Promise<{ success: boolean; message: string }> {
    return request('DELETE', `/mcp/${name}`);
  },

  async mcpAddJson(name: string, jsonConfig: string, scope?: string): Promise<AddServerResult> {
    return request('POST', '/mcp/add-json', { name, json_config: jsonConfig, scope });
  },

  async mcpAddFromClaudeDesktop(): Promise<ImportResult> {
    return request('POST', '/mcp/from-claude-desktop');
  },

  async mcpServe(): Promise<void> {
    await request('POST', '/mcp/serve');
  },

  async mcpTestConnection(name: string): Promise<string> {
    return request('GET', `/mcp/test/${name}`);
  },

  async mcpResetProjectChoices(): Promise<void> {
    await request('POST', '/mcp/reset-choices');
  },

  async mcpGetServerStatus(): Promise<Record<string, ServerStatus>> {
    return request('GET', '/mcp/status');
  },

  async mcpReadProjectConfig(projectPath: string): Promise<MCPProjectConfig> {
    return request('GET', '/mcp/project-config', undefined, { project_path: projectPath });
  },

  async mcpSaveProjectConfig(projectPath: string, config: MCPProjectConfig): Promise<void> {
    await request('POST', '/mcp/project-config', { project_path: projectPath, config });
  },

  // Usage
  async getUsageStats(): Promise<UsageStats> {
    return request('GET', '/usage/stats');
  },

  async getUsageByDateRange(startDate: string, endDate: string): Promise<UsageStats> {
    return request('GET', '/usage/by-date-range', undefined, { start_date: startDate, end_date: endDate });
  },

  async getUsageDetails(limit?: number): Promise<UsageEntry[]> {
    return request('GET', '/usage/details', undefined, limit ? { limit: limit.toString() } : undefined);
  },

  async getSessionStats(since?: string): Promise<ProjectUsage[]> {
    return request('GET', '/usage/sessions', undefined, since ? { since } : undefined);
  },
};

// Export alias for compatibility with desktop code
export const api = webApi;
