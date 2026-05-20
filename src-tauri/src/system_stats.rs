use serde::Serialize;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use std::sync::Mutex;
use std::path::Path;
use std::fs;
use crate::workspace_loader;

#[derive(Serialize, Clone)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
}

pub struct SystemState {
    pub sys: Mutex<System>,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            sys: Mutex::new(System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything())
            )),
        }
    }
}

fn get_dir_size<P: AsRef<Path>>(path: P) -> u64 {
    let mut total_size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total_size += get_dir_size(&path);
            } else if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }
    total_size
}

pub fn get_system_stats(state: &SystemState) -> SystemStats {
    let mut sys = state.sys.lock().unwrap();
    
    // Refresh CPU and memory
    sys.refresh_cpu_all();
    sys.refresh_memory();
    
    let cpu_usage = sys.global_cpu_usage();
    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();
    
    // Calculate disk used by workspaces
    let mut disk_used = 0;
    if let Ok(workspaces) = workspace_loader::list_workspaces() {
        for ws in workspaces {
            disk_used += get_dir_size(ws.root);
        }
    }
    
    // Still get total disk space for context
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut disk_total = 0;
    for disk in &disks {
        disk_total += disk.total_space();
    }
    
    SystemStats {
        cpu_usage,
        memory_used,
        memory_total,
        disk_used,
        disk_total,
    }
}
