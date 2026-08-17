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
    let backend = match &cfg.target_yaml {
        Some(path) => ProbeRsBackend::with_registry(registry_from_yaml(path)?),
        None => ProbeRsBackend::new(),
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
