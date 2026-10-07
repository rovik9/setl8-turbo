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
//! 2. **`remaining_accounts: &[AccountMeta]` passthrough.** This crate
//!    intentionally does not know `setl8-vault`'s exact PDA/account layout
//!    (no state structs live here -- see crate root docs). Each builder
//!    documents the accounts it *does* know about (signers, `product_registry`,
//!    the CPI-auth identity account) and appends `remaining_accounts` after
//!    them, so callers can pass whatever vault-specific accounts
//!    (`TraderState`, `BondPosition`, token accounts, `system_program`, etc.)
//!    turn out to be required, without this crate needing a breaking version
//!    bump every time `setl8-vault`'s internal account layout is finalized or
//!    changes.

mod admin_withdraw_marketing_funds;
mod deposit_fee;
mod deposit_reset;
mod flag_trader_failed;
mod mark_abandoned;
mod pause_product;
mod record_activity;
mod reactivate_product;
mod register_product;
mod request_payout;
mod update_product_config;

pub use admin_withdraw_marketing_funds::*;
pub use deposit_fee::*;
pub use deposit_reset::*;
pub use flag_trader_failed::*;
pub use mark_abandoned::*;
pub use pause_product::*;
pub use record_activity::*;
pub use reactivate_product::*;
pub use register_product::*;
pub use request_payout::*;
pub use update_product_config::*;
