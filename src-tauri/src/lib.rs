mod env_manager;
mod orchestrator;
mod persistence;
mod snapshot_manager;
mod workspace_loader;
mod system_stats;

use std::collections::HashMap;
use std::sync::Mutex;

use orchestrator::{RuntimeWorkspaceState, ServiceRuntimeStatus};
use persistence::PersistedSettings;
use serde::{Deserialize, Serialize};
use snapshot_manager::Snapshot;
use tauri::Manager;
use workspace_loader::WorkspaceConfig;

#[derive(Default)]
struct AppState {
    runtime: Mutex<HashMap<String, RuntimeWorkspaceState>>,
}

struct CombinedState {
    app: AppState,
    system: system_stats::SystemState,
}

impl Default for CombinedState {
    fn default() -> Self {
        Self {
            app: AppState::default(),
            system: system_stats::SystemState::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct WorkspaceStartResponse {
    workspace_id: String,
    status: RuntimeWorkspaceState,
}

#[tauri::command]
fn list_workspaces() -> Result<Vec<WorkspaceConfig>, String> {
    workspace_loader::list_workspaces()
}

#[tauri::command]
fn create_workspace(workspace: WorkspaceConfig) -> Result<WorkspaceConfig, String> {
    workspace_loader::create_workspace(workspace)
}

#[tauri::command]
fn delete_workspace(workspace_id: String) -> Result<(), String> {
    workspace_loader::delete_workspace(&workspace_id)
}

#[tauri::command]
fn detect_docker_services(root: String) -> Result<Vec<workspace_loader::ServiceConfig>, String> {
    workspace_loader::detect_docker_services(&root)
}

#[tauri::command]
fn get_persisted_settings() -> Result<PersistedSettings, String> {
    persistence::load_settings()
}

#[tauri::command]
fn save_persisted_settings(settings: PersistedSettings) -> Result<(), String> {
    persistence::save_settings(&settings)
}

#[tauri::command]
fn start_workspace(workspace_id: String, state: tauri::State<CombinedState>) -> Result<WorkspaceStartResponse, String> {
    let workspace = workspace_loader::list_workspaces()?
        .into_iter()
        .find(|w| w.id == workspace_id)
        .ok_or_else(|| format!("Workspace introuvable: {workspace_id}"))?;

    let merged_env = env_manager::resolve_workspace_env(&workspace);
    let mut runtime_status = orchestrator::start_workspace(&workspace, &merged_env)?;

    runtime_status.last_error = None;

    let mut runtime_guard = state.app.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    runtime_guard.insert(workspace_id.clone(), runtime_status.clone());
    drop(runtime_guard);

    persistence::add_recent_run(
        &workspace_id,
        Some("_all_"),
        "start",
        if runtime_status.global_status == ServiceRuntimeStatus::Running {
            "success"
        } else {
            "failed"
        },
    )?;

    Ok(WorkspaceStartResponse {
        workspace_id,
        status: runtime_status,
    })
}

#[tauri::command]
fn get_workspace_runtime_state(workspace_id: String, state: tauri::State<CombinedState>) -> Result<RuntimeWorkspaceState, String> {
    let mut runtime_guard = state.app.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    let current_state = if let Some(current_state) = runtime_guard.get(&workspace_id).cloned() {
        current_state
    } else {
        let workspace = workspace_loader::list_workspaces()?
            .into_iter()
            .find(|w| w.id == workspace_id)
            .ok_or_else(|| format!("Workspace introuvable: {workspace_id}"))?;
        let attached_state = orchestrator::attach_workspace_runtime(&workspace)?;
        runtime_guard.insert(workspace_id.clone(), attached_state.clone());
        attached_state
    };

    let refreshed = orchestrator::refresh_workspace_state(current_state)?;
    runtime_guard.insert(workspace_id, refreshed.clone());

    Ok(refreshed)
}

#[tauri::command]
fn stop_workspace(workspace_id: String, state: tauri::State<CombinedState>) -> Result<RuntimeWorkspaceState, String> {
    let mut runtime_guard = state.app.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    let current_state = runtime_guard
        .get(&workspace_id)
        .cloned()
        .ok_or_else(|| format!("Workspace non démarré: {workspace_id}"))?;

    let stopped_state = orchestrator::stop_workspace(current_state);
    runtime_guard.insert(workspace_id.clone(), stopped_state.clone());
    drop(runtime_guard);

    persistence::add_recent_run(&workspace_id, Some("_all_"), "stop", "success")?;

    Ok(stopped_state)
}

#[tauri::command]
fn stop_workspace_probes(workspace_id: String) -> Result<(), String> {
    orchestrator::stop_workspace_probes(&workspace_id);
    Ok(())
}

#[tauri::command]
fn get_logs(workspace_id: String, state: tauri::State<CombinedState>) -> Result<Vec<String>, String> {
    let _ = state;
    Ok(orchestrator::get_logs(&workspace_id))
}

#[tauri::command]
fn save_snapshot(workspace_id: String, name: String, state: tauri::State<CombinedState>) -> Result<Snapshot, String> {
    let runtime_guard = state.app.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    let runtime = runtime_guard
        .get(&workspace_id)
        .cloned()
        .ok_or_else(|| format!("Impossible de sauvegarder un snapshot: workspace non démarré ({workspace_id})"))?;
    drop(runtime_guard);

    let snapshot = snapshot_manager::build_snapshot(&workspace_id, &name, &runtime);
    persistence::save_snapshot(&snapshot)?;

    Ok(snapshot)
}

#[tauri::command]
fn restore_snapshot(workspace_id: String, name: String, state: tauri::State<CombinedState>) -> Result<RuntimeWorkspaceState, String> {
    let snapshot = persistence::find_snapshot(&workspace_id, &name)?
        .ok_or_else(|| format!("Snapshot introuvable: {name}"))?;

    let restored = snapshot_manager::restore_snapshot(&snapshot);
    let mut runtime_guard = state.app.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    runtime_guard.insert(workspace_id, restored.clone());

    Ok(restored)
}

#[tauri::command]
fn get_os_username() -> Result<String, String> {
    let name = whoami::realname();
    Ok(if name.is_empty() { whoami::username() } else { name })
}

#[tauri::command]
fn get_os_email() -> Result<String, String> {
    use std::process::Command;

    fn get_upn_email() -> Option<String> {
        if !cfg!(target_os = "windows") {
            return None;
        }

        let mut cmd = Command::new("whoami");
        cmd.arg("/UPN");
        orchestrator::apply_production_process_flags(&mut cmd);

        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }

        let upn = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if upn.is_empty() || !upn.contains('@') {
            None
        } else {
            Some(upn)
        }
    }

    fn get_microsoft_account_email() -> Option<String> {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-Command",
            r#"Get-ItemProperty "HKCU:\Software\Microsoft\IdentityCRL\UserExtendedProperties\*" | Select-Object -ExpandProperty PSChildName"#,
        ]);
        orchestrator::apply_production_process_flags(&mut cmd);

        let output = cmd
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        stdout
            .lines()
            .map(str::trim)
            .find(|line| line.contains('@'))
            .map(String::from)
    }

    get_upn_email()
        .or_else(get_microsoft_account_email)
        .ok_or_else(|| "Aucun email Microsoft trouvé".to_string())
}

#[tauri::command]
async fn is_docker_connected() -> bool {
    use bollard::Docker;
    use std::env;

    let docker_host = env::var("DOCKER_HOST")
        .unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "tcp://127.0.0.1:2375".to_string()
            } else {
                "unix:///var/run/docker.sock".to_string()
            }
        });

    let docker = if docker_host.starts_with("tcp://") {
        let http_url = docker_host.replacen("tcp://", "http://", 1);
        Docker::connect_with_http(
            &http_url,
            2, // timeout court pour ne pas bloquer l'UI
            bollard::API_DEFAULT_VERSION,
        )
    } else {
        Docker::connect_with_local_defaults()
    };

    match docker {
        Ok(client) => client.ping().await.is_ok(),
        Err(err) => {
            println!("Failed to connect to Docker: {}", err);

            false
        },
    }
}

#[tauri::command]
fn get_recent_runs(limit: usize) -> Result<Vec<persistence::RecentRun>, String> {
    persistence::get_recent_runs(limit)
}

#[tauri::command]
fn add_recent_run(workspace_id: String, service_name: Option<String>, action: String, status: String) -> Result<(), String> {
    persistence::add_recent_run(&workspace_id, service_name.as_deref(), &action, &status)
}

#[tauri::command]
fn get_system_stats(state: tauri::State<CombinedState>) -> system_stats::SystemStats {
    system_stats::get_system_stats(&state.system)
}

#[tauri::command]
fn close_splashscreen(window: tauri::WebviewWindow) -> Result<(), String> {
    if let Some(splashscreen) = window.get_webview_window("splashscreen") {
        splashscreen.close().map_err(|err| err.to_string())?;
    }

    if let Some(main) = window.get_webview_window("main") {
        main.show().map_err(|err| err.to_string())?;
        main.set_focus().map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("Starting application...");

    orchestrator::init_host_flags();

    persistence::init_schema().expect("error while initializing sqlite schema");

    println!("persistance is initialized");

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .manage(CombinedState::default())
        .invoke_handler(tauri::generate_handler![
            list_workspaces,
            create_workspace,
            delete_workspace,
            detect_docker_services,
            get_persisted_settings,
            save_persisted_settings,
            start_workspace,
            get_workspace_runtime_state,
            stop_workspace,
            stop_workspace_probes,
            get_logs,
            save_snapshot,
            restore_snapshot,
            get_os_username,
            get_os_email,
            is_docker_connected,
            get_recent_runs,
            add_recent_run,
            get_system_stats,
            close_splashscreen
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let _ = window;
                orchestrator::stop_all();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
