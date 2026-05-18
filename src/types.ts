export type ServiceRuntimeStatus =
  | "idle"
  | "starting"
  | "stopping"
  | "running"
  | "failed"
  | "blocked"
  | "stopped";

export interface ServiceConfig {
  name: string;
  command: string;
  cwd?: string;
  depends_on: string[];
  mode: string;
  env: Record<string, string>;
}

export interface WorkspaceConfig {
  id: string;
  name: string;
  root: string;
  services: ServiceConfig[];
  open: string[];
  env_files: string[];
  env: Record<string, string>;
}

export interface ServiceRuntimeState {
  name: string;
  status: ServiceRuntimeStatus;
  message?: string;
}

export interface RuntimeWorkspaceState {
  workspace_id: string;
  global_status: ServiceRuntimeStatus;
  services: ServiceRuntimeState[];
  last_error?: string;
}

export interface WorkspaceStartResponse {
  workspace_id: string;
  status: RuntimeWorkspaceState;
}
