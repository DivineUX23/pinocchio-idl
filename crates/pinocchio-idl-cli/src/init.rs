use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::init_agents::run_init_agents;

pub fn run_init(name: &str, target_dir: Option<&Path>) -> Result<PathBuf> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        bail!("Program name cannot be empty");
    }

    // Validate identifier characters
    if !trimmed_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!(
            "Invalid program name `{}`. Use alphanumeric characters, dashes, or underscores.",
            trimmed_name
        );
    }

    let base_dir = target_dir.unwrap_or(Path::new("."));
    let project_dir = base_dir.join(trimmed_name);

    if project_dir.exists() {
        let entries = fs::read_dir(&project_dir)
            .with_context(|| format!("Failed to read directory {}", project_dir.display()))?
            .count();
        if entries > 0 {
            bail!(
                "Directory `{}` already exists and is not empty.",
                project_dir.display()
            );
        }
    }

    // Create directories
    let src_dir = project_dir.join("src");
    let workflows_dir = project_dir.join(".github").join("workflows");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("Failed to create {}", src_dir.display()))?;
    fs::create_dir_all(&workflows_dir)
        .with_context(|| format!("Failed to create {}", workflows_dir.display()))?;

    // 1. Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
description = "Solana program built with Pinocchio and pinocchio-idl"

[lib]
crate-type = ["cdylib", "lib"]

[dependencies]
pinocchio = "0.11"
pinocchio-pubkey = "0.3"
pinocchio-system = "0.6"
pinocchio-idl-macros = "0.1.2"

[features]
default = []
"#,
        name = trimmed_name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml).context("Failed to write Cargo.toml")?;

    // 2. src/lib.rs
    let lib_rs = r#"use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    msg,
};
use pinocchio_idl_macros::{p_constant, p_error, p_instruction, p_state, p_type};

// Replace with your actual deployed program ID
pinocchio::address::declare_id!("11111111111111111111111111111111111111111");

/// Protocol version identifier.
#[p_constant]
pub const PROTOCOL_VERSION: u8 = 1;

/// Custom program errors.
#[p_error]
pub enum ProgramCustomError {
    /// Amount passed to instruction is zero or invalid.
    InvalidAmount,
    /// Counter is already at maximum value.
    #[p_code = 101]
    CounterOverflow,
}

/// Lifecycle status for account state.
#[p_type]
pub enum AccountStatus {
    /// Account is active and can receive updates.
    Active,
    /// Account has been suspended.
    Suspended,
}

/// State stored in the on-chain account.
#[p_state(inject)]
pub struct CounterAccount {
    /// Authority who can update the counter.
    pub authority: [u8; 32],
    /// Current counter value.
    pub count: u64,
    /// Account lifecycle status.
    pub status: AccountStatus,
}

/// Initialize a new counter account.
#[p_instruction(
    id = 0,
    inject,
    accounts = [
        authority(signer, mut),
        counter(mut, pda=["counter", authority]),
        system_program
    ],
    data = [
        initial_count: u64 = data[0..8]
    ]
)]
pub fn initialize(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [authority, counter, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    msg!("Initializing counter program with pinocchio-idl!");
    Ok(())
}
"#;
    fs::write(src_dir.join("lib.rs"), lib_rs).context("Failed to write src/lib.rs")?;

    // 3. .github/workflows/idl-check.yml
    let workflow_yml = r#"name: IDL Drift Check

on:
  push:
    branches: [main]
  pull_request:

jobs:
  check-idl:
    name: Check IDL Drift
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install pinocchio-idl
        run: cargo install pinocchio-idl
      - name: Check IDL matches source code
        run: cargo pinocchio-idl check
"#;
    fs::write(workflows_dir.join("idl-check.yml"), workflow_yml)
        .context("Failed to write .github/workflows/idl-check.yml")?;

    // 4. .agents/AGENTS.md
    let _ = run_init_agents(&project_dir, true, false);

    // 5. README.md
    let readme_md = format!(
        r#"# {name}

A high-performance Solana program built with [Pinocchio](https://github.com/febo/pinocchio) and [pinocchio-idl](https://github.com/DivineUX23/pinocchio-idl).

## Quick Start

- **Generate Anchor IDL**: `cargo pinocchio-idl generate`
- **Generate Codama IDL**: `cargo pinocchio-idl generate -f codama`
- **Generate TypeScript SDK**: `cargo pinocchio-idl client`
- **Check for IDL drift**: `cargo pinocchio-idl check`
- **Run diagnostics**: `cargo pinocchio-idl doctor`
- **AI Agent Rules**: `.agents/AGENTS.md`
"#,
        name = trimmed_name
    );
    fs::write(project_dir.join("README.md"), readme_md).context("Failed to write README.md")?;

    // 6. .gitignore
    let gitignore = r#"/target
Cargo.lock
"#;
    fs::write(project_dir.join(".gitignore"), gitignore).context("Failed to write .gitignore")?;

    println!(
        "\x1b[32m\x1b[1m Initialized\x1b[0m Pinocchio Solana program `{}` [{}]",
        trimmed_name,
        project_dir.display()
    );
    println!("\x1b[36m\x1b[1m  Next steps\x1b[0m Run the following commands to build your IDL:");
    println!("    cd {}", project_dir.display());
    println!("    cargo pinocchio-idl generate");

    Ok(project_dir)
}
