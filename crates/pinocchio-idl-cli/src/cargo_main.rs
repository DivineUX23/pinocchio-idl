use clap::Parser;
use pinocchio_idl_cli::commands::{Cli, run};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args()
        .enumerate()
        .filter_map(|(i, arg)| {
            if i == 1 && arg == "pinocchio-idl" {
                None
            } else {
                Some(arg)
            }
        })
        .collect();

    let cli = Cli::parse_from(args);

    if let Err(e) = run(cli) {
        eprintln!("\x1b[31m\x1b[1m       Error\x1b[0m {:?}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
