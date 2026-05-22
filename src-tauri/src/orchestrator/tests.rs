use super::{
    build_accessibility_targets_by_service, derive_global_status, is_endpoint_accessible,
    parse_compose_ps_json_statuses, parse_compose_ps_published_ports, service_probe_status,
    is_service_accessible, start_order_levels,
    extract_probe_port, ServiceRuntimeState, ServiceRuntimeStatus, TcpProbeStatus,
};
use crate::workspace_loader::{ServiceConfig, WorkspaceConfig};
use std::collections::HashMap;
use std::net::TcpListener;
use std::thread;

#[test]
fn resolves_dependency_levels() {
    let services = vec![
        ServiceConfig {
            name: "api".to_string(),
            display_name: None,
            command: "echo api".to_string(),
            cwd: None,
            depends_on: vec![],
            mode: "foreground".to_string(),
            kind: "web".to_string(),
            env: HashMap::new(),
            ports: vec![],
            image: None,
        },
        ServiceConfig {
            name: "web".to_string(),
            display_name: None,
            command: "echo web".to_string(),
            cwd: None,
            depends_on: vec!["api".to_string()],
            mode: "foreground".to_string(),
            kind: "web".to_string(),
            env: HashMap::new(),
            ports: vec![],
            image: None,
        },
    ];

    let workspace = WorkspaceConfig {
        id: "w".to_string(),
        name: "w".to_string(),
        root: "C:\\".to_string(),
        services,
        open: vec![],
        env_files: vec![],
        env: HashMap::new(),
    };

    let levels = start_order_levels(&workspace).expect("ordre valide");
    assert_eq!(levels.len(), 2);
    assert_eq!(levels[0][0].name, "api");
    assert_eq!(levels[1][0].name, "web");
}

#[test]
fn derives_stopped_when_no_service_is_running() {
    let services = vec![ServiceRuntimeState {
        name: "echo".to_string(),
        display_name: None,
        status: ServiceRuntimeStatus::Stopped,
        message: Some("terminé".to_string()),
    }];

    let global = derive_global_status(&services);
    assert_eq!(global, ServiceRuntimeStatus::Stopped);
}

#[test]
fn derives_starting_when_any_service_is_starting() {
    let services = vec![
        ServiceRuntimeState {
            name: "api".to_string(),
            display_name: None,
            status: ServiceRuntimeStatus::Starting,
            message: Some("en attente".to_string()),
        },
        ServiceRuntimeState {
            name: "worker".to_string(),
            display_name: None,
            status: ServiceRuntimeStatus::Running,
            message: Some("ok".to_string()),
        },
    ];

    let global = derive_global_status(&services);
    assert_eq!(global, ServiceRuntimeStatus::Starting);
}

#[test]
fn derives_stopped_when_services_move_from_starting_to_stopped() {
    let services = vec![
        ServiceRuntimeState {
            name: "api".to_string(),
            display_name: None,
            status: ServiceRuntimeStatus::Starting,
            message: Some("chargement".to_string()),
        },
        ServiceRuntimeState {
            name: "worker".to_string(),
            display_name: None,
            status: ServiceRuntimeStatus::Stopped,
            message: Some("arrêté".to_string()),
        },
    ];

    let global = derive_global_status(&services);
    assert_eq!(global, ServiceRuntimeStatus::Stopped);
}

#[test]
fn maps_accessibility_targets_per_service() {
    let service_names = vec!["backend".to_string(), "db".to_string()];
    let targets = vec![
        "backend=http://localhost:4000".to_string(),
        "db=http://localhost:5432".to_string(),
    ];

    let mapping = build_accessibility_targets_by_service(&service_names, &targets);

    assert_eq!(
        mapping.get("backend").cloned().unwrap_or_default(),
        vec!["http://localhost:4000".to_string()]
    );
    assert_eq!(
        mapping.get("db").cloned().unwrap_or_default(),
        vec!["http://localhost:5432".to_string()]
    );
}

#[test]
fn keeps_web_service_starting_when_targets_are_defined_but_not_mapped() {
    let accessible = is_service_accessible("w", "backend", &[], true);
    assert!(!accessible);
}

#[test]
fn keeps_non_web_service_starting_when_targets_are_defined_but_not_mapped() {
    let accessible = is_service_accessible("w", "db", &[], true);
    assert!(!accessible);
}

#[test]
fn keeps_service_starting_when_no_accessibility_targets_are_defined() {
    let probe_status = service_probe_status("w", "backend", &[], false, None);
    assert_eq!(probe_status, TcpProbeStatus::Loading);
}

#[test]
fn keeps_http_service_starting_when_server_accepts_but_sends_no_http_response() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener ok");
    let addr = listener.local_addr().expect("addr ok");

    thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            drop(stream);
        }
    });

    let endpoint = format!("http://127.0.0.1:{}", addr.port());
    let accessible = is_endpoint_accessible("w", "backend", &endpoint);

    assert!(!accessible);
}

#[test]
fn extracts_probe_port_from_plain_port_target() {
    assert_eq!(extract_probe_port("4000"), Some(4000));
}

#[test]
fn parses_compose_ps_json_status_and_exit_code() {
    let raw = br#"[
        {"Service":"backend","State":"exited","ExitCode":137},
        {"Service":"db","State":"running","ExitCode":0}
    ]"#;

    let statuses = parse_compose_ps_json_statuses(raw);

    assert_eq!(statuses.get("backend"), Some(&("exited".to_string(), Some(137))));
    assert_eq!(statuses.get("db"), Some(&("running".to_string(), Some(0))));
}

#[test]
fn parses_compose_ps_json_published_tcp_ports() {
    let raw = br#"[
        {"Service":"backend","Publishers":[{"PublishedPort":4000,"Protocol":"tcp"},{"PublishedPort":4444,"Protocol":"udp"}]},
        {"Service":"db","Publishers":[{"PublishedPort":5432,"Protocol":"tcp"}]}
    ]"#;

    let ports = parse_compose_ps_published_ports(raw);

    assert_eq!(ports.get("backend").cloned().unwrap_or_default(), vec![4000]);
    assert_eq!(ports.get("db").cloned().unwrap_or_default(), vec![5432]);
}
