mod env_manager;
mod orchestrator;
mod persistence;
mod snapshot_manager;
mod workspace_loader;

use std::collections::HashMap;
use std::sync::Mutex;

use orchestrator::{RuntimeWorkspaceState, ServiceRuntimeStatus};
use persistence::PersistedSettings;
use serde::{Deserialize, Serialize};
use snapshot_manager::Snapshot;
use workspace_loader::WorkspaceConfig;

#[derive(Default)]
struct AppState {
    runtime: Mutex<HashMap<String, RuntimeWorkspaceState>>,
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
fn start_workspace(workspace_id: String, state: tauri::State<AppState>) -> Result<WorkspaceStartResponse, String> {
    let workspace = workspace_loader::list_workspaces()?
        .into_iter()
        .find(|w| w.id == workspace_id)
        .ok_or_else(|| format!("Workspace introuvable: {workspace_id}"))?;

    let merged_env = env_manager::resolve_workspace_env(&workspace);
    let mut runtime_status = orchestrator::start_workspace(&workspace, &merged_env)?;

    runtime_status.last_error = None;

    let mut runtime_guard = state.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    runtime_guard.insert(workspace_id.clone(), runtime_status.clone());
    drop(runtime_guard);

    persistence::add_recent_run(
        &workspace_id,
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
fn get_workspace_runtime_state(workspace_id: String, state: tauri::State<AppState>) -> Result<RuntimeWorkspaceState, String> {
    let mut runtime_guard = state.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
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
fn stop_workspace(workspace_id: String, state: tauri::State<AppState>) -> Result<RuntimeWorkspaceState, String> {
    let mut runtime_guard = state.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    let current_state = runtime_guard
        .get(&workspace_id)
        .cloned()
        .ok_or_else(|| format!("Workspace non démarré: {workspace_id}"))?;

    let stopped_state = orchestrator::stop_workspace(current_state);
    runtime_guard.insert(workspace_id.clone(), stopped_state.clone());
    drop(runtime_guard);

    persistence::add_recent_run(&workspace_id, "stop", "success")?;

    Ok(stopped_state)
}

#[tauri::command]
fn stop_workspace_probes(workspace_id: String) -> Result<(), String> {
    orchestrator::stop_workspace_probes(&workspace_id);
    Ok(())
}

#[tauri::command]
fn get_logs(workspace_id: String, state: tauri::State<AppState>) -> Result<Vec<String>, String> {
    let _ = state;
    Ok(orchestrator::get_logs(&workspace_id))
}

#[tauri::command]
fn save_snapshot(workspace_id: String, name: String, state: tauri::State<AppState>) -> Result<Snapshot, String> {
    let runtime_guard = state.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
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
fn restore_snapshot(workspace_id: String, name: String, state: tauri::State<AppState>) -> Result<RuntimeWorkspaceState, String> {
    let snapshot = persistence::find_snapshot(&workspace_id, &name)?
        .ok_or_else(|| format!("Snapshot introuvable: {name}"))?;

    let restored = snapshot_manager::restore_snapshot(&snapshot);
    let mut runtime_guard = state.runtime.lock().map_err(|_| "Impossible de verrouiller le runtime".to_string())?;
    runtime_guard.insert(workspace_id, restored.clone());

    Ok(restored)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("Starting application...");

    persistence::init_schema().expect("error while initializing sqlite schema");

    println!("persistance is initialized");

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
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
            restore_snapshot
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
