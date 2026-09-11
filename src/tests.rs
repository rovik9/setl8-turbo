//! Smoke tests: every builder produces an `Instruction` with the expected
//! discriminator prefix and the documented fixed account count (before
//! `remaining_accounts`). Not a substitute for on-chain integration testing
//! against a real `setl8-vault` once one exists -- just a guard against
//! obvious regressions (wrong account order, discriminator drift, signer
//! flags, etc.) in this crate's own shapes.

use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

use crate::instructions::*;
use crate::types::ChallengeSize;
use crate::{derive_sector_authority, SECTOR_AUTHORITY_SEED};

fn pk() -> Pubkey {
    Pubkey::new_unique()
}

#[test]
fn register_product_shape() {
    let ix = register_product(
        pk(),
        pk(),
        pk(),
        pk(),
        &[],
        RegisterProductArgs {
            product_program_id: pk(),
            fee_split_bps: 500,
            challenge_sizes: vec![ChallengeSize { size: 10_000, cost: 100 }],
            max_payout_count: 3,
        },
    );
    assert_eq!(ix.accounts.len(), 3);
    assert!(ix.accounts[0].is_signer && !ix.accounts[0].is_writable);
    assert!(ix.accounts[1].is_signer && !ix.accounts[1].is_writable);
    assert!(!ix.accounts[2].is_signer && ix.accounts[2].is_writable);
    assert_eq!(&ix.data[..8], &REGISTER_PRODUCT_DISCRIMINATOR);
}

#[test]
fn reactivate_product_shape() {
    let ix = reactivate_product(
        pk(),
        pk(),
        pk(),
        pk(),
        &[],
        ReactivateProductArgs { product_program_id: pk() },
    );
    assert_eq!(ix.accounts.len(), 3);
    assert_eq!(&ix.data[..8], &REACTIVATE_PRODUCT_DISCRIMINATOR);
}

#[test]
fn update_product_config_shape() {
    let ix = update_product_config(
        pk(),
        pk(),
        pk(),
        pk(),
        &[],
        UpdateProductConfigArgs {
            product_program_id: pk(),
            challenge_sizes: vec![ChallengeSize { size: 100_000, cost: 1_000 }],
            fee_split_bps: 750,
            max_payout_count: 5,
        },
    );
    assert_eq!(ix.accounts.len(), 3);
    assert_eq!(&ix.data[..8], &UPDATE_PRODUCT_CONFIG_DISCRIMINATOR);
}

#[test]
fn admin_withdraw_marketing_funds_shape() {
    let ix = admin_withdraw_marketing_funds(
        pk(),
        pk(),
        pk(),
        pk(),
        pk(),
        &[],
        AdminWithdrawMarketingFundsArgs { amount: 1_000_000 },
    );
    assert_eq!(ix.accounts.len(), 4);
    assert_eq!(&ix.data[..8], &ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR);
}

#[test]
fn deposit_fee_shape() {
    let ix = deposit_fee(
        pk(),
        pk(),
        pk(),
        &[],
        DepositFeeArgs { amount: 50, product_program_id: pk(), challenge_id: 1 },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(ix.accounts[0].is_signer, "sector_authority must be a signer");
    assert_eq!(&ix.data[..8], &DEPOSIT_FEE_DISCRIMINATOR);
}

#[test]
fn request_payout_shape() {
    let ix = request_payout(
        pk(),
        pk(),
        pk(),
        &[],
        RequestPayoutArgs {
            trader_wallet: pk(),
            amount: 500,
            product_program_id: pk(),
            challenge_id: 1,
            proposed_request_id: 7,
        },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(ix.accounts[0].is_signer, "sector_authority must be a signer");
    assert_eq!(&ix.data[..8], &REQUEST_PAYOUT_DISCRIMINATOR);
}

#[test]
fn flag_trader_failed_shape() {
    let ix = flag_trader_failed(
        pk(),
        pk(),
        pk(),
        &[],
        FlagTraderFailedArgs { trader_wallet: pk(), product_program_id: pk(), challenge_id: 1 },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(ix.accounts[0].is_signer, "sector_authority must be a signer");
    assert_eq!(&ix.data[..8], &FLAG_TRADER_FAILED_DISCRIMINATOR);
}

#[test]
fn remaining_accounts_are_appended_after_fixed_accounts() {
    let extra = AccountMeta::new(pk(), false);
    let ix = flag_trader_failed(
        pk(),
        pk(),
        pk(),
        std::slice::from_ref(&extra),
        FlagTraderFailedArgs { trader_wallet: pk(), product_program_id: pk(), challenge_id: 1 },
    );
    assert_eq!(ix.accounts.len(), 3);
    assert_eq!(ix.accounts[2], extra);
}

#[test]
fn sector_authority_derivation_is_deterministic_and_program_scoped() {
    let sector_program_id = pk();
    let (authority_a, bump_a) = derive_sector_authority(&sector_program_id);
    let (authority_b, bump_b) = derive_sector_authority(&sector_program_id);
    assert_eq!(authority_a, authority_b);
    assert_eq!(bump_a, bump_b);

    let (expected, expected_bump) =
        Pubkey::find_program_address(&[SECTOR_AUTHORITY_SEED], &sector_program_id);
    assert_eq!(authority_a, expected);
    assert_eq!(bump_a, expected_bump);

    // A different program ID must derive a different authority PDA -- this is
    // what makes the account a proof of *that specific program's* identity.
    let (authority_other, _) = derive_sector_authority(&pk());
    assert_ne!(authority_a, authority_other);
}
