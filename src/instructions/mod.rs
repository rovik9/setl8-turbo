//! One module per `setl8-vault` instruction a sector program (or the SL8/Rov
//! admin multisig) invokes via CPI. Each module exports:
//! - a Borsh-derived `*Args` struct (the instruction payload)
//! - a `*_DISCRIMINATOR` constant (see [`crate::build_instruction_data`] docs
//!   for the Anchor-discriminator assumption this depends on)
//! - a builder function returning a raw
//!   [`solana_program::instruction::Instruction`]
//!
//! Two design choices apply to every builder function here, both flagged as
//! placeholders in `README.md`:
//!
//! 1. **No hardcoded `VAULT_PROGRAM_ID`.** `setl8-vault` isn't deployed yet,
//!    so every builder takes `vault_program_id: Pubkey` as an explicit
//!    parameter instead. TODO(vault-deploy): once `setl8-vault` has a real
//!    deployed address, consider adding a `pub const VAULT_PROGRAM_ID: Pubkey`
//!    to this crate for convenience -- until then, callers must supply it.
//!
//! 2. **`remaining_accounts: &[AccountMeta]` passthrough.** This crate does
//!    not derive or hardcode `setl8-vault`'s PDAs, pool/mint addresses or
//!    wallets (no state structs live here -- see crate root docs), so each
//!    builder fixes only the two accounts it can know (`sector_authority`
//!    and `product_registry`) and appends the caller-supplied
//!    `remaining_accounts` slice after them. For the token-moving
//!    instructions the vault's current layout is documented: `deposit_fee`,
//!    `deposit_reset` and `request_payout` list the **exact position and
//!    flags** of every account they expect, and the caller must pass them in
//!    that order. The vault validates them with its own constraints (seeds,
//!    `VaultState` fields, owner/mint checks, program ids), so a wrong
//!    address is rejected rather than silently accepted. If the vault appends
//!    accounts later, the docs here must be updated to match; keeping the
//!    slice open means that needs no breaking version bump.

mod admin_withdraw_marketing_funds;
mod deposit_fee;
mod deposit_reset;
mod flag_trader_failed;
mod mark_abandoned;
mod pause_product;
mod reactivate_product;
mod record_activity;
mod register_product;
mod request_payout;
mod update_product_config;

pub use admin_withdraw_marketing_funds::*;
pub use deposit_fee::*;
pub use deposit_reset::*;
pub use flag_trader_failed::*;
pub use mark_abandoned::*;
pub use pause_product::*;
pub use reactivate_product::*;
pub use record_activity::*;
pub use register_product::*;
pub use request_payout::*;
pub use update_product_config::*;
