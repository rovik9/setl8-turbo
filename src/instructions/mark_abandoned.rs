use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use crate::build_instruction_data;

/// `sha256("global:mark_abandoned")[..8]`. See
/// [`crate::build_instruction_data`] for the discriminator-scheme assumption.
pub const MARK_ABANDONED_DISCRIMINATOR: [u8; 8] = [2, 20, 252, 203, 247, 72, 6, 175];

/// Instruction payload for `mark_abandoned`.
///
/// **Permissionless.** Anyone (a keeper bot, a sector program, a curious
/// user) may call this against a challenge whose inactivity window has
/// elapsed, so records nobody ever revisits still end up in a terminal state.
/// The vault rejects the call unless the challenge is `Active` and genuinely
/// past the window, so a caller can never abandon a live challenge.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct MarkAbandonedArgs {
    pub trader_wallet: Pubkey,
    pub product_program_id: Pubkey,
    pub challenge_id: u64,
}

/// Builds a `mark_abandoned` instruction.
///
/// # Accounts
/// 0. `[signer]` `caller` -- any signer. Only pays the transaction fee; has
///    no authority over the outcome.
/// 1. `[]` `product_registry` -- read-only, for pause accounting.
///    2..N `remaining_accounts` -- the challenge's `TraderState` account
///    (writable, first).
pub fn mark_abandoned(
    vault_program_id: Pubkey,
    caller: Pubkey,
    product_registry: Pubkey,
    remaining_accounts: &[AccountMeta],
    args: MarkAbandonedArgs,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(caller, true),
        AccountMeta::new_readonly(product_registry, false),
    ];
    accounts.extend_from_slice(remaining_accounts);

    Instruction {
        program_id: vault_program_id,
        accounts,
        data: build_instruction_data(MARK_ABANDONED_DISCRIMINATOR, &args),
    }
}
