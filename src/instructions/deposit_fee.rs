use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:deposit_fee")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const DEPOSIT_FEE_DISCRIMINATOR: [u8; 8] = [11, 51, 105, 140, 198, 229, 7, 77];

/// Instruction payload for `deposit_fee`.
///
/// Called by a sector program via CPI when a trader buys a challenge. **The
/// trader pays the vault directly**: `amount` base units of USDC or USDT
/// (classic SPL Token, 6 decimals) move from the trader's own token account
/// inside this one instruction. The sector program no longer collects the fee
/// itself and never holds the funds.
///
/// Consequently the trader must **sign the transaction**. The sector's CPI
/// carries the trader's signature through as an account-meta signer, so it
/// only works when that signature is already in the outer transaction; there
/// is no way for a sector program to pay on a trader's behalf.
///
/// # Payment split
/// `pool = floor(amount * fee_split_bps / 10_000)` goes to the vault's payout
/// pool for the **same mint** as the payment, and the exact remainder
/// (`amount - pool`) goes to a token account of that mint owned by the SL8
/// wallet. `fee_split_bps` is the product's value in its `ProductRegistry`.
/// A leg of zero is skipped, not an error. Everything is validated before
/// any state is written, and a failed transfer reverts the whole
/// instruction, so no `TraderState` is created without payment.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct DepositFeeArgs {
    /// Fee being paid, in 6-decimal base units of the mint used. Must equal
    /// the `cost` of the registered `ChallengeSize` whose `size` is
    /// `account_size`, and the trader's token account must hold at least this
    /// much.
    pub amount: u64,
    pub product_program_id: Pubkey,
    pub challenge_id: u64,
    /// Added in v0.3.0 (breaking). The vault keys each `TraderState` by
    /// wallet + product + challenge, so it must be told whose challenge this
    /// is. Appended last: Borsh field order is the wire order, and the vault
    /// handler's parameter order must match. It must also be the key of the
    /// `trader` signer account (the vault rejects any other account).
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
/// The vault declares 12 accounts in this order. This crate adds positions 0
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
///    account for `product_program_id`. The vault declares it `mut`.
/// 2. `[writable]` `trader_state` -- the new challenge's `TraderState`
///    (`remaining_accounts[0]`). Created by this instruction, so a reused
///    `challenge_id` fails.
/// 3. `[signer, writable]` `payer` -- pays rent for `trader_state`. The
///    sector authority PDA holds no lamports, so pass a funded signer.
/// 4. `[]` `system_program`.
/// 5. `[]` `vault_state` -- the vault's singleton `VaultState`.
/// 6. `[signer]` `trader` -- the paying trader. Read-only, but **must sign**
///    and must be the `trader_wallet` argument.
/// 7. `[writable]` `trader_token_account` -- the trader's own token account
///    for `mint` (source of the payment).
/// 8. `[]` `mint` -- the USDC or USDT mint (one of the two in `vault_state`).
/// 9. `[writable]` `pool_token_account` -- the vault's payout pool for `mint`.
/// 10. `[writable]` `sl8_token_account` -- a `mint` token account owned by the
///     SL8 wallet in `vault_state`.
/// 11. `[]` `token_program` -- classic SPL Token. Token-2022 is rejected.
///
/// This crate hardcodes none of the vault-side addresses (PDAs, pools, mints,
/// wallets), by design: they depend on the vault's deployment, and the vault
/// verifies every one of them against its own seeds and `VaultState`, so a
/// wrong address is rejected rather than silently accepted. The caller
/// supplies them.
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
