use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use pinocchio_idl_cli::{build_idl, read_metadata};
use pinocchio_idl_core::render;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

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
    /// Generate an IDL from a Pinocchio program
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

        /// Output format: anchor (default) or codama
        #[arg(long, short = 'f', default_value = "anchor")]
        format: OutputFormat,
    },
    /// Check if the generated IDL matches the existing file
    Check {
        /// Path to Cargo.toml
        #[arg(long, short = 'm', default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Output path for the existing IDL to compare against
        #[arg(long, short = 'o', default_value = "idl.json")]
        out: PathBuf,

        /// Source directory override
        #[arg(long, short = 's')]
        src: Option<PathBuf>,

        /// Output format: anchor (default) or codama
        #[arg(long, short = 'f', default_value = "anchor")]
        format: OutputFormat,
    },
    /// Scan the codebase for missing or duplicate IDL annotations
    Doctor {
        /// Path to Cargo.toml
        #[arg(long, short = 'm', default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Source directory override
        #[arg(long, short = 's')]
        src: Option<PathBuf>,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Initialize AI agent rules for annotating the codebase
    InitAgents {
        /// Target directory to generate the rules in
        #[arg(long, short = 'd', default_value = ".")]
        dir: PathBuf,

        /// Overwrite existing agent files
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// Anchor-compatible IDL (default)
    Anchor,
    /// Native Codama rootNode JSON
    Codama,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprint!("Fatal Error: {:?}", e);
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
            format,
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

            let json = match format {
                OutputFormat::Codama => render::to_codama_json(&idl)
                    .context("Failed to serialize IDL to Codama JSON")?,
                OutputFormat::Anchor => serde_json::to_string_pretty(&idl)
                    .context("Failed to serialize IDL to Anchor JSON")?,
            };

            std::fs::write(&out, &json)
                .with_context(|| format!("Failed to write IDL to {}", out.display()))?;

            println!(
                "\x1b[32m\x1b[1m    Finished\x1b[0m generation in {:.2?} [{}]",
                start_time.elapsed(),
                out.display()
            );

            Ok(())
        }
        Command::Check {
            manifest_path,
            out,
            src,
            format,
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

            let generated_json = match format {
                OutputFormat::Codama => render::to_codama_json(&idl)
                    .context("Failed to serialize generated IDL to Codama JSON")?,
                OutputFormat::Anchor => serde_json::to_string_pretty(&idl)
                    .context("Failed to serialize generated IDL to Anchor JSON")?,
            };

            let existing_json = std::fs::read_to_string(&out).with_context(|| {
                format!(
                    "Failed to read existing IDL at {}. Did you run `generate` first?",
                    out.display()
                )
            })?;

            if generated_json.trim() == existing_json.trim() {
                println!(
                    "\x1b[32m\x1b[1m    Verified\x1b[0m IDL matches source in {:.2?} [{}]",
                    start_time.elapsed(),
                    out.display()
                );
                Ok(())
            } else {
                anyhow::bail!(
                    "\x1b[31m\x1b[1m    Drifted\x1b[0m IDL does not match the file at {}. Run `cargo pinocchio-idl generate` to update it.",
                    out.display()
                );
            }
        }
        Command::Doctor {
            manifest_path,
            src,
            json,
        } => {
            let (_, lib_path) = read_metadata(&manifest_path)
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

            let report = pinocchio_idl_cli::doctor::run_doctor(&src_dir)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.findings.is_empty() {
                println!(
                    "\x1b[32m\x1b[1m    No issues\x1b[0m Doctor found no missing annotations."
                );
            } else {
                for finding in &report.findings {
                    println!(
                        "\x1b[33m\x1b[1m{}:{}: \x1b[0m{}",
                        finding.file, finding.line, finding.message
                    );
                }
                anyhow::bail!("Doctor found {} issue(s)", report.findings.len());
            }

            Ok(())
        }
        Command::InitAgents { dir, force } => {
            pinocchio_idl_cli::init_agents::run_init_agents(&dir, force)
        }
    }
}
