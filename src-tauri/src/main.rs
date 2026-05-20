// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    dev_workspace_manager_lib::run()
}
