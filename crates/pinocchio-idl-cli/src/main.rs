use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pinocchio_idl_cli::{build_idl, read_metadata, write_idl};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "pinocchio-idl",
    version,
    about = "Generate an Anchor-compatible IDL for Pinocchio Solana program",
    long_about = "The official CLI for generating Anchor-compatible IDL files from Pinocchio programs.\n\n\
                  Zero runtime overhead. Full Anchor + Codama compatibility.",
    after_help = "EXAMPLES:\n    \
                  pinocchio-idl generate\n    \
                  pinocchio-idl generate --manifest-path ./Cargo.toml --out ./target/idl.json"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate an Anchor-compatible IDL from a Pinocchio program (default command)
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
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprint!("Fatal Error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS

    /*
    match cli.command {
        Command::Generate { manifest_path, out } => {
            let src_dir = manifest_path.parent().unwrap_or(Path::new(".")).join("src");

            let metadata = read_metadata(&manifest_path).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });

            let result = build_idl(&src_dir, metadata)
                .map_err(|e| e.to_string())
                .and_then(|idl| write_idl(&idl, &out).map_err(|e| e.to_string()));

            if let Err(msg) = result {
                eprintln!("{msg}");
                std::process::exit(1);
            }

            println!("wrote {}", out.display());
        }
    }
    */
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Generate {
            manifest_path,
            out,
            src,
        } => {
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

            let idl = build_idl(&src_dir, metadata).context("IDL generation process failed")?;

            write_idl(&idl, &out)
                .with_context(|| format!("Failed to write IDL to {}", out.display()))?;

            println!("✅ Successfully wrote {}", out.display());
            Ok(())
        }
    }
}
