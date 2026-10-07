use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:deposit_reset")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const DEPOSIT_RESET_DISCRIMINATOR: [u8; 8] = [25, 27, 129, 85, 180, 120, 189, 155];

/// Instruction payload for `deposit_reset`.
///
/// A phase-specific reset: the trader pays a reduced price to restart at the
/// phase they failed, instead of rebuying from the start. The vault -- not the
/// sector program -- carries the payout progress over: it reads
/// `prev_challenge_id`'s record itself and copies `payout_count` and account
/// size into the new record, so a sector bug can never hand a reset trader
/// more payouts than they have left.
///
/// Only a `Failed` record can be reset (not `Abandoned`), and each failed
/// record can be reset at most once; a full rebuy goes through `deposit_fee`
/// as a brand-new challenge.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct DepositResetArgs {
    /// Reset price being paid, in the vault's base unit. The vault checks it
    /// equals `account_size * reset_price_bps[reset_phase] / 10_000` for the
    /// previous record's account size.
    pub amount: u64,
    pub trader_wallet: Pubkey,
    pub product_program_id: Pubkey,
    /// The `Failed` challenge being reset.
    pub prev_challenge_id: u64,
    /// The fresh challenge ID for the reset attempt. Must be unused.
    pub new_challenge_id: u64,
    /// Index into the product's `reset_price_bps` table (0-based phase the
    /// trader failed in). Sector-supplied and used **only for pricing** -- the
    /// vault cannot see which phase a trader failed in. A wrong value changes
    /// what is charged, never how many payouts remain.
    pub reset_phase: u8,
}

/// Builds a `deposit_reset` instruction.
///
/// # Accounts
/// 0. `[signer]` `sector_authority` -- CPI-auth identity, same as
///    `deposit_fee`.
/// 1. `[writable]` `product_registry`.
///    2..N `remaining_accounts` -- in this order: the previous challenge's
///    `TraderState` (writable, marked reset-used), the new challenge's
///    `TraderState` (writable, created), the rent payer (signer, writable),
///    the system program, followed by the same token-movement accounts as
///    `deposit_fee`.
pub fn deposit_reset(
    vault_program_id: Pubkey,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: DepositResetArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sector_authority, true),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(DEPOSIT_RESET_DISCRIMINATOR, &args),
    }
}
