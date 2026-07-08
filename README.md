# pinocchio-idl

**IDL generation tooling for [Pinocchio](https://github.com/febo/pinocchio) Solana programs.**

`pinocchio-idl` brings IDL generation to the Pinocchio ecosystem — **no Anchor, no framework wrappers, zero runtime overhead.** Annotate your Pinocchio instruction handlers and account state structs with two proc-macro attributes, run one CLI command, and get a fully-structured `idl.json` that is compatible with both the [Anchor IDL spec](https://www.anchor-lang.com/) and [Codama](https://github.com/codama-idl/codama).

The macros do double duty: they **generate IDL metadata** _and_ **auto-inject security validation** (account-count bounds checks, signer/writable guards) directly into your instruction handlers **at compile time** — so you get correctness enforcement with no runtime cost and no framework in your dependency tree.


---

## Features

- **Compile-time security injection** — `#[p_instruction]` rewrites your handler at compile time to inject account-count bounds checking and per-account `signer` / `writable` guards. No runtime framework, no trait vtables, just the checks you declared.
- **`#[p_instruction(...)]`** — Declare accounts (writable, signer, PDA seeds, relations, fixed addresses) and instruction data (byte-slice extraction) in a concise attribute DSL.
- **`#[p_state]`** — Derive a compile-time `SPACE` constant and an Anchor-compatible 8-byte `DISCRIMINATOR` (SHA-256 of `"account:<StructName>"`) for any account state struct.
- **Anchor + Codama compatible IDL** — The generated `idl.json` satisfies the Anchor IDL spec and is directly consumable by [Codama](https://github.com/codama-idl/codama) for client generation.
- **Zero runtime overhead** — All macro expansion happens at Rust compile time. The CLI is a pure static-analysis tool that never invokes the compiler.
- **Zero framework wrappers** — No Anchor, no additional runtime traits or abstractions. Your Pinocchio program stays exactly as lean as you wrote it.

---

## Workspace Layout

```
pinocchio-idl/
├── crates/
│   ├── pinocchio-idl-core/      # Shared parsing types & IDL structs
│   ├── pinocchio-idl-macros/    # Proc-macro crate (#[p_instruction], #[p_state])
│   ├── pinocchio-idl-cli/       # CLI binary (pinocchio-idl build)
│   └── fixtures/
│       └── escrow-fixture/      # Example Pinocchio program using the macros
└── Cargo.toml                   # Workspace root
```


---

## Architecture Diagram


![alt text](image.png)


---

## Installation

### CLI — `pinocchio-idl build`

Install the binary directly from GitHub (no crates.io required):

```bash
cargo install --git https://github.com/DivineUX23/pinocchio-idl.git pinocchio-idl-cli
```

Cargo will clone the repo, compile the `pinocchio-idl-cli` crate, and place the `pinocchio-idl` binary on your `PATH`.

Verify the install:

```bash
pinocchio-idl --version
```

---

## Usage

### 1. Add the macro dependency to your Pinocchio program

In your program's `Cargo.toml`, point directly at this GitHub repository:

```toml
[dependencies]
pinocchio-idl-macros = { git = "https://github.com/DivineUX23/pinocchio-idl.git" }
```

To pin to a specific branch or commit for reproducible builds:

```toml
pinocchio-idl-macros = { git = "https://github.com/DivineUX23/pinocchio-idl.git", branch = "main" }
# or
pinocchio-idl-macros = { git = "https://github.com/DivineUX23/pinocchio-idl.git", rev = "<commit-sha>" }
```

---

### 2. Annotate your program

#### `#[p_state]` — Account state struct

Decorate any named-field struct to get a compile-time `SPACE` constant and an Anchor-compatible `DISCRIMINATOR`:

```rust
use pinocchio_idl_macros::p_state;

#[p_state]
pub struct Escrow {
    pub seed:    u64,
    pub maker:   Pubkey,
    pub mint_a:  Pubkey,
    pub mint_b:  Pubkey,
    pub receive: u64,
    pub bump:    u8,
}
```

This expands to:

```rust
impl Escrow {
    pub const SPACE: usize = 8 + 8 + 32 + 32 + 32 + 8 + 1; // 8-byte discriminator + fields
    pub const DISCRIMINATOR: [u8; 8] = [/* sha256("account:Escrow")[..8] */];
}
```

Supported field types: `u8`, `i8`, `bool`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`, `Pubkey`, and fixed-size arrays thereof.

---

#### `#[p_instruction(...)]` — Instruction handler

Annotate your handler function to declare its accounts and data layout:

```rust
use pinocchio_idl_macros::p_instruction;

#[p_instruction(
    id = 0,
    accounts = [
        maker(signer, mut),
        escrow(mut, pda = ["escrow", maker, seed], state = Escrow),
        vault(mut, relations = [escrow, mint_a]),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
    ],
    data = [
        seed:    u64 = data[0..8],
        receive: u64 = data[8..16],
        bump:    u8  = data[16]
    ]
)]
pub fn process_make_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [maker, mint_a, mint_b, escrow, vault, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };
    // ... your logic
    Ok(())
}
```

**Account constraint reference:**

| Constraint | Syntax | Effect |
|---|---|---|
| Writable | `mut` | Validates `account.is_writable()` at runtime |
| Signer | `signer` | Validates `account.is_signer()` at runtime |
| PDA seeds | `pda = ["literal", account_name, arg_name]` | Recorded in IDL for client-side PDA derivation |
| Linked state | `state = StructName` | Associates an account with its `#[p_state]` type |
| Fixed address | `address = "Base58..."` | Records a known program/sysvar address in the IDL |
| Relations | `relations = [other, another]` | Records account relationships in the IDL |

**Data field syntax:** `field_name: Type = data[start..end]` or `data[index]`

#### What the macro injects at compile time

When the Rust compiler expands `#[p_instruction]`, it **rewrites your function body** to prepend the following guards — no boilerplate you have to write:

```rust
// 1. Bounds check — inserted at the very top of the function
if accounts.len() < 4 {   // number of declared accounts
    return Err(ProgramError::NotEnoughAccountKeys);
}

// 2. Per-account constraint guards — inserted after your account bindings
if !maker.is_signer() {
    return Err(ProgramError::MissingRequiredSignature);
}
if !escrow.is_writable() {
    return Err(ProgramError::MissingRequiredSignature);
}
// ...and so on for every declared constraint
```

All of this happens at **compile time** with zero runtime overhead and without any framework in your dependency tree — you just annotate your function and the macro handles the rest.

---

### 3. Generate the IDL

From inside your Pinocchio program directory (where your `Cargo.toml` lives):

```bash
pinocchio-idl build
```

This produces `idl.json` in the current directory. Options:

```bash
pinocchio-idl build \
  --manifest-path path/to/Cargo.toml \   # default: ./Cargo.toml
  --out target/idl/my_program.idl.json   # default: ./idl.json
```

The generated file is a valid **Anchor-compatible IDL** and is also directly consumable by **[Codama](https://github.com/codama-idl/codama)** for automatic client-code generation in TypeScript, Rust, and other languages.

---

## Example: Escrow Program

A complete working example lives in [`crates/fixtures/escrow-fixture/src/lib.rs`](crates/fixtures/escrow-fixture/src/lib.rs):

```rust
use pinocchio::{AccountView, ProgramResult, error::ProgramError};
use pinocchio::pubkey::Pubkey;
use pinocchio_pubkey::declare_id;
use pinocchio_idl_macros::{p_instruction, p_state};

declare_id!("11111111111111111111111111111111111111111");

#[p_state]
pub struct Escrow {
    pub seed:    u64,
    pub maker:   Pubkey,
    pub mint_a:  Pubkey,
    pub mint_b:  Pubkey,
    pub receive: u64,
    pub bump:    u8,
}

#[p_instruction(
    id = 0,
    accounts = [
        maker(signer, mut),
        escrow(mut, pda = ["escrow", maker, seed], state = Escrow),
        vault(mut, relations = [escrow, mint_a]),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
    ],
    data = [
        seed:    u64 = data[0..8],
        receive: u64 = data[8..16],
        bump:    u8  = data[16]
    ]
)]
pub fn process_make_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [maker, mint_a, mint_b, escrow, vault, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };
    Ok(())
}
```

---

## How it Works

```
Your Pinocchio source (.rs files)
        │
        ▼
  pinocchio-idl build
        │
        ├─ Walks all .rs files in src/
        ├─ Parses each file with `syn`
        ├─ Discovers #[p_instruction] and #[p_state] items
        ├─ Reads program name/version from Cargo.toml
        └─ Serializes to Anchor-compatible idl.json
```

The `#[p_instruction]` and `#[p_state]` macros work **independently** of the CLI — they expand at Rust compile time, injecting validation code into your handlers. The CLI is a pure static analysis tool that re-parses your source without invoking the Rust compiler.

---

## IDL Output Format

The generated `idl.json` follows the Anchor IDL spec and is also **Codama compatible**:

```json
{
  "address": "<your program id>",
  "metadata": {
    "name": "your-program",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "..."
  },
  "instructions": [
    {
      "name": "process_make_instruction",
      "discriminator": [0],
      "accounts": [
        { "name": "maker", "writable": true, "signer": true },
        { "name": "escrow", "writable": true, "pda": { "seeds": [...] } },
        ...
      ],
      "args": [
        { "name": "seed",    "type": "u64" },
        { "name": "receive", "type": "u64" },
        { "name": "bump",    "type": "u8"  }
      ]
    }
  ],
  "accounts": [...],
  "types": [...],
  "errors": [],
  "constants": []
}
```

---

## Building from Source

```bash
git clone https://github.com/DivineUX23/pinocchio-idl.git
cd pinocchio-idl
cargo build --workspace
```

Run the tests:

```bash
cargo test --workspace
```

---

## Limitations & Roadmap

This is a beta / capstone-phase project. The following known gaps exist:

### Current Limitations

| Area | Status |
|---|---|
| `errors` IDL section | Always emitted as `[]` — no annotation convention for custom program errors exists yet |
| `constants` IDL section | Always emitted as `[]` — no annotation for on-chain constants yet |
| PDA on-chain verification | The `pda = [...]` constraint is **recorded in the IDL** for client-side derivation but the corresponding on-chain `create_program_address` check is currently disabled in the macro (marked `/* Disabled for now */` in source) |
| Field types | `#[p_state]` and instruction data only support primitives (`u8`–`u128`, `bool`, `Pubkey`) and fixed-size arrays — `Vec`, `String`, `Option`, enums, and nested structs are not yet supported |
| Multi-file module re-exports | The CLI walks `src/` recursively but only discovers items declared directly with `#[p_instruction]` / `#[p_state]` — items re-exported via `pub use` from external crates are not picked up |
| No `cargo-pinocchio-idl` integration | Must be invoked as a standalone binary; no `cargo` subcommand plugin yet |

### Roadmap Ideas

- [ ] `#[p_error]` attribute to populate the `errors` IDL section
- [ ] `#[p_constant]` attribute for on-chain constants
- [ DONE ] Re-enable and stabilize PDA on-chain verification in the macro
- [ ] Support `Vec<T>`, `Option<T>`, and enum field types in `#[p_state]`
- [ ] Publish to crates.io
- [ ] `cargo pinocchio-idl` plugin


### potential next step:

pub fn process_make(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    // 1. One macro call handles unpacking, data parsing, and security guards.
    p_parse!(
        accounts = [
            maker(signer, mut),
            escrow(mut, pda = ["escrow", maker, seed]),
        ],
        args = [
            seed: u64,
            receive: u64,
            bump: u8
        ]
    );

    // 2. You just write your business logic! 
    // `maker`, `escrow`, `seed`, `receive`, and `bump` are now fully typed, 
    // securely validated variables ready to use.
    
    msg!("Maker is: {:?}", maker.key());
    msg!("Seed is: {}", seed);

    Ok(())
}



If the AST sees TokenProgram, output "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" into the IDL.
If it sees SystemProgram, output "11111111111111111111111111111111".
If it sees AssociatedTokenProgram, output "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".



cargo run -- build   --manifest-path /home/divine/turbine/acc-turbine/pinocchio-idl/crates/fixtures/escrow-fixture/Cargo.toml   --out /home/divine/turbine/acc-turbine/pinocchio-idl/crates/fixtures/escrow-fixture/idl.json