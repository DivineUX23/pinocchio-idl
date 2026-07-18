#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer},
    error::ProgramError, sysvars::{Sysvar, rent::Rent},
};
use pinocchio_idl_macros::{p_constant, p_error, p_event, p_instruction, p_state};
use pinocchio_system::instructions::CreateAccount;
/// The const program ID.
pub const ID: ::solana_address::Address = ::solana_address::Address::from_str_const(
    "11111111111111111111111111111111111111111",
);
/// Returns `true` if given address is the ID.
pub fn check_id(id: &::solana_address::Address) -> bool {
    id == &ID
}
/// Returns the ID.
pub const fn id() -> ::solana_address::Address {
    { ID }
}
pub const MAX_ESCROW_DURATION: u64 = 60 * 60 * 24 * 30;
pub const ESCROW_VERSION: u8 = 1;
pub enum EscrowError {
    /// item already taken
    AlreadyTaken,
    /// item is zero
    ZeroAmount,
    /// item is invalid
    InvalidMint,
    /// item has expired
    Expired,
}
#[repr(C)]
pub struct Escrow {
    pub seed: u64,
    pub maker: [u8; 32],
    pub mint_a: [u8; 32],
    pub mint_b: [u8; 8],
    pub receive: u64,
    pub bump: u8,
    pub authority: Option<[u8; 32]>,
}
impl Escrow {
    pub const SPACE: usize = std::mem::size_of::<Self>();
    pub const DISCRIMINATOR: [u8; 8] = [
        31u8, 213u8, 123u8, 187u8, 186u8, 22u8, 218u8, 155u8,
    ];
}
pub fn process_make_instruction(
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    if accounts.len() < 7usize {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let [maker, vault, mint_a, mint_b, escrow, token_program, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let seed = <u64>::from_le_bytes(
        data
            .get(0..8)
            .and_then(|s| s.try_into().ok())
            .ok_or(ProgramError::InvalidArgument)?,
    );
    let receive = <u64>::from_le_bytes(
        data
            .get(8..16)
            .and_then(|s| s.try_into().ok())
            .ok_or(ProgramError::InvalidArgument)?,
    );
    let bump = <u8>::from_le_bytes(
        data
            .get(16..17)
            .and_then(|s| s.try_into().ok())
            .ok_or(ProgramError::InvalidArgument)?,
    );
    if !maker.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !vault.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }
    if !escrow.is_writable() {
        return Err(ProgramError::InvalidAccountData);
    }
    {
        let __expected_pda = ::pinocchio::Address::from(
            pinocchio_pubkey::derive_address(
                &[b"escrow", mint_b.address().as_ref(), &seed.to_le_bytes()],
                None,
                &[
                    6u8, 221u8, 246u8, 225u8, 215u8, 101u8, 161u8, 147u8, 217u8, 203u8,
                    225u8, 70u8, 206u8, 235u8, 121u8, 172u8, 28u8, 180u8, 133u8, 237u8,
                    95u8, 91u8, 55u8, 145u8, 58u8, 140u8, 245u8, 133u8, 126u8, 255u8,
                    0u8, 169u8,
                ],
            ),
        );
        if escrow.address() != &__expected_pda {
            return Err(ProgramError::InvalidArgument);
        }
    }
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
    let escrow_data = unsafe {
        &mut *(escrow.borrow_unchecked_mut().as_mut_ptr() as *mut Escrow)
    };
    let mut maker_bytes = [0u8; 32];
    maker_bytes.copy_from_slice(maker.address().as_ref());
    let mut mint_a_bytes = [0u8; 32];
    mint_a_bytes.copy_from_slice(mint_a.address().as_ref());
    let mut mint_b_bytes = [0u8; 8];
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
pub struct EscrowCreated {
    pub maker: [u8; 32],
    pub amount: u64,
}
