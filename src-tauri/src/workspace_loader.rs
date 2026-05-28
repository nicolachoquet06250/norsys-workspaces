use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use bollard::Docker;
use bollard::query_parameters::ListContainersOptionsBuilder;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceConfig {
    pub name: String,
    pub display_name: Option<String>,
    pub command: String,
    pub cwd: Option<String>,
    pub depends_on: Vec<String>,
    pub mode: String,
    pub kind: String,
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub image: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceServiceImage {
    pub workspace_id: String,
    pub workspace_name: String,
    pub service_name: String,
    pub image: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceServiceVolume {
    pub workspace_id: String,
    pub workspace_name: String,
    pub service_name: String,
    pub volume: String,
    pub host_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceNetwork {
    pub workspace_id: String,
    pub workspace_name: String,
    pub service_name: String,
    pub network: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DockerComposeVolume {
    Short(String),
    Long {
        #[serde(default)]
        source: Option<String>,
        #[serde(default)]
        target: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct DockerComposeService {
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    volumes: Vec<DockerComposeVolume>,
    #[serde(default)]
    ports: Vec<DockerComposePort>,
    #[serde(default)]
    labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DockerComposePort {
    Short(String),
    Long {
        #[serde(default)]
        published: Option<serde_yaml::Value>,
        #[serde(default)]
        target: Option<serde_yaml::Value>,
    },
}

#[derive(Debug, Deserialize)]
struct DockerComposeFile {
    #[serde(default)]
    services: HashMap<String, DockerComposeService>,
}

#[derive(Debug, Deserialize)]
struct DockerApiContainer {
    #[serde(default)]
    names: Vec<String>,
    #[serde(default)]
    labels: HashMap<String, String>,
    #[serde(default)]
    ports: Vec<DockerApiPort>,
    #[serde(default, rename = "Mounts")]
    mounts: Vec<DockerApiMount>,
    #[serde(default, rename = "NetworkSettings")]
    network_settings: Option<DockerApiNetworkSettings>,
}

#[derive(Debug, Deserialize)]
struct DockerApiNetworkSettings {
    #[serde(default, rename = "Networks")]
    networks: HashMap<String, DockerApiNetwork>,
}

#[derive(Debug, Deserialize)]
struct DockerApiNetwork {
    #[serde(default, rename = "NetworkID")]
    network_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DockerApiPort {
    #[serde(default, rename = "IP")]
    ip: Option<String>,
    #[serde(default, rename = "PrivatePort")]
    private_port: Option<u16>,
    #[serde(default, rename = "PublicPort")]
    public_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct DockerApiMount {
    #[serde(default, rename = "Source")]
    source: String,
    #[serde(default, rename = "Destination")]
    destination: String,
}

pub fn to_human_friendly(name: &str) -> String {
    let parts: Vec<&str> = if name.contains('-') {
        name.split('-').collect()
    } else if name.contains('_') {
        name.split('_').collect()
    } else {
        vec![name]
    };

    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn list_workspaces() -> Result<Vec<WorkspaceConfig>, String> {
    let mut workspaces = crate::persistence::load_workspace_catalog()?;
    let mut changed = false;

    for workspace in &mut workspaces {
        if let Ok(detected_services) = detect_docker_services(&workspace.root) {
            if !detected_services.is_empty() && detected_services != workspace.services {
                workspace.services = detected_services;
                changed = true;
            }
        }
    }

    if changed {
        crate::persistence::upsert_workspace_catalog(&workspaces)?;
    }

    Ok(workspaces)
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

    if compose_yaml_path.is_file() || compose_yml_path.is_file() {
        let compose_path = if compose_yaml_path.is_file() {
            compose_yaml_path
        } else {
            compose_yml_path
        };

        let content = fs::read_to_string(&compose_path)
            .map_err(|e| format!("Impossible de lire {}: {e}", compose_path.display()))?;

        let compose: DockerComposeFile = serde_yaml::from_str(&content)
            .map_err(|e| format!("docker-compose invalide ({}): {e}", compose_path.display()))?;

        let mut service_names: Vec<String> = compose.services.keys().cloned().collect();
        service_names.sort();

        let docker_ports_by_service = docker_ports_for_workspace(trimmed_root).unwrap_or_default();

        let services = service_names
            .into_iter()
            .map(|name| {
                let ports = docker_ports_by_service
                    .get(&name)
                    .cloned()
                    .unwrap_or_default();

                let compose_service = compose.services.get(&name);
                let depends_on = compose_service
                    .map(|s| s.depends_on.clone())
                    .unwrap_or_default();

                let kind = compose_service
                    .and_then(|s| s.labels.get("dev.workspace.manager.kind"))
                    .cloned()
                    .unwrap_or_else(|| "web".to_string());

                let image = compose_service
                    .and_then(|s| s.image.clone());

                ServiceConfig {
                    name: name.clone(),
                    display_name: Some(to_human_friendly(&name)),
                    command: format!("docker compose up {name}"),
                    cwd: Some(trimmed_root.to_string()),
                    depends_on,
                    mode: "background".to_string(),
                    kind,
                    env: HashMap::new(),
                    ports,
                    image,
                }
            })
            .collect();

        return Ok(services);
    }

    // Détection de projets par fichiers caractéristiques
    let mut services = Vec::new();

    // Node.js (package.json)
    let package_json_path = Path::new(trimmed_root).join("package.json");
    if package_json_path.is_file() {
        if let Ok(content) = fs::read_to_string(&package_json_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                let mut kind = "node".to_string();
                if let Some(deps) = v.get("dependencies").and_then(|d| d.as_object()) {
                    if deps.contains_key("vue") {
                        kind = "vue".to_string();
                    } else if deps.contains_key("react") {
                        kind = "react".to_string();
                    } else if deps.contains_key("svelte") {
                        kind = "svelte".to_string();
                    }
                }
                if let Some(dev_deps) = v.get("devDependencies").and_then(|d| d.as_object()) {
                    if kind == "node" {
                        if dev_deps.contains_key("vue") {
                            kind = "vue".to_string();
                        } else if dev_deps.contains_key("react") {
                            kind = "react".to_string();
                        } else if dev_deps.contains_key("svelte") {
                            kind = "svelte".to_string();
                        }
                    }
                }

                services.push(ServiceConfig {
                    name: "app".to_string(),
                    display_name: Some(format!("App ({})", kind)),
                    command: "npm run dev".to_string(),
                    cwd: Some(trimmed_root.to_string()),
                    depends_on: vec![],
                    mode: "process".to_string(),
                    kind,
                    env: HashMap::new(),
                    ports: vec![],
                    image: None,
                });
            }
        }
    }

    // PHP (composer.json)
    let composer_json_path = Path::new(trimmed_root).join("composer.json");
    if composer_json_path.is_file() {
        let mut kind = "php".to_string();
        if let Ok(content) = fs::read_to_string(&composer_json_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(require) = v.get("require").and_then(|r| r.as_object()) {
                    if require.contains_key("laravel/framework") {
                        kind = "laravel".to_string();
                    } else if require.contains_key("symfony/symfony") || require.contains_key("symfony/framework-bundle") {
                        kind = "symfony".to_string();
                    }
                }
            }
        }
        services.push(ServiceConfig {
            name: "php".to_string(),
            display_name: Some(format!("App ({})", kind)),
            command: "php -S localhost:8000".to_string(),
            cwd: Some(trimmed_root.to_string()),
            depends_on: vec![],
            mode: "process".to_string(),
            kind,
            env: HashMap::new(),
            ports: vec!["8000".to_string()],
            image: None,
        });
    }

    // Rust (Cargo.toml)
    let cargo_toml_path = Path::new(trimmed_root).join("Cargo.toml");
    if cargo_toml_path.is_file() {
        services.push(ServiceConfig {
            name: "rust".to_string(),
            display_name: Some("Rust App".to_string()),
            command: "cargo run".to_string(),
            cwd: Some(trimmed_root.to_string()),
            depends_on: vec![],
            mode: "process".to_string(),
            kind: "rust".to_string(),
            env: HashMap::new(),
            ports: vec![],
            image: None,
        });
    }

    // Java (pom.xml)
    let pom_xml_path = Path::new(trimmed_root).join("pom.xml");
    if pom_xml_path.is_file() {
        services.push(ServiceConfig {
            name: "java".to_string(),
            display_name: Some("Java App".to_string()),
            command: "mvn spring-boot:run".to_string(),
            cwd: Some(trimmed_root.to_string()),
            depends_on: vec![],
            mode: "process".to_string(),
            kind: "java".to_string(),
            env: HashMap::new(),
            ports: vec![],
            image: None,
        });
    }

    Ok(services)
}

pub fn list_workspace_service_images() -> Result<Vec<WorkspaceServiceImage>, String> {
    let workspaces = crate::persistence::load_workspace_catalog()?;
    let mut images = Vec::new();

    for workspace in workspaces {
        for service in &workspace.services {
            if let Some(image) = service.image.as_ref().map(|i| i.trim()).filter(|i| !i.is_empty()) {
                images.push(WorkspaceServiceImage {
                    workspace_id: workspace.id.clone(),
                    workspace_name: workspace.name.clone(),
                    service_name: service.display_name.clone().unwrap_or_else(|| service.name.clone()),
                    image: image.to_string(),
                });
            }
        }
    }

    Ok(images)
}

fn parse_short_volume_entry(value: &str) -> Option<(String, Option<String>)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if !trimmed.contains(':') {
        return Some((trimmed.to_string(), None));
    }

    let mut split_at = trimmed.rfind(':')?;
    let before_last = &trimmed[..split_at];
    let after_last = trimmed[split_at + 1..].trim();

    let looks_like_mode = !after_last.is_empty()
        && after_last
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ',' || c == '-');

    if looks_like_mode {
        if let Some(prev_colon) = before_last.rfind(':') {
            split_at = prev_colon;
        }
    }

    let source = trimmed[..split_at].trim().to_string();
    let target = trimmed[split_at + 1..]
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();

    if target.is_empty() {
        return None;
    }

    if source.is_empty() {
        return Some((target, None));
    }

    Some((target, Some(source)))
}

fn connect_docker_client() -> Option<Docker> {
    let docker_host = env::var("DOCKER_HOST").unwrap_or_else(|_| {
        if cfg!(target_os = "windows") {
            "tcp://127.0.0.1:2375".to_string()
        } else {
            "unix:///var/run/docker.sock".to_string()
        }
    });

    if docker_host.starts_with("tcp://") {
        let http_url = docker_host.replacen("tcp://", "http://", 1);
        Docker::connect_with_http(&http_url, 2, bollard::API_DEFAULT_VERSION).ok()
    } else {
        Docker::connect_with_local_defaults().ok()
    }
}

fn fetch_docker_containers() -> Option<Vec<DockerApiContainer>> {
    let docker = connect_docker_client()?;
    let runtime = Runtime::new().ok()?;

    let containers = runtime
        .block_on(async {
            docker
                .list_containers(Some(
                    ListContainersOptionsBuilder::default()
                        .all(true)
                        .build(),
                ))
                .await
                .ok()
        })?;

    Some(
        containers
            .into_iter()
            .map(|container| DockerApiContainer {
                names: container.names.unwrap_or_default(),
                labels: container.labels.unwrap_or_default(),
                ports: container
                    .ports
                    .unwrap_or_default()
                    .into_iter()
                    .map(|port| DockerApiPort {
                        ip: port.ip,
                        private_port: Some(port.private_port),
                        public_port: port.public_port,
                    })
                    .collect(),
                mounts: container
                    .mounts
                    .unwrap_or_default()
                    .into_iter()
                    .map(|mount| DockerApiMount {
                        source: mount.source.unwrap_or_default(),
                        destination: mount.destination.unwrap_or_default(),
                    })
                    .collect(),
                network_settings: container.network_settings.map(|ns| DockerApiNetworkSettings {
                    networks: ns
                        .networks
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(name, network)| {
                            (
                                name,
                                DockerApiNetwork {
                                    network_id: network.network_id,
                                },
                            )
                        })
                        .collect(),
                }),
            })
            .collect(),
    )
}

fn normalize_workspace_root(root: &str) -> String {
    let mut normalized = root
        .trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase();

    for prefix in ["/run/desktop/mnt/host/", "/host_mnt/", "/mnt/"] {
        if let Some(remainder) = normalized.strip_prefix(prefix) {
            if let Some((drive, path)) = remainder.split_once('/') {
                if drive.len() == 1 && drive.chars().all(|c| c.is_ascii_alphabetic()) {
                    normalized = format!("{}:/{}", drive, path.trim_start_matches('/'));
                    break;
                }
            }
        }
    }

    normalized
}

fn clean_container_name(name: &str) -> String {
    name.trim_start_matches('/').to_string()
}

fn strip_trailing_container_index(name: &str) -> &str {
    if let Some((base, suffix)) = name.rsplit_once('-') {
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            return base;
        }
    }

    if let Some((base, suffix)) = name.rsplit_once('_') {
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            return base;
        }
    }

    name
}

fn extract_service_name_from_container(container: &DockerApiContainer) -> Option<String> {
    container
        .labels
        .get("com.docker.compose.service")
        .cloned()
        .or_else(|| {
            let compose_project = container
                .labels
                .get("com.docker.compose.project")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());

            container
                .names
                .iter()
                .map(|name| clean_container_name(name))
                .find_map(|clean_name| {
                    if let Some(project) = compose_project.as_ref() {
                        for separator in ['-', '_'] {
                            let prefix = format!("{project}{separator}");
                            if clean_name.starts_with(&prefix) {
                                let remainder = clean_name.trim_start_matches(&prefix);
                                let service_name = strip_trailing_container_index(remainder).trim();
                                if !service_name.is_empty() {
                                    return Some(service_name.to_string());
                                }
                            }
                        }
                    }

                    let service_name = strip_trailing_container_index(&clean_name).trim();
                    if service_name.is_empty() {
                        None
                    } else {
                        Some(service_name.to_string())
                    }
                })
        })
}

fn container_belongs_to_workspace(container: &DockerApiContainer, workspace_root: &str) -> bool {
    let normalized_workspace = normalize_workspace_root(workspace_root);
    if normalized_workspace.is_empty() {
        return false;
    }

    let label_keys = [
        "com.docker.compose.project.working_dir",
        "com.docker.compose.project.config_files",
    ];

    label_keys.iter().any(|key| {
        container
            .labels
            .get(*key)
            .map(|value| normalize_workspace_root(value).contains(&normalized_workspace))
            .unwrap_or(false)
    })
}

fn docker_ports_for_workspace(workspace_root: &str) -> Option<HashMap<String, Vec<String>>> {
    let containers = fetch_docker_containers()?;
    let mut ports_by_service = HashMap::new();

    for container in containers {
        if !container_belongs_to_workspace(&container, workspace_root) {
            continue;
        }
        let Some(service_name) = extract_service_name_from_container(&container) else {
            continue;
        };

        let mut ports = container
            .ports
            .into_iter()
            .filter_map(|port| port.public_port.map(|public| public.to_string()))
            .collect::<Vec<_>>();

        ports.sort();
        ports.dedup();
        ports_by_service.insert(service_name, ports);
    }

    Some(ports_by_service)
}

fn docker_mounts_for_workspace_service(
    workspace_root: &str,
    service_name: &str,
) -> Option<HashMap<String, String>> {
    let containers = fetch_docker_containers()?;
    let mut mounts = HashMap::new();

    for container in containers {
        if !container_belongs_to_workspace(&container, workspace_root) {
            continue;
        }

        let Some(container_service_name) = extract_service_name_from_container(&container) else {
            continue;
        };

        if !container_service_name.eq_ignore_ascii_case(service_name) {
            continue;
        }

        for mount in container.mounts {
            let destination = mount.destination.trim();
            let source = mount.source.trim();
            if destination.is_empty() || source.is_empty() {
                continue;
            }
            mounts.insert(destination.to_string(), normalize_volume_path(source));
        }
    }

    Some(mounts)
}

fn resolve_host_path(workspace_root: &str, source: Option<&str>) -> Option<String> {
    let raw_source = source?.trim();
    if raw_source.is_empty() {
        return None;
    }

    let is_path_like = raw_source.starts_with("./")
        || raw_source.starts_with("../")
        || raw_source.starts_with('/')
        || raw_source.starts_with('~')
        || raw_source.contains('\\');

    if !is_path_like {
        return None;
    }

    let source_path = Path::new(raw_source);
    let resolved = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        Path::new(workspace_root).join(source_path)
    };

    Some(resolved.to_string_lossy().to_string())
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(v) => Some(v.trim().to_string()),
        serde_yaml::Value::Number(v) => Some(v.to_string()),
        _ => None,
    }
}

fn extract_compose_service_ports(service: &DockerComposeService) -> Vec<String> {
    service
        .ports
        .iter()
        .filter_map(|port| match port {
            DockerComposePort::Short(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            DockerComposePort::Long { published, target } => {
                let published_value = published.as_ref().and_then(yaml_value_to_string);
                let target_value = target.as_ref().and_then(yaml_value_to_string);

                match (published_value, target_value) {
                    (Some(published_port), Some(target_port)) if !published_port.is_empty() && !target_port.is_empty() => {
                        Some(format!("{published_port}:{target_port}"))
                    }
                    (Some(published_port), _) if !published_port.is_empty() => Some(published_port),
                    (_, Some(target_port)) if !target_port.is_empty() => Some(target_port),
                    _ => None,
                }
            }
        })
        .collect()
}

fn normalize_volume_path(path: &str) -> String {
    if !cfg!(target_os = "windows") {
        return path.to_string();
    }

    let trimmed = path.trim();
    if let Some(rest) = trimmed.strip_prefix("/mnt/") {
        let mut parts = rest.splitn(2, '/');
        if let Some(drive) = parts.next().filter(|d| d.len() == 1) {
            let tail = parts.next().unwrap_or_default().replace('/', "\\");
            if tail.is_empty() {
                return format!("{}:\\", drive.to_uppercase());
            }
            return format!("{}:\\{}", drive.to_uppercase(), tail);
        }
    }

    trimmed.replace('/', "\\")
}

fn resolve_named_volume_mountpoint(source: &str) -> Option<String> {
    let volume_name = source.trim();
    if volume_name.is_empty() {
        return None;
    }

    let docker = connect_docker_client()?;

    let runtime = Runtime::new().ok()?;
    let volume = runtime.block_on(async { docker.inspect_volume(volume_name).await.ok() })?;
    let mountpoint = volume.mountpoint;

    Some(normalize_volume_path(&mountpoint))
}

pub fn list_workspace_service_volumes() -> Result<Vec<WorkspaceServiceVolume>, String> {
    let workspaces = crate::persistence::load_workspace_catalog()?;
    let mut volumes = Vec::new();

    for workspace in workspaces {
        let compose_yaml_path = Path::new(&workspace.root).join("docker-compose.yaml");
        let compose_yml_path = Path::new(&workspace.root).join("docker-compose.yml");

        let compose_path = if compose_yaml_path.is_file() {
            compose_yaml_path
        } else if compose_yml_path.is_file() {
            compose_yml_path
        } else {
            continue;
        };

        let content = fs::read_to_string(&compose_path)
            .map_err(|e| format!("Impossible de lire {}: {e}", compose_path.display()))?;

        let compose: DockerComposeFile = serde_yaml::from_str(&content)
            .map_err(|e| format!("docker-compose invalide ({}): {e}", compose_path.display()))?;

        let mut service_names: Vec<String> = compose.services.keys().cloned().collect();
        service_names.sort();

        let mut docker_volumes_by_service = HashMap::new();

        for service_name in service_names {
            let service_docker_volumes = docker_mounts_for_workspace_service(&workspace.root, &service_name)
                .unwrap_or_default();
            if !service_docker_volumes.is_empty() {
                crate::persistence::save_workspace_service_volumes(
                    &workspace.id,
                    &service_name,
                    &service_docker_volumes,
                )?;
                docker_volumes_by_service.insert(service_name.clone(), service_docker_volumes);
            }

            if let Some(compose_service) = compose.services.get(&service_name) {
                let cached_volumes = crate::persistence::load_workspace_service_volumes(&workspace.id, &service_name)
                    .unwrap_or_default();

                for volume in &compose_service.volumes {
                    let (volume_label, source) = match volume {
                        DockerComposeVolume::Short(value) => {
                            if let Some((target, parsed_source)) = parse_short_volume_entry(value) {
                                (target, parsed_source)
                            } else {
                                continue;
                            }
                        }
                        DockerComposeVolume::Long { source, target } => {
                            let target = target.clone().unwrap_or_default().trim().to_string();
                            if target.is_empty() {
                                continue;
                            }
                            (target, source.clone())
                        }
                    };

                    let host_path = docker_volumes_by_service
                        .get(&service_name)
                        .and_then(|mounts| mounts.get(&volume_label).cloned())
                        .or_else(|| cached_volumes.get(&volume_label).cloned())
                        .or_else(|| resolve_host_path(&workspace.root, source.as_deref()))
                        .map(|path| normalize_volume_path(&path))
                        .or_else(|| source.as_deref().and_then(resolve_named_volume_mountpoint));

                    volumes.push(WorkspaceServiceVolume {
                        workspace_id: workspace.id.clone(),
                        workspace_name: workspace.name.clone(),
                        service_name: to_human_friendly(&service_name),
                        volume: volume_label,
                        host_path,
                    });
                }
            }
        }
    }

    Ok(volumes)
}

pub fn list_workspace_networks() -> Result<Vec<WorkspaceNetwork>, String> {
    let workspaces = list_workspaces()?;
    let containers = fetch_docker_containers().unwrap_or_default();

    let mut networks = Vec::new();

    for workspace in workspaces {
        let root = normalize_workspace_root(&workspace.root);

        for container in &containers {
            if container_belongs_to_workspace(container, &root) {
                if let Some(service_name) = extract_service_name_from_container(container) {
                    if let Some(ns) = &container.network_settings {
                        for network_name in ns.networks.keys() {
                            networks.push(WorkspaceNetwork {
                                workspace_id: workspace.id.clone(),
                                workspace_name: workspace.name.clone(),
                                service_name: service_name.clone(),
                                network: network_name.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(networks)
}
