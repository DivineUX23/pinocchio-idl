use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

pub const AGENTS_MD: &str = r#"# Pinocchio IDL & Solana Program Rules

This repository uses `pinocchio-idl` to generate deterministic Anchor and Codama IDLs using **active** procedural macros.

When developing or annotating Solana programs in this repository, follow these rules:

## 1. Instruction Annotation (`#[p_instruction]`)
- Annotate instruction handler functions with:
  ```rust
  #[p_instruction(
      id = 0,
      inject, // Optional: Injects compile-time bounds checking, signer/mut guards, and PDA verification
      accounts = [
          maker(signer, mut),
          escrow(mut, pda = ["escrow", maker, seed]),
          vault(mut, ata = [escrow, mint_a]),
          referrer(optional),
          system_program,
          token_program
      ],
      data = [
          amount: u64 = data[0..8]
      ]
  )]
  pub fn process_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult { ... }
  ```
- **Injection:** Use `inject` when security validation and PDA bounds checks should be derived and injected at compile time into the handler.
- **Account constraints:** Use `signer`, `mut`, `optional`, `state = <Account>`, `pda = [...]`, and `ata = [...]`.
- **Well-known programs:** `system_program`, `token_program`, and `associated_token_program` are automatically resolved to canonical on-chain addresses.
- **Slice extraction DSL:** Specify argument types and byte slice extraction offsets in `data = [...]`.

## 2. State Accounts (`#[p_state]`)
- Annotate structs representing on-chain state accounts with `#[p_state]`.
- Add `inject` to automatically derive compile-time `SPACE` constant and 8-byte Anchor `DISCRIMINATOR`:
  ```rust
  #[p_state(inject)]
  pub struct Escrow {
      pub maker: [u8; 32],
      pub amount: u64,
      pub status: EscrowStatus,
  }
  ```

## 3. Events (`#[p_event]`)
- Annotate event structs with `#[p_event]`.
- Add `inject` to automatically derive event `DISCRIMINATOR` and `SPACE` constants:
  ```rust
  #[p_event(inject)]
  pub struct EscrowCreated {
      pub maker: [u8; 32],
      pub amount: u64,
  }
  ```

## 4. Custom Types (`#[p_type]`)
- Annotate custom enums and structs with `#[p_type]` so they are included in the IDL's `types` / `definedTypes`:
  ```rust
  #[p_type]
  pub enum EscrowStatus {
      Active,
      Completed,
      Cancelled,
  }
  ```

## 5. Errors (`#[p_error]`)
- Annotate custom error enums with `#[p_error]`. Use `#[p_code = X]` on variants to specify explicit numeric error codes:
  ```rust
  #[p_error]
  pub enum EscrowError {
      #[p_code = 100]
      InvalidMint,
      AlreadyTaken,
  }
  ```

## 6. Constants (`#[p_constant]`)
- Annotate public numeric constants with `#[p_constant]` to export them to the IDL:
  ```rust
  #[p_constant]
  pub const PROTOCOL_VERSION: u8 = 1;
  ```

## 7. Documentation
- Write standard Rust doc comments (`///`) on instructions, state structs, enums, variants, and fields. They will be automatically parsed into IDL documentation nodes.
- Do NOT invent or guess signer, writable, optional, PDA, or event metadata.

## 8. CLI Commands & Verification
After modifying program code or annotations, run:
- `cargo pinocchio-idl generate` - Generates Anchor-compatible `idl.json`.
- `cargo pinocchio-idl generate -f codama` - Generates native Codama `codama.json`.
- `cargo pinocchio-idl client` (or `cargo pinocchio-idl codegen --ts ./clients/js`) - Generates TypeScript SDK client via Codama.
- `cargo pinocchio-idl check` - Verifies committed IDL against source code (zero drift).
- `cargo pinocchio-idl doctor` - Scans for missing, unannotated, or duplicated annotations.

## Expected Output From AI Assistants
- Concise unified diff of changes.
- Table of added or modified instructions, accounts, events, and types.
- Verification status confirming `cargo pinocchio-idl check` / `doctor` passes.
"#;

pub fn run_init_agents(dir: &Path, force: bool, all: bool) -> Result<()> {
    if !dir.exists() {
        bail!("target directory does not exist: {}", dir.display());
    }

    // 1. Always write .agents/AGENTS.md
    let agents_dir = dir.join(".agents");
    if !agents_dir.exists() {
        fs::create_dir_all(&agents_dir)
            .with_context(|| format!("failed to create {}", agents_dir.display()))?;
    }

    let agents_path = agents_dir.join("AGENTS.md");
    write_agent_file(&agents_path, AGENTS_MD, force)?;

    println!(
        "\x1b[32m\x1b[1m  Scaffolded\x1b[0m AI agent rules at {}",
        agents_path.display()
    );

    // 2. If `all` is requested, also generate CLAUDE.md, Cursor rules, and Copilot instructions
    if all {
        // CLAUDE.md (Anthropic / Claude Code)
        let claude_path = dir.join("CLAUDE.md");
        write_agent_file(&claude_path, AGENTS_MD, force)?;
        println!(
            "\x1b[32m\x1b[1m  Scaffolded\x1b[0m Claude Code rules at {}",
            claude_path.display()
        );

        // .cursor/rules/pinocchio.mdc (Cursor IDE)
        let cursor_dir = dir.join(".cursor").join("rules");
        if !cursor_dir.exists() {
            fs::create_dir_all(&cursor_dir)
                .with_context(|| format!("failed to create {}", cursor_dir.display()))?;
        }
        let cursor_rule = format!(
            "---\ndescription: Pinocchio Solana Program and IDL Guidelines\nglobs: **/*.rs\n---\n\n{}",
            AGENTS_MD
        );
        let cursor_path = cursor_dir.join("pinocchio.mdc");
        write_agent_file(&cursor_path, &cursor_rule, force)?;
        println!(
            "\x1b[32m\x1b[1m  Scaffolded\x1b[0m Cursor IDE rule at {}",
            cursor_path.display()
        );

        // .github/copilot-instructions.md (GitHub Copilot)
        let github_dir = dir.join(".github");
        if !github_dir.exists() {
            fs::create_dir_all(&github_dir)
                .with_context(|| format!("failed to create {}", github_dir.display()))?;
        }
        let copilot_path = github_dir.join("copilot-instructions.md");
        write_agent_file(&copilot_path, AGENTS_MD, force)?;
        println!(
            "\x1b[32m\x1b[1m  Scaffolded\x1b[0m GitHub Copilot instructions at {}",
            copilot_path.display()
        );
    }

    println!(
        "\x1b[36m\x1b[1m        Info\x1b[0m AI coding assistants will follow these rules when editing the program."
    );

    Ok(())
}

fn write_agent_file(path: &Path, content: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        println!(
            "\x1b[33m\x1b[1m     Skipped\x1b[0m {} already exists (pass --force to overwrite)",
            path.display()
        );
        return Ok(());
    }

    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
