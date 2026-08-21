use clap::{Args, Subcommand};

use super::{parse_u32_arg, CliError};
use cmsis_dap_core::backend::OptionByte;
use cmsis_dap_core::session::SessionManager;

#[derive(Debug, Args)]
pub struct OptionArgs {
    #[command(subcommand)]
    pub action: OptionAction,
}

#[derive(Debug, Subcommand)]
pub enum OptionAction {
    /// Read chip option bytes.
    Read,
    /// Write a chip option byte.
    Write(OptionWriteArgs),
}

#[derive(Debug, Args)]
pub struct OptionWriteArgs {
    /// Option byte name (RDP, USER, DATA0, DATA1).
    pub name: String,
    /// Value to write.
    #[arg(value_parser = parse_u32_arg)]
    pub value: u32,
}

pub fn option_read(session: &mut SessionManager) -> Result<serde_json::Value, CliError> {
    let bytes = session.backend().read_option_bytes()?;
    Ok(serde_json::json!({ "option_bytes": bytes }))
}

pub fn option_write(
    session: &mut SessionManager,
    args: &OptionWriteArgs,
) -> Result<serde_json::Value, CliError> {
    let byte = OptionByte {
        name: args.name.clone(),
        address: 0,
        value: args.value,
        description: None,
    };
    session.backend().write_option_bytes(&[byte])?;
    Ok(serde_json::json!({ "written": true }))
}
