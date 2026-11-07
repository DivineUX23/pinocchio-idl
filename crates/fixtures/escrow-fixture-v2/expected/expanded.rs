#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer},
    error::ProgramError, sysvars::{Sysvar, rent::Rent},
};
use pinocchio_idl_macros::{p_constant, p_error, p_event, p_instruction, p_state, p_type};
use pinocchio_system::instructions::CreateAccount;
/// The const program ID.
pub const ID: ::solana_address::Address = ::solana_address::Address::from_str_const(
    "4Nd1mBQtrMJVYVfKf2PJy9NZPdUZmG3nP99rr4rUP3Fi",
);
/// Returns `true` if given address is the ID.
pub fn check_id(id: &::solana_address::Address) -> bool {
    id == &ID
}
/// Returns the ID.
pub const fn id() -> ::solana_address::Address {
    { ID }
}
/// Maximum duration an escrow offer remains valid before expiring (30 days in seconds).
pub const MAX_ESCROW_DURATION: u64 = 60 * 60 * 24 * 30;
/// Current protocol schema version for escrow state accounts.
pub const ESCROW_VERSION: u8 = 2;
/// Errors returned by the escrow program during validation and execution.
pub enum EscrowError {
    /// The escrow offer has already been accepted and fulfilled by another taker.
    AlreadyTaken,
    /// Requested exchange amount cannot be zero.
    ZeroAmount,
    /// Invalid token mint provided for swap; does not match escrow terms.
    InvalidMint,
    /// Escrow offer has expired past MAX_ESCROW_DURATION.
    Expired,
    /// Referral account was specified but is invalid or not writable.
    InvalidReferrer,
}
/// Lifecycle status of an on-chain escrow account.
#[repr(u8)]
pub enum EscrowStatus {
    /// Escrow has been created, vault is funded, waiting for a taker.
    Pending = 0,
    /// Escrow swap was successfully executed and completed.
    Completed = 1,
    /// Escrow was cancelled by maker and remaining funds refunded.
    Cancelled = 2,
}
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for EscrowStatus {}
#[automatically_derived]
impl ::core::clone::Clone for EscrowStatus {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for EscrowStatus {}
#[automatically_derived]
impl ::core::fmt::Debug for EscrowStatus {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                EscrowStatus::Pending => "Pending",
                EscrowStatus::Completed => "Completed",
                EscrowStatus::Cancelled => "Cancelled",
            },
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for EscrowStatus {}
#[automatically_derived]
impl ::core::cmp::PartialEq for EscrowStatus {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ::core::intrinsics::discriminant_value(self)
            == ::core::intrinsics::discriminant_value(other)
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for EscrowStatus {}
/// Fee and reward configuration applied to the escrow trade.
#[repr(C)]
pub struct FeeConfig {
    /// Protocol fee in basis points (1 bps = 0.01%).
    pub maker_fee_bps: u16,
    /// Referral reward in basis points allocated to the optional referrer.
    pub referral_bps: u16,
}
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for FeeConfig {}
#[automatically_derived]
impl ::core::clone::Clone for FeeConfig {
    #[inline]
    fn clone(&self) -> Self {
        let _: ::core::clone::AssertParamIsClone<u16>;
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for FeeConfig {}
#[automatically_derived]
impl ::core::fmt::Debug for FeeConfig {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "FeeConfig",
            "maker_fee_bps",
            &self.maker_fee_bps,
            "referral_bps",
            &&self.referral_bps,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for FeeConfig {}
#[automatically_derived]
impl ::core::cmp::PartialEq for FeeConfig {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.maker_fee_bps == other.maker_fee_bps
            && self.referral_bps == other.referral_bps
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for FeeConfig {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_fields_are_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<u16>;
    }
}
#[repr(C)]
/// On-chain state tracking an escrow offer between a maker and prospective taker.
pub struct Escrow {
    /// Unique user-provided seed for deriving the escrow PDA address.
    pub seed: u64,
    /// Public key of the maker who initiated and funded the escrow.
    pub maker: [u8; 32],
    /// Mint address of the tokens deposited into the escrow vault (Token A).
    pub mint_a: [u8; 32],
    /// Mint address of the tokens expected in exchange from the taker (Token B).
    pub mint_b: [u8; 32],
    /// Required amount of Token B to complete the exchange.
    pub receive: u64,
    /// Canonical bump seed for the escrow PDA.
    pub bump: u8,
    /// Current lifecycle status of this escrow.
    pub status: EscrowStatus,
    /// Fee parameters governing protocol fees and optional referral splits.
    pub fee_config: FeeConfig,
    /// Optional authority allowed to cancel or dispute this escrow.
    pub authority: Option<[u8; 32]>,
}
impl Escrow {
    pub const SPACE: usize = std::mem::size_of::<Self>();
    pub const DISCRIMINATOR: [u8; 8] = [
        31u8, 213u8, 123u8, 187u8, 186u8, 22u8, 218u8, 155u8,
    ];
}
/// Initialize a new escrow offer, deposit Token A into the vault, and set terms.
///
/// Supports an optional `referrer` account for affiliate fee sharing.
pub fn process_make_instruction(
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    if accounts.len() < 7usize {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let [maker, vault, mint_a, mint_b, escrow, token_program, system_program,
    ref_slice @ ..] = accounts else {
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
    let referrer = ref_slice.first();
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
    let mut mint_b_bytes = [0u8; 32];
    mint_b_bytes.copy_from_slice(mint_b.address().as_ref());
    escrow_data.seed = seed;
    escrow_data.maker = maker_bytes;
    escrow_data.mint_a = mint_a_bytes;
    escrow_data.mint_b = mint_b_bytes;
    escrow_data.receive = receive;
    escrow_data.bump = bump;
    escrow_data.status = EscrowStatus::Pending;
    escrow_data.fee_config = FeeConfig {
        maker_fee_bps: 25,
        referral_bps: if referrer.is_some() { 10 } else { 0 },
    };
    escrow_data.authority = None;
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
/// Event emitted when a new escrow offer is successfully created and funded.
pub struct EscrowCreated {
    /// Public key of the escrow maker.
    pub maker: [u8; 32],
    /// Amount of Token B required to take the escrow.
    pub amount: u64,
    /// Referral reward in basis points allocated to referrer (if any).
    pub referral_bps: u16,
}
