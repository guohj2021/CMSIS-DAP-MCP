use cmsis_dap_mcp::backend::probe_rs::ProbeRsBackend;
use cmsis_dap_mcp::cli::AppConfig;
use cmsis_dap_mcp::mcp::CmsisDapMcp;
use cmsis_dap_mcp::security::SecurityPolicy;
use cmsis_dap_mcp::session::SessionManager;
use rmcp::ServiceExt;
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
        Some(path) => {
            let yaml = std::fs::read_to_string(path)?;
            let mut registry = probe_rs::config::Registry::from_builtin_families();
            let name = registry
                .add_target_family_from_yaml(&yaml)
                .map_err(|e| format!("failed to parse target yaml {}: {e}", path.display()))?;
            tracing::info!("loaded target family {name} from {}", path.display());
            ProbeRsBackend::with_registry(registry)
        }
        None => ProbeRsBackend::new(),
    };
    tracing::info!(
        "starting cmsis-dap-mcp (destructive={})",
        cfg.allow_destructive
    );
    let session = SessionManager::new(Box::new(backend));
    let policy = SecurityPolicy {
        allow_destructive: cfg.allow_destructive,
    };
    let mcp = CmsisDapMcp::new(session, policy);
    let running = mcp.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;
    Ok(())
}
