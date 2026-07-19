# pinocchio-idl

[![Crates.io Version](https://img.shields.io/crates/v/pinocchio-idl)](https://crates.io/crates/pinocchio-idl)
[![Crates.io License](https://img.shields.io/crates/l/pinocchio-idl)](https://crates.io/crates/pinocchio-idl)
[![CI](https://github.com/DivineUX23/pinocchio-idl/actions/workflows/ci.yml/badge.svg)](https://github.com/DivineUX23/pinocchio-idl/actions/workflows/ci.yml)
[![Rust: 1.89+](https://img.shields.io/badge/rust-1.89%2B-orange)](https://www.rust-lang.org)

> **The ultimate Solana Pinocchio IDL generator for generating reproducible Anchor and Codama compatible IDLs.**
> Write Solana programs with Pinocchio's raw performance and Anchor's developer experience. `pinocchio-idl` uses **active macros** to automatically generate `DISCRIMINATOR` arrays, `SPACE` constants, and 100% Codama-compatible IDLs with zero runtime framework bloat. 

---

## Table of Contents

- [What it does](#what-it-does)
- [Why pinocchio-idl?](#why-pinocchio-idl)
- [Quick Start](#quick-start)
- [Features](#features)
- [Workspace Crates](#workspace-crates)
- [Installation](#installation)
- [Usage](#usage)
  - [1. Adding the Macro Dependency](#1-adding-the-macro-dependency)
  - [2. Annotating Your Program](#2-annotating-your-program)
  - [3. Generating the IDL](#3-generating-the-idl)
  - [4. Generate Codama](#4-generate-codama)
  - [5. GitHub Actions CI](#5-github-actions-ci)
- [Migration from Anchor](#migration-from-anchor)
- [Example: Escrow Program](#example-escrow-program)
- [How It Works](#how-it-works)
- [IDL Output Format](#idl-output-format)
- [Compiler Invariants and Security Rules](#compiler-invariants-and-security-rules)
- [Building from Source](#building-from-source)
- [Contributing](#contributing)

---

## What it does

Solana developers need an IDL (Interface Definition Language) file so that client toolsTypeScript SDKs, explorers, and Codama code generatorsknow how to talk to their programs. Anchor programs get this for free. Pinocchio programs, which deliberately avoid Anchor to stay lean and fast, need a dedicated IDL generator.

`pinocchio-idl` is the most powerful tool for this job. You annotate your existing Pinocchio code with a small set of macros, run one CLI command, and get a fully structured and reproducible `idl.json` that is 100% compatible with the [Anchor IDL specification](https://www.anchor-lang.com/) and [Codama](https://github.com/codama-idl/codama).

**Annotate with macros -> run the CLI -> get `idl.json` or `codama.json`.**

Unlike passive parsers, our macros are **active**. They don't just generate JSONthey optionally implement `pub const DISCRIMINATOR` and `pub const SPACE` for your accounts. By flipping a simple `inject` flag, they can even actively inject runtime validation guards (PDA derivation, bounds checking, signer checks) directly into your functions at compile-time. This ensures absolute **reproducibility** of your interface behavior, making sure your Rust source code and your IDL are deterministically aligned.

---

## Why pinocchio-idl?

If you are migrating from Anchor or evaluating alternatives, here is what you gain:

| Metric | Anchor | Pinocchio + pinocchio-idl |
|---|---|---|
| Compute Units (Baseline) | ~649 CUs | ~108 CUs |
| Security Validation | Runtime Framework | Compile-time Injection |
| Codama IDL Support | Yes | Yes |
| Binary Size | Large (Framework Bloat) | Minimal (Zero-dependency) |

---


## Quick Start

Three steps: install the CLI, annotate your program, generate the IDL.

Real-world example programs are available in the [pinocchio-idl-examples](https://github.com/DivineUX23/pinocchio-idl-examples) repository.

### Step 1 - Install the CLI

```bash
cargo install pinocchio-idl
```

Verify:

```bash
cargo pinocchio-idl --version
```

### Step 2 - Add the macro dependency to your program

```toml
# Cargo.toml
[dependencies]
pinocchio-idl-macros = "0.1.0"
```

### Step 3 - Annotate and generate


```rust
use pinocchio_idl_macros::{p_instruction, p_state};

// Annotate your states with the p_state macro
#[p_state]
pub struct Counter {
    pub count: u64,
}

// Annotate your instructions with the p_instruction macro
#[p_instruction(
    id = 0,
    accounts = [
        payer(signer, mut),
        counter(mut, state = Counter),
        system_program
    ],
    data = [
        action:    u8  = data[0]
    ]
)]
pub fn process_increment(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    // Write your raw pinocchio logic here!
    Ok(())
}
```

Generate the IDL (default is Anchor format):
```bash
# Run from the directory containing your Cargo.toml
cargo pinocchio-idl generate
# -> idl.json written to the current directory
```

The output `idl.json` is Anchor-compatible and directly consumable by [Codama](https://github.com/codama-idl/codama).

Or generate native Codama format:
```bash
cargo pinocchio-idl generate --format codama
# -> codama.json written to the current directory
```

---

## Features

1. **Dual Formats (Anchor & Codama):** We natively support generating standard Anchor JSON and native Codama JSON. Run `cargo pinocchio-idl generate --format codama` to get the Codama schema instantly, guaranteeing **reproducibility** and **compatibility** across the Solana SDK ecosystem.
2. **Inline Declarations:** No need to split your Accounts and Arguments into separate boilerplate structs. `#[p_instruction(...)]` is a declarative attribute DSL that lets you specify accounts and instruction data (byte slice extraction) strictly inline on your handler function.
3. **Active Macros, Not Passive Tags:** Other tools use "passive" macros that just tag AST nodes for later reflection. Ours are active: `#[p_state(inject)]` and `#[p_event(inject)]` derive a compile-time SPACE constant and an Anchor-compatible 8-byte DISCRIMINATOR, computed as the SHA-256 of `account:<StructName>` or `event:<StructName>`, for any named-field struct. Say goodbye to manually typing `pub const DISCRIMINATOR: [u8; 8] = [...].` Without `inject`, both attributes fall back to pure IDL generation with no code injected.
4. **Optional Security Injection:** Need to validate a PDA? Add `inject` to `#[p_instruction(inject, ...)]` and the macro updates your handler at compile time to prepend bounds checking, signer/writable guards, and PDA verification using `create_program_address`, so you get the security guarantee without paying for a bump search. No runtime framework, no trait vtables, just inlined checks.
5. **Team Workflow Support and AI Ready:** `cargo pinocchio-idl init-agents` sets up your repo's `.agents/AGENTS.md` rule with custom instructions for AI coding assistants. IDL generation itself is fully deterministic, giving you high reproducibility across developer environments and CI.
6. **Zero Runtime Overhead, Zero Framework Weight:** All macro expansion happens at Rust compile time, so programs using `pinocchio-idl` stay exactly as lean as a hand-written Pinocchio program. The CLI itself is a pure static-analysis tool built on `syn`; it never invokes `rustc`, which keeps it fast in CI.
7. **Well-known Program Resolution:** Accounts using reserved names (system_program, token_program, etc.) are automatically mapped to their canonical on-chain addresses in the generated IDL.

---

## Workspace Crates

This repository is a Cargo workspace. All crates share this README.

| Crate | Role |
|---|---|
| [`pinocchio-idl-cli`](crates/pinocchio-idl-cli/) | CLI binary (`pinocchio-idl generate`). Install this on the development machine to generate IDLs. |
| [`pinocchio-idl-macros`](crates/pinocchio-idl-macros/) | Proc-macro crate providing `#[p_instruction]`, `#[p_state]`, `#[p_error]`, and `#[p_constant]`. Add this as a direct dependency of the Pinocchio program. |
| [`pinocchio-idl-core`](crates/pinocchio-idl-core/) | Shared parsing types and IDL serialization structures. This is an internal transitive dependency of the CLI and macros crates; programs do not need to depend on it directly. |

```text
pinocchio-idl/
+-- crates/
|   +-- pinocchio-idl-core/      # Shared parsing types & IDL structs (internal)
|   +-- pinocchio-idl-macros/    # Proc-macro crate
|   +-- pinocchio-idl-cli/       # CLI binary
|   +-- fixtures/
|       +-- escrow-fixture/      # Reference Pinocchio program
+-- Cargo.toml                   # Workspace root
```

---

## Installation

**Minimum Supported Rust Version:** 1.89

### CLI

Install both the standalone binary and the cargo subcommand in one command:

```bash
cargo install pinocchio-idl
```

This places **two** binaries on your `PATH`:

| Binary | Invoked as |
|---|---|
| `pinocchio-idl` | `pinocchio-idl generate` |
| `cargo-pinocchio-idl` | `cargo pinocchio-idl generate` |

Confirm the installation:

```bash
pinocchio-idl --version
cargo pinocchio-idl --version
```

---

## Usage

### 1. Adding the Macro Dependency

Add `pinocchio-idl-macros` to the program's `Cargo.toml`:

```toml
[dependencies]
pinocchio-idl-macros = "0.1.0"
```

---

### 2. Annotating Your Program

#### `#[p_state]` - Account State Struct

Apply `#[p_state]` to any named-field struct to include it in your IDL. To automatically derive a compile-time `SPACE` constant and an Anchor-compatible `DISCRIMINATOR`, simply add the `inject` flag:

```rust
use pinocchio_idl_macros::p_state;

#[p_state(inject)]
pub struct Escrow {
    pub seed:    u64,
    pub maker:   Pubkey,
    pub mint_a:  Pubkey,
    pub mint_b:  Pubkey,
    pub receive: u64,
    pub bump:    u8,
}
```

The macro expands to:

```rust
impl Escrow {
    pub const SPACE: usize = 8 + 32 + 32 + 32 + 8 + 1;
    pub const DISCRIMINATOR: [u8; 8] = [31, 213, 123, 187, 186, 22, 218, 155];
}
```

#### `#[p_event]` - Event Struct

Apply `#[p_event]` to easily generate Codama-compatible events. Add the `inject` flag to automatically calculate the standard event discriminator and `SPACE` constants:

```rust
use pinocchio_idl_macros::p_event;

#[p_event(inject)]
pub struct EscrowCreated {
    pub maker: [u8; 32],
    pub amount: u64,
}
```

**Supported field types:**

| Type | IDL type | `SPACE` (bytes) |
|---|---|---|
| `u8`, `i8`, `bool` | `u8` / `i8` / `bool` | 1 |
| `u16`, `i16` | `u16` / `i16` | 2 |
| `u32`, `i32` | `u32` / `i32` | 4 |
| `u64`, `i64` | `u64` / `i64` | 8 |
| `u128`, `i128` | `u128` / `i128` | 16 |
| `Pubkey` / `Address` | `pubkey` | 32 |
| `[u8; 32]` | `pubkey` | 32 |
| `[T; N]` | `[T; N]` | `sizeof(T)` x N |
| `Vec<T>` | `vec<T>` | 4 (length prefix only) |
| `Option<T>` | `{"option": T}` | 1 + `sizeof(T)` |
| Custom enum or struct | name as-is | unsupported - use a primitive or `[u8; N]` |

---

#### `#[p_instruction(...)]` - Instruction Handler

Apply `#[p_instruction]` to a handler function to declare its account list and data layout:

```rust
use pinocchio_idl_macros::p_instruction;

#[p_instruction(
    id = 0,
    accounts = [
        maker(signer, mut),
        escrow(mut, pda = ["escrow", maker, seed, bump], state = Escrow),
        mint_a,
        vault_a(mut, ata = [escrow, mint_a])
    ],
    data = [
        seed:    u64 = data[0..8],
        receive: u64 = data[8..16],
        bump:    u8  = data[16..17]
    ]
)]
pub fn process_make_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [maker, escrow, mint_a, vault_a, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // instruction logic follows
    Ok(())
}
```

**Account constraint reference:**

| Constraint | Syntax | Behaviour |
|---|---|---|
| Writable | `mut` | Validates `account.is_writable()` at runtime |
| Signer | `signer` | Validates `account.is_signer()` at runtime |
| PDA seeds | `pda = ["literal", acc, arg, program = "Base58..."]` | Recorded in the IDL and verified on-chain. The `program` key is optional; it defaults to `crate::ID`. |
| ATA | `ata = [owner, mint]` | Recorded in the IDL and validated on-chain via `pinocchio_token::state::Account`. Requires exactly two expressions. |
| Linked state | `state = StructName` | Associates the account with a `#[p_state]` type in the IDL |
| Fixed address | `address = "Base58..."` | Records a known program or sysvar address in the IDL |
| Relations | `relations = [a, b]` | Records account relationships in the IDL |

**Data field syntax:** `field_name: Type = data[start..end]` or `data[index]`

#### Optional Compile-Time Injection

If you *do* want `pinocchio-idl` to automatically generate bounds checking, signer validations, and PDA verifications inside your instruction logic, you can optionally pass the `inject` flag:

```rust
#[p_instruction(inject, id = 0, accounts = [...])]
```

When the Rust compiler processes `inject`, the macro updates the function body to prepend generated guards entirely at compile time. No runtime framework, dynamic dispatch, or additional traits are introduced.

```rust
// 1. Account count bounds check - inserted at the top of the function
if accounts.len() < 5 {
    return Err(ProgramError::NotEnoughAccountKeys);
}

// 2. Per-account constraint guards - inserted after account bindings
if !maker.is_signer() {
    return Err(ProgramError::MissingRequiredSignature);
}
if !escrow.is_writable() {
    return Err(ProgramError::MissingRequiredSignature);
}
// ...continued for all declared constraints
```

These guards are generated entirely at compile time. No runtime framework, dynamic dispatch, or additional traits are introduced.


---

#### `#[p_error]` - Program Error Enum

Apply `#[p_error]` to an error enum to emit all variants into the `errors` section of the IDL. Doc comments (`///`) supply the human-readable message; `#[p_code = N]` overrides the default sequential error code for a given variant.

After a `#[p_code = N]` override, subsequent variants resume from their ordinal position in the enum, not from the overridden value.

```rust
use pinocchio_idl_macros::p_error;

#[p_error]
pub enum EscrowError {
    /// The escrow has already been taken.
    AlreadyTaken,          // code 0
    /// The offer amount is zero.
    ZeroAmount,            // code 1
    #[p_code = 100]
    /// Invalid mint provided.
    InvalidMint,           // code 100
    /// The escrow has expired.
    Expired,               // code 3 (ordinal position, not 101)
}
```

- `#[p_code = N]` affects only the variant it decorates; subsequent variants continue from their ordinal position.
- The `#[p_code]` attribute is stripped at compile time and is never seen by rustc.
- If a variant has no doc comment, the variant name is used as the IDL message.

---

#### `#[p_constant]` - On-Chain Constant

Apply `#[p_constant]` to any `const` item to include it in the `constants` section of the IDL:

```rust
use pinocchio_idl_macros::p_constant;

#[p_constant]
pub const MAX_ESCROW_DURATION: u64 = 60 * 60 * 24 * 30;

#[p_constant]
pub const ESCROW_VERSION: u8 = 1;
```

The CLI reads the constant's name, type, and value expression directly from the source AST and serialises them into the IDL.

---

### 3. Generating the IDL

Run either command from the directory containing the program's `Cargo.toml`:

```bash
# Standalone binary
pinocchio-idl generate

# Cargo subcommand (identical behaviour)
cargo pinocchio-idl generate
```

Both produce `idl.json` in the current directory. Available options:

```bash
pinocchio-idl generate \
  --manifest-path path/to/Cargo.toml \     # default: ./Cargo.toml
  --out target/idl/my_program.idl.json \   # default: ./idl.json
  --src path/to/src                        # default: derived from Cargo.toml

# Same flags work with cargo pinocchio-idl generate
```

The output is a valid Anchor-compatible IDL and is directly consumable by [Codama](https://github.com/codama-idl/codama) for automated client-code generation in TypeScript, Rust, and other target languages.

---

### 4. Generate Codama

If you want to generate a native Codama JSON payload natively:

```bash
cargo pinocchio-idl generate --format codama
# Or using the short alias
cargo pinocchio-idl generate -f codama
```

---

### 5. GitHub Actions CI

Automate IDL generation in your CI pipeline to ensure your IDL stays in sync with your program source. Add this to your `.github/workflows/main.yml`:

```yaml
name: Generate IDL

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]

jobs:
  generate-idl:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Install pinocchio-idl
        run: cargo install pinocchio-idl

      - name: Generate IDL
        run: cargo pinocchio-idl generate
        # Optional: Specify manifest/out path if not at root
        # run: cargo pinocchio-idl generate -m programs/my-program/Cargo.toml -o idl.json

      - name: Upload IDL Artifact
        uses: actions/upload-artifact@v4
        with:
          name: program-idl
          path: idl.json # Match your output path
```

---

## Migration from Anchor

Developers coming from Anchor can directly translate their existing mental models into `pinocchio-idl`:

| Anchor | Pinocchio-IDL |
|---|---|
| `#[account(mut, signer)]` | `account(mut, signer)` |
| `#[account(init, payer = user, space = 8 + 32)]` | `account(init = [user, mint], state = StructName)` |
| `#[account(seeds = [b"escrow", maker.key().as_ref()], bump)]` | `account(pda = ["escrow", maker, bump])` |
| `#[account(has_one = owner)]` | `account(relations = [owner])` |

---

## Example: Escrow Program

A self-contained reference implementation is available in [`crates/fixtures/escrow-fixture/src/lib.rs`](crates/fixtures/escrow-fixture/src/lib.rs). Additional programs annotated with `pinocchio-idl` are maintained in the [pinocchio-idl-examples](https://github.com/DivineUX23/pinocchio-idl-examples) repository.

```rust
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_idl_macros::{p_constant, p_error, p_instruction, p_state};
use pinocchio_system::instructions::CreateAccount;

pinocchio::address::declare_id!("11111111111111111111111111111111111111111");

#[p_constant]
pub const MAX_ESCROW_DURATION: u64 = 60 * 60 * 24 * 30;

#[p_constant]
pub const ESCROW_VERSION: u8 = 1;

#[p_error]
pub enum EscrowError {
    /// The escrow has already been taken.
    AlreadyTaken,
    /// The offer amount is zero.
    ZeroAmount,
    #[p_code = 100]
    /// Invalid mint provided.
    InvalidMint,
    /// The escrow has expired.
    Expired,
}

#[p_state]
pub struct Escrow {
    pub seed:      u64,
    pub maker:     [u8; 32],
    pub mint_a:    [u8; 32],
    pub mint_b:    [u8; 32],
    pub receive:   u64,
    pub bump:      u8,
    pub authority: Option<[u8; 32]>,
}

// NOTE: The bump is passed as an explicit seed in pda = [...].
// See "Compiler Invariants" section 1 for the rationale.
#[p_instruction(
    id = 0,
    accounts = [
        maker(signer, mut),
        vault_a(mut, init = [escrow, mint_a]),
        mint_a,
        mint_b,
        escrow(mut, pda = ["escrow", mint_b, seed, bump, program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"], state = Escrow),
        vault_b(mut, init = [maker, mint_b]),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
        system_program
    ],
    data = [
        seed:    u64 = data[0..8],
        receive: u64 = data[8..16],
        bump:    u8  = data[16..17]
    ]
)]
pub fn process_make_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    // All account bindings must be extracted contiguously before any other logic.
    // See "Compiler Invariants" section 5.
    let [maker, vault_a, mint_a, mint_b, escrow, vault_b, token_program, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // ...

    Ok(())
}
```

---

## How It Works

```
Source files (src/**/*.rs)
        |
        v
  pinocchio-idl generate
        |
        +-- Recursively walks src/
        +-- Parses each file using syn
        +-- Collects #[p_instruction], #[p_state], #[p_error], #[p_constant] items
        +-- Reads program name and version from Cargo.toml
        +-- Serialises to idl.json (Anchor-compatible)
```

The `#[p_instruction]` and `#[p_state]` macros operate independently of the CLI. They expand during the normal Rust compilation pass, injecting validation code into the annotated functions. The CLI is a pure static-analysis tool that re-parses source files without invoking the Rust compiler.

---

## IDL Output Format

The generated `idl.json` conforms to the Anchor IDL specification and is consumable by Codama:

```json
{
  "address": "<program id>",
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
        {
          "name": "escrow",
          "writable": true,
          "pda": {
            "seeds": [
              { "kind": "const",   "value": [101, 115, 99, 114, 111, 119] },
              { "kind": "account", "path": "maker" },
              { "kind": "arg",     "path": "seed" },
              { "kind": "arg",     "path": "bump" }
            ]
          }
        },
        { "name": "tokenProgram", "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" }
      ],
      "args": [
        { "name": "seed",    "type": "u64" },
        { "name": "receive", "type": "u64" },
        { "name": "bump",    "type": "u8"  }
      ]
    }
  ],
  "accounts": [
    { "name": "Escrow", "discriminator": [ "..." ] }
  ],
  "types": [
    {
      "name": "Escrow",
      "type": {
        "kind": "struct",
        "fields": [
          { "name": "seed",      "type": "u64" },
          { "name": "maker",     "type": "pubkey" },
          { "name": "authority", "type": { "option": "pubkey" } }
        ]
      }
    }
  ],
  "errors": [
    { "code": 0,   "name": "AlreadyTaken", "msg": "The escrow has already been taken." },
    { "code": 1,   "name": "ZeroAmount",   "msg": "The offer amount is zero." },
    { "code": 100, "name": "InvalidMint",  "msg": "Invalid mint provided." }
  ],
  "constants": [
    { "name": "ESCROW_VERSION",      "type": "u8",  "value": "1" },
    { "name": "MAX_ESCROW_DURATION", "type": "u64", "value": "60 * 60 * 24 * 30" }
  ]
}
```

---

## Compiler Invariants and Security Rules

The following invariants are strictly enforced by the macro implementation to guarantee on-chain security. Failure to adhere to these rules will result in either compile-time rejection or runtime verification failure.

---

### 1. PDA Bump Must Be an Explicit Seed

`pinocchio-idl` does not implement bump-search (`find_program_address`-style) validation. PDA address derivation is performed against exactly the seed list provided; no automatic bump trial loop is executed.

The bump must be included as an explicit entry in the `pda = [...]` seed list, sourced from instruction data. Omitting it causes `derive_address` to operate on a different input than the canonical address and will result in account validation failure at runtime even for a correctly derived account.

```rust
// Incorrect - bump absent from seed list; on-chain PDA verification will always fail
escrow(mut, pda = ["escrow", maker, seed], state = Escrow)

// Correct - bump present as an explicit seed sourced from instruction data
escrow(mut, pda = ["escrow", maker, seed, bump], state = Escrow)
```

The `data` section of the same instruction must declare the `bump` field:

```rust
data = [
    seed:    u64 = data[0..8],
    receive: u64 = data[8..16],
    bump:    u8  = data[16..17]
]
```

---

### 2. The Generated IDL Contains the Bump as an Explicit Seed Entry

Because bumps are passed as explicit seeds, the generated IDL records the bump as a seed of kind `arg`. Anchor client tooling that performs automatic bump inference - internally calling `findProgramAddressSync` and discarding the bump seed - will construct a seed list that does not match the on-chain verification.

When consuming this IDL through such tooling, verify that client-side PDA derivation includes the bump as an explicit seed and matches the seed order declared in the IDL.

```typescript
// The bump must be passed explicitly on the client side
const [escrowPda] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("escrow"),
    makerPubkey.toBuffer(),
    seedBuffer,
    Buffer.from([bump]),   // corresponds to the `arg` seed in the IDL
  ],
  programId
);
```

---

### 3. `ata = [...]` Requires Exactly Two Expressions

The `ata` constraint expects exactly two expressions - the owner account followed by the mint account - in that order. Any other length is rejected at macro-expansion time.

```rust
// Correct - owner, then mint
vault(mut, ata = [owner, mint_a])

// Incorrect - single expression
vault(mut, ata = [owner])

// Incorrect - three expressions
vault(mut, ata = [owner, mint_a, extra])
```


---

### 4. Well-Known Program Account Names Are Resolved Automatically

Accounts whose names match the following identifiers are automatically assigned the corresponding canonical mainnet address in the generated IDL, without requiring an explicit `address = "..."` annotation:

| Account name | Resolved address |
|---|---|
| `system_program` | `11111111111111111111111111111111` |
| `token_program` | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` |
| `token_2022_program` | `TokenzQdBNbEquZQ8KKDMkFJJExVEYQ2qqcKgLQv7JN` |
| `associated_token_program` | `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL` |
| `rent` | `SysvarRent111111111111111111111111111111111` |
| `clock` | `SysvarC1ock11111111111111111111111111111111` |

If any of these names is used for an unrelated account, the resolution still applies. Use a distinct name or an explicit `address = "..."` override to suppress the inference:

```rust
// Rename or pin the address to prevent incorrect resolution
my_custom_program(address = "ActualProgramAddressHere...")
```

---

### 5. Account Extractions Must Be Contiguous at the Start of the Function Body

The macro locates account variable bindings within the function body (array destructuring, direct indexing, or `.get()` / `.next()` calls) and inserts generated validation guards immediately after the final binding it identifies.

All account extraction statements must appear contiguously at the top of the function body, before any other logic. Inserting an unrelated statement between account extractions will cause the injection point to be determined prematurely, which may result in `"not found"` compile-time errors for subsequently declared accounts.

```rust
// Incorrect - a msg! call between account extractions shifts the injection point
pub fn process(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let maker = accounts[0];
    msg!("processing");   // breaks the contiguous extraction block
    let vault = accounts[1];
    // validation guards injected here, before remaining accounts are in scope
    Ok(())
}

// Correct - all accounts extracted before any other logic
pub fn process(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [maker, vault, mint_a, escrow, token_program, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // Validation guards are injected here, after all bindings are in scope
    msg!("processing");
    Ok(())
}
```

---

### 6. Data Field Index Safety

Range-based data fields (`field: T = data[start..end]`) are the recommended default. The `#[p_instruction]` macro automatically injects bounds checking for single-index fields (`data[N]`), returning `ProgramError::InvalidArgument` if the data slice is too short.

Use range-based extraction as the default:

```rust
data = [
    seed:    u64 = data[0..8],
    receive: u64 = data[8..16],
    bump:    u8  = data[16]
]
```

---

## Building from Source

```bash
git clone https://github.com/DivineUX23/pinocchio-idl.git
cd pinocchio-idl
cargo build --workspace
```

To run the test suite:

```bash
cargo test --workspace
```

---

## Limitations and Roadmap

This project is under active development. The following constraints are present in the current release.

### Current Limitations

| Area | Notes |
|---|---|
| Pinocchio compatibility | `AccountView` and the updated PDA APIs introduced in Pinocchio >= 0.11 are fully supported. Versions prior to 0.10 are not. |
| Multi-file module re-exports | The CLI traverses `src/` recursively, but only discovers items annotated directly with `#[p_instruction]`, `#[p_state]`, `#[p_error]`, or `#[p_constant]`. Items re-exported via `pub use` from external crates are not discovered. |
| Complex field types in `#[p_state]` | Custom enum and nested struct fields cannot be sized automatically. Use a primitive type or a `[u8; N]` wrapper, or compute the account size manually. |
| PDA bump-search validation | Bump-search (`find_program_address`-style) validation is not implemented. The bump must be supplied as an explicit seed. See [section 1](#1-pda-bump-must-be-an-explicit-seed). |

### Supported Account Binding Styles

`#[p_instruction]` recognises three patterns for account variable extraction:

**Array destructuring:**
```rust
let [maker, vault, mint_a, escrow] = accounts else {
    return Err(ProgramError::NotEnoughAccountKeys);
};
```

**Direct indexing:**
```rust
let maker = &accounts[0];
let vault = &accounts[1];
```

**Method calls (`get`, `get_mut`, `next`):**
```rust
let maker = accounts.get(0).ok_or(ProgramError::NotEnoughAccountKeys)?;
let vault = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
```

> Regardless of which style is used, all account extraction statements must appear contiguously at the start of the function body. See [section 5](#5-account-extractions-must-be-contiguous-at-the-start-of-the-function-body).

### Roadmap

- [ ] `p_parse!` declarative macro for combined account unpacking, data parsing, and security guard injection at a single call site

---

## Contributing

Contributions are welcome. To get started:

1. Fork the repository and clone it locally.
2. Build the workspace with `cargo build --workspace`.
3. Run the test suite with `cargo test --workspace`.
4. Open a pull request with a clear description of the change.

Please open an issue before submitting large feature PRs so the approach can be agreed on first. Bug reports, documentation improvements, and new example programs are all appreciated.
