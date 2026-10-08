use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:deposit_reset")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const DEPOSIT_RESET_DISCRIMINATOR: [u8; 8] = [25, 27, 129, 85, 180, 120, 189, 155];

/// Instruction payload for `deposit_reset`.
///
/// A phase-specific reset: the trader pays a reduced price to restart at the
/// phase they failed, instead of rebuying from the start. The vault -- not the
/// sector program -- carries the payout progress over: it reads
/// `prev_challenge_id`'s record itself and copies `payout_count` and account
/// size into the new record, so a sector bug can never hand a reset trader
/// more payouts than they have left.
///
/// Only a `Failed` record can be reset (not `Abandoned`), and each failed
/// record can be reset at most once; a full rebuy goes through `deposit_fee`
/// as a brand-new challenge.
///
/// **The trader pays the vault directly**, exactly as in `deposit_fee`: the
/// reset price moves from the trader's own token account (USDC or USDT,
/// classic SPL Token, 6 decimals), and the sector program never collects or
/// holds it. The trader must therefore **sign the transaction**; the sector's
/// CPI carries that signature through, so it only works when the trader's
/// signature is in the outer transaction.
///
/// # Payment split
/// Same rule as `deposit_fee`: `pool = floor(amount * fee_split_bps / 10_000)`
/// to the payout pool of the same mint, the exact remainder to a token account
/// of that mint owned by the SL8 wallet. A rejected payment reverts the whole
/// transaction, so it never burns the previous record's one-time reset.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct DepositResetArgs {
    /// Reset price being paid, in 6-decimal base units of the mint used. The
    /// vault checks it equals `floor(account_size * reset_price_bps[reset_phase]
    /// / 10_000)` for the previous record's account size, and that the
    /// trader's token account holds at least this much.
    pub amount: u64,
    /// The paying trader. Must be the key of the `trader` signer account.
    pub trader_wallet: Pubkey,
    pub product_program_id: Pubkey,
    /// The `Failed` challenge being reset.
    pub prev_challenge_id: u64,
    /// The fresh challenge ID for the reset attempt. Must be unused.
    pub new_challenge_id: u64,
    /// Index into the product's `reset_price_bps` table (0-based phase the
    /// trader failed in). Sector-supplied and used **only for pricing** -- the
    /// vault cannot see which phase a trader failed in. A wrong value changes
    /// what is charged, never how many payouts remain.
    pub reset_phase: u8,
}

/// Builds a `deposit_reset` instruction.
///
/// # Accounts
/// The vault declares 13 accounts in this order. This crate adds positions 0
/// and 1; positions 2 onward are `remaining_accounts`, which the caller
/// supplies **in exactly this order and with exactly these flags**.
///
/// 0. `[signer]` `sector_authority` -- CPI-auth identity, same as
///    `deposit_fee`.
/// 1. `[writable]` `product_registry` -- the vault declares it `mut`.
/// 2. `[writable]` `prev_trader_state` -- the `Failed` challenge being reset
///    (`remaining_accounts[0]`). Marked reset-used.
/// 3. `[writable]` `new_trader_state` -- the new challenge's `TraderState`.
///    Created by this instruction, so a reused `new_challenge_id` fails.
/// 4. `[signer, writable]` `payer` -- pays rent for `new_trader_state`.
/// 5. `[]` `system_program`.
/// 6. `[]` `vault_state` -- the vault's singleton `VaultState`.
/// 7. `[signer]` `trader` -- the paying trader. Read-only, but **must sign**
///    and must be the `trader_wallet` argument.
/// 8. `[writable]` `trader_token_account` -- the trader's own token account
///    for `mint` (source of the payment).
/// 9. `[]` `mint` -- the USDC or USDT mint (one of the two in `vault_state`).
/// 10. `[writable]` `pool_token_account` -- the vault's payout pool for `mint`.
/// 11. `[writable]` `sl8_token_account` -- a `mint` token account owned by the
///     SL8 wallet in `vault_state`.
/// 12. `[]` `token_program` -- classic SPL Token. Token-2022 is rejected.
///
/// Positions 5-12 are the same accounts, in the same order, as `deposit_fee`
/// positions 4-11. As there, this crate hardcodes none of the vault-side
/// addresses; the vault verifies each against its own seeds and `VaultState`.
pub fn deposit_reset(
    vault_program_id: Pubkey,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: DepositResetArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sector_authority, true),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(DEPOSIT_RESET_DISCRIMINATOR, &args),
    }
}
