use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_idl_macros::{p_constant, p_error, p_event, p_instruction, p_state, p_type};
use pinocchio_system::instructions::CreateAccount;

pinocchio::address::declare_id!("4Nd1mBQtrMJVYVfKf2PJy9NZPdUZmG3nP99rr4rUP3Fi");

/// Maximum duration an escrow offer remains valid before expiring (30 days in seconds).
#[p_constant]
pub const MAX_ESCROW_DURATION: u64 = 60 * 60 * 24 * 30;

/// Current protocol schema version for escrow state accounts.
#[p_constant]
pub const ESCROW_VERSION: u8 = 2;

/// Errors returned by the escrow program during validation and execution.
#[p_error]
pub enum EscrowError {
    /// The escrow offer has already been accepted and fulfilled by another taker.
    AlreadyTaken,
    /// Requested exchange amount cannot be zero.
    ZeroAmount,
    /// Invalid token mint provided for swap; does not match escrow terms.
    #[p_code = 100]
    InvalidMint,
    /// Escrow offer has expired past MAX_ESCROW_DURATION.
    Expired,
    /// Referral account was specified but is invalid or not writable.
    InvalidReferrer,
}

/// Lifecycle status of an on-chain escrow account.
#[p_type]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EscrowStatus {
    /// Escrow has been created, vault is funded, waiting for a taker.
    Pending = 0,
    /// Escrow swap was successfully executed and completed.
    Completed = 1,
    /// Escrow was cancelled by maker and remaining funds refunded.
    Cancelled = 2,
}

/// Fee and reward configuration applied to the escrow trade.
#[p_type]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct FeeConfig {
    /// Protocol fee in basis points (1 bps = 0.01%).
    pub maker_fee_bps: u16,
    /// Referral reward in basis points allocated to the optional referrer.
    pub referral_bps: u16,
}

/// On-chain state tracking an escrow offer between a maker and prospective taker.
#[p_state(inject)]
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

/// Initialize a new escrow offer, deposit Token A into the vault, and set terms.
///
/// Supports an optional `referrer` account for affiliate fee sharing.
#[p_instruction(
    id = 0,
    inject,
    accounts = [
        maker(signer, mut),
        vault(mut, init=[escrow, mint_a]),
        mint_a,
        mint_b,
        escrow(mut, pda=["escrow", mint_b, seed, program="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"], state=Escrow),
        token_program(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
        system_program,
        referrer(optional)
    ],
    data = [
        seed: u64 = data[0..8],
        receive: u64 = data[8..16],
        bump: u8 = data[16..17]
    ]
)]
pub fn process_make_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        vault,
        mint_a,
        mint_b,
        escrow,
        token_program,
        system_program,
        ref_slice @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

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

    let escrow_data = unsafe { &mut *(escrow.borrow_unchecked_mut().as_mut_ptr() as *mut Escrow) };

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
#[p_event]
pub struct EscrowCreated {
    /// Public key of the escrow maker.
    pub maker: [u8; 32],
    /// Amount of Token B required to take the escrow.
    pub amount: u64,
    /// Referral reward in basis points allocated to referrer (if any).
    pub referral_bps: u16,
}
