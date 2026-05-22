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
  display_name?: string;
  command: string;
  cwd?: string;
  depends_on: string[];
  mode: string;
  ports?: string[];
  kind: string;
  env: Record<string, string>;
  image?: string;
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
  display_name?: string;
  status: ServiceRuntimeStatus;
  message?: string;
  last_transition?: number;
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

export interface RecentRun {
  id: number;
  workspace_id: string;
  service_name: string | null;
  action: string;
  status: string;
  created_at: string;
}

export interface SystemStats {
  cpu_usage: number;
  memory_used: number;
  memory_total: number;
  disk_used: number;
  disk_total: number;
}

export interface WorkspaceServiceImage {
  workspace_id: string;
  workspace_name: string;
  service_name: string;
  image: string;
}

export interface WorkspaceServiceVolume {
  workspace_id: string;
  workspace_name: string;
  service_name: string;
  volume: string;
  host_path: string | null;
}

export interface WorkspaceNetwork {
  workspace_id: string;
  workspace_name: string;
  service_name: string;
  network: string;
}
