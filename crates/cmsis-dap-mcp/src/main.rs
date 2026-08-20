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

    let backend = match (&cfg.target_yaml, &cfg.flm) {
        (Some(path), _) => ProbeRsBackend::with_registry(registry_from_yaml(path)?),
        (None, Some(flm_path)) => {
            let flash_start = cfg.flash_start.ok_or("flash-start required with flm")?;
            let flash_size = cfg.flash_size.ok_or("flash-size required with flm")?;
            let sram_start = cfg.sram_start.ok_or("sram-start required with flm")?;
            let sram_size = cfg.sram_size.ok_or("sram-size required with flm")?;
            let registry = cmsis_dap_core::flm::registry_from_flm(
                flm_path,
                cfg.target.as_deref(),
                flash_start,
                flash_size,
                sram_start,
                sram_size,
                &cfg.core,
            )?;
            ProbeRsBackend::with_registry(registry)
        }
        (None, None) => ProbeRsBackend::new(),
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
