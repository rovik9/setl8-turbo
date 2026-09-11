use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:request_payout")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const REQUEST_PAYOUT_DISCRIMINATOR: [u8; 8] = [5, 176, 110, 197, 172, 177, 64, 200];

/// Instruction payload for `request_payout`.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct RequestPayoutArgs {
    pub trader_wallet: Pubkey,
    /// Payout amount requested, in whatever base unit `setl8-vault`
    /// standardizes on. Opaque to this crate.
    pub amount: u64,
    pub product_program_id: Pubkey,
    pub challenge_id: u64,
    /// The sector program's proposed next request ID for this
    /// trader+product+challenge.
    ///
    /// This is a **mutual-agreement check, not a value the vault blindly
    /// trusts**: `setl8-vault` independently tracks its own expected next
    /// request ID for the (trader_wallet, product_program_id, challenge_id)
    /// tuple, and rejects this instruction on mismatch. The sector program
    /// proposes what it believes the next ID is; the vault is the source of
    /// truth and enforces agreement rather than accepting whatever is passed
    /// in. This crate does not implement or encode that check -- it only
    /// documents the expectation for callers.
    pub proposed_request_id: u64,
}

/// Builds a `request_payout` instruction.
///
/// # Accounts
/// 0. `[signer]` `sector_authority` -- the CPI-auth identity account: the
///    calling sector program's own PDA, derived as
///    [`crate::derive_sector_authority`] under **its own** program ID using
///    [`crate::SECTOR_AUTHORITY_SEED`], and signed via `invoke_signed`. The
///    vault authenticates the caller by checking that this account's key
///    equals `find_program_address(&[SECTOR_AUTHORITY_SEED],
///    &registry.product_program_id)` for `product_program_id` below --
///    cryptographic proof of caller identity, since only the real calling
///    program can produce a valid `invoke_signed` signature for a PDA
///    derived from its own program ID.
/// 1. `[writable]` `product_registry` -- the vault's `ProductRegistry`
///    account, read to validate `product_program_id`/`challenge_id` and
///    (assumed) written as part of payout bookkeeping. Marked writable as a
///    conservative default -- confirm against `setl8-vault`'s actual
///    mutation needs.
///    2..N `remaining_accounts` -- trader/payout state accounts (`TraderState`,
///    `BondPosition`, `BondCapTracker`, etc.), token-movement accounts, and
///    any other accounts `setl8-vault` requires. None of that layout is known
///    to this crate by design. An empty slice is valid for now.
pub fn request_payout(
    vault_program_id: Pubkey,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: RequestPayoutArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sector_authority, true),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(REQUEST_PAYOUT_DISCRIMINATOR, &args),
    }
}
