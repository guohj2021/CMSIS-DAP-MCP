use clap::Parser;
use cmsis_dap_cli::cmd::{make_backend, output, run, CliArgs};
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let args = CliArgs::parse();
    init_tracing(&args);
    let backend = if let Some(flm_path) = &args.flm {
        let flash_start = match args.flash_start {
            Some(v) => v,
            None => {
                eprintln!("cmsis-dap-cli: --flash-start is required with --flm");
                return ExitCode::from(2);
            }
        };
        let flash_size = match args.flash_size {
            Some(v) => v,
            None => {
                eprintln!("cmsis-dap-cli: --flash-size is required with --flm");
                return ExitCode::from(2);
            }
        };
        let sram_start = match args.sram_start {
            Some(v) => v,
            None => {
                eprintln!("cmsis-dap-cli: --sram-start is required with --flm");
                return ExitCode::from(2);
            }
        };
        let sram_size = match args.sram_size {
            Some(v) => v,
            None => {
                eprintln!("cmsis-dap-cli: --sram-size is required with --flm");
                return ExitCode::from(2);
            }
        };
        match cmsis_dap_core::flm::registry_from_flm(
            flm_path,
            args.target.as_deref(),
            flash_start,
            flash_size,
            sram_start,
            sram_size,
            &args.core,
        ) {
            Ok(registry) => {
                Box::new(cmsis_dap_core::backend::probe_rs::ProbeRsBackend::with_registry(registry))
                    as Box<dyn cmsis_dap_core::backend::Backend>
            }
            Err(e) => {
                eprintln!("cmsis-dap-cli: error: {e}");
                return ExitCode::from(1);
            }
        }
    } else {
        match make_backend(args.target_yaml.as_deref()) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("cmsis-dap-cli: error: {e}");
                return ExitCode::from(1);
            }
        }
    };
    let json_mode = args.json;
    match run(args, backend) {
        Ok(Some(output)) => {
            output::print_result(json_mode, &output);
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cmsis-dap-cli: error: {e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn init_tracing(args: &CliArgs) {
    let filter = EnvFilter::try_new(&args.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    match &args.log_file {
        Some(path) => {
            if let Ok(file) = std::fs::File::create(path) {
                let _ = tracing_subscriber::fmt()
                    .with_env_filter(filter)
                    .with_writer(file)
                    .try_init();
            }
        }
        None => {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(std::io::stderr)
                .try_init();
        }
    }
}
