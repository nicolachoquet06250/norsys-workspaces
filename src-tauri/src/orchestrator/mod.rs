use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::{HashSet, VecDeque};
use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::workspace_loader::{ServiceConfig, WorkspaceConfig};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) fn apply_production_process_flags(cmd: &mut Command) {
    #[cfg(windows)]
    // CREATE_NO_WINDOW: 0x08000000 (Ne pas créer de nouvelle fenêtre console)
    cmd.creation_flags(CREATE_NO_WINDOW);

    #[cfg(unix)]
    // Mettre le processus dans son propre groupe de processus pour le détacher de la session courante
    cmd.process_group(0);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRuntimeStatus {
    Idle,
    Starting,
    Running,
    Failed,
    Blocked,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRuntimeState {
    pub name: String,
    pub display_name: Option<String>,
    pub status: ServiceRuntimeStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeWorkspaceState {
    pub workspace_id: String,
    pub global_status: ServiceRuntimeStatus,
    pub services: Vec<ServiceRuntimeState>,
    pub last_error: Option<String>,
}

const MAX_LOG_LINES: usize = 10_000;

#[derive(Default)]
struct RuntimeRegistry {
    workspaces: HashMap<String, WorkspaceRuntimeHandles>,
}

#[derive(Default)]
struct HttpProbeRegistry {
    results: HashMap<String, TcpProbeStatus>,
    in_flight: HashSet<String>,
    stop_flags: HashMap<String, Arc<AtomicBool>>,
    depends_on: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpProbeStatus {
    Loading,
    Ready,
    Error,
}

struct WorkspaceRuntimeHandles {
    services: HashMap<String, Child>,
    logs: Arc<Mutex<VecDeque<String>>>,
    docker_compose: Option<DockerComposeRuntime>,
    compose_logs_runtime: Option<ComposeLogsRuntime>,
    accessibility_targets_by_service: HashMap<String, Vec<String>>,
}

#[derive(Clone)]
struct ComposeLogsRuntime {
    stop_flag: Arc<AtomicBool>,
    state_snapshot: Arc<Mutex<Option<RuntimeWorkspaceState>>>,
}

#[derive(Clone)]
struct DockerComposeRuntime {
    workspace_id: String,
    workspace_root: String,
    service_names: Vec<String>,
    command_bin: DockerComposeCommand,
    depends_on_map: HashMap<String, Vec<String>>,
}

#[derive(Clone, Copy)]
enum DockerComposeCommand {
    DockerComposePlugin,
    DockerComposeLegacy,
}

fn derive_global_status(services: &[ServiceRuntimeState]) -> ServiceRuntimeStatus {
    if services.iter().any(|service| service.status == ServiceRuntimeStatus::Failed) {
        ServiceRuntimeStatus::Failed
    } else if services
        .iter()
        .any(|service| service.status == ServiceRuntimeStatus::Starting)
    {
        let has_running = services
            .iter()
            .any(|service| service.status == ServiceRuntimeStatus::Running);
        let has_stopped = services
            .iter()
            .any(|service| service.status == ServiceRuntimeStatus::Stopped);

        if has_stopped && !has_running {
            ServiceRuntimeStatus::Stopped
        } else {
            ServiceRuntimeStatus::Starting
        }
    } else if services
        .iter()
        .any(|service| service.status == ServiceRuntimeStatus::Running || service.status == ServiceRuntimeStatus::Starting)
    {
        ServiceRuntimeStatus::Running
    } else if services.iter().all(|service| service.status == ServiceRuntimeStatus::Stopped) {
        ServiceRuntimeStatus::Stopped
    } else {
        ServiceRuntimeStatus::Idle
    }
}

fn registry() -> &'static Mutex<RuntimeRegistry> {
    static REGISTRY: OnceLock<Mutex<RuntimeRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(RuntimeRegistry::default()))
}

fn http_probe_registry() -> &'static Mutex<HttpProbeRegistry> {
    static REGISTRY: OnceLock<Mutex<HttpProbeRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HttpProbeRegistry::default()))
}

fn clear_workspace_probe_state(workspace_id: &str) {
    let workspace_prefix = format!("{}::", workspace_id.to_ascii_lowercase());
    if let Ok(mut guard) = http_probe_registry().lock() {
        for (key, stop_flag) in guard.stop_flags.iter() {
            if key.starts_with(&workspace_prefix) {
                stop_flag.store(true, Ordering::SeqCst);
            }
        }
        guard
            .stop_flags
            .retain(|key, _| !key.starts_with(&workspace_prefix));
        guard
            .results
            .retain(|key, _| !key.starts_with(&workspace_prefix));
        guard.in_flight.retain(|key| !key.starts_with(&workspace_prefix));
    }
}

pub fn stop_workspace_probes(workspace_id: &str) {
    clear_workspace_probe_state(workspace_id);

    if let Ok(mut guard) = registry().lock() {
        if let Some(runtime) = guard.workspaces.get_mut(workspace_id) {
            if let Some(compose_logs_runtime) = &runtime.compose_logs_runtime {
                compose_logs_runtime.stop_flag.store(true, Ordering::Relaxed);
            }
        }
    }
}

fn has_compose_file(workspace_root: &str) -> bool {
    let root = Path::new(workspace_root);
    root.join("docker-compose.yaml").is_file() || root.join("docker-compose.yml").is_file()
}

fn has_wsl_binary() -> bool {
    *WSL_BINARY_AVAILABLE.get_or_init(|| {
        let mut cmd = Command::new("wsl");
        apply_production_process_flags(&mut cmd);
        let detected = cmd
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok();

        eprintln!("[orchestrator] binaire WSL détecté: {detected}");
        detected
    })
}

static WINDOWS_HOST: OnceLock<bool> = OnceLock::new();
static WSL_BINARY_AVAILABLE: OnceLock<bool> = OnceLock::new();

pub fn init_host_flags() {
    let _ = WINDOWS_HOST.get_or_init(|| cfg!(target_os = "windows"));
    let _ = WSL_BINARY_AVAILABLE.get_or_init(|| {
        let mut cmd = Command::new("wsl");
        apply_production_process_flags(&mut cmd);
        let detected = cmd
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok();

        eprintln!("[orchestrator] binaire WSL détecté: {detected}");
        detected
    });
}

fn is_windows_host() -> bool {
    *WINDOWS_HOST.get_or_init(|| cfg!(target_os = "windows"))
}

fn compose_command_label(command: DockerComposeCommand) -> &'static str {
    match command {
        DockerComposeCommand::DockerComposePlugin => "docker compose",
        DockerComposeCommand::DockerComposeLegacy => "docker-compose",
    }
}

fn run_compose_command(workspace_root: &str, command: DockerComposeCommand, args: &[&str]) -> Result<std::process::Output, String> {
    let command_label = compose_command_label(command);
    let use_wsl = if is_windows_host() {
        if !has_wsl_binary() {
            return Err("WSL est requis sous Windows pour exécuter les commandes docker compose".to_string());
        }
        true
    } else {
        false
    };
    let mut cmd = if use_wsl {
        let mut cmd = Command::new("wsl");
        match command {
            DockerComposeCommand::DockerComposePlugin => {
                cmd.arg("docker");
                cmd.arg("compose");
            }
            DockerComposeCommand::DockerComposeLegacy => {
                cmd.arg("docker-compose");
            }
        }
        cmd
    } else {
        match command {
            DockerComposeCommand::DockerComposePlugin => {
                let mut cmd = Command::new("docker");
                cmd.arg("compose");
                cmd
            }
            DockerComposeCommand::DockerComposeLegacy => Command::new("docker-compose"),
        }
    };

    let display_cmd = if use_wsl {
        format!("wsl {command_label} {}", args.join(" "))
    } else {
        format!("{command_label} {}", args.join(" "))
    };
    eprintln!("[orchestrator] exécution compose: `{display_cmd}` (cwd: {workspace_root})");

    apply_production_process_flags(&mut cmd);

    cmd.args(args).current_dir(workspace_root);
    let output = cmd
        .output()
        .map_err(|e| format!("Impossible d'exécuter docker compose: {e}"))?;

    eprintln!(
        "[orchestrator] résultat compose `{display_cmd}`: success={} code={:?}",
        output.status.success(),
        output.status.code()
    );

    Ok(output)
}

fn detect_compose_command(workspace_root: &str) -> Result<DockerComposeCommand, String> {
    let plugin_probe = run_compose_command(workspace_root, DockerComposeCommand::DockerComposePlugin, &["version"]);
    if let Ok(output) = plugin_probe {
        if output.status.success() {
            eprintln!("[orchestrator] runtime compose détecté: docker compose");
            return Ok(DockerComposeCommand::DockerComposePlugin);
        }
    }

    let legacy_probe = run_compose_command(workspace_root, DockerComposeCommand::DockerComposeLegacy, &["version"]);
    if let Ok(output) = legacy_probe {
        if output.status.success() {
            eprintln!("[orchestrator] runtime compose détecté: docker-compose");
            return Ok(DockerComposeCommand::DockerComposeLegacy);
        }
    }

    Err("Impossible de trouver `docker compose` ou `docker-compose` sur la machine".to_string())
}

fn parse_running_services(stdout: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn collect_compose_logs(compose_runtime: &DockerComposeRuntime) -> Option<Vec<String>> {
    let output = run_compose_command(
        &compose_runtime.workspace_root,
        compose_runtime.command_bin,
        &["logs", "--no-color", "--tail", "1000"],
    );

    let Ok(output) = output else {
        return None;
    };

    if !output.status.success() {
        return None;
    }

    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn spawn_compose_logs_worker(
    compose_runtime: DockerComposeRuntime,
    logs: Arc<Mutex<VecDeque<String>>>,
    stop_flag: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        while !stop_flag.load(Ordering::Relaxed) {
            if let Some(collected) = collect_compose_logs(&compose_runtime) {
                if let Ok(mut guard) = logs.lock() {
                    guard.clear();
                    guard.extend(collected.into_iter().take(MAX_LOG_LINES));
                }
            }

            for _ in 0..5 {
                if stop_flag.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(200));
            }
        }
    });
}

fn spawn_compose_state_worker(
    compose_runtime: DockerComposeRuntime,
    accessibility_targets_by_service: HashMap<String, Vec<String>>,
    state_snapshot: Arc<Mutex<Option<RuntimeWorkspaceState>>>,
    stop_flag: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        while !stop_flag.load(Ordering::Relaxed) {
            if let Ok(state) = build_compose_runtime_state(
                &compose_runtime.workspace_id,
                &compose_runtime.workspace_root,
                compose_runtime.command_bin,
                &compose_runtime.service_names,
                &accessibility_targets_by_service,
                &compose_runtime.depends_on_map,
            ) {
                if let Ok(mut guard) = state_snapshot.lock() {
                    *guard = Some(state);
                }
            }

            for _ in 0..5 {
                if stop_flag.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(200));
            }
        }
    });
}

fn parse_compose_ps_json_statuses(stdout: &[u8]) -> HashMap<String, (String, Option<i64>)> {
    let mut statuses = HashMap::new();
    let lines = String::from_utf8_lossy(stdout);

    let items = if let Ok(value) = serde_json::from_slice::<Value>(stdout) {
        value.as_array().cloned().unwrap_or_default()
    } else {
        lines
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
            .collect::<Vec<_>>()
    };

    for item in &items {
        let Some(service) = item.get("Service").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if service.is_empty() {
            continue;
        }

        let state = item
            .get("State")
            .and_then(Value::as_str)
            .map(|raw| raw.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let exit_code = item.get("ExitCode").and_then(Value::as_i64);

        statuses.insert(service.to_string(), (state, exit_code));
    }

    statuses
}

fn parse_compose_ps_published_ports(stdout: &[u8]) -> HashMap<String, Vec<u16>> {
    let mut ports_by_service: HashMap<String, Vec<u16>> = HashMap::new();
    let lines = String::from_utf8_lossy(stdout);

    let items = if let Ok(value) = serde_json::from_slice::<Value>(stdout) {
        value.as_array().cloned().unwrap_or_default()
    } else {
        lines
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
            .collect::<Vec<_>>()
    };

    for item in &items {
        let Some(service) = item.get("Service").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if service.is_empty() {
            continue;
        }

        let Some(publishers) = item.get("Publishers").and_then(Value::as_array) else {
            continue;
        };

        let service_ports = ports_by_service.entry(service.to_string()).or_default();
        for publisher in publishers {
            let protocol = publisher
                .get("Protocol")
                .and_then(Value::as_str)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if !protocol.is_empty() && protocol != "tcp" {
                continue;
            }

            let Some(port_u64) = publisher.get("PublishedPort").and_then(Value::as_u64) else {
                continue;
            };

            let Ok(port) = u16::try_from(port_u64) else {
                continue;
            };

            if !service_ports.contains(&port) {
                service_ports.push(port);
            }
        }
    }

    ports_by_service
}

fn parse_endpoint_host_port(endpoint: &str) -> Option<(String, u16)> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (scheme, rest) = if let Some((scheme, rest)) = trimmed.split_once("://") {
        (Some(scheme.to_ascii_lowercase()), rest)
    } else {
        (None, trimmed)
    };

    let authority = rest.split('/').next().unwrap_or(rest).trim();
    let authority = authority.rsplit('@').next().unwrap_or(authority);

    if authority.is_empty() {
        return None;
    }

    if authority.starts_with('[') {
        let close = authority.find(']')?;
        let host = authority[..=close].to_string();
        let tail = authority.get(close + 1..).unwrap_or("");
        if let Some(port_str) = tail.strip_prefix(':') {
            let port = port_str.parse::<u16>().ok()?;
            return Some((host, port));
        }
        let default_port = if scheme.as_deref() == Some("https") { 443 } else { 80 };
        return Some((host, default_port));
    }

    if let Some((host, port_str)) = authority.rsplit_once(':') {
        if !host.contains(':') && !port_str.is_empty() && port_str.chars().all(|c| c.is_ascii_digit()) {
            let port = port_str.parse::<u16>().ok()?;
            return Some((host.to_string(), port));
        }
    }

    let default_port = if scheme.as_deref() == Some("https") { 443 } else { 80 };
    Some((authority.to_string(), default_port))
}

fn parse_endpoint_components(endpoint: &str) -> Option<(Option<String>, String, u16, String)> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (scheme, rest) = if let Some((scheme, rest)) = trimmed.split_once("://") {
        (Some(scheme.to_ascii_lowercase()), rest)
    } else {
        (None, trimmed)
    };

    let (authority, path_and_query) = if let Some((authority, path)) = rest.split_once('/') {
        (authority.trim(), format!("/{path}"))
    } else {
        (rest.trim(), "/".to_string())
    };
    let authority = authority.rsplit('@').next().unwrap_or(authority);

    if authority.is_empty() {
        return None;
    }

    if authority.starts_with('[') {
        let close = authority.find(']')?;
        let host = authority[..=close].to_string();
        let tail = authority.get(close + 1..).unwrap_or("");
        if let Some(port_str) = tail.strip_prefix(':') {
            let port = port_str.parse::<u16>().ok()?;
            return Some((scheme, host, port, path_and_query));
        }
        let default_port = if scheme.as_deref() == Some("https") { 443 } else { 80 };
        return Some((scheme, host, default_port, path_and_query));
    }

    if let Some((host, port_str)) = authority.rsplit_once(':') {
        if !host.contains(':') && !port_str.is_empty() && port_str.chars().all(|c| c.is_ascii_digit()) {
            let port = port_str.parse::<u16>().ok()?;
            return Some((scheme, host.to_string(), port, path_and_query));
        }
    }

    let default_port = if scheme.as_deref() == Some("https") { 443 } else { 80 };
    Some((scheme, authority.to_string(), default_port, path_and_query))
}

fn extract_probe_port(endpoint: &str) -> Option<u16> {
    let trimmed = endpoint.trim();
    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return trimmed.parse::<u16>().ok();
    }

    if let Some((_, port)) = parse_endpoint_host_port(endpoint) {
        return Some(port);
    }
    parse_endpoint_components(endpoint).map(|(_, _, port, _)| port)
}

fn is_database_service(service_name: &str) -> bool {
    let lowered = service_name.to_ascii_lowercase();
    ["postgres", "mysql", "mariadb", "mongodb", "redis", "oracle", "sqlserver", "mssql", "cassandra", "neo4j", "elasticsearch", "opensearch", "mariadb-server"]
        .iter()
        .any(|keyword| lowered.contains(keyword))
}

fn get_service_ready_patterns(service_name: &str) -> Vec<&'static str> {
    let lowered = service_name.to_ascii_lowercase();
    if lowered.contains("mysql") || lowered.contains("mariadb") {
        vec![
            "ready for connections",
            "port: 3306",
            "mysqld: ready for connections",
            "ready to accept connections",
            "mysql: [warning]", // Souvent présent juste avant d'être prêt
            "version: '8.",     // MySQL 8 startup
            "version: '5.7",    // MySQL 5.7 startup
            "starting mysql",
            "initialized",
        ]
    } else if lowered.contains("postgres") {
        vec!["database system is ready to accept connections"]
    } else if lowered.contains("mongodb") {
        vec!["waiting for connections", "waiting for connections on port"]
    } else if lowered.contains("redis") {
        vec!["ready to accept connections", "the server is now ready to accept connections"]
    } else {
        vec![]
    }
}

use std::net::TcpStream;

fn run_nc_probe(workspace_id: &str, service_name: &str, port: u16) -> Result<TcpProbeStatus, String> {
    // Tentative de connexion TCP native en premier
    let address = format!("127.0.0.1:{}", port);
    
    // On augmente le timeout à 1s pour plus de robustesse
    let tcp_success = TcpStream::connect_timeout(&address.parse().unwrap(), Duration::from_millis(1000)).is_ok();

    if !tcp_success {
        return Ok(TcpProbeStatus::Loading);
    }

    eprintln!("[orchestrator] TCP native probe REUSSIE pour `{}` sur port {}", service_name, port);

    // Si c'est une base de données, on vérifie AUSSI les logs pour être sûr qu'elle est prête
    if is_database_service(service_name) {
        let ready_patterns = get_service_ready_patterns(service_name);
        
        // Si on a des patterns spécifiques, on les cherche.
        if !ready_patterns.is_empty() {
            if let Ok(guard) = registry().lock() {
                if let Some(runtime) = guard.workspaces.get(workspace_id) {
                    if let Ok(logs_guard) = runtime.logs.lock() {
                        let service_name_lowered = service_name.to_ascii_lowercase();
                        
                        // On compte le nombre de fois où on a vu TCP OK pour ce service
                        // pour implémenter un "soft fail" après plusieurs tentatives.
                        // On utilise le registre in_flight pour stocker un compteur temporaire ?
                        // Non, plus simple : on regarde la longueur des logs ou on accepte 
                        // que si TCP est OK et qu'on a déjà cherché plusieurs fois, on valide.
                        
                        // On cherche d'abord les lignes qui contiennent le nom du service ET un pattern
                        let is_ready_strict = logs_guard.iter().rev().take(500).any(|line| {
                            let line_lowered = line.to_ascii_lowercase();
                            let belongs_to_service = line_lowered.contains(&service_name_lowered);
                            belongs_to_service && ready_patterns.iter().any(|pattern| line_lowered.contains(pattern))
                        });

                        if is_ready_strict {
                            eprintln!("[orchestrator] Patterns de logs trouvés (strict) pour `{}`. Service prêt.", service_name);
                            return Ok(TcpProbeStatus::Ready);
                        }

                        // Si on ne trouve pas en strict, on cherche juste le pattern dans les logs les plus récents
                        // car Docker Compose peut omettre le nom du service dans certaines conditions de logs
                        let is_ready_loose = logs_guard.iter().rev().take(100).any(|line| {
                            let line_lowered = line.to_ascii_lowercase();
                            ready_patterns.iter().any(|pattern| line_lowered.contains(pattern))
                        });

                        if is_ready_loose {
                            eprintln!("[orchestrator] Patterns de logs trouvés (loose) pour `{}`. Service prêt.", service_name);
                            return Ok(TcpProbeStatus::Ready);
                        }
                        
                        // Si le port est ouvert (TCP OK) mais qu'on ne trouve pas le log,
                        // et que les logs sont déjà conséquents (ou qu'on a attendu un peu), 
                        // on finit par valider pour ne pas bloquer l'utilisateur.
                        if logs_guard.len() > 30 {
                             eprintln!("[orchestrator] TCP OK pour `{}`, logs présents ({}) mais pattern non trouvé. Validation par précaution.", service_name, logs_guard.len());
                             return Ok(TcpProbeStatus::Ready);
                        }

                        eprintln!("[orchestrator] TCP OK pour `{}`, mais patterns de logs non trouvés. En attente...", service_name);
                        return Ok(TcpProbeStatus::Loading);
                    }
                }
            }
        }
    }

    Ok(TcpProbeStatus::Ready)
}

fn run_http_probe(port: u16) -> Result<TcpProbeStatus, String> {
    let mut cmd = if is_windows_host() {
        if !has_wsl_binary() {
            return Err("WSL est requis sous Windows pour exécuter curl".to_string());
        }
        let mut cmd = Command::new("wsl");
        cmd.arg("curl");
        cmd
    } else {
        Command::new("curl")
    };

    apply_production_process_flags(&mut cmd);

    // -I: HEAD request, -s: silent, -o /dev/null: discard output, -w %{http_code}: print only status code
    // --connect-timeout 2: avoid hanging
    let output = cmd
        .args([
            "-I", "-s", 
            "-o", "/dev/null", 
            "-w", "%{http_code}", 
            "--connect-timeout", "2",
            &format!("http://localhost:{}", port)
        ])
        .output()
        .map_err(|error| format!("Impossible d'exécuter curl: {error}"))?;

    let status_code = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Si on a un code HTTP (même 404, 500), le serveur HTTP est là et répond.
    // 000 signifie généralement "Connection refused" ou timeout par curl
    if !status_code.is_empty() && status_code != "000" {
        return Ok(TcpProbeStatus::Ready);
    }

    Ok(TcpProbeStatus::Loading)
}


fn spawn_probe_until_success(
    workspace_id: String,
    endpoint_key: String,
    service_name: String,
    port: u16,
    stop_flag: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut has_been_ready = false;

        eprintln!(
            "[orchestrator] démarrage sonde continue pour `{}` sur localhost:{}",
            service_name, port
        );

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            // Fréquence des sondes : 1000ms pour éviter de surcharger (DDOS)
            const PROBE_INTERVAL_MS: u64 = 1000;

            // Vérifier les dépendances avant de tester le port
            let mut can_probe = true;
            let mut missing_dep = None;

            {
                if let Ok(guard) = http_probe_registry().lock() {
                    if let Some(deps) = guard.depends_on.get(&endpoint_key) {
                        // Pour chaque dépendance, on vérifie si elle est prête dans le registre
                        for dep_name in deps {
                            let dep_lower = dep_name.to_ascii_lowercase();
                            let workspace_lower = workspace_id.to_ascii_lowercase();
                            let prefix = format!("{}::{}::", workspace_lower, dep_lower);
                            
                            let dep_ready = guard.results.iter().any(|(k, v)| {
                                k.starts_with(&prefix) && *v == TcpProbeStatus::Ready
                            });

                            if !dep_ready {
                                can_probe = false;
                                missing_dep = Some(dep_name.clone());
                                eprintln!("[orchestrator] sonde `{}` suspendue : attend `{}` (prefix: {})", service_name, dep_name, prefix);
                                break;
                            }
                        }
                    }
                }
            }

            if !can_probe {
                if let Ok(mut guard) = http_probe_registry().lock() {
                    guard.results.insert(endpoint_key.clone(), TcpProbeStatus::Loading);
                }
                thread::sleep(Duration::from_millis(PROBE_INTERVAL_MS));
                continue;
            }

            let is_db = is_database_service(&service_name);
            eprintln!("{} is db: {}", &service_name, is_db.to_string());
            let is_web = is_probably_web_service(&service_name);
            let probe_result = if is_db {
                run_nc_probe(&workspace_id, &service_name, port)
            } else if is_web {
                run_http_probe(port)
            } else {
                run_nc_probe(&workspace_id, &service_name, port)
            };

            match probe_result {
                Ok(TcpProbeStatus::Ready) => {
                    eprintln!("[orchestrator] sonde REUSSIE pour `{}` sur localhost:{}", service_name, port);
                    has_been_ready = true;
                    if let Ok(mut guard) = http_probe_registry().lock() {
                        guard.results.insert(endpoint_key.clone(), TcpProbeStatus::Ready);
                    }
                }
                Ok(TcpProbeStatus::Loading) => {
                    let next_status = if has_been_ready {
                        TcpProbeStatus::Error
                    } else {
                        TcpProbeStatus::Loading
                    };
                    if let Ok(mut guard) = http_probe_registry().lock() {
                        guard.results.insert(endpoint_key.clone(), next_status);
                    }
                }
                Ok(TcpProbeStatus::Error) => {
                    let next_status = if has_been_ready {
                        TcpProbeStatus::Error
                    } else {
                        TcpProbeStatus::Loading
                    };
                    if let Ok(mut guard) = http_probe_registry().lock() {
                        guard.results.insert(endpoint_key.clone(), next_status);
                    }
                }
                Err(_) => {
                    let next_status = if has_been_ready {
                        TcpProbeStatus::Error
                    } else {
                        TcpProbeStatus::Loading
                    };
                    if let Ok(mut guard) = http_probe_registry().lock() {
                        guard.results.insert(endpoint_key.clone(), next_status);
                    }
                }
            }

            thread::sleep(Duration::from_millis(PROBE_INTERVAL_MS));
        }

        if let Ok(mut guard) = http_probe_registry().lock() {
            guard.in_flight.remove(&endpoint_key);
            guard.results.remove(&endpoint_key);
            guard.stop_flags.remove(&endpoint_key);
        }
    });
}

fn get_endpoint_probe_status(
    workspace_id: &str,
    service_name: &str,
    endpoint: &str,
    depends_on: Option<&[String]>,
) -> TcpProbeStatus {
    let port_opt = extract_probe_port(endpoint);
    let Some(port) = port_opt else {
        eprintln!("[orchestrator] impossible d'extraire le port de l'endpoint: {}", endpoint);
        return TcpProbeStatus::Loading;
    };

    let endpoint_key = format!(
        "{}::{}::{}",
        workspace_id.to_ascii_lowercase(),
        service_name.to_ascii_lowercase(),
        endpoint.to_ascii_lowercase()
    );
    if let Ok(mut guard) = http_probe_registry().lock() {
        if let Some(deps) = depends_on {
            guard.depends_on.insert(endpoint_key.clone(), deps.to_vec());
        }

        if !guard.in_flight.contains(&endpoint_key) {
            eprintln!("[orchestrator] lancement nouvelle sonde pour {}", endpoint_key);
            guard.in_flight.insert(endpoint_key.clone());
            guard
                .results
                .insert(endpoint_key.clone(), TcpProbeStatus::Loading);
            
            let stop_flag = Arc::new(AtomicBool::new(false));
            guard
                .stop_flags
                .insert(endpoint_key.clone(), stop_flag.clone());
            spawn_probe_until_success(
                workspace_id.to_string(),
                endpoint_key.clone(),
                service_name.to_string(),
                port,
                stop_flag,
            );
        }

        return guard
            .results
            .get(&endpoint_key)
            .copied()
            .unwrap_or(TcpProbeStatus::Loading);
    }

    TcpProbeStatus::Loading
}

fn is_endpoint_accessible(workspace_id: &str, service_name: &str, endpoint: &str) -> bool {
    get_endpoint_probe_status(workspace_id, service_name, endpoint, None) == TcpProbeStatus::Ready
}

fn service_probe_status(
    workspace_id: &str,
    service_name: &str,
    targets: &[String],
    any_target_defined: bool,
    depends_on: Option<&[String]>,
) -> TcpProbeStatus {
    eprintln!("[orchestrator] service_probe_status pour `{}` avec targets {:?}", service_name, targets);
    if targets.is_empty() {
        let _ = any_target_defined;
        return TcpProbeStatus::Loading;
    }

    let mut has_loading = false;
    for target in targets {
        let status = get_endpoint_probe_status(workspace_id, service_name, target, depends_on);
        eprintln!("[orchestrator] vérification accessibilité endpoint: {} -> {:?}", target, status);
        match status {
            TcpProbeStatus::Ready => {}
            TcpProbeStatus::Error => return TcpProbeStatus::Error,
            TcpProbeStatus::Loading => has_loading = true,
        }
    }

    if has_loading {
        TcpProbeStatus::Loading
    } else {
        TcpProbeStatus::Ready
    }
}

fn find_service_log_error(logs: &VecDeque<String>, service_name: &str) -> Option<String> {
    let service_token = format!("[{service_name}]").to_ascii_lowercase();
    let error_keywords = ["error", "exception", "fatal", "panic", "failed", "eaddrinuse", "refused"];

    logs.iter().rev().find_map(|line| {
        let lowered = line.to_ascii_lowercase();
        if lowered.contains(&service_token)
            && error_keywords.iter().any(|keyword| lowered.contains(keyword))
        {
            Some(line.clone())
        } else {
            None
        }
    })
}

fn are_all_targets_accessible(workspace_id: &str, service_name: &str, targets: &[String]) -> bool {
    if targets.is_empty() {
        return true;
    }

    for target in targets {
        eprintln!("[orchestrator] vérification accessibilité endpoint: {}", target);
        if !is_endpoint_accessible(workspace_id, service_name, target) {
            return false;
        }
    }

    true
}

fn is_probably_web_service(service_name: &str) -> bool {
    let lowered = service_name.to_ascii_lowercase();
    if is_database_service(service_name) {
        return false;
    }
    ["frontend", "backend", "api", "web", "app", "adminer", "phpmyadmin", "swagger", "dashboard"]
        .iter()
        .any(|keyword| lowered.contains(keyword))
}

fn is_service_accessible(
    workspace_id: &str,
    service_name: &str,
    targets: &[String],
    any_target_defined: bool,
) -> bool {
    eprintln!(
        "[orchestrator] accessibilité service `{}`: {} cible(s), cibles_globales_definies={}",
        service_name,
        targets.len(),
        any_target_defined
    );

    if targets.is_empty() {
        let _ = any_target_defined;
        let _ = is_probably_web_service(service_name);
        eprintln!(
            "[orchestrator] service `{}` en attente: aucune cible d'accessibilité mappée",
            service_name
        );
        return false;
    }

    are_all_targets_accessible(workspace_id, service_name, targets)
}

fn parse_service_scoped_target(raw_target: &str) -> (Option<String>, String) {
    let trimmed = raw_target.trim();
    if let Some((service_name, endpoint)) = trimmed.split_once('=') {
        let service = service_name.trim();
        let target = endpoint.trim();
        if !service.is_empty() && !target.is_empty() {
            return (Some(service.to_string()), target.to_string());
        }
    }
    (None, trimmed.to_string())
}

fn choose_default_web_service(service_names: &[String]) -> Option<String> {
    let web_keywords = ["web", "api", "backend", "front", "ui", "app"];
    service_names
        .iter()
        .find(|name| {
            let lowered = name.to_ascii_lowercase();
            web_keywords.iter().any(|keyword| lowered.contains(keyword))
        })
        .cloned()
        .or_else(|| service_names.first().cloned())
}

fn build_accessibility_targets_by_service(
    service_names: &[String],
    raw_targets: &[String],
) -> HashMap<String, Vec<String>> {
    let mut targets_by_service: HashMap<String, Vec<String>> = service_names
        .iter()
        .cloned()
        .map(|service_name| (service_name, Vec::new()))
        .collect();

    let mut unscoped_targets: Vec<String> = Vec::new();

    for raw_target in raw_targets {
        let (scoped_service, endpoint) = parse_service_scoped_target(raw_target);
        if endpoint.is_empty() {
            continue;
        }

        if let Some(scoped_service_name) = scoped_service {
            if let Some((_, targets)) = targets_by_service
                .iter_mut()
                .find(|(service_name, _)| service_name.eq_ignore_ascii_case(&scoped_service_name))
            {
                targets.push(endpoint);
            } else {
                eprintln!(
                    "[orchestrator] cible d'accessibilité non mappée (service inconnu `{}`), fallback sur service web par défaut",
                    scoped_service_name
                );
                unscoped_targets.push(endpoint);
            }
        } else {
            unscoped_targets.push(endpoint);
        }
    }

    if unscoped_targets.is_empty() || service_names.is_empty() {
        return targets_by_service;
    }

    if service_names.len() == 1 {
        if let Some(targets) = targets_by_service.get_mut(&service_names[0]) {
            targets.extend(unscoped_targets);
        }
        return targets_by_service;
    }

    if unscoped_targets.len() == service_names.len() {
        for (index, target) in unscoped_targets.into_iter().enumerate() {
            if let Some(targets) = targets_by_service.get_mut(&service_names[index]) {
                targets.push(target);
            }
        }
        return targets_by_service;
    }

    if let Some(default_service) = choose_default_web_service(service_names) {
        if let Some(targets) = targets_by_service.get_mut(&default_service) {
            targets.extend(unscoped_targets);
        }
    }

    eprintln!(
        "[orchestrator] mapping cibles d'accessibilité: {:?}",
        targets_by_service
    );

    targets_by_service
}

fn list_compose_services(workspace_root: &str, command_bin: DockerComposeCommand) -> Result<Vec<String>, String> {
    let output = run_compose_command(workspace_root, command_bin, &["config", "--services"])?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Impossible de lister les services docker-compose".to_string()
        } else {
            format!("Impossible de lister les services docker-compose: {stderr}")
        });
    }

    Ok(parse_running_services(&output.stdout))
}

fn build_compose_runtime_state(
    workspace_id: &str,
    workspace_root: &str,
    command_bin: DockerComposeCommand,
    service_names: &[String],
    accessibility_targets_by_service: &HashMap<String, Vec<String>>,
    depends_on_map: &HashMap<String, Vec<String>>,
) -> Result<RuntimeWorkspaceState, String> {
    let ps_all_output = run_compose_command(workspace_root, command_bin, &["ps", "--all", "--format", "json"])?;
    if !ps_all_output.status.success() {
        let stderr = String::from_utf8_lossy(&ps_all_output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Impossible de récupérer l'état détaillé des services docker-compose".to_string()
        } else {
            format!("Impossible de récupérer l'état détaillé des services docker-compose: {stderr}")
        });
    }
    let detailed_statuses = parse_compose_ps_json_statuses(&ps_all_output.stdout);
    let published_ports_by_service = parse_compose_ps_published_ports(&ps_all_output.stdout);

    let output = run_compose_command(
        workspace_root,
        command_bin,
        &["ps", "--services", "--filter", "status=running"],
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Impossible de récupérer l'état des services docker-compose".to_string()
        } else {
            format!("Impossible de récupérer l'état des services docker-compose: {stderr}")
        });
    }

    let running = parse_running_services(&output.stdout);
    let running_set: HashSet<&str> = running.iter().map(String::as_str).collect();
    let any_target_defined = accessibility_targets_by_service
        .values()
        .any(|targets| !targets.is_empty());

    let mut services = service_names
        .iter()
        .map(|name| {
            if running_set.contains(name.as_str()) {
                let service_targets = accessibility_targets_by_service
                    .get(name)
                    .cloned()
                    .unwrap_or_default();
                let resolved_targets = if service_targets.is_empty() {
                    published_ports_by_service
                        .get(name)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|port| port.to_string())
                        .collect::<Vec<_>>()
                } else {
                    service_targets
                };
                let service_deps = depends_on_map.get(name).map(|d| d.as_slice());
                
                // On utilise une version simplifiée pour éviter la récursion infinie ou les blocages de verrous
                let endpoint_key_prefix = format!(
                    "{}::{}::",
                    workspace_id.to_ascii_lowercase(),
                    name.to_ascii_lowercase()
                );
                
                let mut probe_status = TcpProbeStatus::Loading;
                let has_ready = if let Ok(guard) = http_probe_registry().lock() {
                    // On cherche s'il y a un résultat Ready pour ce service
                    guard.results.iter().any(|(k, v)| {
                        k.starts_with(&endpoint_key_prefix) && *v == TcpProbeStatus::Ready
                    })
                } else {
                    false
                };
                
                if has_ready {
                    probe_status = TcpProbeStatus::Ready;
                } else {
                    // On force quand même l'appel à service_probe_status pour s'assurer que les sondes sont lancées
                    // Note: service_probe_status va lui-même verrouiller le registre, d'où l'importance d'avoir lâché le verrou avant.
                    probe_status = service_probe_status(workspace_id, name, &resolved_targets, any_target_defined, service_deps);
                }
                
                eprintln!("[orchestrator] service `{}` (prefix: {}) probe_status final: {:?}", name, endpoint_key_prefix, probe_status);
                
                let (status, message) = match probe_status {
                    TcpProbeStatus::Ready => (ServiceRuntimeStatus::Running, "Service prêt et accessible".to_string()),
                    TcpProbeStatus::Loading => {
                        // ...
                        // Vérifier si on attend une dépendance
                        let mut waiting_for_dep = None;
                        if let Some(deps) = depends_on_map.get(name) {
                            for dep in deps {
                                let dep_targets = accessibility_targets_by_service.get(dep).cloned().unwrap_or_default();
                                let _ = if dep_targets.is_empty() {
                                    published_ports_by_service
                                        .get(dep)
                                        .cloned()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|port| port.to_string())
                                        .collect::<Vec<_>>()
                                } else {
                                    dep_targets
                                };
                                
                                let dep_ready = running_set.contains(dep.as_str()) && {
                                    let dep_prefix = format!("{}::{}::", workspace_id.to_ascii_lowercase(), dep.to_ascii_lowercase());
                                    let is_ready = if let Ok(p_guard) = http_probe_registry().lock() {
                                        p_guard.results.iter().any(|(k, v)| k.starts_with(&dep_prefix) && *v == TcpProbeStatus::Ready)
                                    } else {
                                        false
                                    };
                                    is_ready
                                };

                                if !dep_ready {
                                    eprintln!("[orchestrator] `build_compose_runtime_state` : `{}` attend `{}` (dep_ready={})", name, dep, dep_ready);
                                    waiting_for_dep = Some(dep.clone());
                                    break;
                                }
                            }
                        }

                        let msg = if let Some(dep) = waiting_for_dep {
                            format!("En attente de la dépendance : {}", dep)
                        } else if is_database_service(name) {
                            "Base de données en cours d'initialisation...".to_string()
                        } else {
                            "Conteneur démarré, en attente d'accessibilité".to_string()
                        };
                        (ServiceRuntimeStatus::Starting, msg)
                    }
                    TcpProbeStatus::Error => (ServiceRuntimeStatus::Failed, "Conteneur inaccessible après démarrage".to_string()),
                };

                ServiceRuntimeState {
                    name: name.clone(),
                    display_name: Some(crate::workspace_loader::to_human_friendly(name)),
                    status,
                    message: Some(message),
                }
            } else {
                let (state, exit_code) = detailed_statuses
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| (String::new(), None));
                let is_failed = matches!(state.as_str(), "exited" | "dead") && exit_code.unwrap_or(0) != 0;
                ServiceRuntimeState {
                    name: name.clone(),
                    display_name: Some(crate::workspace_loader::to_human_friendly(name)),
                    status: if is_failed {
                        ServiceRuntimeStatus::Failed
                    } else {
                        ServiceRuntimeStatus::Stopped
                    },
                    message: Some(if is_failed {
                        format!("Conteneur terminé en erreur (code {})", exit_code.unwrap_or(-1))
                    } else {
                        "Conteneur non démarré".to_string()
                    }),
                }
            }
        })
        .collect::<Vec<_>>();

    for i in 0..services.len() {
        if services[i].status == ServiceRuntimeStatus::Running {
            let service_name = &services[i].name;
            if let Some(deps) = depends_on_map.get(service_name) {
                let mut all_deps_running = true;
                let mut missing_dep = String::new();

                for dep in deps {
                    let dep_status = services
                        .iter()
                        .find(|s| s.name.eq_ignore_ascii_case(dep))
                        .map(|s| s.status.clone())
                        .unwrap_or(ServiceRuntimeStatus::Stopped);

                    if dep_status != ServiceRuntimeStatus::Running {
                        all_deps_running = false;
                        missing_dep = dep.clone();
                        break;
                    }
                }

                if !all_deps_running {
                    services[i].status = ServiceRuntimeStatus::Starting;
                    services[i].message = Some(format!(
                        "Prêt, mais en attente de la dépendance : {}",
                        missing_dep
                    ));
                }
            }
        }
    }

    Ok(RuntimeWorkspaceState {
        workspace_id: workspace_id.to_string(),
        global_status: derive_global_status(&services),
        services,
        last_error: None,
    })
}

fn start_order_levels(workspace: &WorkspaceConfig) -> Result<Vec<Vec<ServiceConfig>>, String> {
    let mut levels: Vec<Vec<ServiceConfig>> = Vec::new();
    let mut available: HashSet<String> = HashSet::new();
    let mut remaining = workspace.services.clone();

    while !remaining.is_empty() {
        let (ready, not_ready): (Vec<ServiceConfig>, Vec<ServiceConfig>) = remaining
            .into_iter()
            .partition(|service| service.depends_on.iter().all(|dep| available.contains(dep)));

        if ready.is_empty() {
            return Err("Dépendances circulaires ou services dépendants introuvables".to_string());
        }

        for service in &ready {
            available.insert(service.name.clone());
        }

        levels.push(ready);
        remaining = not_ready;
    }

    Ok(levels)
}

fn build_child_command(service: &ServiceConfig, workspace: &WorkspaceConfig, env: &HashMap<String, String>) -> Result<Command, String> {
    let mut cmd = if is_windows_host() {
        if !has_wsl_binary() {
            return Err("WSL est requis sous Windows pour exécuter les commandes de services".to_string());
        }
        let mut cmd = Command::new("wsl");
        cmd.args(["bash", "-lc", service.command.as_str()]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-lc", service.command.as_str()]);
        cmd
    };

    let cwd = service
        .cwd
        .as_ref()
        .map(|relative| PathBuf::from(&workspace.root).join(relative))
        .unwrap_or_else(|| PathBuf::from(&workspace.root));

    cmd.current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    apply_production_process_flags(&mut cmd);

    for (k, v) in env {
        cmd.env(k, v);
    }
    for (k, v) in &service.env {
        cmd.env(k, v);
    }

    Ok(cmd)
}

fn spawn_log_reader(logs: Arc<Mutex<VecDeque<String>>>, service_name: String, stream_name: &'static str, reader: impl BufRead + Send + 'static) {
    thread::spawn(move || {
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(mut guard) = logs.lock() {
                guard.push_back(format!("[{service_name}][{stream_name}] {line}"));
                while guard.len() > MAX_LOG_LINES {
                    guard.pop_front();
                }
            }
        }
    });
}

fn spawn_service(
    service: &ServiceConfig,
    workspace: &WorkspaceConfig,
    env: &HashMap<String, String>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> Result<Child, String> {
    let mut child = build_child_command(service, workspace, env)?
        .spawn()
        .map_err(|e| format!("Impossible de lancer {}: {e}", service.name))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        spawn_log_reader(logs.clone(), service.name.clone(), "stdout", reader);
    }

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        spawn_log_reader(logs, service.name.clone(), "stderr", reader);
    }

    Ok(child)
}

pub fn start_workspace(workspace: &WorkspaceConfig, env: &HashMap<String, String>) -> Result<RuntimeWorkspaceState, String> {
    eprintln!(
        "[orchestrator] démarrage workspace `{}` ({})",
        workspace.id, workspace.root
    );

    clear_workspace_probe_state(&workspace.id);

    if has_compose_file(&workspace.root) {
        eprintln!("[orchestrator] mode docker compose activé");
        let command_bin = detect_compose_command(&workspace.root)?;
        let up_output = run_compose_command(&workspace.root, command_bin, &["up", "--build", "-d"])?;

        if !up_output.status.success() {
            let stderr = String::from_utf8_lossy(&up_output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&up_output.stdout).trim().to_string();
            let details = if !stderr.is_empty() { stderr } else { stdout };
            return Err(if details.is_empty() {
                "La commande docker compose up --build -d a échoué".to_string()
            } else {
                format!("La commande docker compose up --build -d a échoué: {details}")
            });
        }

        let service_names = if workspace.services.is_empty() {
            list_compose_services(&workspace.root, command_bin)?
        } else {
            workspace.services.iter().map(|service| service.name.clone()).collect()
        };

        let depends_on_map: HashMap<String, Vec<String>> = workspace
            .services
            .iter()
            .map(|s| (s.name.clone(), s.depends_on.clone()))
            .collect();

        let logs = Arc::new(Mutex::new(VecDeque::new()));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let state_snapshot = Arc::new(Mutex::new(None));
        let compose_runtime = DockerComposeRuntime {
            workspace_id: workspace.id.clone(),
            workspace_root: workspace.root.clone(),
            service_names: service_names.clone(),
            command_bin,
            depends_on_map: depends_on_map.clone(),
        };
        let accessibility_targets_by_service =
            build_accessibility_targets_by_service(&service_names, &workspace.open);
        let mut guard = registry()
            .lock()
            .map_err(|_| "Impossible de verrouiller le registre runtime".to_string())?;
        guard.workspaces.insert(
            workspace.id.clone(),
            WorkspaceRuntimeHandles {
                services: HashMap::new(),
                logs: logs.clone(),
                docker_compose: Some(compose_runtime.clone()),
                compose_logs_runtime: Some(ComposeLogsRuntime {
                    stop_flag: stop_flag.clone(),
                    state_snapshot: state_snapshot.clone(),
                }),
                accessibility_targets_by_service: accessibility_targets_by_service.clone(),
            },
        );
        drop(guard);

        spawn_compose_logs_worker(compose_runtime.clone(), logs, stop_flag.clone());
        spawn_compose_state_worker(
            compose_runtime.clone(),
            accessibility_targets_by_service.clone(),
            state_snapshot.clone(),
            stop_flag,
        );

        let initial_state = build_compose_runtime_state(
            &workspace.id,
            &workspace.root,
            command_bin,
            &service_names,
            &accessibility_targets_by_service,
            &depends_on_map,
        )?;

        if let Ok(mut guard) = state_snapshot.lock() {
            *guard = Some(initial_state.clone());
        }

        return Ok(initial_state);
    }

    let levels = start_order_levels(workspace)?;
    let execution_mode = env
        .get("ORCHESTRATION_MODE")
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "sequential".to_string());
    eprintln!("[orchestrator] mode d'orchestration services: {execution_mode}");

    let logs = Arc::new(Mutex::new(VecDeque::new()));
    let mut launched: HashMap<String, Child> = HashMap::new();
    let mut states: Vec<ServiceRuntimeState> = Vec::new();

    for level in levels {
        if execution_mode == "parallel" {
            for service in &level {
                match spawn_service(service, workspace, env, logs.clone()) {
                    Ok(child) => {
                        launched.insert(service.name.clone(), child);
                        states.push(ServiceRuntimeState {
                            name: service.name.clone(),
                            display_name: service.display_name.clone(),
                            status: ServiceRuntimeStatus::Starting,
                            message: Some("Service démarré (parallèle), en attente d'accessibilité".to_string()),
                        });
                    }
                    Err(error) => {
                        states.push(ServiceRuntimeState {
                            name: service.name.clone(),
                            display_name: service.display_name.clone(),
                            status: ServiceRuntimeStatus::Failed,
                            message: Some(error.clone()),
                        });
                        let state = RuntimeWorkspaceState {
                            workspace_id: workspace.id.clone(),
                            global_status: ServiceRuntimeStatus::Failed,
                            services: states,
                            last_error: Some(error),
                        };
                        let stopped = stop_workspace(state);
                        return Ok(stopped);
                    }
                }
            }
        } else {
            for service in &level {
                match spawn_service(service, workspace, env, logs.clone()) {
                    Ok(child) => {
                        launched.insert(service.name.clone(), child);
                        states.push(ServiceRuntimeState {
                            name: service.name.clone(),
                            display_name: service.display_name.clone(),
                            status: ServiceRuntimeStatus::Starting,
                            message: Some("Service démarré (séquentiel), en attente d'accessibilité".to_string()),
                        });
                    }
                    Err(error) => {
                        states.push(ServiceRuntimeState {
                            name: service.name.clone(),
                            display_name: service.display_name.clone(),
                            status: ServiceRuntimeStatus::Failed,
                            message: Some(error.clone()),
                        });
                        let state = RuntimeWorkspaceState {
                            workspace_id: workspace.id.clone(),
                            global_status: ServiceRuntimeStatus::Failed,
                            services: states,
                            last_error: Some(error),
                        };
                        let stopped = stop_workspace(state);
                        return Ok(stopped);
                    }
                }
            }
        }
    }

    let mut guard = registry()
        .lock()
        .map_err(|_| "Impossible de verrouiller le registre runtime".to_string())?;
    guard.workspaces.insert(
        workspace.id.clone(),
        WorkspaceRuntimeHandles {
            services: launched,
            logs,
            docker_compose: None,
            compose_logs_runtime: None,
            accessibility_targets_by_service: build_accessibility_targets_by_service(
                &workspace
                    .services
                    .iter()
                    .map(|service| service.name.clone())
                    .collect::<Vec<_>>(),
                &workspace.open,
            ),
        },
    );

    let state = RuntimeWorkspaceState {
        workspace_id: workspace.id.clone(),
        global_status: ServiceRuntimeStatus::Starting,
        services: states,
        last_error: None,
    };

    refresh_workspace_state(state)
}

pub fn attach_workspace_runtime(workspace: &WorkspaceConfig) -> Result<RuntimeWorkspaceState, String> {
    clear_workspace_probe_state(&workspace.id);

    if has_compose_file(&workspace.root) {
        let command_bin = detect_compose_command(&workspace.root)?;
        let service_names = if workspace.services.is_empty() {
            list_compose_services(&workspace.root, command_bin)?
        } else {
            workspace
                .services
                .iter()
                .map(|service| service.name.clone())
                .collect()
        };

        let depends_on_map: HashMap<String, Vec<String>> = workspace
            .services
            .iter()
            .map(|s| (s.name.clone(), s.depends_on.clone()))
            .collect();

        let logs = Arc::new(Mutex::new(VecDeque::new()));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let state_snapshot = Arc::new(Mutex::new(None));
        let compose_runtime = DockerComposeRuntime {
            workspace_id: workspace.id.clone(),
            workspace_root: workspace.root.clone(),
            service_names: service_names.clone(),
            command_bin,
            depends_on_map: depends_on_map.clone(),
        };
        let accessibility_targets_by_service =
            build_accessibility_targets_by_service(&service_names, &workspace.open);

        let mut guard = registry()
            .lock()
            .map_err(|_| "Impossible de verrouiller le registre runtime".to_string())?;
        guard.workspaces.insert(
            workspace.id.clone(),
            WorkspaceRuntimeHandles {
                services: HashMap::new(),
                logs: logs.clone(),
                docker_compose: Some(compose_runtime.clone()),
                compose_logs_runtime: Some(ComposeLogsRuntime {
                    stop_flag: stop_flag.clone(),
                    state_snapshot: state_snapshot.clone(),
                }),
                accessibility_targets_by_service: accessibility_targets_by_service.clone(),
            },
        );
        drop(guard);

        spawn_compose_logs_worker(compose_runtime.clone(), logs, stop_flag.clone());
        spawn_compose_state_worker(
            compose_runtime,
            accessibility_targets_by_service.clone(),
            state_snapshot.clone(),
            stop_flag,
        );

        let state = build_compose_runtime_state(
            &workspace.id,
            &workspace.root,
            command_bin,
            &service_names,
            &accessibility_targets_by_service,
            &depends_on_map,
        )?;

        if let Ok(mut guard) = state_snapshot.lock() {
            *guard = Some(state.clone());
        }

        return Ok(state);
    }

    let services = workspace
        .services
        .iter()
        .map(|service| ServiceRuntimeState {
            name: service.name.clone(),
            display_name: service.display_name.clone(),
            status: ServiceRuntimeStatus::Stopped,
            message: Some("Service non démarré".to_string()),
        })
        .collect::<Vec<_>>();

    let state = RuntimeWorkspaceState {
        workspace_id: workspace.id.clone(),
        global_status: derive_global_status(&services),
        services,
        last_error: None,
    };

    Ok(state)
}

pub fn get_logs(workspace_id: &str) -> Vec<String> {
    if let Ok(guard) = registry().lock() {
        if let Some(runtime) = guard.workspaces.get(workspace_id) {
            if let Ok(logs) = runtime.logs.lock() {
                return logs.iter().cloned().collect();
            }
        }
    }
    Vec::new()
}

pub fn stop_workspace(mut current_state: RuntimeWorkspaceState) -> RuntimeWorkspaceState {
    clear_workspace_probe_state(&current_state.workspace_id);

    if let Ok(mut guard) = registry().lock() {
        if let Some(mut runtime) = guard.workspaces.remove(&current_state.workspace_id) {
            if let Some(compose_logs_runtime) = runtime.compose_logs_runtime.take() {
                compose_logs_runtime.stop_flag.store(true, Ordering::Relaxed);
            }
            if let Some(compose_runtime) = runtime.docker_compose {
                let _ = run_compose_command(&compose_runtime.workspace_root, compose_runtime.command_bin, &["down"]);
            }
            for (_, child) in &mut runtime.services {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    current_state.global_status = ServiceRuntimeStatus::Stopped;
    for service in &mut current_state.services {
        service.status = ServiceRuntimeStatus::Stopped;
        service.message = Some("Service arrêté proprement".to_string());
    }
    current_state
}

pub fn refresh_workspace_state(mut current_state: RuntimeWorkspaceState) -> Result<RuntimeWorkspaceState, String> {
    let mut guard = registry()
        .lock()
        .map_err(|_| "Impossible de verrouiller le registre runtime".to_string())?;

    if let Some(runtime) = guard.workspaces.get_mut(&current_state.workspace_id) {
        if runtime.docker_compose.is_some() {
            let snapshot = runtime
                .compose_logs_runtime
                .as_ref()
                .map(|runtime| runtime.state_snapshot.clone());
            drop(guard);

            if let Some(snapshot) = snapshot {
                if let Ok(guard) = snapshot.lock() {
                    if let Some(state) = guard.clone() {
                        return Ok(state);
                    }
                }
            }

            return Ok(current_state);
        }

        let workspace_id = current_state.workspace_id.clone();
        let mut finished_services = Vec::new();
        let any_target_defined = runtime
            .accessibility_targets_by_service
            .values()
            .any(|targets| !targets.is_empty());

        for service_state in &mut current_state.services {
            let service_error_log = runtime
                .logs
                .lock()
                .ok()
                .and_then(|logs| find_service_log_error(&logs, &service_state.name));

            match runtime.services.get_mut(&service_state.name) {
                Some(child) => match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        finished_services.push(service_state.name.clone());
                        if exit_status.success() {
                            service_state.status = ServiceRuntimeStatus::Stopped;
                            service_state.message = Some("Processus terminé".to_string());
                        } else {
                            service_state.status = ServiceRuntimeStatus::Failed;
                            service_state.message = Some(format!("Processus terminé en erreur ({exit_status})"));
                        }
                    }
                    Ok(None) => {
                        if let Some(error_log) = service_error_log {
                            service_state.status = ServiceRuntimeStatus::Failed;
                            service_state.message = Some(format!("Erreur détectée dans les logs: {error_log}"));
                            continue;
                        }

                        let service_targets = runtime
                            .accessibility_targets_by_service
                            .get(&service_state.name)
                            .cloned()
                            .unwrap_or_default();
                        
                        // Note: Pour refresh_workspace_state (mode process direct), on n'a pas forcément de depends_on_map ici
                        // Mais cette fonction est surtout pour le mode "background" sans Docker Compose.
                        let probe_status = service_probe_status(
                            &workspace_id,
                            &service_state.name,
                            &service_targets,
                            any_target_defined,
                            None,
                        );
                        match probe_status {
                            TcpProbeStatus::Ready => {
                                service_state.status = ServiceRuntimeStatus::Running;
                                service_state.message = Some("Processus en cours d'exécution".to_string());
                            }
                            TcpProbeStatus::Loading => {
                                service_state.status = ServiceRuntimeStatus::Starting;
                                service_state.message = Some(
                                    "Processus démarré, en attente d'accessibilité".to_string(),
                                );
                            }
                            TcpProbeStatus::Error => {
                                service_state.status = ServiceRuntimeStatus::Failed;
                                service_state.message = Some(
                                    "Processus inaccessible après démarrage".to_string(),
                                );
                            }
                        }
                    }
                    Err(error) => {
                        finished_services.push(service_state.name.clone());
                        service_state.status = ServiceRuntimeStatus::Failed;
                        service_state.message = Some(format!("Impossible d'inspecter le processus: {error}"));
                    }
                },
                None => {
                    if service_state.status == ServiceRuntimeStatus::Running || service_state.status == ServiceRuntimeStatus::Starting {
                        service_state.status = ServiceRuntimeStatus::Stopped;
                        service_state.message = Some("Aucun processus actif".to_string());
                    }
                }
            }
        }

        for service_name in finished_services {
            runtime.services.remove(&service_name);
        }
    } else if current_state.global_status == ServiceRuntimeStatus::Running
        || current_state.global_status == ServiceRuntimeStatus::Starting
    {
        for service in &mut current_state.services {
            if service.status == ServiceRuntimeStatus::Running || service.status == ServiceRuntimeStatus::Starting {
                service.status = ServiceRuntimeStatus::Stopped;
                service.message = Some("Aucun processus actif".to_string());
            }
        }
    }

    current_state.global_status = derive_global_status(&current_state.services);
    current_state.last_error = current_state
        .services
        .iter()
        .find(|service| service.status == ServiceRuntimeStatus::Failed)
        .and_then(|service| service.message.clone());

    Ok(current_state)
}

pub fn stop_all() {
    if let Ok(mut probe_guard) = http_probe_registry().lock() {
        probe_guard.results.clear();
        probe_guard.in_flight.clear();
    }

    if let Ok(mut guard) = registry().lock() {
        let workspaces = std::mem::take(&mut guard.workspaces);
        drop(guard);

        for (_, mut runtime) in workspaces {
            if let Some(compose_logs_runtime) = runtime.compose_logs_runtime.take() {
                compose_logs_runtime.stop_flag.store(true, Ordering::Relaxed);
            }
            if let Some(compose_runtime) = runtime.docker_compose {
                let _ = run_compose_command(&compose_runtime.workspace_root, compose_runtime.command_bin, &["down"]);
            }
            for (_, child) in &mut runtime.services {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests;
