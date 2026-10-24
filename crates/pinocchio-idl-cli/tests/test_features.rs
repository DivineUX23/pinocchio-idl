use pinocchio_idl_cli::build_idl;
use pinocchio_idl_core::{Metadata, render::to_codama_json};
use std::fs;

#[test]
fn test_custom_types_docs_and_optional_accounts() {
    let temp_dir = std::env::temp_dir().join(format!(
        "pinocchio_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let src_code = r#"
use pinocchio::{AccountView, ProgramResult, error::ProgramError};
use pinocchio_idl_macros::{p_instruction, p_state, p_type, p_error};

pinocchio::address::declare_id!("Test111111111111111111111111111111111111111");

/// Status representing the lifecycle of an escrow.
#[p_type]
pub enum EscrowStatus {
    /// Escrow has been created and is waiting.
    Pending,
    /// Escrow was completed.
    Completed,
    /// Escrow was cancelled by maker.
    Cancelled,
}

/// Configuration for protocol trading fees.
#[p_type]
pub struct FeeConfig {
    /// Basis points for maker fee.
    pub maker_fee_bps: u16,
    /// Basis points for taker fee.
    pub taker_fee_bps: u16,
}

/// Escrow account state stored on-chain.
#[p_state]
pub struct EscrowAccount {
    /// The creator of this escrow.
    pub maker: [u8; 32],
    /// Amount to receive.
    pub amount: u64,
    /// Current lifecycle status.
    pub status: EscrowStatus,
}

/// Initialize a new escrow with optional referrer.
#[p_instruction(
    id = 1,
    accounts = [
        maker(signer, mut),
        escrow(mut),
        referrer(optional)
    ],
    data = [
        amount: u64 = data[0..8]
    ]
)]
pub fn initialize_escrow(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [maker, escrow, referrer] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    Ok(())
}
"#;

    fs::write(temp_dir.join("lib.rs"), src_code).unwrap();

    let metadata = Metadata {
        name: "test-features".to_string(),
        version: "0.1.0".to_string(),
        spec: "0.1.0".to_string(),
        description: "Test program".to_string(),
    };

    let idl = build_idl(&temp_dir, metadata).expect("should build IDL successfully");

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);

    // 1. Verify Instruction and doc extraction
    assert_eq!(idl.instructions.len(), 1);
    let ix = &idl.instructions[0];
    assert_eq!(ix.name, "initialize_escrow");
    assert_eq!(
        ix.docs.as_deref(),
        Some(&["Initialize a new escrow with optional referrer.".to_string()][..])
    );
    assert_eq!(ix.accounts.len(), 3);

    // 2. Verify optional account
    let referrer = ix.accounts.iter().find(|a| a.name == "referrer").unwrap();
    assert!(referrer.optional, "referrer should be marked optional");

    let maker = ix.accounts.iter().find(|a| a.name == "maker").unwrap();
    assert!(!maker.optional, "maker should not be marked optional");
    assert!(maker.signer);
    assert!(maker.writable);

    // 3. Verify #[p_state] docs and fields
    let escrow_acc = idl
        .accounts
        .iter()
        .find(|a| a.name == "EscrowAccount")
        .unwrap();
    assert_eq!(
        escrow_acc.docs.as_deref(),
        Some(&["Escrow account state stored on-chain.".to_string()][..])
    );

    // 4. Verify custom types (enum and struct) in types
    let escrow_type = idl
        .types
        .iter()
        .find(|t| t.name == "EscrowAccount")
        .unwrap();
    let status_field = escrow_type
        .r#type
        .fields
        .iter()
        .find(|f| f.name == "status")
        .unwrap();
    assert_eq!(
        status_field.docs.as_deref(),
        Some(&["Current lifecycle status.".to_string()][..])
    );
    assert!(
        matches!(status_field.r#type, pinocchio_idl_core::FieldType::Defined(ref d) if d == "EscrowStatus")
    );

    let fee_type = idl.types.iter().find(|t| t.name == "FeeConfig").unwrap();
    assert_eq!(fee_type.r#type.kind, "struct");
    assert_eq!(
        fee_type.docs.as_deref(),
        Some(&["Configuration for protocol trading fees.".to_string()][..])
    );
    let maker_fee_field = fee_type
        .r#type
        .fields
        .iter()
        .find(|f| f.name == "maker_fee_bps")
        .unwrap();
    assert_eq!(
        maker_fee_field.docs.as_deref(),
        Some(&["Basis points for maker fee.".to_string()][..])
    );

    let status_enum = idl.types.iter().find(|t| t.name == "EscrowStatus").unwrap();
    assert_eq!(status_enum.r#type.kind, "enum");
    assert_eq!(
        status_enum.docs.as_deref(),
        Some(&["Status representing the lifecycle of an escrow.".to_string()][..])
    );
    assert_eq!(status_enum.r#type.variants.len(), 3);
    assert_eq!(status_enum.r#type.variants[0].name, "Pending");
    assert_eq!(
        status_enum.r#type.variants[0].docs.as_deref(),
        Some(&["Escrow has been created and is waiting.".to_string()][..])
    );

    // 5. Test serialization to Anchor JSON
    let anchor_json = serde_json::to_string_pretty(&idl).expect("should serialize to Anchor JSON");
    assert!(anchor_json.contains("\"optional\": true"));
    assert!(anchor_json.contains("\"name\": \"EscrowStatus\""));
    assert!(anchor_json.contains("\"kind\": \"enum\""));
    assert!(anchor_json.contains("\"name\": \"Pending\""));
    assert!(anchor_json.contains("\"name\": \"FeeConfig\""));
    assert!(anchor_json.contains("Initialize a new escrow with optional referrer."));

    // 6. Test serialization to Codama JSON
    let codama_str = to_codama_json(&idl).expect("should serialize to Codama JSON");
    assert!(codama_str.contains("\"isOptional\": true"));
    assert!(codama_str.contains("\"kind\": \"enumTypeNode\""));
    assert!(codama_str.contains("\"kind\": \"enumEmptyVariantTypeNode\""));
    assert!(codama_str.contains("Initialize a new escrow with optional referrer."));
}

#[test]
fn test_project_init() {
    let temp_dir = std::env::temp_dir().join(format!(
        "pinocchio_init_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let project_dir = pinocchio_idl_cli::run_init("my_counter_program", Some(&temp_dir))
        .expect("init should succeed");

    assert!(project_dir.join("Cargo.toml").exists());
    assert!(project_dir.join("src/lib.rs").exists());
    assert!(project_dir.join(".github/workflows/idl-check.yml").exists());
    assert!(project_dir.join(".agents/AGENTS.md").exists());
    assert!(project_dir.join("README.md").exists());
    assert!(project_dir.join(".gitignore").exists());

    // Verify the scaffolded program produces a valid IDL out of the box!
    let metadata = Metadata {
        name: "my_counter_program".to_string(),
        version: "0.1.0".to_string(),
        spec: "0.1.0".to_string(),
        description: "Scaffolded program".to_string(),
    };

    let idl = build_idl(&project_dir.join("src"), metadata)
        .expect("scaffolded program should build IDL successfully");

    assert_eq!(idl.instructions.len(), 1);
    assert_eq!(idl.instructions[0].name, "initialize");
    assert_eq!(idl.accounts.len(), 1);
    assert_eq!(idl.accounts[0].name, "CounterAccount");
    assert_eq!(idl.constants.len(), 1);
    assert_eq!(idl.constants[0].name, "PROTOCOL_VERSION");
    assert_eq!(idl.errors.len(), 2);
    assert_eq!(
        idl.types.iter().filter(|t| t.r#type.kind == "enum").count(),
        1
    );

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_instant_codama_client_generation() {
    let temp_dir = std::env::temp_dir().join(format!(
        "pinocchio_client_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let project_dir = pinocchio_idl_cli::run_init("counter_program", Some(&temp_dir))
        .expect("init should succeed");

    let client_out = project_dir.join("clients/js");
    let options = pinocchio_idl_cli::ClientOptions {
        manifest_path: project_dir.join("Cargo.toml"),
        out_dir: client_out.clone(),
        src_dir: None,
        package_name: Some("@my-protocol/counter-client".to_string()),
        run: false,
        script_only: true,
    };

    let result = pinocchio_idl_cli::run_client(options).expect("run_client should succeed");
    assert_eq!(result, client_out);

    // Verify generated client scaffolding
    assert!(client_out.join("codama.json").exists());
    assert!(client_out.join("codama.mjs").exists());
    assert!(client_out.join("package.json").exists());
    assert!(client_out.join("tsconfig.json").exists());
    assert!(client_out.join("src").exists());

    let pkg_json = fs::read_to_string(client_out.join("package.json")).unwrap();
    assert!(pkg_json.contains("\"name\": \"@my-protocol/counter-client\""));
    assert!(pkg_json.contains("\"@codama/renderers-js\""));

    let codama_json = fs::read_to_string(client_out.join("codama.json")).unwrap();
    assert!(codama_json.contains("\"kind\": \"rootNode\""));
    assert!(codama_json.contains("counterProgram"));

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_init_agents_and_all_targets() {
    let temp_dir = std::env::temp_dir().join(format!(
        "pinocchio_agents_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    // 1. Run without --all
    pinocchio_idl_cli::run_init_agents(&temp_dir, false, false)
        .expect("init_agents should succeed");
    assert!(temp_dir.join(".agents/AGENTS.md").exists());
    assert!(!temp_dir.join("CLAUDE.md").exists());

    let content = fs::read_to_string(temp_dir.join(".agents/AGENTS.md")).unwrap();
    assert!(content.contains("# Pinocchio IDL & Solana Program Rules"));
    assert!(content.contains("#[p_instruction"));
    assert!(content.contains("#[p_event"));
    assert!(content.contains("cargo pinocchio-idl client"));

    // 2. Run with --all and force
    pinocchio_idl_cli::run_init_agents(&temp_dir, true, true)
        .expect("init_agents --all should succeed");
    assert!(temp_dir.join(".agents/AGENTS.md").exists());
    assert!(temp_dir.join("CLAUDE.md").exists());
    assert!(temp_dir.join(".cursor/rules/pinocchio.mdc").exists());
    assert!(temp_dir.join(".github/copilot-instructions.md").exists());

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_command_parsing() {
    use clap::Parser;
    use pinocchio_idl_cli::{Cli, Command, OutputFormat};
    use std::path::PathBuf;

    // Test Generate + aliases (gen, build)
    let cli = Cli::try_parse_from(["pinocchio-idl", "generate"]).unwrap();
    assert!(matches!(cli.command, Command::Generate { .. }));

    let cli = Cli::try_parse_from(["pinocchio-idl", "gen", "-f", "codama"]).unwrap();
    if let Command::Generate { format, .. } = cli.command {
        assert_eq!(format, OutputFormat::Codama);
    } else {
        panic!("expected Command::Generate");
    }

    let cli = Cli::try_parse_from(["pinocchio-idl", "build", "--out", "custom.json"]).unwrap();
    if let Command::Generate { out, .. } = cli.command {
        assert_eq!(out, Some(PathBuf::from("custom.json")));
    } else {
        panic!("expected Command::Generate");
    }

    // Test Check + verify alias
    let cli = Cli::try_parse_from(["pinocchio-idl", "check"]).unwrap();
    assert!(matches!(cli.command, Command::Check { .. }));

    let cli = Cli::try_parse_from(["pinocchio-idl", "verify"]).unwrap();
    assert!(matches!(cli.command, Command::Check { .. }));

    // Test Doctor + lint, diagnose aliases
    let cli = Cli::try_parse_from(["pinocchio-idl", "doctor", "--json"]).unwrap();
    if let Command::Doctor { json, .. } = cli.command {
        assert!(json);
    } else {
        panic!("expected Command::Doctor");
    }

    let cli = Cli::try_parse_from(["pinocchio-idl", "lint"]).unwrap();
    assert!(matches!(cli.command, Command::Doctor { .. }));

    let cli = Cli::try_parse_from(["pinocchio-idl", "diagnose"]).unwrap();
    assert!(matches!(cli.command, Command::Doctor { .. }));

    // Test Init + new alias
    let cli = Cli::try_parse_from(["pinocchio-idl", "init", "vault-program"]).unwrap();
    if let Command::Init { name, .. } = cli.command {
        assert_eq!(name, "vault-program");
    } else {
        panic!("expected Command::Init");
    }

    let cli = Cli::try_parse_from(["pinocchio-idl", "new", "escrow-program"]).unwrap();
    if let Command::Init { name, .. } = cli.command {
        assert_eq!(name, "escrow-program");
    } else {
        panic!("expected Command::Init");
    }

    // Test InitAgents + aliases (init_agents, agents)
    let cli = Cli::try_parse_from(["pinocchio-idl", "init-agents", "--all", "--force"]).unwrap();
    if let Command::InitAgents { all, force, .. } = cli.command {
        assert!(all);
        assert!(force);
    } else {
        panic!("expected Command::InitAgents");
    }

    let cli = Cli::try_parse_from(["pinocchio-idl", "init_agents"]).unwrap();
    assert!(matches!(cli.command, Command::InitAgents { .. }));

    let cli = Cli::try_parse_from(["pinocchio-idl", "agents"]).unwrap();
    assert!(matches!(cli.command, Command::InitAgents { .. }));

    // Test Client + aliases (codegen, sdk, ts)
    let cli = Cli::try_parse_from(["pinocchio-idl", "client"]).unwrap();
    assert!(matches!(cli.command, Command::Client { .. }));

    let cli =
        Cli::try_parse_from(["pinocchio-idl", "codegen", "--ts", "sdk/ts", "--no-run"]).unwrap();
    if let Command::Client { out, no_run, .. } = cli.command {
        assert_eq!(out, PathBuf::from("sdk/ts"));
        assert!(no_run);
    } else {
        panic!("expected Command::Client");
    }

    let cli = Cli::try_parse_from(["pinocchio-idl", "sdk", "--script-only"]).unwrap();
    if let Command::Client { script_only, .. } = cli.command {
        assert!(script_only);
    } else {
        panic!("expected Command::Client");
    }

    let cli = Cli::try_parse_from(["pinocchio-idl", "ts", "--package-name", "@org/sdk"]).unwrap();
    if let Command::Client { package_name, .. } = cli.command {
        assert_eq!(package_name, Some("@org/sdk".to_string()));
    } else {
        panic!("expected Command::Client");
    }
}
