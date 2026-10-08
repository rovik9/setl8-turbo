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
///    account for `product_program_id`. The vault only reads it here (it
///    declares it read-only); the builder keeps marking it writable, which
///    takes a write lock on the registry and means the sector's outer
///    transaction must also mark it writable.
/// 2. `[writable]` `trader_state` -- the challenge's `TraderState`
///    (`remaining_accounts[0]`), marked `Failed`.
///
/// The vault declares exactly these 3 accounts; no token accounts are involved.
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
