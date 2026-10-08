use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;
use crate::types::PoolSide;

/// `sha256("global:admin_withdraw_marketing_funds")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR: [u8; 8] =
    [149, 0, 251, 20, 103, 248, 17, 186];

/// Instruction payload for `admin_withdraw_marketing_funds`.
///
/// Serialized as `pool` (one byte, `Usdc` = 0 and `Usdt` = 1, see
/// [`PoolSide`]) followed by `amount` (`u64`, little-endian): 9 bytes, or 17
/// with the discriminator. The field order is the wire order.
///
/// **Breaking change in v0.4.1:** earlier versions had only `amount` and a
/// different account list that did not match the vault.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct AdminWithdrawMarketingFundsArgs {
    /// Which of the vault's two payout pools to take from. It selects the
    /// mint, the pool token account and the SL8 destination account that must
    /// be passed; pools are never combined.
    pub pool: PoolSide,
    /// Amount to take, in 6-decimal base units of that pool's token. Must be
    /// greater than zero and no more than what the pool's 25% reserve leaves
    /// available (see below).
    pub amount: u64,
}

/// Builds an `admin_withdraw_marketing_funds` instruction.
///
/// **A documented exception to "no admin key on money".** Both admins together
/// (SL8 and Rov, a 2-of-2) may move up to **75% of one pool's live balance**
/// to the SL8 wallet's token account:
///
/// * The destination is fixed: a token account of the chosen mint owned by the
///   vault's SL8 wallet. No destination can be named.
/// * `reserve = max(stored_floor, ceil(live * 25%))` and
///   `withdrawable = live - reserve`, using the pool's **live** balance. The
///   stored floor is only set by `finalize_heartbeat` (zero before the first
///   one), so the live 25% is what protects the pool until then.
/// * `amount` must be `> 0` and `<= withdrawable`, else the vault fails with
///   `ZeroAmount` / `WithdrawalExceedsReserve`.
/// * There is **no deduction for open claims, bond liabilities or the current
///   heartbeat cycle**, and no cycle gating: it can be called repeatedly at any
///   time (so a pool can shrink geometrically), and a product pause does not
///   block it. Queued claims then settle pro rata against what is left.
///
/// # Accounts
/// The vault declares exactly these 7 accounts, in this order; there are no
/// remaining accounts.
///
/// 0. `[signer]` `sl8_admin` -- SL8's half of the 2-of-2 admin multisig. The
///    vault requires this to be its built-in SL8 admin key (a compile-time
///    constant in the vault); any other key fails with
///    `MissingMultisigSignature`.
/// 1. `[signer]` `rov_admin` -- Rov's half. The vault requires this to be its
///    built-in Rov admin key, same error otherwise. Both signatures are
///    required.
/// 2. `[writable]` `vault_state` -- the vault's singleton `VaultState`
///    (it records the cumulative amount withdrawn).
/// 3. `[]` `mint` -- the mint of the chosen `pool`; must be the vault's USDC or
///    USDT mint for that side.
/// 4. `[writable]` `pool_token_account` -- the vault's payout pool for that
///    side.
/// 5. `[writable]` `sl8_token_account` -- a `mint` token account owned by the
///    vault's SL8 wallet. The only possible destination.
/// 6. `[]` `token_program` -- classic SPL Token; Token-2022 is rejected.
///
/// Admin keys and vault-side addresses are parameters, never constants here
/// (see README). The vault checks the admin keys against its built-in
/// constants, and `mint`, `pool_token_account` and `sl8_token_account` against
/// its `VaultState` (`InvalidMint` / `InvalidTokenAccount`), so a wrong address
/// is rejected rather than silently accepted.
#[allow(clippy::too_many_arguments)]
pub fn admin_withdraw_marketing_funds(
    vault_program_id: Pubkey,
    sl8_admin: Pubkey,
    rov_admin: Pubkey,
    vault_state: Pubkey,
    mint: Pubkey,
    pool_token_account: Pubkey,
    sl8_token_account: Pubkey,
    token_program: Pubkey,
    args: AdminWithdrawMarketingFundsArgs,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(sl8_admin, true),
        AccountMeta::new_readonly(rov_admin, true),
        AccountMeta::new(vault_state, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(pool_token_account, false),
        AccountMeta::new(sl8_token_account, false),
        AccountMeta::new_readonly(token_program, false),
    ];

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR, &args),
    }
}
