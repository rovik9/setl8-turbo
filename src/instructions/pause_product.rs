use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:pause_product")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const PAUSE_PRODUCT_DISCRIMINATOR: [u8; 8] = [146, 44, 126, 129, 251, 223, 185, 185];

/// Instruction payload for `pause_product`.
///
/// Manual, planned pause (e.g. for an upgrade). The vault records the reason
/// as "planned upgrade" so the public status display can tell it apart from a
/// reconciliation-deficit auto-pause. Mirrors `reactivate_product`: both
/// admin signatures required.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct PauseProductArgs {
    pub product_program_id: Pubkey,
}

/// Builds a `pause_product` instruction.
///
/// # Accounts
/// 0. `[signer]` `sl8_admin` -- SL8's half of the 2-of-2 admin multisig.
/// 1. `[signer]` `rov_admin` -- Rov's half. Both are required.
/// 2. `[writable]` `product_registry`.
///    3..N `remaining_accounts` -- any further accounts `setl8-vault` requires.
pub fn pause_product(
    vault_program_id: Pubkey,
    sl8_admin: Pubkey,
    rov_admin: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: PauseProductArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sl8_admin, true),
        AccountMeta::new_readonly(rov_admin, true),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(PAUSE_PRODUCT_DISCRIMINATOR, &args),
    }
}
