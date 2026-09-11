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
    pub product_id: Pubkey,
    pub challenge_id: u64,
}

/// Builds a `flag_trader_failed` instruction.
///
/// # Accounts
/// 0. `[]` `calling_program_identity` -- the CPI-auth identity account. See
///    [`crate::INSTRUCTIONS_SYSVAR_ID`] doc comment for the flagged assumption
///    this represents (Instructions-sysvar-based introspection, not a
///    signer-PDA scheme). The vault reads the calling program's ID from it and
///    checks that against `ProductRegistry.product_program_id` for
///    `product_id`.
/// 1. `[writable]` `product_registry` -- the vault's `ProductRegistry`
///    account, read to validate `product_id`/`challenge_id` and (assumed)
///    written as part of failure bookkeeping. Marked writable as a
///    conservative default -- confirm against `setl8-vault`'s actual
///    mutation needs.
///    2..N `remaining_accounts` -- trader/challenge state accounts (`TraderState`,
///    `BondPosition`, `BondCapTracker`, etc.) and any other accounts
///    `setl8-vault` requires. None of that layout is known to this crate by
///    design. An empty slice is valid for now.
pub fn flag_trader_failed(
    vault_program_id: Pubkey,
    calling_program_identity: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: FlagTraderFailedArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(calling_program_identity, false),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(FLAG_TRADER_FAILED_DISCRIMINATOR, &args),
    }
}
