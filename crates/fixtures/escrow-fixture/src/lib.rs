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
    /// item already taken
    AlreadyTaken,
    /// item is zero
    ZeroAmount,
    /// item is invalid
    #[p_code = 100]
    InvalidMint,
    /// item has expired
    Expired,
}

#[p_state]
pub struct Escrow {
    pub seed: u64,
    pub maker: [u8; 32],
    pub mint_a: [u8; 32],
    pub mint_b: [u8; 32],
    pub receive: u64,
    pub bump: u8,
    pub authority: Option<[u8; 32]>,
}

#[p_instruction(
    id = 0,
    accounts = [
        maker(signer, mut),
        vault(mut, relations=[escrow, mint_a]),
        mint_a,
        mint_b,
        escrow(mut, pda=["escrow", mint_b, seed], state=Escrow),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
        system_program
    ],
    data = [
        seed: u64 = data[0..8],
        receive: u64 = data[8..16],
        bump: u8 = data[16]
    ]
)]
pub fn process_make_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [maker, vault, mint_a, mint_b, escrow, token_program, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let _disc = Escrow::DISCRIMINATOR;
    let _space = Escrow::SPACE;

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    CreateAccount {
        from: maker,
        to: escrow,
        lamports: Rent::get()?.try_minimum_balance(Escrow::SPACE)?,
        space: Escrow::SPACE as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[signer])?;

    let escrow_data = unsafe { &mut *(escrow.borrow_unchecked_mut().as_mut_ptr() as *mut Escrow) };

    let mut maker_bytes = [0u8; 32];
    maker_bytes.copy_from_slice(maker.address().as_ref());

    let mut mint_a_bytes = [0u8; 32];
    mint_a_bytes.copy_from_slice(mint_a.address().as_ref());

    let mut mint_b_bytes = [0u8; 32];
    mint_b_bytes.copy_from_slice(mint_b.address().as_ref());

    escrow_data.maker = maker_bytes;
    escrow_data.mint_a = mint_a_bytes;
    escrow_data.mint_b = mint_b_bytes;
    escrow_data.receive = receive;
    escrow_data.seed = seed;
    escrow_data.bump = bump;

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: escrow,
        mint: mint_a,
        token_program,
        system_program,
    }
    .invoke()?;

    Ok(())
}
