use cmsis_dap_core::backend::probe_rs::{registry_from_yaml, ProbeRsBackend};
use cmsis_dap_core::backend::Protocol;
use cmsis_dap_core::gdb::{connect_and_serve, GdbServerOptions};
use cmsis_dap_core::remote;
use cmsis_dap_core::security::SecurityPolicy;
use cmsis_dap_core::session::SessionManager;
use cmsis_dap_mcp::cli::AppConfig;
use cmsis_dap_mcp::mcp::CmsisDapMcp;
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
        "starting cmsis-dap-mcp (destructive={})",
        cfg.allow_destructive
    );
    let session = Arc::new(Mutex::new(SessionManager::new(Box::new(backend))));
    let policy = SecurityPolicy {
        allow_destructive: cfg.allow_destructive,
    };

    if let Some(port) = cfg.tcp {
        let shared = Arc::clone(&session);
        tokio::spawn(async move {
            if let Err(e) = remote::serve(&shared, &format!("127.0.0.1:{port}")).await {
                eprintln!("remote TCP server error: {e}");
            }
        });
    }

    if let Some(port) = cfg.gdb_port {
        let options = GdbServerOptions {
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
        std::thread::spawn(move || {
            if let Err(e) = connect_and_serve(options, Some(&format!("127.0.0.1:{port}"))) {
                eprintln!("GDB server error: {e}");
            }
        });
    }

    let mcp = CmsisDapMcp::from_shared(session, policy);
    let running = mcp.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;
    Ok(())
}
