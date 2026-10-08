use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:request_payout")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const REQUEST_PAYOUT_DISCRIMINATOR: [u8; 8] = [5, 176, 110, 197, 172, 177, 64, 200];

/// Instruction payload for `request_payout`.
///
/// **This queues a payout; it does not pay it.** An accepted request creates a
/// `PayoutClaim` owed to the trader and moves **no tokens**. The claim is paid
/// later by the vault's permissionless heartbeat, pro rata, with any unpaid
/// remainder carried over to the next cycle; see [`crate::heartbeat`] for how
/// that works. The sector program does not move funds and is not told when a
/// claim is paid.
///
/// # Reading the result
/// The vault reports the outcome through Solana return data, and a successful
/// CPI alone says nothing about it:
/// * [`crate::PayoutOutcome::Paid`] means **accepted and queued**. A claim was
///   created and `amount` is now owed to the trader. No tokens moved.
/// * [`crate::PayoutOutcome::Abandoned`] means the challenge was past its
///   inactivity window. The vault flipped it to `Abandoned` and **created no
///   claim** for this call. (Claims queued by earlier requests stay owed.)
///
/// For an otherwise-valid call (active product, non-zero amount, `Active`
/// challenge), a stale challenge does **not** produce an error: it returns `Ok`
/// with `Abandoned`, because an error would revert the status write. Calls that
/// fail those earlier checks still error. The sector program MUST read the
/// return data (`solana_program::program::get_return_data`, checking it came
/// from the vault program) before telling anyone a payout was queued.
///
/// # Keep the payout tally in step
/// After every request that returns `Paid`, the sector must bump its payout
/// tally by one request and by `amount`, in the same transaction, and must not
/// touch it for `Abandoned`. See [`crate::payout_tally`] for the contract. The
/// vault is specified to pause a product whose records and tally disagree, but
/// as of vault commit `be97396` it does not read the tally yet.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct RequestPayoutArgs {
    pub trader_wallet: Pubkey,
    /// Amount to queue, in 6-decimal dollar units (USDC and USDT are both
    /// worth $1). Must be greater than zero. This is what is **owed**, not
    /// what will be paid in any one cycle: a cycle may pay only a fraction.
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
    ///
    /// It is also part of the `payout_claim` address (see
    /// [`crate::derive_payout_claim`]), so the sector must derive that account
    /// from the same value it passes here.
    pub proposed_request_id: u64,
}

/// Builds a `request_payout` instruction.
///
/// # Accounts
/// The vault declares 7 accounts in this order. This crate adds positions 0
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
/// 3. `[writable]` `vault_state` -- the vault's singleton `VaultState`
///    (`remaining_accounts[1]`). Written: it holds the open-claims counters
///    and the current heartbeat cycle id. (It was read-only before the payout
///    queue; a read-only flag here now fails.)
/// 4. `[writable]` `payout_claim` -- the new claim's PDA
///    (`remaining_accounts[2]`), under the **vault's** program ID with seeds
///    `[PAYOUT_CLAIM_SEED, trader_state, proposed_request_id.to_le_bytes()]`;
///    derive it with [`crate::derive_payout_claim`]. It must not exist yet.
///    The vault creates it only after every check has passed, so an
///    `Abandoned` result leaves nothing behind.
/// 5. `[signer, writable]` `payer` -- pays the claim account's rent
///    (`remaining_accounts[3]`). The sector authority PDA holds no lamports,
///    so pass a funded signer the sector controls (typically the same payer
///    used for `deposit_fee`); the sector's CPI needs this second signer in the
///    outer transaction.
/// 6. `[]` `system_program` (`remaining_accounts[4]`).
///
/// Only `sector_authority` and `payer` sign. This crate hardcodes none of the
/// vault-side addresses (PDAs, wallets), by design: they depend on the vault's
/// deployment, and the vault checks each against its own seeds and
/// `VaultState`, so a wrong address is rejected rather than silently accepted.
/// The caller supplies them.
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
