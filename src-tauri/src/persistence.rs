use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use directories::ProjectDirs;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::snapshot_manager::Snapshot;
use crate::workspace_loader::{ServiceConfig, WorkspaceConfig};

#[derive(Debug, Clone)]
pub struct PersistedServiceVolume {
    pub workspace_id: String,
    pub service_name: String,
    pub volume: String,
    pub host_path: String,
    pub updated_at: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRun {
    pub id: i32,
    pub workspace_id: String,
    pub service_name: Option<String>,
    pub action: String,
    pub status: String,
    pub created_at: String,
}

fn ensure_recent_runs_service_name_column(conn: &Connection) -> Result<(), String> {
    match conn.execute(
        "ALTER TABLE recent_runs ADD COLUMN service_name TEXT",
        [],
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_text = e.to_string();
            if error_text.contains("duplicate column name") {
                Ok(())
            } else {
                Err(format!("Impossible d'ajouter la colonne service_name: {error_text}"))
            }
        }
    }
}

fn ensure_service_volumes_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS service_volumes (
            workspace_id TEXT NOT NULL,
            service_name TEXT NOT NULL,
            volume TEXT NOT NULL,
            host_path TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (workspace_id, service_name, volume)
        );
        CREATE INDEX IF NOT EXISTS idx_service_volumes_workspace_service
            ON service_volumes(workspace_id, service_name);
        ",
    )
    .map_err(|e| format!("Impossible d'initialiser la table service_volumes: {e}"))
}

fn ensure_notifications_read_state_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS notifications_read_state (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            last_read_recent_run_id INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );
        INSERT OR IGNORE INTO notifications_read_state(id, last_read_recent_run_id, updated_at)
        VALUES(1, 0, '0');
        ",
    )
    .map_err(|e| format!("Impossible d'initialiser la table notifications_read_state: {e}"))
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
            service_name TEXT,
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
    ensure_recent_runs_service_name_column(&conn)?;
    ensure_service_volumes_table(&conn)?;
    ensure_notifications_read_state_table(&conn)?;
    Ok(())
}

pub fn save_workspace_service_volumes(
    workspace_id: &str,
    service_name: &str,
    volumes: &HashMap<String, String>,
) -> Result<(), String> {
    let workspace_id = workspace_id.trim();
    let service_name = service_name.trim();
    if workspace_id.is_empty() || service_name.is_empty() {
        return Ok(());
    }

    let mut conn = open_db()?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let now = now_unix();

    for (volume, host_path) in volumes {
        let volume = volume.trim();
        let host_path = host_path.trim();
        if volume.is_empty() || host_path.is_empty() {
            continue;
        }

        tx.execute(
            "INSERT INTO service_volumes(workspace_id, service_name, volume, host_path, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(workspace_id, service_name, volume)
             DO UPDATE SET host_path=excluded.host_path, updated_at=excluded.updated_at",
            params![workspace_id, service_name, volume, host_path, now],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())
}

pub fn load_workspace_service_volumes(
    workspace_id: &str,
    service_name: &str,
) -> Result<HashMap<String, String>, String> {
    let workspace_id = workspace_id.trim();
    let service_name = service_name.trim();
    if workspace_id.is_empty() || service_name.is_empty() {
        return Ok(HashMap::new());
    }

    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT volume, host_path
             FROM service_volumes
             WHERE workspace_id = ?1 AND service_name = ?2",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![workspace_id, service_name], |row| {
            let volume: String = row.get(0)?;
            let host_path: String = row.get(1)?;
            Ok((volume, host_path))
        })
        .map_err(|e| e.to_string())?;

    let mut results = HashMap::new();
    for row in rows {
        let (volume, host_path) = row.map_err(|e| e.to_string())?;
        results.insert(volume, host_path);
    }

    Ok(results)
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

pub fn get_setting(key: &str) -> Result<Option<String>, String> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![key]).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let value: String = row.get(0).map_err(|e| e.to_string())?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

pub fn set_setting(key: &str, value: &str) -> Result<(), String> {
    let conn = open_db()?;
    conn.execute(
        "INSERT INTO settings(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
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

pub fn add_recent_run(workspace_id: &str, service_name: Option<&str>, action: &str, status: &str) -> Result<(), String> {
    let conn = open_db()?;
    conn.execute(
        "INSERT INTO recent_runs(workspace_id, service_name, action, status, created_at) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![workspace_id, service_name, action, status, now_unix()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_recent_runs(limit: usize) -> Result<Vec<RecentRun>, String> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, workspace_id, service_name, action, status, created_at
             FROM (
                 SELECT * FROM recent_runs WHERE service_name IS NOT NULL ORDER BY id DESC
             )
             GROUP BY workspace_id, service_name
             ORDER BY id DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([limit], |row| {
            Ok(RecentRun {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                service_name: row.get(2)?,
                action: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    Ok(results)
}

pub fn get_unread_notifications_count() -> Result<usize, String> {
    let conn = open_db()?;
    let last_read_id: i64 = conn
        .query_row(
            "SELECT last_read_recent_run_id FROM notifications_read_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let unread_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (
                 SELECT id
                 FROM recent_runs
                 WHERE service_name IS NOT NULL AND id > ?1
                 GROUP BY workspace_id, service_name
             )",
            params![last_read_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(unread_count.max(0) as usize)
}

pub fn mark_notifications_as_read() -> Result<(), String> {
    let conn = open_db()?;
    let latest_recent_run_id: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(id), 0) FROM recent_runs WHERE service_name IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE notifications_read_state
         SET last_read_recent_run_id = ?1, updated_at = ?2
         WHERE id = 1",
        params![latest_recent_run_id, now_unix()],
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