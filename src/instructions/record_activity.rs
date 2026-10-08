use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:record_activity")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const RECORD_ACTIVITY_DISCRIMINATOR: [u8; 8] = [199, 86, 104, 65, 200, 211, 71, 50];

/// Instruction payload for `record_activity`.
///
/// Called by a sector program whenever a trader does something that counts as
/// activity (any order action, or a phase pass) so the vault can refresh the
/// challenge's `last_activity_timestamp`. The vault uses that timestamp for
/// its inactivity (abandonment) rule; this crate encodes no timing numbers.
///
/// The vault throttles repeat calls on its own side, so a sector program may
/// call this on every order action without worrying about cost or abuse --
/// but throttling client-side as well saves a CPI.
///
/// The vault returns an [`crate::ActivityOutcome`] via Solana return data.
/// A sector program MUST read it (`solana_program::program::get_return_data`)
/// and stop treating the challenge as live when it is `Abandoned`.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct RecordActivityArgs {
    pub trader_wallet: Pubkey,
    pub product_program_id: Pubkey,
    pub challenge_id: u64,
}

/// Builds a `record_activity` instruction.
///
/// # Accounts
/// 0. `[signer]` `sector_authority` -- CPI-auth identity, same as
///    `deposit_fee` (PDA of the calling sector program, signed via
///    `invoke_signed`).
/// 1. `[]` `product_registry` -- read-only: the vault reads the product's
///    pause accounting so the inactivity clock can freeze during pauses.
/// 2. `[writable]` `trader_state` (`remaining_accounts[0]`) -- the
///    challenge's `TraderState`. The vault declares exactly these 3 accounts.
pub fn record_activity(
    vault_program_id: Pubkey,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: RecordActivityArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(sector_authority, true),
        AccountMeta::new_readonly(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(RECORD_ACTIVITY_DISCRIMINATOR, &args),
    }
}
