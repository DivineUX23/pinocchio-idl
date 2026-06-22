<<<<<<< HEAD
# PinIDL

**Anchor-compatible IDL generation and security validation for raw [Pinocchio](https://github.com/febo/pinocchio) programs — zero runtime overhead, zero framework wrappers.**

Pinocchio gives you full control over your on-chain code. PinIDL gives you the DX back: automatic security checks injected at compile time, and a frontend-ready `idl.json` generated from your source without touching your binary.

---

## What it does

PinIDL has two engines that work independently:

**Compile-time engine (`pinocchio-idl-macros`)** — the `#[p_instruction]` and `#[p_state]` proc-macros. They auto-inject security validation into your program at compile time. The resulting binary is identical to one you'd have written by hand — no wrappers, no heap allocations, no CU overhead.

**Offline engine (`pinocchio-idl-cli`)** — a CLI tool that parses your `.rs` source files and outputs an Anchor-compatible `idl.json`. Feed that file into [Codama](https://github.com/codama-idl/codama) to generate TypeScript or Rust client SDKs.

---

## Quick start: using PinIDL in your own program

This walks through adding PinIDL to an existing Pinocchio program from scratch, using an escrow as the example.

### Step 1 — add the dependency

In your program's `Cargo.toml`:

```toml
[dependencies]
pinocchio-idl-macros = { git = "https://github.com/DivineUX23/pinocchio-idl" }
```

To lock to a specific commit (recommended once you're past development):

```toml
pinocchio-idl-macros = { git = "https://github.com/DivineUX23/pinocchio-idl", rev = "abc1234" }
```

### Step 2 — annotate your state structs

Tag each on-chain state struct with `#[p_state]`. PinIDL computes the byte size and discriminator automatically:

```rust
// src/state.rs
use pinocchio::pubkey::Pubkey;
use pinocchio_idl_macros::p_state;

#[p_state]
pub struct EscrowState {
=======
# pinocchio-idl

**IDL generation tooling for [Pinocchio](https://github.com/febo/pinocchio) Solana programs.**

`pinocchio-idl` brings Anchor-compatible IDL generation to the Pinocchio ecosystem — without pulling in Anchor itself. Annotate your Pinocchio instruction handlers and account state structs with a pair of proc-macro attributes, run one CLI command, and get a fully-structured `idl.json` that any Anchor-compatible client (TypeScript, Rust, etc.) can consume.

> **Status:** Beta / Capstone project. Not yet published to crates.io — install directly from GitHub (see below).

---

## Features

- **`#[p_instruction(...)]`** — Annotate instruction handler functions to declare accounts (writable, signer, PDA seeds, relations, fixed addresses) and instruction data (byte-slice extraction).
- **`#[p_state]`** — Annotate account state structs to auto-derive `SPACE` and an Anchor-compatible 8-byte `DISCRIMINATOR`.
- **`pinocchio-idl build`** — CLI command that statically analyzes your program source, discovers all annotated items, and emits a structured `idl.json`.
- Zero runtime overhead — all macro work happens at compile time; the CLI is a pure static analysis tool.

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
>>>>>>> 431b462 (readme updated)
    pub seed:    u64,
    pub maker:   Pubkey,
    pub mint_a:  Pubkey,
    pub mint_b:  Pubkey,
    pub receive: u64,
    pub bump:    u8,
}
<<<<<<< HEAD

// PinIDL generates these automatically — use them anywhere in your program:
// EscrowState::SPACE         → 121 (8 discriminator prefix + field sizes)
// EscrowState::DISCRIMINATOR → [u8; 8] (sha256("account:EscrowState")[..8])
```

You can use `SPACE` directly when creating the account:

```rust
CreateAccount {
    from: payer,
    to: escrow,
    lamports: Rent::get()?.minimum_balance(EscrowState::SPACE),
    space: EscrowState::SPACE as u64,
    owner: &crate::ID,
}.invoke()?;
```

### Step 3 — annotate your instruction handlers

Add `#[p_instruction(...)]` above each instruction function. Declare every constraint you need in the header — PinIDL injects all the checks for you:

```rust
// src/instructions/make.rs
use pinocchio::{AccountView, ProgramResult, error::ProgramError};
use pinocchio_idl_macros::p_instruction;
use crate::state::EscrowState;
=======
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
>>>>>>> 431b462 (readme updated)

#[p_instruction(
    id = 0,
    accounts = [
        maker(signer, mut),
<<<<<<< HEAD
        mint_a,
        mint_b,
        escrow(mut, pda = ["escrow", maker, seed], state = EscrowState),
        vault(mut),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
        system_program(address = "11111111111111111111111111111111")
=======
        escrow(mut, pda = ["escrow", maker, seed], state = Escrow),
        vault(mut, relations = [escrow, mint_a]),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
>>>>>>> 431b462 (readme updated)
    ],
    data = [
        seed:    u64 = data[0..8],
        receive: u64 = data[8..16],
        bump:    u8  = data[16]
    ]
)]
pub fn process_make_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
<<<<<<< HEAD
    // Bind accounts however you normally would — slice destructuring or indexing,
    // both work fine:
    let [maker, mint_a, mint_b, escrow, vault, _token_program, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    // `seed`, `receive`, and `bump` are already parsed and available here —
    // PinIDL injected the from_le_bytes calls before this line.
    // `maker`, `vault`, etc. have already been validated against their
    // declared constraints (signer, mut, address) — also before this line.

    // Your business logic — no boilerplate above it:
    CreateAccount {
        from: maker,
        to: escrow,
        lamports: Rent::get()?.minimum_balance(EscrowState::SPACE),
        space: EscrowState::SPACE as u64,
        owner: &crate::ID,
    }.invoke()?;

    let escrow_state = EscrowState::from_account_info(escrow)?;
    escrow_state.set_seed(seed);
    escrow_state.set_maker(*maker.address());
    escrow_state.set_receive(receive);
    escrow_state.set_bump(bump);

=======
    let [maker, mint_a, mint_b, escrow, vault, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };
    // ... your logic
>>>>>>> 431b462 (readme updated)
    Ok(())
}
```

<<<<<<< HEAD
What PinIDL silently injects above your code:

```rust
// 1. Bounds check — before any accounts[N] access that would otherwise panic
if accounts.len() < 7 {
    return Err(ProgramError::NotEnoughAccountKeys);
}
// 2. Data bounds checks + parsing
if data.len() < 16 { return Err(ProgramError::InvalidInstructionData); }
let seed    = u64::from_le_bytes(data[0..8].try_into().unwrap());
let receive = u64::from_le_bytes(data[8..16].try_into().unwrap());
if data.len() <= 16 { return Err(ProgramError::InvalidInstructionData); }
let bump: u8 = data[16];
// 3. Account constraint checks
if !maker.is_writable() { return Err(ProgramError::InvalidAccountData); }
if !maker.is_signer()   { return Err(ProgramError::MissingRequiredSignature); }
if !escrow.is_writable() { return Err(ProgramError::InvalidAccountData); }
if !vault.is_writable()  { return Err(ProgramError::InvalidAccountData); }
```

### Step 4 — install the CLI

```bash
cargo install --git https://github.com/DivineUX23/pinocchio-idl pinocchio-idl-cli
```

This puts `pinocchio-idl` on your PATH globally. You only need to do this once.

### Step 5 — generate your IDL

Run from your program's root directory (where `Cargo.toml` lives):

```bash
pinocchio-idl build
```

That's it. `idl.json` appears in the same directory.

### Step 6 — generate your TypeScript client with Codama

```bash
# In your project root
npm install codama @codama/nodes-from-anchor @codama/renderers-js
npx codama init
# When prompted, point it at your idl.json
npx codama run
```

Or inline:

```typescript
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderJavaScriptVisitor } from "@codama/renderers-js";
import { visit } from "codama";
import fs from "fs";

const idl = JSON.parse(fs.readFileSync("idl.json", "utf-8"));
const root = rootNodeFromAnchor(idl);
visit(root, renderJavaScriptVisitor("./src/generated"));
```

---

## Account constraints reference

Constraints go inside parentheses after the account name in `accounts = [...]`:

```rust
accounts = [
    maker(signer, mut),
    escrow(mut, pda = ["escrow", maker, seed], state = EscrowState),
    vault(mut, relations = [escrow, mint_a]),
    token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
    mint_a,    // no parens needed for unconstrained accounts
]
```

| Constraint | Syntax | Effect |
|---|---|---|
| Writable | `mut` | Injects `if !x.is_writable()` check |
| Signer | `signer` | Injects `if !x.is_signer()` check |
| PDA seeds | `pda = ["seed", account, arg]` | Seeds written into IDL for client-side derivation |
| State mapping | `state = MyStruct` | Records which state struct belongs to this account in the IDL |
| Fixed address | `address = "Base58..."` | Address written directly into the IDL |
| Relations | `relations = [a, b]` | Dependency hints in the IDL for client-side account resolution |

**Constraints can be combined** — `escrow(mut, signer, state = EscrowState)` is valid.

### PDA seed kinds

Seeds in `pda = [...]` are classified automatically by what you write:

| What you write | What it becomes in the IDL |
|---|---|
| `"escrow"` or `b"escrow"` | `{ "kind": "const", "value": [101, 115, ...] }` |
| An account name declared above it in `accounts = [...]` | `{ "kind": "account", "path": "maker" }` |
| A field name declared in `data = [...]` | `{ "kind": "arg", "path": "seed" }` |

---

## Data fields reference

Every field in `data = [...]` maps a variable name and type to a byte offset in `data`:

```rust
data = [
    amount:   u64 = data[0..8],    // range — u64::from_le_bytes(data[0..8])
    duration: i64 = data[8..16],   // signed types work the same way
    bump:     u8  = data[16],      // single byte index — data[16]
]
```

Supported types: `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`.

All declared fields become local variables available immediately in your function body. You never write the parsing code manually.

---

## `#[p_state]` reference

```rust
#[p_state]
pub struct MyState {
    pub field: u64,
    // ...
}

// Generates:
// impl MyState {
//     pub const SPACE: usize = 8 + <sum of field sizes>;
//     pub const DISCRIMINATOR: [u8; 8] = <sha256("account:MyState")[..8]>;
// }
```

The discriminator formula is identical to Anchor's — `sha256("account:StructName")[..8]` — so the value in your `idl.json` and the value in your compiled program are always identical.

Supported field types: `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`, `bool`, `Pubkey`, `Address`, fixed-size arrays (e.g. `[u8; 32]`).

---

## CLI reference

```
pinocchio-idl build [OPTIONS]

Options:
  --manifest-path <PATH>   Path to Cargo.toml [default: Cargo.toml]
  --out <PATH>             Output path for idl.json [default: idl.json]
  -h, --help               Print help
```

```bash
# Run from your program root — simplest case
pinocchio-idl build

# Write IDL somewhere specific
pinocchio-idl build --out target/idl/my_program.json

# Run from outside the program directory
pinocchio-idl build \
    --manifest-path /path/to/my-program/Cargo.toml \
    --out /path/to/my-program/idl.json
```

The CLI reads your `Cargo.toml` for `name`, `version`, and `description`, then walks your `src/` directory and picks up:

- `declare_id!(...)` or `pinocchio_pubkey::declare_id!(...)` → `address`
- `#[p_instruction(...)]` functions → `instructions` list
- `#[p_state]` structs → `accounts` + `types` lists

Accounts bound in a function body but not explicitly listed in `accounts = [...]` are picked up automatically via static analysis and added to that instruction's accounts list with no constraints.

---

## One required convention

Your accounts parameter must be literally named `accounts`:

```rust
// ✅ works — type and position don't matter, the name does
pub fn process_make_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult
pub fn process_make_instruction(data: &[u8], accounts: &mut [AccountView]) -> ProgramResult

// ❌ compile error: "`process_make_instruction` must take a parameter literally named `accounts`"
pub fn process_make_instruction(accs: &[AccountView], data: &[u8]) -> ProgramResult
```

---

## Known limitations (v0.1)

**PDA on-chain verification is not yet injected.** Seeds declared in `pda = [...]` are fully written into the IDL for client-side derivation, but the macro does not inject a `create_program_address` check into your compiled program. Verify PDAs manually in your instruction handler for now.

**`errors` and `constants` IDL sections are always empty.** No annotation convention for these exists yet.

**`[u8; 8]` fields in state structs become `"bytes"` in the IDL**, not `"u64"` or `"i64"`, because signedness can't be recovered from a byte array. Declare numeric fields as `i64`/`u64` directly if you need the correct IDL type.

**Instruction discriminators are single bytes.** The `id = N` value in your macro becomes `"discriminator": [N]` in the IDL. Real Anchor programs use 8-byte `sha256("global:<name>")` discriminators. If a Codama renderer rejects single-byte discriminators, use `setInstructionDiscriminatorsVisitor` to reconcile.

---

## Supported versions

Tested against `pinocchio = "0.11.2"`. The macros emit bare `ProgramError` variant names and resolve against whatever is already in scope in your instruction file — any pinocchio version that brings `ProgramError` into scope via `use` should work without changes.

---

## Installation

### CLI

```bash
cargo install --git https://github.com/DivineUX23/pinocchio-idl pinocchio-idl-cli
```

### Macros (in your program's Cargo.toml)

```toml
[dependencies]
pinocchio-idl-macros = { git = "https://github.com/DivineUX23/pinocchio-idl" }
=======
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

The macro injects account-count bounds checking and per-account constraint validation directly into your function body — **at compile time**, with zero overhead at runtime beyond the checks themselves.

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

The generated file is a valid Anchor-compatible IDL, usable with any Anchor TypeScript client or IDL viewer.

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

The generated `idl.json` follows the Anchor IDL spec:

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
>>>>>>> 431b462 (readme updated)
```

---

## Contributing

<<<<<<< HEAD
```
crates/
├── pinocchio-idl-core/     # Shared parsing types, Idl* structs, helper functions
├── pinocchio-idl-macros/   # #[p_instruction] and #[p_state] proc-macros
└── pinocchio-idl-cli/      # CLI tool + IDL assembly logic
```

```bash
git clone https://github.com/DivineUX23/pinocchio-idl
cd pinocchio-idl
cargo test --workspace
cargo clippy --workspace --all-targets
```
=======
Issues and PRs are welcome. This project is in active development as part of the Turbine Solana accelerator program.

---

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
>>>>>>> 431b462 (readme updated)
