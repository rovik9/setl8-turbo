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
//!   `BondPosition`, `BondCapTracker`, ...) -- those live in `setl8-vault`
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

pub mod instructions;
pub mod types;

#[cfg(test)]
mod tests;

pub use instructions::*;
pub use types::ChallengeSize;

use borsh::BorshSerialize;

/// Length in bytes of the Anchor-style instruction discriminator used by
/// every instruction in this crate.
pub const DISCRIMINATOR_LEN: usize = 8;

/// The Instructions sysvar ID, re-exported here because it's expected to be
/// the CPI-auth identity account on every CPI-auth-context instruction
/// (`deposit_fee`, `request_payout`, `flag_trader_failed`).
///
/// **Flagged assumption**: this crate assumes `setl8-vault` authenticates its
/// direct caller via Solana's instruction-introspection pattern (reading the
/// calling program's ID off this sysvar and checking it against
/// `ProductRegistry.product_program_id`), rather than e.g. a signer-PDA-based
/// identity scheme. This was not specified in the original request and needs
/// confirmation once `setl8-vault`'s CPI-auth check is actually implemented.
/// If the mechanism differs, every CPI-auth-context builder function in
/// `src/instructions/` needs its `calling_program_identity` account
/// reconsidered.
pub use solana_program::sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID;

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
    args.serialize(&mut data)
        .expect("Borsh serialization of instruction args is infallible for the types in this crate");
    data
}
