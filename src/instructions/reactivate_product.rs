use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:reactivate_product")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const REACTIVATE_PRODUCT_DISCRIMINATOR: [u8; 8] = [111, 53, 231, 254, 125, 104, 3, 193];

/// Instruction payload for `reactivate_product`.
///
/// Re-enables a previously deactivated product. This crate has no knowledge
/// of what "deactivated" means state-wise (that's `ProductRegistry`'s field
/// layout, owned by `setl8-vault`) -- it only defines the instruction shape.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct ReactivateProductArgs {
    /// The sector program's on-chain program ID identifying which registered
    /// product to reactivate.
    pub product_program_id: Pubkey,
}

/// Builds a `reactivate_product` instruction.
///
/// # Accounts
/// 0. `[signer]` `sl8_admin` -- SL8's half of the 2-of-2 admin multisig.
/// 1. `[signer]` `rov_admin` -- Rov's half of the 2-of-2 admin multisig. Both
///    are required.
/// 2. `[writable]` `product_registry` -- the vault's `ProductRegistry` account
///    holding the product entry being reactivated.
///    3..N `remaining_accounts` -- any further accounts `setl8-vault` requires.
///    An empty slice is valid for now.
///
/// No CPI-auth identity account here -- admin-multisig-gated, not
/// CPI-auth-gated (same as `register_product`).
pub fn reactivate_product(
    vault_program_id: Pubkey,
    sl8_admin: Pubkey,
    rov_admin: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: ReactivateProductArgs,
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
        data: build_instruction_data(REACTIVATE_PRODUCT_DISCRIMINATOR, &args),
    }
}
