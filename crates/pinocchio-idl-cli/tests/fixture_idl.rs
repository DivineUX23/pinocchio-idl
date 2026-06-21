use std::path::Path;
use pinocchio_idl_cli::build_idl;
use pinocchio_idl_core::Metadata;

fn fixture_src_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/escrow-fixture/src")
}

fn test_metadata() -> Metadata {
    Metadata {
        name: "escrow-fixture".to_string(),
        version: "0.0.0".to_string(),
        spec: "0.1.0".to_string(),
        description: "fixture".to_string(),
    }
}

#[test]
fn builds_idl_from_fixture() {
    let idl = build_idl(&fixture_src_dir(), test_metadata())
        .expect("fixture should produce a valid Idl");

    assert_eq!(idl.address, "11111111111111111111111111111111111111111");
    assert_eq!(idl.instructions.len(), 1);

    let make = &idl.instructions[0];
    //assert_eq!(make.name, "make");
    assert_eq!(make.discriminator, vec![0]);
    assert_eq!(make.accounts.len(), 6);

    let escrow = make.accounts.iter().find(|a| a.name == "escrow").unwrap();
    assert!(escrow.pda_seeds.is_some());
    assert_eq!(escrow.state.as_deref(), Some("Escrow"));

    let vault = make.accounts.iter().find(|a| a.name == "vault").unwrap();
    assert_eq!(vault.relations.as_deref(), Some(&["escrow".to_string(), "mint_a".to_string()][..]));

    let token_program = make.accounts.iter().find(|a| a.name == "token_program").unwrap();
    assert_eq!(token_program.address.as_deref(), Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"));

    assert_eq!(idl.accounts[0].name, "Escrow");
    assert_eq!(idl.accounts[0].discriminator.len(), 8);
    assert_eq!(idl.types[0].r#type.fields.len(), 6);
}