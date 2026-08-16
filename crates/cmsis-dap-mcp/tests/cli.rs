use cmsis_dap_mcp::cli::{AppConfig, CliError};

#[test]
fn parses_destructive_flag() {
    let cfg = AppConfig::parse_from(["cmsis-dap-mcp", "--allow-destructive"]).unwrap();
    assert!(cfg.allow_destructive);
}

#[test]
fn rejects_unknown_protocol() {
    let err = AppConfig::parse_from(["cmsis-dap-mcp", "--protocol", "i2c"]).unwrap_err();
    assert!(matches!(err, CliError::InvalidProtocol(_)));
}

#[test]
fn parses_target_yaml_flag() {
    let cfg = AppConfig::parse_from(["cmsis-dap-mcp", "--target-yaml", "target.yaml"]).unwrap();
    assert_eq!(
        cfg.target_yaml.as_deref(),
        Some(std::path::Path::new("target.yaml"))
    );
}
