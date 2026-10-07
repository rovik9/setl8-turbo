use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:deposit_fee")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const DEPOSIT_FEE_DISCRIMINATOR: [u8; 8] = [11, 51, 105, 140, 198, 229, 7, 77];

/// Instruction payload for `deposit_fee`.
///
/// Called by a sector program (not directly by a trader) to deposit a
/// previously-collected challenge fee into the vault. No `trader_wallet`
/// field exists here by design -- the spec this crate was built from didn't
/// include one, implying the sector program has already collected the fee on
/// its own side before CPI-ing in. Which account(s) the funds move between is
/// a token/account-layout detail `setl8-vault` owns; see
/// `remaining_accounts` on the builder below.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct DepositFeeArgs {
    /// Fee amount being deposited, in whatever base unit `setl8-vault`
    /// standardizes on. Opaque to this crate.
    pub amount: u64,
    pub product_program_id: Pubkey,
    pub challenge_id: u64,
    /// Added in v0.3.0 (breaking). The vault keys each `TraderState` by
    /// wallet + product + challenge, so it must be told whose challenge this
    /// is. Appended last: Borsh field order is the wire order, and the vault
    /// handler's parameter order must match.
    pub trader_wallet: Pubkey,
    /// Added in v0.3.0 (breaking). The tier being purchased. The vault checks
    /// that `(account_size, amount)` is an exact `ChallengeSize` entry in the
    /// product's registry, and stores `account_size` so a later `deposit_reset`
    /// can be priced from it.
    pub account_size: u64,
}

/// Builds a `deposit_fee` instruction.
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
///    (assumed) written to track deposited fees. Marked writable as a
///    conservative default -- confirm against `setl8-vault`'s actual
///    mutation needs.
///    2..N `remaining_accounts` -- in this order: the new challenge's
///    `TraderState` (writable, created), the rent payer (signer, writable),
///    the system program, then token-movement accounts (source, destination,
///    token program, etc.) and anything else `setl8-vault` requires.
///    None of that layout is known to this crate by design. An empty slice is
///    valid for now.
pub fn deposit_fee(
    vault_program_id: Pubkey,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: DepositFeeArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sector_authority, true),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(DEPOSIT_FEE_DISCRIMINATOR, &args),
    }
}
