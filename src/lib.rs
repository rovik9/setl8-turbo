//! `setl8-shared-interfaces`
//!
//! Shared CPI interface definitions for cross-program calls between the Setl8
//! vault program and sector programs (lev-trading, and future
//! options/predictions/etc).
//!
//! This crate defines **instruction shapes only**:
//! - instruction argument structs (the Borsh-serialized data payload)
//! - builder functions that construct a raw
//!   [`solana_program::instruction::Instruction`]
//! - documented account ordering per instruction, including which account
//!   carries the CPI-auth identity check on the vault side
//!
//! It deliberately does **not** contain:
//! - any account/state struct definitions (`ProductRegistry`, `TraderState`,
//!   `PayoutClaim`, ...) -- those live in `setl8-vault`. (`BondPosition` and
//!   `BondCapTracker` are *planned* for the vault's bond module and are not in
//!   the vault source yet.) The one exception is the sector-owned
//!   [`PayoutTally`], whose byte layout is a
//!   cross-program contract and so is defined here
//! - any business logic (floor math, graduation checks, abandonment sweeps)
//! - any protocol numbers (fees, caps, challenge costs)
//!
//! Sector programs and the vault program each depend on this crate as an
//! external versioned dependency, not a copied file, so that a signature
//! change here fails to compile in a consumer that hasn't updated to match.
//!
//! See `README.md` for the full list of open placeholders / assumptions this
//! v0.1.0 scaffold makes, that need confirmation before `setl8-vault` starts
//! depending on this crate for real.

pub mod heartbeat;
pub mod instructions;
pub mod payout_tally;
pub mod types;

#[cfg(test)]
mod tests;

pub use instructions::*;
pub use payout_tally::{
    derive_payout_tally, PayoutTally, TallyError, PAYOUT_TALLY_MAGIC, PAYOUT_TALLY_MIN_LEN,
    PAYOUT_TALLY_SEED, PAYOUT_TALLY_VERSION,
};
pub use types::{ActivityOutcome, ChallengeSize, PayoutOutcome};

use borsh::BorshSerialize;
use solana_program::pubkey::Pubkey;

/// Length in bytes of the Anchor-style instruction discriminator used by
/// every instruction in this crate.
pub const DISCRIMINATOR_LEN: usize = 8;

/// Seed for every sector program's `sector_authority` PDA -- the account that
/// proves CPI-caller identity to `setl8-vault` on every CPI-auth-context
/// instruction (`deposit_fee`, `request_payout`, `flag_trader_failed`).
///
/// Each sector program derives its own authority PDA as
/// `Pubkey::find_program_address(&[SECTOR_AUTHORITY_SEED], &sector_program_id)`
/// (see [`derive_sector_authority`]) -- under **its own** program ID, not the
/// vault's. It signs its CPI into the vault via `invoke_signed` using that
/// PDA's seeds. `setl8-vault` authenticates the caller by checking that the
/// signer account's key equals
/// `find_program_address(&[SECTOR_AUTHORITY_SEED], &registry.product_program_id)`
/// for the `product_program_id` on the instruction. Only the real calling
/// program can produce a valid `invoke_signed` signature for a PDA derived
/// from its own program ID, so this is cryptographic proof of caller
/// identity, not a self-reported claim.
///
/// This replaces an earlier v0.1.0 draft of this crate that assumed
/// Instructions-sysvar introspection for this purpose -- that mechanism
/// doesn't actually prove CPI-caller identity (the sysvar is for inspecting
/// sibling top-level instructions, not the immediate CPI caller), so it's
/// gone entirely. PDA-signer verification is the correct mechanism and is
/// what every CPI-auth-context builder in this crate now uses.
pub const SECTOR_AUTHORITY_SEED: &[u8] = b"setl8_sector_authority";

/// Derives a sector program's `sector_authority` PDA under its own program
/// ID, using [`SECTOR_AUTHORITY_SEED`]. Returns `(pubkey, bump)`.
///
/// A sector program calls this (or the equivalent `create_program_address`
/// with a cached bump, for efficiency) to get the account it must pass as
/// `sector_authority` to `deposit_fee`/`request_payout`/`flag_trader_failed`,
/// and to sign its `invoke_signed` CPI into the vault with
/// `&[SECTOR_AUTHORITY_SEED, &[bump]]`.
pub fn derive_sector_authority(sector_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SECTOR_AUTHORITY_SEED], sector_program_id)
}

/// Seed of a `PayoutClaim` PDA: `[PAYOUT_CLAIM_SEED, trader_state,
/// request_id.to_le_bytes()]` under the **vault's** program ID. One claim is
/// created per `request_payout` that returns `PayoutOutcome::Paid`
/// ("accepted and queued"); a later heartbeat cycle pays it, fully or in part.
/// See [`derive_payout_claim`].
pub const PAYOUT_CLAIM_SEED: &[u8] = b"payout_claim";

/// Derives the `PayoutClaim` PDA for one accepted `request_payout`. Returns
/// `(pubkey, bump)`.
///
/// `trader_state` is the challenge's `TraderState` account and `request_id` is
/// the `proposed_request_id` the vault accepted (the challenge's
/// `payout_count` after the request). A sector program needs this address to
/// pass the `payout_claim` account in `request_payout`'s `remaining_accounts`.
///
/// Unlike [`derive_sector_authority`] this is derived under the **vault's**
/// program ID, which this crate deliberately does not hardcode (see README),
/// so the caller supplies it.
pub fn derive_payout_claim(
    vault_program_id: &Pubkey,
    trader_state: &Pubkey,
    request_id: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PAYOUT_CLAIM_SEED,
            trader_state.as_ref(),
            &request_id.to_le_bytes(),
        ],
        vault_program_id,
    )
}

/// Concatenates an Anchor-style 8-byte discriminator with the Borsh-serialized
/// instruction args to form raw instruction data.
///
/// **Flagged assumption**: discriminators in this crate are precomputed as
/// `sha256("global:<instruction_name>")[..8]`, matching Anchor's default
/// instruction discriminator convention. This is a reasonable default because
/// the real `setl8-vault` repo (checked while scaffolding this crate) depends
/// on `anchor-lang 0.32.1`, but it is still an assumption about how that
/// program's `#[program]` module is written -- Anchor does allow overriding
/// this convention. If `setl8-vault` doesn't use Anchor's standard
/// discriminator, every `*_DISCRIMINATOR` constant in `src/instructions/*.rs`
/// needs to be regenerated to match.
pub fn build_instruction_data<T: BorshSerialize>(
    discriminator: [u8; DISCRIMINATOR_LEN],
    args: &T,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(DISCRIMINATOR_LEN + 64);
    data.extend_from_slice(&discriminator);
    args.serialize(&mut data).expect(
        "Borsh serialization of instruction args is infallible for the types in this crate",
    );
    data
}
