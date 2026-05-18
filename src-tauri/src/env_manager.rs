use std::collections::HashMap;

use crate::workspace_loader::WorkspaceConfig;

pub fn resolve_workspace_env(workspace: &WorkspaceConfig) -> HashMap<String, String> {
    let mut resolved = HashMap::new();

    for (key, value) in &workspace.env {
        resolved.insert(key.clone(), value.clone());
    }

    resolved.insert("WORKSPACE_NAME".to_string(), workspace.name.clone());
    resolved.insert("WORKSPACE_ROOT".to_string(), workspace.root.clone());

    resolved
}
