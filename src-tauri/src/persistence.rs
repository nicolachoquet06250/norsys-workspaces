use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use directories::ProjectDirs;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::snapshot_manager::Snapshot;
use crate::workspace_loader::{ServiceConfig, WorkspaceConfig};

fn ensure_workspaces_services_column(conn: &Connection) -> Result<(), String> {
    match conn.execute(
        "ALTER TABLE workspaces ADD COLUMN services_json TEXT NOT NULL DEFAULT '[]'",
        [],
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_text = e.to_string();
            if error_text.contains("duplicate column name") {
                Ok(())
            } else {
                Err(format!("Impossible d'ajouter la colonne services_json: {error_text}"))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSettings {
    pub last_workspace_id: Option<String>,
    pub ui_state: String,
}

fn now_unix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn db_path() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "dev", "dev-workspace-manager")
        .ok_or_else(|| "Impossible de résoudre le dossier applicatif".to_string())?;
    let data_dir = dirs.data_local_dir();
    fs::create_dir_all(data_dir).map_err(|e| format!("Impossible de créer le dossier DB: {e}"))?;
    Ok(data_dir.join("workspace_state.sqlite"))
}

fn open_db() -> Result<Connection, String> {
    let path = db_path()?;
    Connection::open(path).map_err(|e| format!("Impossible d'ouvrir SQLite: {e}"))
}

pub fn init_schema() -> Result<(), String> {
    let conn = open_db()?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            root TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS recent_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            workspace_id TEXT NOT NULL,
            action TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_recent_runs_workspace ON recent_runs(workspace_id);

        CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            workspace_id TEXT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            runtime_json TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_snapshots_workspace ON snapshots(workspace_id);

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| format!("Impossible d'initialiser le schéma SQLite: {e}"))?;

    ensure_workspaces_services_column(&conn)?;
    Ok(())
}

pub fn upsert_workspace_catalog(workspaces: &[WorkspaceConfig]) -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let now = now_unix();
    for workspace in workspaces {
        tx.execute(
            "INSERT INTO workspaces(id, name, root, services_json, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, root=excluded.root, services_json=excluded.services_json, updated_at=excluded.updated_at",
            params![
                workspace.id,
                workspace.name,
                workspace.root,
                serde_json::to_string(&workspace.services).map_err(|e| e.to_string())?,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn load_workspace_catalog() -> Result<Vec<WorkspaceConfig>, String> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, root, services_json
             FROM workspaces
             ORDER BY name ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let services_json: String = row.get(3)?;
            let services = serde_json::from_str::<Vec<ServiceConfig>>(&services_json).unwrap_or_default();
            Ok(WorkspaceConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                root: row.get(2)?,
                services,
                open: vec![],
                env_files: vec![],
                env: HashMap::new(),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut workspaces = Vec::new();
    for row in rows {
        workspaces.push(row.map_err(|e| e.to_string())?);
    }

    Ok(workspaces)
}

pub fn create_workspace(workspace: &WorkspaceConfig) -> Result<(), String> {
    let conn = open_db()?;
    let services_json = serde_json::to_string(&workspace.services).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO workspaces(id, name, root, services_json, updated_at)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![workspace.id, workspace.name, workspace.root, services_json, now_unix()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_workspace(workspace_id: &str) -> Result<(), String> {
    let conn = open_db()?;
    let deleted_rows = conn
        .execute("DELETE FROM workspaces WHERE id = ?1", params![workspace_id])
        .map_err(|e| e.to_string())?;

    if deleted_rows == 0 {
        return Err(format!("Workspace introuvable: {workspace_id}"));
    }

    Ok(())
}

pub fn save_settings(settings: &PersistedSettings) -> Result<(), String> {
    let conn = open_db()?;
    conn.execute(
        "INSERT INTO settings(key, value) VALUES('last_workspace_id', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![settings.last_workspace_id.clone().unwrap_or_default()],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO settings(key, value) VALUES('ui_state', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![settings.ui_state],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn load_settings() -> Result<PersistedSettings, String> {
    let conn = open_db()?;
    let last_workspace_id: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'last_workspace_id'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let ui_state: String = conn
        .query_row("SELECT value FROM settings WHERE key = 'ui_state'", [], |row| row.get(0))
        .unwrap_or_else(|_| "idle".to_string());

    Ok(PersistedSettings {
        last_workspace_id: if last_workspace_id.is_empty() {
            None
        } else {
            Some(last_workspace_id)
        },
        ui_state,
    })
}

pub fn add_recent_run(workspace_id: &str, action: &str, status: &str) -> Result<(), String> {
    let conn = open_db()?;
    conn.execute(
        "INSERT INTO recent_runs(workspace_id, action, status, created_at) VALUES(?1, ?2, ?3, ?4)",
        params![workspace_id, action, status, now_unix()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_snapshot(snapshot: &Snapshot) -> Result<(), String> {
    let conn = open_db()?;
    let runtime_json = serde_json::to_string(&snapshot.runtime).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO snapshots(workspace_id, name, created_at, runtime_json) VALUES(?1, ?2, ?3, ?4)",
        params![snapshot.workspace_id, snapshot.name, snapshot.created_at, runtime_json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn find_snapshot(workspace_id: &str, name: &str) -> Result<Option<Snapshot>, String> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT workspace_id, name, created_at, runtime_json
             FROM snapshots
             WHERE workspace_id = ?1 AND name = ?2
             ORDER BY id DESC
             LIMIT 1",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query(params![workspace_id, name]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let runtime_json: String = row.get(3).map_err(|e| e.to_string())?;
        let runtime = serde_json::from_str(&runtime_json).map_err(|e| e.to_string())?;
        return Ok(Some(Snapshot {
            workspace_id: row.get(0).map_err(|e| e.to_string())?,
            name: row.get(1).map_err(|e| e.to_string())?,
            created_at: row.get(2).map_err(|e| e.to_string())?,
            runtime,
        }));
    }

    Ok(None)
}