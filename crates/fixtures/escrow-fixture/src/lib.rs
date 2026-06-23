use pinocchio::{AccountView, ProgramResult, error::ProgramError};
use pinocchio::pubkey::Pubkey;
//use pinocchio_pubkey::declare_id;
use pinocchio_idl_macros::{p_instruction, p_state};

pinocchio_pubkey::declare_id!("11111111111111111111111111111111111111111");

#[p_state]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive: u64,
    pub bump: u8,
}


#[p_instruction(
    id = 0,
    accounts = [
        maker(signer, mut),
        escrow(mut, pda=["escrow", maker, seed], state=Escrow),
        vault(mut, pda=["vault", maker, seed]),
        escrow_vault(mut, pda=[escrow, maker, seed]),

        vault(mut, relations=[escrow, mint_a]),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
    ],
    data = [
        seed: u64 = data[0..8],
        receive: u64 = data[8..16],
        bump: u8 = data[16]
    ]
)]
pub fn process_make_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [maker, mint_a, mint_b, escrow, vault, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    let space = Escrow::DISCRIMINATOR;
    let space = Escrow::SPACE;


    Ok(())
}