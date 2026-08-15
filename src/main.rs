use cmsis_dap_mcp::cli::AppConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = AppConfig::parse_from(std::env::args_os())?;
    let filter = EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).init();
    tracing::info!("starting cmsis-dap-mcp (destructive={})", cfg.allow_destructive);
    Ok(())
}