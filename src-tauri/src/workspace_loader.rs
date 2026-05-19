use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub command: String,
    pub cwd: Option<String>,
    pub depends_on: Vec<String>,
    pub mode: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub id: String,
    pub name: String,
    pub root: String,
    pub services: Vec<ServiceConfig>,
    pub open: Vec<String>,
    pub env_files: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct DockerComposeService {
    #[serde(default)]
    depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DockerComposeFile {
    #[serde(default)]
    services: HashMap<String, DockerComposeService>,
}

pub fn list_workspaces() -> Result<Vec<WorkspaceConfig>, String> {
    crate::persistence::load_workspace_catalog()
}

pub fn create_workspace(workspace: WorkspaceConfig) -> Result<WorkspaceConfig, String> {
    let name = workspace.name.trim().to_string();
    let root = workspace.root.trim().to_string();

    if name.is_empty() {
        return Err("Le nom du workspace est requis".to_string());
    }

    if root.is_empty() {
        return Err("Le chemin racine du workspace est requis".to_string());
    }

    let workspace_to_create = WorkspaceConfig {
        id: workspace.id,
        name,
        root,
        services: workspace.services,
        open: workspace.open,
        env_files: workspace.env_files,
        env: workspace.env,
    };

    crate::persistence::create_workspace(&workspace_to_create)?;
    Ok(workspace_to_create)
}

pub fn delete_workspace(workspace_id: &str) -> Result<(), String> {
    let trimmed_id = workspace_id.trim();

    if trimmed_id.is_empty() {
        return Err("L'identifiant du workspace est requis".to_string());
    }

    crate::persistence::delete_workspace(trimmed_id)
}

pub fn detect_docker_services(root: &str) -> Result<Vec<ServiceConfig>, String> {
    let trimmed_root = root.trim();
    if trimmed_root.is_empty() {
        return Ok(vec![]);
    }

    let compose_yaml_path = Path::new(trimmed_root).join("docker-compose.yaml");
    let compose_yml_path = Path::new(trimmed_root).join("docker-compose.yml");

    let compose_path = if compose_yaml_path.is_file() {
        compose_yaml_path
    } else if compose_yml_path.is_file() {
        compose_yml_path
    } else {
        return Ok(vec![]);
    };

    let content = fs::read_to_string(&compose_path)
        .map_err(|e| format!("Impossible de lire {}: {e}", compose_path.display()))?;

    let compose: DockerComposeFile = serde_yaml::from_str(&content)
        .map_err(|e| format!("docker-compose invalide ({}): {e}", compose_path.display()))?;

    let mut service_names: Vec<String> = compose.services.keys().cloned().collect();
    service_names.sort();

    let services = service_names
        .into_iter()
        .map(|name| {
            let depends_on = compose
                .services
                .get(&name)
                .map(|s| s.depends_on.clone())
                .unwrap_or_default();

            ServiceConfig {
                name: name.clone(),
                command: format!("docker compose up {name}"),
                cwd: Some(trimmed_root.to_string()),
                depends_on,
                mode: "background".to_string(),
                env: HashMap::new(),
            }
        })
        .collect();

    Ok(services)
}
