use clap::{Parser, Subcommand};
use pinocchio_idl_cli::{build_idl, read_metadata, write_idl};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "pinocchio-idl", version, about = "Generate an Anchor-compatible IDL for Pinocchio programs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Build {
        #[arg(long, default_value = "Cargo.toml")]
        manifest_path: PathBuf,
        #[arg(long, default_value = "idl.json")]
        out: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { manifest_path, out } => {
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
}