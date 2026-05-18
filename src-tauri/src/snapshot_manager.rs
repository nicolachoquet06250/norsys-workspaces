use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::orchestrator::RuntimeWorkspaceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub name: String,
    pub workspace_id: String,
    pub created_at: String,
    pub runtime: RuntimeWorkspaceState,
}

pub fn build_snapshot(workspace_id: &str, name: &str, runtime: &RuntimeWorkspaceState) -> Snapshot {
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());

    Snapshot {
        name: name.to_string(),
        workspace_id: workspace_id.to_string(),
        created_at,
        runtime: runtime.clone(),
    }
}

pub fn restore_snapshot(snapshot: &Snapshot) -> RuntimeWorkspaceState {
    snapshot.runtime.clone()
}
