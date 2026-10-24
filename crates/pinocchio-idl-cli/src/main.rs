use clap::Parser;
use pinocchio_idl_cli::commands::{Cli, run};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("\x1b[31m\x1b[1m       Error\x1b[0m {:?}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
