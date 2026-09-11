use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:admin_withdraw_marketing_funds")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR: [u8; 8] =
    [149, 0, 251, 20, 103, 248, 17, 186];

/// Instruction payload for `admin_withdraw_marketing_funds`.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct AdminWithdrawMarketingFundsArgs {
    /// Amount to withdraw, in whatever base unit `setl8-vault` standardizes
    /// on. Opaque to this crate.
    pub amount: u64,
}

/// Builds an `admin_withdraw_marketing_funds` instruction.
///
/// # Accounts
/// 0. `[signer]` `sl8_admin` -- SL8's half of the 2-of-2 admin multisig.
/// 1. `[signer]` `rov_admin` -- Rov's half of the 2-of-2 admin multisig. Both
///    are required.
/// 2. `[writable]` `marketing_funds_source` -- the vault-controlled account
///    marketing funds are withdrawn from. Exact PDA/account this maps to is
///    `setl8-vault`'s decision, not this crate's.
/// 3. `[writable]` `destination_wallet` -- the fixed SL8 marketing wallet
///    per the spec. **Not hardcoded here** -- pass the current SL8 marketing
///    wallet pubkey at call time (see this crate's design note on why no
///    wallet addresses are baked into builder functions). `setl8-vault` is
///    expected to independently enforce, as its own business logic, that
///    this matches its configured marketing destination -- this crate does
///    not perform or encode that check.
///    4..N `remaining_accounts` -- any further accounts `setl8-vault` requires
///    (e.g. a token program if funds are SPL-token-denominated). An empty
///    slice is valid for now.
///
/// No CPI-auth identity account here -- admin-multisig-gated, not
/// CPI-auth-gated.
pub fn admin_withdraw_marketing_funds(
    vault_program_id: Pubkey,
    sl8_admin: Pubkey,
    rov_admin: Pubkey,
    marketing_funds_source: Pubkey,
    destination_wallet: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: AdminWithdrawMarketingFundsArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sl8_admin, true),
        AccountMeta::new_readonly(rov_admin, true),
        AccountMeta::new(marketing_funds_source, false),
        AccountMeta::new(destination_wallet, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR, &args),
    }
}
