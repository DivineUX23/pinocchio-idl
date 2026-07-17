use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pinocchio_idl_cli::{build_idl, read_metadata, write_idl};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "cargo pinocchio-idl",
    version,
    about = "Generate an Anchor-compatible IDL for Pinocchio Solana programs (cargo subcommand)",
    long_about = "The official cargo subcommand for generating Anchor-compatible IDL files from \
                  Pinocchio programs.\n\nZero runtime overhead. Full Anchor + Codama compatibility.",
    after_help = "EXAMPLES:\n    \
                  cargo pinocchio-idl generate\n    \
                  cargo pinocchio-idl generate --manifest-path ./Cargo.toml --out ./target/idl.json"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate an Anchor-compatible IDL from a Pinocchio program
    Generate {
        /// Path to Cargo.toml
        #[arg(long, short = 'm', default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Output path for the generated IDL
        #[arg(long, short = 'o', default_value = "idl.json")]
        out: PathBuf,

        /// Source directory override
        #[arg(long, short = 's')]
        src: Option<PathBuf>,
    },
}

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
        eprintln!("Fatal Error: {:?}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Generate {
            manifest_path,
            out,
            src,
        } => {
            let start_time = Instant::now();

            let (metadata, lib_path) = read_metadata(&manifest_path)
                .with_context(|| format!("Failed to read metadata {}", manifest_path.display()))?;

            let src_dir = src.unwrap_or_else(|| {
                if let Some(p) = lib_path {
                    manifest_path
                        .parent()
                        .unwrap_or(Path::new("."))
                        .join(p)
                        .parent()
                        .unwrap_or(Path::new("."))
                        .to_path_buf()
                } else {
                    manifest_path.parent().unwrap_or(Path::new(".")).join("src")
                }
            });

            let idl = build_idl(&src_dir, metadata)
                .context("IDL generation process failed - check #[p_instruction(...)]")?;

            write_idl(&idl, &out)
                .with_context(|| format!("Failed to write IDL to {}", out.display()))?;

            //println!("✅ Successfully wrote {}", out.display());

            println!(
                "\x1b[32m\x1b[1m    Finished\x1b[0m generation in {:.2?} [{}]",
                start_time.elapsed(),
                out.display()
            );

            Ok(())
        }
    }
}
