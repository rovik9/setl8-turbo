//! Smoke tests: every builder produces an `Instruction` with the expected
//! discriminator prefix and the documented fixed account count (before
//! `remaining_accounts`). Not a substitute for on-chain integration testing
//! against a real `setl8-vault` once one exists -- just a guard against
//! obvious regressions (wrong account order, discriminator drift, signer
//! flags, etc.) in this crate's own shapes.

use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

use crate::instructions::*;
use crate::types::{ActivityOutcome, ChallengeSize, PayoutOutcome};
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
            challenge_sizes: vec![ChallengeSize {
                size: 10_000,
                cost: 100,
            }],
            max_payout_count: 3,
            reset_price_bps: vec![100, 150, 200, 300, 450],
        },
    );
    assert_eq!(ix.accounts.len(), 3);
    assert!(ix.accounts[0].is_signer && ix.accounts[0].is_writable);
    assert!(ix.accounts[1].is_signer && !ix.accounts[1].is_writable);
    assert!(!ix.accounts[2].is_signer && ix.accounts[2].is_writable);
    assert_eq!(&ix.data[..8], &REGISTER_PRODUCT_DISCRIMINATOR);
}

/// The vault makes `sl8_admin` the `init` payer of the registry PDA, so the
/// builder must mark it writable; `rov_admin` only co-signs. Regression test
/// for v0.3.0, which had `sl8_admin` read-only (worked only when `sl8_admin`
/// was also the fee payer).
#[test]
fn register_product_sl8_admin_is_writable_signer_and_rov_admin_is_readonly_signer() {
    let (sl8, rov, registry) = (pk(), pk(), pk());
    let ix = register_product(
        pk(),
        sl8,
        rov,
        registry,
        &[],
        RegisterProductArgs {
            product_program_id: pk(),
            fee_split_bps: 500,
            challenge_sizes: vec![],
            max_payout_count: 3,
            reset_price_bps: vec![],
        },
    );
    assert_eq!(ix.accounts[0].pubkey, sl8);
    assert!(ix.accounts[0].is_signer, "sl8_admin must sign");
    assert!(
        ix.accounts[0].is_writable,
        "sl8_admin pays registry rent: must be writable"
    );
    assert_eq!(ix.accounts[1].pubkey, rov);
    assert!(ix.accounts[1].is_signer, "rov_admin must sign");
    assert!(
        !ix.accounts[1].is_writable,
        "rov_admin is never a fund destination: read-only"
    );
    assert_eq!(ix.accounts[2].pubkey, registry);
}

#[test]
fn reactivate_product_shape() {
    let ix = reactivate_product(
        pk(),
        pk(),
        pk(),
        pk(),
        &[],
        ReactivateProductArgs {
            product_program_id: pk(),
        },
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
            challenge_sizes: vec![ChallengeSize {
                size: 100_000,
                cost: 1_000,
            }],
            fee_split_bps: 750,
            max_payout_count: 5,
            reset_price_bps: vec![],
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
        DepositFeeArgs {
            amount: 50,
            product_program_id: pk(),
            challenge_id: 1,
            trader_wallet: pk(),
            account_size: 2_500,
        },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(
        ix.accounts[0].is_signer,
        "sector_authority must be a signer"
    );
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
    assert!(
        ix.accounts[0].is_signer,
        "sector_authority must be a signer"
    );
    assert_eq!(&ix.data[..8], &REQUEST_PAYOUT_DISCRIMINATOR);
}

#[test]
fn flag_trader_failed_shape() {
    let ix = flag_trader_failed(
        pk(),
        pk(),
        pk(),
        &[],
        FlagTraderFailedArgs {
            trader_wallet: pk(),
            product_program_id: pk(),
            challenge_id: 1,
        },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(
        ix.accounts[0].is_signer,
        "sector_authority must be a signer"
    );
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
        FlagTraderFailedArgs {
            trader_wallet: pk(),
            product_program_id: pk(),
            challenge_id: 1,
        },
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

#[test]
fn record_activity_shape() {
    let ix = record_activity(
        pk(),
        pk(),
        pk(),
        &[],
        RecordActivityArgs {
            trader_wallet: pk(),
            product_program_id: pk(),
            challenge_id: 9,
        },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(
        ix.accounts[0].is_signer && !ix.accounts[0].is_writable,
        "sector_authority signs, read-only"
    );
    assert!(
        !ix.accounts[1].is_writable,
        "registry is read-only for record_activity"
    );
    assert_eq!(&ix.data[..8], &RECORD_ACTIVITY_DISCRIMINATOR);
}

#[test]
fn mark_abandoned_shape() {
    let ix = mark_abandoned(
        pk(),
        pk(),
        pk(),
        &[],
        MarkAbandonedArgs {
            trader_wallet: pk(),
            product_program_id: pk(),
            challenge_id: 9,
        },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(ix.accounts[0].is_signer, "caller must sign (fee payer)");
    assert!(!ix.accounts[1].is_writable);
    assert_eq!(&ix.data[..8], &MARK_ABANDONED_DISCRIMINATOR);
}

#[test]
fn deposit_reset_shape() {
    let ix = deposit_reset(
        pk(),
        pk(),
        pk(),
        &[],
        DepositResetArgs {
            amount: 75,
            trader_wallet: pk(),
            product_program_id: pk(),
            prev_challenge_id: 4,
            new_challenge_id: 5,
            reset_phase: 2,
        },
    );
    assert_eq!(ix.accounts.len(), 2);
    assert!(
        ix.accounts[0].is_signer,
        "sector_authority must be a signer"
    );
    assert_eq!(&ix.data[..8], &DEPOSIT_RESET_DISCRIMINATOR);
}

#[test]
fn pause_product_shape() {
    let ix = pause_product(
        pk(),
        pk(),
        pk(),
        pk(),
        &[],
        PauseProductArgs {
            product_program_id: pk(),
        },
    );
    assert_eq!(ix.accounts.len(), 3);
    assert!(
        ix.accounts[0].is_signer && ix.accounts[1].is_signer,
        "both admins must sign"
    );
    assert!(ix.accounts[2].is_writable);
    assert_eq!(&ix.data[..8], &PAUSE_PRODUCT_DISCRIMINATOR);
}

#[test]
fn register_product_args_round_trip_with_reset_table() {
    use borsh::{BorshDeserialize, BorshSerialize};
    let args = RegisterProductArgs {
        product_program_id: pk(),
        fee_split_bps: 6500,
        challenge_sizes: vec![ChallengeSize {
            size: 2_500,
            cost: 30,
        }],
        max_payout_count: 5,
        reset_price_bps: vec![100, 125, 150, 200, 300, 450],
    };
    let mut buf = Vec::new();
    args.serialize(&mut buf).unwrap();
    assert_eq!(RegisterProductArgs::try_from_slice(&buf).unwrap(), args);
}

#[test]
fn outcome_enums_round_trip_and_reject_unknown() {
    for o in [
        ActivityOutcome::Recorded,
        ActivityOutcome::Throttled,
        ActivityOutcome::Abandoned,
    ] {
        assert_eq!(ActivityOutcome::from_u8(o as u8), Some(o));
    }
    for o in [PayoutOutcome::Paid, PayoutOutcome::Abandoned] {
        assert_eq!(PayoutOutcome::from_u8(o as u8), Some(o));
    }
    assert_eq!(ActivityOutcome::from_u8(3), None);
    assert_eq!(PayoutOutcome::from_u8(2), None);
}

/// Guards against a typo in any hardcoded discriminator: recompute
/// `sha256("global:<name>")[..8]` and compare. Uses the sha2-free check below
/// via a tiny known-answer table generated from the same rule.
#[test]
fn new_discriminators_match_anchor_rule_known_answers() {
    assert_eq!(
        RECORD_ACTIVITY_DISCRIMINATOR,
        [199, 86, 104, 65, 200, 211, 71, 50]
    );
    assert_eq!(
        MARK_ABANDONED_DISCRIMINATOR,
        [2, 20, 252, 203, 247, 72, 6, 175]
    );
    assert_eq!(
        DEPOSIT_RESET_DISCRIMINATOR,
        [25, 27, 129, 85, 180, 120, 189, 155]
    );
    assert_eq!(
        PAUSE_PRODUCT_DISCRIMINATOR,
        [146, 44, 126, 129, 251, 223, 185, 185]
    );
}

// ---------------------------------------------------------------------------
// Documented token-movement account layouts (v0.3.2).
//
// Each table below mirrors the `# Accounts` section of the builder's doc
// comment, which was checked against the vault's `Accounts` structs. The
// tests pin that a caller who passes the documented `remaining_accounts` in
// order gets exactly `[fixed accounts..., remaining in order]` back, with the
// fixed accounts' flags as documented and the remaining flags untouched.
// ---------------------------------------------------------------------------

/// `(name, is_signer, is_writable)` for one documented remaining account.
type Spec = (&'static str, bool, bool);

/// Fresh metas for a documented layout, with the caller-chosen flags.
fn remaining_from(spec: &[Spec]) -> Vec<AccountMeta> {
    spec.iter()
        .map(|&(_, signer, writable)| {
            if writable {
                AccountMeta::new(pk(), signer)
            } else {
                AccountMeta::new_readonly(pk(), signer)
            }
        })
        .collect()
}

/// Asserts the full account list: the two fixed accounts first with their
/// documented flags, then every remaining account unchanged and in order.
fn assert_layout(
    ix: &solana_program::instruction::Instruction,
    sector_authority: Pubkey,
    product_registry: Pubkey,
    spec: &[Spec],
    remaining: &[AccountMeta],
) {
    assert_eq!(remaining.len(), spec.len());
    assert_eq!(ix.accounts.len(), 2 + spec.len());

    assert_eq!(ix.accounts[0].pubkey, sector_authority);
    assert!(ix.accounts[0].is_signer, "sector_authority must sign");
    assert!(!ix.accounts[0].is_writable, "sector_authority is read-only");
    assert_eq!(ix.accounts[1].pubkey, product_registry);
    assert!(!ix.accounts[1].is_signer, "product_registry never signs");
    assert!(ix.accounts[1].is_writable, "product_registry is writable");

    for (i, (name, signer, writable)) in spec.iter().enumerate() {
        let got = &ix.accounts[2 + i];
        assert_eq!(got, &remaining[i], "{name} at position {} moved", 2 + i);
        assert_eq!(got.is_signer, *signer, "{name}: signer flag");
        assert_eq!(got.is_writable, *writable, "{name}: writable flag");
    }
}

const DEPOSIT_FEE_REMAINING: [Spec; 10] = [
    ("trader_state", false, true),
    ("payer", true, true),
    ("system_program", false, false),
    ("vault_state", false, false),
    ("trader", true, false),
    ("trader_token_account", false, true),
    ("mint", false, false),
    ("pool_token_account", false, true),
    ("sl8_token_account", false, true),
    ("token_program", false, false),
];

const DEPOSIT_RESET_REMAINING: [Spec; 11] = [
    ("prev_trader_state", false, true),
    ("new_trader_state", false, true),
    ("payer", true, true),
    ("system_program", false, false),
    ("vault_state", false, false),
    ("trader", true, false),
    ("trader_token_account", false, true),
    ("mint", false, false),
    ("pool_token_account", false, true),
    ("sl8_token_account", false, true),
    ("token_program", false, false),
];

const REQUEST_PAYOUT_REMAINING: [Spec; 9] = [
    ("trader_state", false, true),
    ("vault_state", false, false),
    ("usdc_mint", false, false),
    ("usdt_mint", false, false),
    ("usdc_pool", false, true),
    ("usdt_pool", false, true),
    ("trader_usdc_account", false, true),
    ("trader_usdt_account", false, true),
    ("token_program", false, false),
];

#[test]
fn deposit_fee_documented_layout() {
    let (vault, authority, registry) = (pk(), pk(), pk());
    let remaining = remaining_from(&DEPOSIT_FEE_REMAINING);
    let ix = deposit_fee(
        vault,
        authority,
        registry,
        &remaining,
        DepositFeeArgs {
            amount: 30_000_000,
            product_program_id: pk(),
            challenge_id: 1,
            trader_wallet: pk(),
            account_size: 2_500_000_000,
        },
    );
    assert_eq!(ix.program_id, vault);
    assert_eq!(ix.accounts.len(), 12);
    assert_layout(&ix, authority, registry, &DEPOSIT_FEE_REMAINING, &remaining);
    // The trader is a read-only signer: it authorizes the payment, it is not
    // a destination. Exactly three signers in total.
    assert_eq!(ix.accounts.iter().filter(|a| a.is_signer).count(), 3);
    assert_eq!(&ix.data[..8], &DEPOSIT_FEE_DISCRIMINATOR);
}

#[test]
fn deposit_reset_documented_layout() {
    let (vault, authority, registry) = (pk(), pk(), pk());
    let remaining = remaining_from(&DEPOSIT_RESET_REMAINING);
    let ix = deposit_reset(
        vault,
        authority,
        registry,
        &remaining,
        DepositResetArgs {
            amount: 25_000_000,
            trader_wallet: pk(),
            product_program_id: pk(),
            prev_challenge_id: 1,
            new_challenge_id: 2,
            reset_phase: 0,
        },
    );
    assert_eq!(ix.program_id, vault);
    assert_eq!(ix.accounts.len(), 13);
    assert_layout(
        &ix,
        authority,
        registry,
        &DEPOSIT_RESET_REMAINING,
        &remaining,
    );
    assert_eq!(ix.accounts.iter().filter(|a| a.is_signer).count(), 3);
    assert_eq!(&ix.data[..8], &DEPOSIT_RESET_DISCRIMINATOR);
}

#[test]
fn request_payout_documented_layout() {
    let (vault, authority, registry) = (pk(), pk(), pk());
    let remaining = remaining_from(&REQUEST_PAYOUT_REMAINING);
    let ix = request_payout(
        vault,
        authority,
        registry,
        &remaining,
        RequestPayoutArgs {
            trader_wallet: pk(),
            amount: 500_000_000,
            product_program_id: pk(),
            challenge_id: 1,
            proposed_request_id: 1,
        },
    );
    assert_eq!(ix.program_id, vault);
    assert_eq!(ix.accounts.len(), 11);
    assert_layout(
        &ix,
        authority,
        registry,
        &REQUEST_PAYOUT_REMAINING,
        &remaining,
    );
    // The trader is being paid, not paying: only the sector authority signs.
    assert_eq!(ix.accounts.iter().filter(|a| a.is_signer).count(), 1);
    assert_eq!(&ix.data[..8], &REQUEST_PAYOUT_DISCRIMINATOR);
}

/// The builders append `remaining_accounts` verbatim: a caller's flags win,
/// even where they differ from the documented ones. (The vault, not this
/// crate, rejects a wrong signer/writable combination.)
#[test]
fn remaining_flags_are_preserved_exactly_as_given() {
    let odd = vec![
        AccountMeta::new(pk(), true),
        AccountMeta::new_readonly(pk(), true),
        AccountMeta::new(pk(), false),
        AccountMeta::new_readonly(pk(), false),
    ];
    let fee = deposit_fee(
        pk(),
        pk(),
        pk(),
        &odd,
        DepositFeeArgs {
            amount: 1,
            product_program_id: pk(),
            challenge_id: 1,
            trader_wallet: pk(),
            account_size: 1,
        },
    );
    let reset = deposit_reset(
        pk(),
        pk(),
        pk(),
        &odd,
        DepositResetArgs {
            amount: 1,
            trader_wallet: pk(),
            product_program_id: pk(),
            prev_challenge_id: 1,
            new_challenge_id: 2,
            reset_phase: 0,
        },
    );
    let payout = request_payout(
        pk(),
        pk(),
        pk(),
        &odd,
        RequestPayoutArgs {
            trader_wallet: pk(),
            amount: 1,
            product_program_id: pk(),
            challenge_id: 1,
            proposed_request_id: 1,
        },
    );
    for ix in [&fee, &reset, &payout] {
        assert_eq!(&ix.accounts[2..], &odd[..]);
    }
}
