use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:request_payout")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const REQUEST_PAYOUT_DISCRIMINATOR: [u8; 8] = [5, 176, 110, 197, 172, 177, 64, 200];

/// Instruction payload for `request_payout`.
///
/// The vault pays the trader **in tokens, directly from its payout pool to the
/// trader's own token account**, in the same instruction; the sector program
/// moves no funds. Payment comes from exactly ONE pool: whichever of the USDC
/// and USDT pools holds the larger balance, USDC on a tie. It is never split
/// across the two pools, never falls back to the smaller one and never sums
/// them. If that pool holds less than `amount`, the instruction fails with the
/// vault's `InsufficientPoolBalance` error and every write reverts.
///
/// Both of the trader's token accounts (USDC **and** USDT) must exist and be
/// passed, even though only one is paid into; a trader missing either cannot
/// be paid.
///
/// # Reading the result
/// For an otherwise-valid call (active product, non-zero amount, `Active`
/// challenge), a stale challenge (past its inactivity window) does **not**
/// produce an error: the vault flips it to `Abandoned`, pays nothing and
/// returns `Ok` with `PayoutOutcome::Abandoned` in return data, because an
/// error would revert the status write. Calls that fail those earlier checks
/// still error. The sector program MUST read the return data
/// (`solana_program::program::get_return_data`, checking it came from the
/// vault program) and see [`crate::PayoutOutcome::Paid`] before telling
/// anyone they were paid. A successful CPI alone does not mean money moved.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct RequestPayoutArgs {
    pub trader_wallet: Pubkey,
    /// Payout requested, in 6-decimal base units of the mint paid from (USDC
    /// or USDT, whichever pool the vault selects). Must be greater than zero.
    pub amount: u64,
    pub product_program_id: Pubkey,
    pub challenge_id: u64,
    /// The sector program's proposed next request ID for this
    /// trader+product+challenge.
    ///
    /// This is a **mutual-agreement check, not a value the vault blindly
    /// trusts**: `setl8-vault` independently tracks its own expected next
    /// request ID for the (trader_wallet, product_program_id, challenge_id)
    /// tuple, and rejects this instruction on mismatch. The sector program
    /// proposes what it believes the next ID is; the vault is the source of
    /// truth and enforces agreement rather than accepting whatever is passed
    /// in. This crate does not implement or encode that check -- it only
    /// documents the expectation for callers.
    pub proposed_request_id: u64,
}

/// Builds a `request_payout` instruction.
///
/// # Accounts
/// The vault declares 11 accounts in this order. This crate adds positions 0
/// and 1; positions 2 onward are `remaining_accounts`, which the caller
/// supplies **in exactly this order and with exactly these flags**.
///
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
///    account for `product_program_id`. Written: the vault counts each
///    accepted payout request in it.
/// 2. `[writable]` `trader_state` -- the challenge's `TraderState`
///    (`remaining_accounts[0]`).
/// 3. `[]` `vault_state` -- the vault's singleton `VaultState`. It is the
///    pools' token authority; the vault signs the payout transfer with its
///    seeds, so no extra signer is needed.
/// 4. `[]` `usdc_mint`.
/// 5. `[]` `usdt_mint`.
/// 6. `[writable]` `usdc_pool` -- the vault's USDC payout pool.
/// 7. `[writable]` `usdt_pool` -- the vault's USDT payout pool.
/// 8. `[writable]` `trader_usdc_account` -- the trader's own USDC account
///    (owner must be `trader_wallet`).
/// 9. `[writable]` `trader_usdt_account` -- the trader's own USDT account
///    (owner must be `trader_wallet`). Required even when USDC is the pool
///    paid from.
/// 10. `[]` `token_program` -- classic SPL Token. Token-2022 is rejected.
///
/// No account here signs except `sector_authority`: the trader is being paid,
/// not paying. This crate hardcodes none of the vault-side addresses (PDAs,
/// pools, mints), by design: they depend on the vault's deployment, and the
/// vault verifies every one against its own seeds and `VaultState`, so a wrong
/// address is rejected rather than silently accepted. The caller supplies
/// them.
pub fn request_payout(
    vault_program_id: Pubkey,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: RequestPayoutArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sector_authority, true),
        AccountMeta::new(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(REQUEST_PAYOUT_DISCRIMINATOR, &args),
    }
}
