use cmsis_dap_core::backend::probe_rs::{registry_from_yaml, ProbeRsBackend};
use cmsis_dap_core::backend::Protocol;
use cmsis_dap_core::gdb::GdbServerOptions;
use cmsis_dap_core::session::SessionManager;
use cmsis_dap_mcp::cli::AppConfig;
use cmsis_dap_mcp::config::ServerConfig;
use cmsis_dap_mcp::mcp::CmsisDapMcp;
use cmsis_dap_mcp::runtime::ServerRuntime;
use rmcp::ServiceExt;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = AppConfig::parse_from(std::env::args_os())?;
    let filter = EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    match &cfg.log_file {
        Some(path) => {
            let file = std::fs::File::create(path)?;
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(file)
                .init();
        }
        None => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(std::io::stderr)
                .init();
        }
    }

    // Initial runtime config: merge an optional --config-file with CLI args
    // (CLI wins). With no args and no file the server starts in the
    // to-be-configured state: all read/write tools are usable and destructive
    // tools stay gated until enabled via update_config / --allow-destructive.
    let file_cfg = match &cfg.config_file {
        Some(path) => match cmsis_dap_mcp::config::load_config_file(path) {
            Ok(f) => Some(f),
            Err(e) => {
                tracing::warn!("could not load --config-file {path:?}: {e}");
                None
            }
        },
        None => None,
    };
    let config = ServerConfig::from_cli(&cfg, file_cfg);

    let backend = match &cfg.target_yaml {
        Some(path) => ProbeRsBackend::with_registry(registry_from_yaml(path)?),
        None => ProbeRsBackend::new(),
    };
    tracing::info!(
        "starting cmsis-dap-mcp (destructive={}, tcp={:?}, gdb={:?})",
        config.allow_destructive,
        config.tcp_port,
        config.gdb_port
    );

    let session = Arc::new(Mutex::new(SessionManager::new(Box::new(backend))));

    let gdb_options = GdbServerOptions {
        probe_id: cfg.probe_id.clone(),
        protocol: cfg.protocol.as_deref().map(|p| {
            if p == "jtag" {
                Protocol::Jtag
            } else {
                Protocol::Swd
            }
        }),
        speed_khz: cfg.speed_khz,
        target: cfg.target.clone(),
        target_yaml: cfg.target_yaml.clone(),
        reset_halt: false,
    };

    let runtime = ServerRuntime::from_session(session, config.clone(), gdb_options);

    // Start any servers requested by the initial config. Backward compatible
    // with --tcp / --gdb-port at startup, and a no-op otherwise.
    runtime.reconcile();

    // Optional: auto-reload the config file on change (no restart needed).
    #[cfg(feature = "config-watch")]
    if let Some(path) = &cfg.config_file {
        cmsis_dap_mcp::runtime::spawn_config_watcher(Arc::clone(&runtime), path.clone());
    }

    let mcp = CmsisDapMcp::from_shared(runtime);
    let running = mcp.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;
    Ok(())
}
