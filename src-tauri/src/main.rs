// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/*use bollard::Docker;
use std::env;

fn connect_docker() -> anyhow::Result<Docker> {
    let docker_host = env::var("DOCKER_HOST")
        .unwrap_or_else(|_| "tcp://127.0.0.1:2375".to_string());

    let docker = if docker_host.starts_with("tcp://") {
        let http_url = docker_host.replacen("tcp://", "http://", 1);

        Docker::connect_with_http(
            &http_url,
            120,
            bollard::API_DEFAULT_VERSION,
        )?
    } else {
        Docker::connect_with_local_defaults()?
    };

    Ok(docker)
}*/

fn main() {
    /*let docker = connect_docker().expect("Impossible de se connecter à docker");

    eprintln!("Docker client initialized successfully {docker:?}");*/

    dev_workspace_manager_lib::run()
}
