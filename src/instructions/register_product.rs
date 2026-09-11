use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;
use crate::types::ChallengeSize;

/// `sha256("global:register_product")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const REGISTER_PRODUCT_DISCRIMINATOR: [u8; 8] = [224, 97, 195, 220, 124, 218, 78, 43];

/// Instruction payload for `register_product`.
///
/// Registers a new sector product (e.g. lev-trading) with the vault: which
/// program is allowed to CPI in on its behalf, the fee split it receives, the
/// challenge sizes it offers, and a cap on outstanding payouts. This crate
/// does not validate or interpret any of these values -- that's
/// `setl8-vault`'s job.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct RegisterProductArgs {
    /// The sector program's own on-chain program ID. Stored by the vault in
    /// `ProductRegistry` and later used as the expected CPI-auth identity for
    /// this product on every CPI-auth-context instruction (`deposit_fee`,
    /// `request_payout`, `flag_trader_failed`).
    pub product_program_id: Pubkey,
    /// Basis points of collected fees this product's operator receives.
    pub fee_split_bps: u16,
    /// The challenge size/cost tiers this product offers at registration.
    pub challenge_sizes: Vec<ChallengeSize>,
    /// Cap on outstanding payouts the vault will allow for this product.
    pub max_payout_count: u64,
}

/// Builds a `register_product` instruction.
///
/// # Accounts
/// 0. `[signer]` `sl8_admin` -- SL8's half of the 2-of-2 admin multisig.
/// 1. `[signer]` `rov_admin` -- Rov's half of the 2-of-2 admin multisig. Both
///    are required; there is no single-admin path for this instruction.
/// 2. `[writable]` `product_registry` -- the vault's `ProductRegistry` account
///    (PDA/layout owned by `setl8-vault`, not this crate) that gains the new
///    product entry.
///    3..N `remaining_accounts` -- any further accounts `setl8-vault` requires
///    (e.g. a system program for allocation) once its `ProductRegistry`
///    account layout is finalized. An empty slice is valid for now.
///
/// This instruction has **no CPI-auth identity account** of its own -- it's
/// gated by the admin multisig, not by CPI-auth. `product_program_id` (inside
/// `args`) is the value the vault will later check *against* during
/// CPI-auth-context calls for this product; it isn't checked here.
pub fn register_product(
    vault_program_id: Pubkey,
    sl8_admin: Pubkey,
    rov_admin: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: RegisterProductArgs,
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
        data: build_instruction_data(REGISTER_PRODUCT_DISCRIMINATOR, &args),
    }
}
