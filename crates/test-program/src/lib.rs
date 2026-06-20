use pinocchio_idl_macros::p_instruction;

// Dummy types to make the macro output valid Rust for `cargo expand`
pub struct AccountView;
pub type ProgramResult = Result<(), ()>;

#[p_instruction(
    id = 1,
    accounts = [
        maker(signer),
        escrow(mut, pda=[b"escrow", maker.key().as_ref(), &seed.to_le_bytes()], state=EscrowState)
    ],
    data = [
        seed: u64 = data[0..8]
    ]
)]
pub fn process(program_id: &pinocchio::pubkey::Pubkey, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    // User binds accounts manually
    let [
        contributor,
        maker,
        mint_to_raise,
        fundraiser,
        contributor_account,
        contributor_ata,
        vault,
        _system_program,
        _token_program,
    ] = accounts 
    else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    let maker = &accounts[0];
    let escrow = &accounts[1];

    let hmmmm = <u64>::from_le_bytes(data[0..8].try_into().unwrap());

    // User business logic here...
    Ok(())
}