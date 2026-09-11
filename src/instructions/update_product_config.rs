use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;
use crate::types::ChallengeSize;

/// `sha256("global:update_product_config")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const UPDATE_PRODUCT_CONFIG_DISCRIMINATOR: [u8; 8] = [148, 82, 249, 211, 243, 20, 93, 174];

/// Instruction payload for `update_product_config`.
///
/// Updates an already-registered product's config in place.
///
/// **Naming note (flagged, not silently fixed):** this struct's identifier
/// field is `product_id`, while `RegisterProductArgs`'s equivalent field is
/// `product_program_id`. Both are typed as `Pubkey` and, going by the spec
/// this crate was built from, appear to refer to the same concept (the sector
/// program's on-chain program ID). Kept exactly as specified rather than
/// unified -- confirm whether this is intentional (e.g. `product_id` as a
/// distinct registry key vs. `product_program_id` as the program address) or
/// a naming inconsistency to fix before `setl8-vault` depends on this crate.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct UpdateProductConfigArgs {
    /// See naming note above.
    pub product_id: Pubkey,
    /// Replacement challenge size/cost tiers.
    pub challenge_sizes: Vec<ChallengeSize>,
    /// Replacement fee split, in basis points.
    pub fee_split_bps: u16,
    /// Replacement cap on outstanding payouts.
    pub max_payout_count: u64,
}

/// Builds an `update_product_config` instruction.
///
/// # Accounts
/// 0. `[signer]` `sl8_admin` -- SL8's half of the 2-of-2 admin multisig.
/// 1. `[signer]` `rov_admin` -- Rov's half of the 2-of-2 admin multisig. Both
///    are required.
/// 2. `[writable]` `product_registry` -- the vault's `ProductRegistry` account
///    holding the product entry being updated.
///    3..N `remaining_accounts` -- any further accounts `setl8-vault` requires.
///    An empty slice is valid for now.
///
/// No CPI-auth identity account here -- admin-multisig-gated, not
/// CPI-auth-gated.
pub fn update_product_config(
    vault_program_id: Pubkey,
    sl8_admin: Pubkey,
    rov_admin: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: UpdateProductConfigArgs,
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
        data: build_instruction_data(UPDATE_PRODUCT_CONFIG_DISCRIMINATOR, &args),
    }
}
