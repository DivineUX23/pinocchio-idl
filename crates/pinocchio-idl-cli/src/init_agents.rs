use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

const AGENTS_MD: &str = r#"# Pinocchio IDL Annotation Instructions

This repository uses `pinocchio-idl` to generate deterministic Anchor/Codama IDLs using **active** procedural macros.

When assisting with IDL annotations:

- Use `#[p_instruction(id = X, accounts = [...], data = [...])]` on the instruction handler functions.
- If the instruction requires validation and security checks, add the `inject` flag to the attribute list: e.g. `#[p_instruction(inject, id = ...)]`. This ensures PDA/ATA validation and bounds checking code is injected.
- If no injection is needed, simply omit the `inject` flag: e.g. `#[p_instruction(id = ...)]`.
- Use `#[p_state]` on structs that represent accounts. If injection is needed, use `#[p_state(inject)]`.
- Use `#[p_error]` on enums representing program errors. Use `#[p_code = X]` on variants to specify exact error codes.
- Use `#[p_constant]` on public constants that should be exported to the IDL.
- Do not invent signer, writable, optional, PDA, or event metadata.
- After annotation edits, run `cargo pinocchio-idl generate` and `cargo pinocchio-idl check`.
- Run `cargo pinocchio-idl doctor` to find missing annotations in the codebase.

Expected output from the assistant:
- a concise diff
- a table of annotated instructions/accounts/events
- unresolved assumptions/TODOs
- exact commands run and whether they passed
"#;

pub fn run_init_agents(dir: &Path, force: bool) -> Result<()> {
    if !dir.exists() {
        bail!("target directory does not exist: {}", dir.display());
    }

    let agents_dir = dir.join(".agents");
    if !agents_dir.exists() {
        fs::create_dir_all(&agents_dir)
            .with_context(|| format!("failed to create {}", agents_dir.display()))?;
    }

    let agents_path = agents_dir.join("AGENTS.md");
    if agents_path.exists() && !force {
        println!(
            "{} already exists. Run with --force to overwrite.",
            agents_path.display()
        );
        return Ok(());
    }

    fs::write(&agents_path, AGENTS_MD)
        .with_context(|| format!("failed to write {}", agents_path.display()))?;

    println!(
        "✅ Successfully wrote AI agent instructions to {}",
        agents_path.display()
    );
    println!("Ask your AI coding assistant to follow these rules when working on the codebase.");

    Ok(())
}
