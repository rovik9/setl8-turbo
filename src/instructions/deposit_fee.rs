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
    pub product_id: Pubkey,
    pub challenge_id: u64,
}

/// Builds a `deposit_fee` instruction.
///
/// # Accounts
/// 0. `[]` `calling_program_identity` -- the CPI-auth identity account. See
///    [`crate::INSTRUCTIONS_SYSVAR_ID`] doc comment for the flagged assumption
///    this represents: that `setl8-vault` authenticates its direct caller via
///    Solana instruction introspection (this account is expected to be the
///    Instructions sysvar) rather than a signer-PDA scheme. The vault reads
///    the calling program's ID from it and checks that against
///    `ProductRegistry.product_program_id` for `product_id`.
/// 1. `[writable]` `product_registry` -- the vault's `ProductRegistry`
///    account, read to validate `product_id`/`challenge_id` and (assumed)
///    written to track deposited fees. Marked writable as a conservative
///    default -- confirm against `setl8-vault`'s actual mutation needs.
///    2..N `remaining_accounts` -- token-movement accounts (source, destination,
///    token program, etc.) and any other accounts `setl8-vault` requires.
///    None of that layout is known to this crate by design. An empty slice is
///    valid for now.
pub fn deposit_fee(
    vault_program_id: Pubkey,
    calling_program_identity: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: DepositFeeArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(calling_program_identity, false),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(DEPOSIT_FEE_DISCRIMINATOR, &args),
    }
}
