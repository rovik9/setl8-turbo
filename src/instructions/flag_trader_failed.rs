use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:flag_trader_failed")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const FLAG_TRADER_FAILED_DISCRIMINATOR: [u8; 8] = [60, 230, 114, 103, 27, 235, 37, 129];

/// Instruction payload for `flag_trader_failed`.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct FlagTraderFailedArgs {
    pub trader_wallet: Pubkey,
    pub product_program_id: Pubkey,
    pub challenge_id: u64,
}

/// Builds a `flag_trader_failed` instruction.
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
///    (assumed) written as part of failure bookkeeping. Marked writable as a
///    conservative default -- confirm against `setl8-vault`'s actual
///    mutation needs.
///    2..N `remaining_accounts` -- trader/challenge state accounts (`TraderState`,
///    `BondPosition`, `BondCapTracker`, etc.) and any other accounts
///    `setl8-vault` requires. None of that layout is known to this crate by
///    design. An empty slice is valid for now.
pub fn flag_trader_failed(
    vault_program_id: Pubkey,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: FlagTraderFailedArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sector_authority, true),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(FLAG_TRADER_FAILED_DISCRIMINATOR, &args),
    }
}
