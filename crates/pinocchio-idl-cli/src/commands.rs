use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::{build_idl, read_metadata};
use pinocchio_idl_core::render;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "pinocchio-idl",
    version,
    about = "Generate an Anchor-compatible IDL for Pinocchio Solana program",
    long_about = "The official CLI for generating Anchor-compatible IDL files from Pinocchio programs.\n\n\
                  Zero runtime overhead. Full Anchor + Codama compatibility.",
    after_help = "EXAMPLES:\n    \
                  pinocchio-idl generate\n    \
                  pinocchio-idl generate --manifest-path ./Cargo.toml --out ./target/idl.json\n    \
                  pinocchio-idl generate -f codama\n    \
                  pinocchio-idl client --out ./clients/js\n    \
                  pinocchio-idl doctor\n    \
                  pinocchio-idl init my-program"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Generate an IDL from a Pinocchio program
    #[command(alias = "gen", alias = "build")]
    Generate {
        /// Path to Cargo.toml
        #[arg(long, short = 'm', default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Output path for the generated IDL
        #[arg(long, short = 'o', alias = "output")]
        out: Option<PathBuf>,

        /// Source directory override
        #[arg(long, short = 's')]
        src: Option<PathBuf>,

        /// Output format: anchor (default) or codama
        #[arg(long, short = 'f', alias = "format", default_value = "anchor")]
        format: OutputFormat,
    },
    /// Check if the generated IDL matches the existing file
    #[command(alias = "verify")]
    Check {
        /// Path to Cargo.toml
        #[arg(long, short = 'm', default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Output path for the existing IDL to compare against
        #[arg(long, short = 'o', alias = "output")]
        out: Option<PathBuf>,

        /// Source directory override
        #[arg(long, short = 's')]
        src: Option<PathBuf>,

        /// Output format: anchor (default) or codama
        #[arg(long, short = 'f', alias = "format", default_value = "anchor")]
        format: OutputFormat,
    },
    /// Scan the codebase for missing or duplicate IDL annotations
    #[command(alias = "lint", alias = "diagnose")]
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
    #[command(alias = "init_agents", alias = "agents")]
    InitAgents {
        /// Target directory to generate the rules in
        #[arg(long, short = 'd', default_value = ".")]
        dir: PathBuf,

        /// Overwrite existing agent files
        #[arg(long)]
        force: bool,

        /// Generate rules for all AI assistants (AGENTS.md, CLAUDE.md, and Cursor rules)
        #[arg(long)]
        all: bool,
    },
    /// Scaffold a new Pinocchio Solana program with pinocchio-idl pre-configured
    #[command(alias = "new")]
    Init {
        /// Name of the project / directory to create
        name: String,

        /// Target directory to create the project in (defaults to current dir)
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
    /// Generate a TypeScript/JavaScript client SDK using Codama
    #[command(alias = "codegen", alias = "sdk", alias = "ts")]
    Client {
        /// Path to Cargo.toml
        #[arg(long, short = 'm', default_value = "Cargo.toml")]
        manifest_path: PathBuf,

        /// Output directory for the TypeScript SDK
        #[arg(
            long,
            short = 'o',
            alias = "ts",
            alias = "output",
            default_value = "clients/js"
        )]
        out: PathBuf,

        /// Source directory override
        #[arg(long, short = 's')]
        src: Option<PathBuf>,

        /// Custom npm package name
        #[arg(long)]
        package_name: Option<String>,

        /// Only scaffold the codama.mjs script without running code generation
        #[arg(long)]
        script_only: bool,

        /// Skip running npm install and codama generation
        #[arg(long)]
        no_run: bool,

        /// Run generation immediately using available JS runtime (default: true unless --no-run or --script-only)
        #[arg(long)]
        run: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Anchor-compatible IDL (default)
    Anchor,
    /// Native Codama rootNode JSON
    Codama,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Generate {
            manifest_path,
            out,
            src,
            format,
        } => {
            let start_time = Instant::now();

            let out = out.unwrap_or_else(|| match format {
                OutputFormat::Codama => PathBuf::from("codama.json"),
                OutputFormat::Anchor => PathBuf::from("idl.json"),
            });

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

            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create output directory {}", parent.display())
                })?;
            }

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

            let out = out.unwrap_or_else(|| match format {
                OutputFormat::Codama => PathBuf::from("codama.json"),
                OutputFormat::Anchor => PathBuf::from("idl.json"),
            });

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
                .context("IDL check process failed - check #[p_instruction(...)]")?;

            let generated = match format {
                OutputFormat::Codama => render::to_codama_json(&idl)
                    .context("Failed to serialize IDL to Codama JSON")?,
                OutputFormat::Anchor => serde_json::to_string_pretty(&idl)
                    .context("Failed to serialize IDL to Anchor JSON")?,
            };

            let existing = std::fs::read_to_string(&out)
                .with_context(|| format!("Failed to read existing IDL {}", out.display()))?;

            let gen_val: serde_json::Value = serde_json::from_str(&generated)?;
            let exist_val: serde_json::Value = serde_json::from_str(&existing)?;

            if gen_val != exist_val {
                anyhow::bail!(
                    "IDL in {} is out of date with program source",
                    out.display()
                );
            }

            println!(
                "\x1b[32m\x1b[1m    Verified\x1b[0m IDL is up to date with source in {:.2?} [{}]",
                start_time.elapsed(),
                out.display()
            );

            Ok(())
        }
        Command::Doctor {
            manifest_path,
            src,
            json,
        } => {
            let (metadata, lib_path) = read_metadata(&manifest_path)
                .with_context(|| format!("Failed to read metadata {}", manifest_path.display()))?;
            let _ = metadata;

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

            let report = crate::doctor::run_doctor(&src_dir)?;

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
        Command::InitAgents { dir, force, all } => {
            crate::init_agents::run_init_agents(&dir, force, all)
        }
        Command::Init { name, dir } => {
            crate::run_init(&name, dir.as_deref())?;
            Ok(())
        }
        Command::Client {
            manifest_path,
            out,
            src,
            package_name,
            script_only,
            no_run,
            run: _,
        } => {
            let should_run = !no_run && !script_only;
            crate::run_client(crate::ClientOptions {
                manifest_path,
                out_dir: out,
                src_dir: src,
                package_name,
                run: should_run,
                script_only,
            })?;
            Ok(())
        }
    }
}
