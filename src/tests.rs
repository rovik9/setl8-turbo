//! Smoke tests: every builder produces an `Instruction` with the expected
//! discriminator prefix and the documented fixed account count (before
//! `remaining_accounts`; `admin_withdraw_marketing_funds` has none). Not a substitute for on-chain integration testing
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
    // Rewritten for the v0.4.1 builder (the old signature was removed because it
    // did not match the vault). Stricter than before: it pins the account
    // count, every signer/writable flag, the discriminator and the data length.
    let ix = admin_withdraw_marketing_funds(
        pk(),
        pk(),
        pk(),
        pk(),
        pk(),
        pk(),
        pk(),
        pk(),
        AdminWithdrawMarketingFundsArgs {
            pool: PoolSide::Usdc,
            amount: 1_000_000,
        },
    );
    assert_eq!(ix.accounts.len(), 7);
    assert!(ix.accounts[0].is_signer && !ix.accounts[0].is_writable);
    assert!(ix.accounts[1].is_signer && !ix.accounts[1].is_writable);
    assert!(!ix.accounts[2].is_signer && ix.accounts[2].is_writable);
    assert_eq!(ix.data.len(), 17);
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

// NOTE (v0.4.0): this table is the v0.3.2 *instant-payout* layout. The vault now
// queues payouts and the live layout is `QUEUED_PAYOUT_REMAINING` below. This
// test is kept unchanged as a generic passthrough regression: it proves the
// builder appends whatever `remaining_accounts` it is given, in order, with the
// flags given.
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

// ---------------------------------------------------------------------------
// v0.4.0: queued `request_payout`, `flag_trader_failed` flags, payout tally.
// ---------------------------------------------------------------------------

use std::str::FromStr;

use crate::{
    derive_payout_claim, derive_payout_tally, PayoutTally, TallyError, PAYOUT_CLAIM_SEED,
    PAYOUT_TALLY_MAGIC, PAYOUT_TALLY_MIN_LEN, PAYOUT_TALLY_SEED, PAYOUT_TALLY_VERSION,
};

fn key(s: &str) -> Pubkey {
    Pubkey::from_str(s).expect("valid base58 pubkey in test vector")
}

/// The vault's queued `request_payout` layout (vault be97396): positions 2..6.
/// Names follow the vault's `RequestPayout` struct.
const QUEUED_PAYOUT_REMAINING: [Spec; 5] = [
    ("trader_state", false, true),
    ("vault_state", false, true),
    ("payout_claim", false, true),
    ("payer", true, true),
    ("system_program", false, false),
];

// Note: this checks that the builder passes the documented `remaining_accounts`
// through unchanged and that `derive_payout_claim` yields the address documented
// at position 4. On its own it cannot detect later vault drift; that is covered
// by the `vault_source_*` tests below, which parse the vault's source when it is
// present next to this repo.
#[test]
fn request_payout_queued_layout_round_trips_through_the_builder() {
    let (vault, authority, registry) = (pk(), pk(), pk());
    let trader_state = pk();
    let request_id = 7u64;
    let (claim, _) = derive_payout_claim(&vault, &trader_state, request_id);

    // The documented remaining accounts, with the real claim address in place.
    let remaining = vec![
        AccountMeta::new(trader_state, false),
        AccountMeta::new(pk(), false), // vault_state
        AccountMeta::new(claim, false),
        AccountMeta::new(pk(), true),           // payer
        AccountMeta::new_readonly(pk(), false), // system_program
    ];
    let ix = request_payout(
        vault,
        authority,
        registry,
        &remaining,
        RequestPayoutArgs {
            trader_wallet: pk(),
            amount: 1_500_000_000,
            product_program_id: pk(),
            challenge_id: 1,
            proposed_request_id: request_id,
        },
    );

    assert_eq!(ix.program_id, vault);
    assert_eq!(ix.accounts.len(), 7, "the vault declares 7 accounts");
    assert_layout(
        &ix,
        authority,
        registry,
        &QUEUED_PAYOUT_REMAINING,
        &remaining,
    );
    // Position by position, as documented on the builder.
    assert_eq!(ix.accounts[2].pubkey, trader_state);
    assert_eq!(ix.accounts[4].pubkey, claim, "payout_claim is position 4");
    assert!(ix.accounts[3].is_writable, "vault_state must be writable");
    assert!(ix.accounts[5].is_signer && ix.accounts[5].is_writable);
    // Only the authority and the payer sign; no token accounts are involved.
    assert_eq!(ix.accounts.iter().filter(|a| a.is_signer).count(), 2);
    assert_eq!(&ix.data[..8], &REQUEST_PAYOUT_DISCRIMINATOR);
}

/// The vault declares `product_registry` read-only for `flag_trader_failed`
/// (it only reads `product_program_id`), so the builder must not ask for write
/// access. Pins vault commit be97396's `FlagTraderFailed` struct.
#[test]
fn flag_trader_failed_registry_is_readonly_and_trader_state_is_remaining_zero() {
    let (authority, registry, trader_state) = (pk(), pk(), pk());
    let remaining = [AccountMeta::new(trader_state, false)];
    let ix = flag_trader_failed(
        pk(),
        authority,
        registry,
        &remaining,
        FlagTraderFailedArgs {
            trader_wallet: pk(),
            product_program_id: pk(),
            challenge_id: 1,
        },
    );
    assert_eq!(ix.accounts.len(), 3);
    assert_eq!(ix.accounts[0].pubkey, authority);
    assert!(ix.accounts[0].is_signer && !ix.accounts[0].is_writable);
    assert_eq!(ix.accounts[1].pubkey, registry);
    assert!(!ix.accounts[1].is_signer, "product_registry never signs");
    assert!(
        !ix.accounts[1].is_writable,
        "the vault declares product_registry read-only for flag_trader_failed"
    );
    assert_eq!(ix.accounts[2], remaining[0]);
    assert!(ix.accounts[2].is_writable, "trader_state is written");
}

// ---- PDA golden vectors -----------------------------------------------------
// Computed independently (Python: sha256 + ed25519 off-curve check), not with
// this crate. Program id = [1; 32], trader_state = [2; 32].

fn program_1() -> Pubkey {
    Pubkey::new_from_array([1u8; 32])
}

#[test]
fn seeds_and_constants_are_pinned() {
    assert_eq!(PAYOUT_TALLY_SEED, b"payout_tally");
    assert_eq!(PAYOUT_CLAIM_SEED, b"payout_claim");
    assert_eq!(PAYOUT_TALLY_MAGIC, *b"SL8TALLY");
    assert_eq!(PAYOUT_TALLY_VERSION, 1);
    assert_eq!(PAYOUT_TALLY_MIN_LEN, 25);
}

#[test]
fn derive_payout_tally_golden_vector() {
    assert_eq!(
        program_1().to_string(),
        "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
    );
    let (addr, bump) = derive_payout_tally(&program_1());
    assert_eq!(addr, key("PQUhGK78d1U84ievoN3BsjjxAiq6YZFdhQ2xADyyFDM"));
    assert_eq!(bump, 254);

    // Scoped to the program and distinct from the other crate PDAs.
    assert_ne!(derive_payout_tally(&pk()).0, addr);
    assert_ne!(derive_sector_authority(&program_1()).0, addr);
    assert_eq!(
        derive_sector_authority(&program_1()),
        (key("CsV4dMpH3EUdnWKx8QKXQcmEwVruve8BqQugttjzw6bV"), 254)
    );
}

#[test]
fn derive_payout_claim_golden_vectors() {
    let trader_state = Pubkey::new_from_array([2u8; 32]);
    assert_eq!(
        trader_state.to_string(),
        "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR"
    );
    assert_eq!(
        derive_payout_claim(&program_1(), &trader_state, 1),
        (key("GtPHtkQzSL9AhEdNaWoeRGtXQJ5GgYuLofW5qSUxHBWA"), 254)
    );
    assert_eq!(
        derive_payout_claim(&program_1(), &trader_state, u64::MAX),
        (key("D5aBhVKwzdoa9zRnn9mAx7Zb8EVcwzrEUo8hj8y9dWnL"), 255)
    );

    // Every input matters.
    let base = derive_payout_claim(&program_1(), &trader_state, 1).0;
    assert_ne!(derive_payout_claim(&program_1(), &trader_state, 2).0, base);
    assert_ne!(derive_payout_claim(&program_1(), &pk(), 1).0, base);
    assert_ne!(derive_payout_claim(&pk(), &trader_state, 1).0, base);
}

// ---- PayoutTally ---------------------------------------------------------

fn tally_bytes(count: u64, total: u64) -> [u8; 25] {
    let mut b = [0u8; 25];
    b[..8].copy_from_slice(b"SL8TALLY");
    b[8] = 1;
    b[9..17].copy_from_slice(&count.to_le_bytes());
    b[17..25].copy_from_slice(&total.to_le_bytes());
    b
}

#[test]
fn tally_golden_vector_exact_25_bytes() {
    // Asymmetric, byte-distinct values so any swap/offset/endianness slip shows.
    let t = PayoutTally {
        requested_count: 0x0102_0304_0506_0708,
        requested_total: 0x1112_1314_1516_1718,
    };
    let expected: [u8; 25] = [
        0x53, 0x4C, 0x38, 0x54, 0x41, 0x4C, 0x4C, 0x59, // "SL8TALLY"
        0x01, // version
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // count, little-endian
        0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, // total, little-endian
    ];
    let mut out = [0u8; 25];
    t.write_into(&mut out).unwrap();
    assert_eq!(out, expected);
    assert_eq!(PayoutTally::parse(&expected), Ok(t));

    // A realistic one: 3 requests worth $2,500 in 6-decimal units.
    let real = PayoutTally {
        requested_count: 3,
        requested_total: 2_500_000_000,
    };
    let mut out = [0u8; 25];
    real.write_into(&mut out).unwrap();
    assert_eq!(
        out,
        [
            b'S', b'L', b'8', b'T', b'A', b'L', b'L', b'Y', 1, // header
            3, 0, 0, 0, 0, 0, 0, 0, // count = 3
            0x00, 0xF9, 0x02, 0x95, 0, 0, 0, 0, // total = 2_500_000_000
        ]
    );
}

#[test]
fn tally_round_trips_including_boundaries() {
    let values = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (255, 256),
        (u64::MAX, u64::MAX),
        (u64::MAX, 0),
        (0, u64::MAX),
        (1, u64::MAX),
        (u64::MAX - 1, 2),
        (0x0102_0304_0506_0708, 0x1112_1314_1516_1718),
    ];
    for (count, total) in values {
        let t = PayoutTally {
            requested_count: count,
            requested_total: total,
        };
        let mut buf = [0xAAu8; PAYOUT_TALLY_MIN_LEN];
        t.write_into(&mut buf).unwrap();
        assert_eq!(buf, tally_bytes(count, total));
        let back = PayoutTally::parse(&buf).unwrap();
        assert_eq!(back, t);
        // count and total are never confused with each other
        assert_eq!(back.requested_count, count);
        assert_eq!(back.requested_total, total);
    }
}

#[test]
fn tally_default_is_zero_zero() {
    // A missing account counts as zero/zero; Default is that value.
    assert_eq!(
        PayoutTally::default(),
        PayoutTally {
            requested_count: 0,
            requested_total: 0
        }
    );
}

#[test]
fn tally_parse_rejects_too_short_by_one_byte_and_empty() {
    let full = tally_bytes(5, 9);
    assert_eq!(
        PayoutTally::parse(&full[..24]),
        Err(TallyError::TooShort { len: 24 })
    );
    assert_eq!(
        PayoutTally::parse(&[]),
        Err(TallyError::TooShort { len: 0 })
    );
    assert_eq!(
        PayoutTally::parse(&full[..9]),
        Err(TallyError::TooShort { len: 9 })
    );
    assert!(PayoutTally::parse(&full).is_ok(), "25 bytes is the minimum");
}

#[test]
fn tally_parse_rejects_every_single_byte_magic_error() {
    for i in 0..8 {
        let mut bad = tally_bytes(5, 9);
        bad[i] ^= 0x01;
        assert_eq!(
            PayoutTally::parse(&bad),
            Err(TallyError::BadMagic),
            "magic byte {i} off by one bit must be rejected"
        );
    }
    // One byte off by one *value*, not just one bit.
    let mut bad = tally_bytes(5, 9);
    bad[7] = bad[7].wrapping_add(1);
    assert_eq!(PayoutTally::parse(&bad), Err(TallyError::BadMagic));
    assert_eq!(
        PayoutTally::parse(&[0u8; 25]),
        Err(TallyError::BadMagic),
        "all-zero (freshly allocated) account is not a tally"
    );
}

#[test]
fn tally_parse_rejects_other_versions() {
    for v in [0u8, 2, 3, 255] {
        let mut bad = tally_bytes(5, 9);
        bad[8] = v;
        assert_eq!(
            PayoutTally::parse(&bad),
            Err(TallyError::UnsupportedVersion(v)),
            "version {v}"
        );
    }
}

#[test]
fn tally_parse_check_order_is_length_then_magic_then_version() {
    // short AND bad magic -> TooShort
    let mut short_bad = tally_bytes(1, 1);
    short_bad[0] = 0;
    assert_eq!(
        PayoutTally::parse(&short_bad[..24]),
        Err(TallyError::TooShort { len: 24 })
    );
    // bad magic AND bad version -> BadMagic
    let mut both = tally_bytes(1, 1);
    both[0] = 0;
    both[8] = 9;
    assert_eq!(PayoutTally::parse(&both), Err(TallyError::BadMagic));
}

#[test]
fn tally_longer_account_parses_and_trailing_bytes_are_ignored() {
    let t = PayoutTally {
        requested_count: 42,
        requested_total: 7_000_000,
    };
    for extra in [1usize, 2, 7, 8, 100, 1_000] {
        let mut buf = vec![0xFFu8; PAYOUT_TALLY_MIN_LEN + extra];
        t.write_into(&mut buf).unwrap();
        assert_eq!(PayoutTally::parse(&buf), Ok(t), "{extra} trailing bytes");
        // sector data after byte 25 must not change what is read
        let last = buf.len() - 1;
        buf[last] = 0;
        buf[PAYOUT_TALLY_MIN_LEN] = 0x5A;
        assert_eq!(PayoutTally::parse(&buf), Ok(t));
    }
}

#[test]
fn tally_write_into_rejects_short_buffers_without_writing() {
    let t = PayoutTally {
        requested_count: 1,
        requested_total: 2,
    };
    let mut short = [0xEEu8; 24];
    assert_eq!(
        t.write_into(&mut short),
        Err(TallyError::TooShort { len: 24 })
    );
    assert_eq!(short, [0xEEu8; 24], "nothing written on error");
    let mut empty: [u8; 0] = [];
    assert_eq!(
        t.write_into(&mut empty),
        Err(TallyError::TooShort { len: 0 })
    );
}

#[test]
fn tally_write_into_touches_exactly_the_first_25_bytes() {
    let t = PayoutTally {
        requested_count: u64::MAX,
        requested_total: u64::MAX,
    };
    let mut buf = [0xAAu8; 40];
    t.write_into(&mut buf).unwrap();
    assert_eq!(&buf[..25], &tally_bytes(u64::MAX, u64::MAX));
    assert!(buf[25..].iter().all(|&b| b == 0xAA), "sector data kept");

    // Overwriting an older value replaces every field.
    let newer = PayoutTally {
        requested_count: 1,
        requested_total: 2,
    };
    newer.write_into(&mut buf).unwrap();
    assert_eq!(&buf[..25], &tally_bytes(1, 2));
    assert!(buf[25..].iter().all(|&b| b == 0xAA));
}

#[test]
fn tally_error_messages_are_exact() {
    assert_eq!(
        TallyError::TooShort { len: 24 }.to_string(),
        "payout tally too short: 24 bytes, need at least 25"
    );
    assert_eq!(
        TallyError::BadMagic.to_string(),
        "payout tally has the wrong magic bytes"
    );
    assert_eq!(
        TallyError::UnsupportedVersion(2).to_string(),
        "unsupported payout tally version 2 (expected 1)"
    );
    let _: &dyn std::error::Error = &TallyError::BadMagic;
}

#[test]
fn tally_header_is_validated_on_long_buffers_too() {
    // A bug that only checks the header when len == 25 would pass these.
    let mut bad_version = vec![0x11u8; 125];
    bad_version[..25].copy_from_slice(&tally_bytes(1, 2));
    bad_version[8] = 2;
    assert_eq!(
        PayoutTally::parse(&bad_version),
        Err(TallyError::UnsupportedVersion(2))
    );

    let mut bad_magic = vec![0x22u8; 1_000];
    bad_magic[..25].copy_from_slice(&tally_bytes(1, 2));
    bad_magic[3] ^= 0xFF;
    assert_eq!(PayoutTally::parse(&bad_magic), Err(TallyError::BadMagic));
}

#[test]
fn tally_large_account_round_trips() {
    // Accounts can be large; header handling must not depend on length.
    let t = PayoutTally {
        requested_count: 9,
        requested_total: 8_000_000,
    };
    let mut big = vec![0u8; 1 << 20];
    t.write_into(&mut big).unwrap();
    assert_eq!(PayoutTally::parse(&big), Ok(t));
}

#[test]
fn tally_ignores_distinct_trailing_data_and_write_preserves_it() {
    let t = PayoutTally {
        requested_count: 4,
        requested_total: 6,
    };
    // Non-uniform sector data after byte 25: if parse read count/total from the
    // tail it would return different numbers.
    let mut buf = [0u8; 64];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(3);
    }
    let sector_data = buf[25..].to_vec();
    t.write_into(&mut buf).unwrap();
    assert_eq!(
        &buf[25..],
        &sector_data[..],
        "first write keeps sector data"
    );
    assert_eq!(PayoutTally::parse(&buf), Ok(t));

    let newer = PayoutTally {
        requested_count: 5,
        requested_total: 11,
    };
    newer.write_into(&mut buf).unwrap();
    assert_eq!(&buf[25..], &sector_data[..], "second write keeps it too");
    assert_eq!(PayoutTally::parse(&buf), Ok(newer));
}

// ---------------------------------------------------------------------------
// v0.4.1: admin_withdraw_marketing_funds matches the vault; discriminators;
// vault-source cross-checks.
// ---------------------------------------------------------------------------

use solana_program::instruction::Instruction;

use crate::PoolSide;

/// `sha256("global:<name>")[..8]`, recomputed here rather than trusted.
fn anchor_discriminator(name: &str) -> [u8; 8] {
    let h = solana_program::hash::hash(format!("global:{name}").as_bytes()).to_bytes();
    let mut d = [0u8; 8];
    d.copy_from_slice(&h[..8]);
    d
}

#[test]
fn every_discriminator_equals_sha256_of_the_instruction_name() {
    let table: [(&str, [u8; 8]); 11] = [
        ("register_product", REGISTER_PRODUCT_DISCRIMINATOR),
        ("reactivate_product", REACTIVATE_PRODUCT_DISCRIMINATOR),
        ("update_product_config", UPDATE_PRODUCT_CONFIG_DISCRIMINATOR),
        (
            "admin_withdraw_marketing_funds",
            ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR,
        ),
        ("pause_product", PAUSE_PRODUCT_DISCRIMINATOR),
        ("deposit_fee", DEPOSIT_FEE_DISCRIMINATOR),
        ("deposit_reset", DEPOSIT_RESET_DISCRIMINATOR),
        ("record_activity", RECORD_ACTIVITY_DISCRIMINATOR),
        ("mark_abandoned", MARK_ABANDONED_DISCRIMINATOR),
        ("request_payout", REQUEST_PAYOUT_DISCRIMINATOR),
        ("flag_trader_failed", FLAG_TRADER_FAILED_DISCRIMINATOR),
    ];
    for (name, constant) in &table {
        assert_eq!(&anchor_discriminator(name), constant, "{name}");
    }
    // The one the vault audit found wrong-shaped: pin the literal bytes too.
    assert_eq!(
        ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR,
        [149, 0, 251, 20, 103, 248, 17, 186]
    );
    assert_eq!(
        anchor_discriminator("admin_withdraw_marketing_funds"),
        [149, 0, 251, 20, 103, 248, 17, 186]
    );
}

#[test]
fn pool_side_is_one_borsh_byte_usdc_zero_usdt_one() {
    use borsh::{BorshDeserialize, BorshSerialize};
    assert_eq!(PoolSide::Usdc as u8, 0);
    assert_eq!(PoolSide::Usdt as u8, 1);
    assert_eq!(PoolSide::Usdc.try_to_vec().unwrap(), vec![0]);
    assert_eq!(PoolSide::Usdt.try_to_vec().unwrap(), vec![1]);
    assert_eq!(PoolSide::try_from_slice(&[0]).unwrap(), PoolSide::Usdc);
    assert_eq!(PoolSide::try_from_slice(&[1]).unwrap(), PoolSide::Usdt);
    assert!(PoolSide::try_from_slice(&[2]).is_err());
    assert!(PoolSide::try_from_slice(&[255]).is_err());
    assert!(PoolSide::try_from_slice(&[]).is_err());
}

#[test]
fn admin_withdraw_data_bytes_are_exact_for_both_pools() {
    let build = |pool, amount| {
        admin_withdraw_marketing_funds(
            pk(),
            pk(),
            pk(),
            pk(),
            pk(),
            pk(),
            pk(),
            pk(),
            AdminWithdrawMarketingFundsArgs { pool, amount },
        )
        .data
    };
    let d = ADMIN_WITHDRAW_MARKETING_FUNDS_DISCRIMINATOR;

    // asymmetric amount so any endianness / offset slip shows
    let usdc = build(PoolSide::Usdc, 0x0102_0304_0506_0708);
    assert_eq!(usdc.len(), 17);
    assert_eq!(
        usdc,
        vec![
            d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], // discriminator
            0,    // pool = Usdc
            0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // amount, little-endian
        ]
    );
    let usdt = build(PoolSide::Usdt, 0x0102_0304_0506_0708);
    assert_eq!(usdt.len(), 17);
    assert_eq!(usdt[8], 1, "pool = Usdt");
    assert_eq!(&usdt[9..], &usdc[9..], "amount bytes do not depend on pool");
    assert_eq!(&usdt[..8], &usdc[..8]);

    // a realistic $750.000001 and the extremes
    assert_eq!(
        build(PoolSide::Usdt, 750_000_001),
        vec![d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], 1, 129, 23, 180, 44, 0, 0, 0, 0,]
    );
    let zero = build(PoolSide::Usdc, 0);
    assert_eq!(&zero[8..], &[0u8; 9]);
    let max = build(PoolSide::Usdt, u64::MAX);
    assert_eq!(&max[8..], &[1, 255, 255, 255, 255, 255, 255, 255, 255]);
}

#[test]
fn admin_withdraw_args_round_trip() {
    use borsh::{BorshDeserialize, BorshSerialize};
    for pool in [PoolSide::Usdc, PoolSide::Usdt] {
        for amount in [0u64, 1, 750_000_001, u64::MAX] {
            let args = AdminWithdrawMarketingFundsArgs { pool, amount };
            let bytes = args.try_to_vec().unwrap();
            assert_eq!(bytes.len(), 9);
            assert_eq!(
                AdminWithdrawMarketingFundsArgs::try_from_slice(&bytes).unwrap(),
                args
            );
        }
    }
    // an unknown pool byte is rejected, not coerced
    let mut bad = vec![2u8];
    bad.extend_from_slice(&5u64.to_le_bytes());
    assert!(AdminWithdrawMarketingFundsArgs::try_from_slice(&bad).is_err());
}

/// The vault's `AdminWithdrawMarketingFunds` accounts, in order, as
/// `(name, is_signer, is_writable)`.
const ADMIN_WITHDRAW_ACCOUNTS: [Spec; 7] = [
    ("sl8_admin", true, false),
    ("rov_admin", true, false),
    ("vault_state", false, true),
    ("mint", false, false),
    ("pool_token_account", false, true),
    ("sl8_token_account", false, true),
    ("token_program", false, false),
];

fn admin_withdraw_ix() -> (Instruction, [Pubkey; 8]) {
    let keys: [Pubkey; 8] = std::array::from_fn(|_| pk());
    let ix = admin_withdraw_marketing_funds(
        keys[0],
        keys[1],
        keys[2],
        keys[3],
        keys[4],
        keys[5],
        keys[6],
        keys[7],
        AdminWithdrawMarketingFundsArgs {
            pool: PoolSide::Usdt,
            amount: 5,
        },
    );
    (ix, keys)
}

#[test]
fn admin_withdraw_accounts_order_signers_and_writables() {
    let (ix, keys) = admin_withdraw_ix();
    assert_eq!(ix.program_id, keys[0], "first parameter is the vault id");
    assert_eq!(ix.accounts.len(), 7, "no remaining accounts");
    for (i, (name, signer, writable)) in ADMIN_WITHDRAW_ACCOUNTS.iter().enumerate() {
        let a = &ix.accounts[i];
        // parameters 1..=7 map to accounts 0..=6, in order
        assert_eq!(a.pubkey, keys[i + 1], "{name}: position {i}");
        assert_eq!(a.is_signer, *signer, "{name}: signer flag");
        assert_eq!(a.is_writable, *writable, "{name}: writable flag");
    }
    assert_eq!(ix.accounts.iter().filter(|a| a.is_signer).count(), 2);
    assert_eq!(ix.accounts.iter().filter(|a| a.is_writable).count(), 3);
}

// ---- cross-check against the vault's source text -----------------------------
// The vault repo is a read-only sibling. When its source is on disk, parse its
// `#[derive(Accounts)]` structs and compare account names, order, signer and
// writable flags with what these builders produce, so drift (like the old
// `flag_trader_failed` registry flag or the old admin_withdraw layout) fails a
// test instead of failing on-chain. Override the location with
// SETL8_VAULT_SRC; set SETL8_REQUIRE_VAULT_SRC=1 to fail instead of skip when it
// is missing. Skipped (with a message) otherwise, e.g. in a checkout without the
// vault next to it.

fn vault_src_dir() -> Option<std::path::PathBuf> {
    let dir = match std::env::var("SETL8_VAULT_SRC") {
        Ok(p) => std::path::PathBuf::from(p),
        Err(_) => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../setl8-vault/programs/core-vault/src"),
    };
    if dir.join("lib.rs").is_file() {
        Some(dir)
    } else {
        assert!(
            std::env::var("SETL8_REQUIRE_VAULT_SRC").is_err(),
            "SETL8_REQUIRE_VAULT_SRC is set but the vault source was not found at {}",
            dir.display()
        );
        eprintln!("skipping vault cross-check: {} not found", dir.display());
        None
    }
}

fn has_word(text: &str, word: &str) -> bool {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|w| w == word)
}

/// `(name, is_signer, is_writable)` for each field of `pub struct <name><'info>`.
///
/// Deliberately simple text parsing. Known limits: a signer is recognised only
/// by a type that starts with `Signer`; writable only by the words `mut`,
/// `init` or `init_if_needed` in the field's `#[account(..)]` attribute;
/// attributes must end a line with `]`. That holds for every struct it is applied
/// to (the self-test below shows the shapes it handles); a new shape would make
/// it fail loudly rather than pass.
fn vault_accounts(src: &str, struct_name: &str) -> Vec<(String, bool, bool)> {
    let marker = format!("pub struct {struct_name}<");
    let start = src
        .find(&marker)
        .unwrap_or_else(|| panic!("struct {struct_name} not found in the vault source"));
    let open = src[start..].find('{').unwrap() + start + 1;
    let close = src[open..].find("\n}").unwrap() + open;

    let mut out = Vec::new();
    let mut attr = String::new();
    let mut depth = 0i32;
    let mut in_attr = false;
    for line in src[open..close].lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if in_attr || t.starts_with("#[") {
            in_attr = true;
            attr.push_str(t);
            attr.push(' ');
            depth += t.matches('(').count() as i32 - t.matches(')').count() as i32;
            if depth <= 0 && t.ends_with(']') {
                in_attr = false;
            }
            continue;
        }
        if let Some(rest) = t.strip_prefix("pub ") {
            let (name, ty) = rest.split_once(':').expect("field has a type");
            let writable = has_word(&attr, "mut")
                || has_word(&attr, "init")
                || has_word(&attr, "init_if_needed");
            out.push((
                name.trim().to_string(),
                ty.trim().starts_with("Signer"),
                writable,
            ));
            attr.clear();
            depth = 0;
        }
    }
    out
}

/// Panics if the builder's accounts differ from the vault struct in count, name
/// order, signer or writable flag. `names` are the builder-side names, in order.
fn assert_matches_vault(file: &str, struct_name: &str, names: &[&str], ix: &Instruction) {
    let Some(dir) = vault_src_dir() else { return };
    let text = std::fs::read_to_string(dir.join(file)).expect("read vault source file");
    let vault = vault_accounts(&text, struct_name);
    let vault_names: Vec<&str> = vault.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(
        vault_names, names,
        "{struct_name}: account names/order differ from the vault"
    );
    assert_eq!(
        ix.accounts.len(),
        vault.len(),
        "{struct_name}: account count differs from the vault"
    );
    for (i, (name, signer, writable)) in vault.iter().enumerate() {
        assert_eq!(
            ix.accounts[i].is_signer, *signer,
            "{struct_name}.{name}: signer flag differs from the vault"
        );
        assert_eq!(
            ix.accounts[i].is_writable, *writable,
            "{struct_name}.{name}: writable flag differs from the vault"
        );
    }
}

fn names_of(fixed: &[&'static str], rest: &[Spec]) -> Vec<&'static str> {
    fixed
        .iter()
        .copied()
        .chain(rest.iter().map(|s| s.0))
        .collect()
}

#[test]
fn vault_source_admin_withdraw_matches_the_builder() {
    let (ix, _) = admin_withdraw_ix();
    let names: Vec<&str> = ADMIN_WITHDRAW_ACCOUNTS.iter().map(|s| s.0).collect();
    assert_matches_vault(
        "instructions/admin/admin_withdraw_marketing_funds.rs",
        "AdminWithdrawMarketingFunds",
        &names,
        &ix,
    );

    // The handler's parameter order and the enum's variant order are wire format.
    let Some(dir) = vault_src_dir() else { return };
    let lib = std::fs::read_to_string(dir.join("lib.rs")).unwrap();
    let sig = lib
        .find("pub fn admin_withdraw_marketing_funds(")
        .expect("vault exposes admin_withdraw_marketing_funds");
    let sig = &lib[sig..sig + lib[sig..].find(") -> Result").unwrap()];
    let compact: String = sig.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.ends_with("ctx:Context<AdminWithdrawMarketingFunds>,pool:PoolSide,amount:u64,"),
        "vault takes exactly (ctx, pool, amount) in that order: {sig}"
    );

    let vs = std::fs::read_to_string(dir.join("state/vault_state.rs")).unwrap();
    let e = vs.find("pub enum PoolSide {").expect("PoolSide enum");
    let body = &vs[e..e + vs[e..].find('}').unwrap()];
    let usdc = body.find("Usdc").expect("Usdc variant");
    let usdt = body.find("Usdt").expect("Usdt variant");
    assert!(usdc < usdt, "Usdc is variant 0, Usdt variant 1");
    assert!(
        !body.contains('='),
        "explicit enum discriminants would change the Borsh index: {body}"
    );
}

#[test]
fn vault_source_flag_trader_failed_matches_the_builder() {
    let remaining = [AccountMeta::new(pk(), false)];
    let ix = flag_trader_failed(
        pk(),
        pk(),
        pk(),
        &remaining,
        FlagTraderFailedArgs {
            trader_wallet: pk(),
            product_program_id: pk(),
            challenge_id: 1,
        },
    );
    assert_matches_vault(
        "instructions/sector/flag_trader_failed.rs",
        "FlagTraderFailed",
        &["sector_authority", "product_registry", "trader_state"],
        &ix,
    );
}

#[test]
fn vault_source_deposit_fee_deposit_reset_and_request_payout_match_the_documented_layouts() {
    let fee_rest = remaining_from(&DEPOSIT_FEE_REMAINING);
    let fee = deposit_fee(
        pk(),
        pk(),
        pk(),
        &fee_rest,
        DepositFeeArgs {
            amount: 1,
            product_program_id: pk(),
            challenge_id: 1,
            trader_wallet: pk(),
            account_size: 1,
        },
    );
    assert_matches_vault(
        "instructions/sector/deposit_fee.rs",
        "DepositFee",
        &names_of(
            &["sector_authority", "product_registry"],
            &DEPOSIT_FEE_REMAINING,
        ),
        &fee,
    );

    let reset_rest = remaining_from(&DEPOSIT_RESET_REMAINING);
    let reset = deposit_reset(
        pk(),
        pk(),
        pk(),
        &reset_rest,
        DepositResetArgs {
            amount: 1,
            trader_wallet: pk(),
            product_program_id: pk(),
            prev_challenge_id: 1,
            new_challenge_id: 2,
            reset_phase: 0,
        },
    );
    assert_matches_vault(
        "instructions/sector/deposit_reset.rs",
        "DepositReset",
        &names_of(
            &["sector_authority", "product_registry"],
            &DEPOSIT_RESET_REMAINING,
        ),
        &reset,
    );

    let payout_rest = remaining_from(&QUEUED_PAYOUT_REMAINING);
    let payout = request_payout(
        pk(),
        pk(),
        pk(),
        &payout_rest,
        RequestPayoutArgs {
            trader_wallet: pk(),
            amount: 1,
            product_program_id: pk(),
            challenge_id: 1,
            proposed_request_id: 1,
        },
    );
    assert_matches_vault(
        "instructions/sector/request_payout.rs",
        "RequestPayout",
        &names_of(
            &["sector_authority", "product_registry"],
            &QUEUED_PAYOUT_REMAINING,
        ),
        &payout,
    );
}

#[test]
fn the_vault_source_parser_reads_flags_correctly() {
    // Guards the cross-check itself: it must not pass vacuously.
    let src = r#"
#[derive(Accounts)]
pub struct Demo<'info> {
    pub plain_signer: Signer<'info>,

    #[account(
        mut,
        seeds = [A, b.as_ref()],
        bump = x.bump,
    )]
    pub writable_pda: Box<Account<'info, X>>,

    /// docs
    #[account(address = K @ E::Bad)]
    pub readonly_signer: Signer<'info>,

    #[account(init, payer = payer, space = 8, seeds = [S], bump)]
    pub created: Box<Account<'info, X>>,

    pub program: Program<'info, System>,
}
"#;
    assert_eq!(
        vault_accounts(src, "Demo"),
        vec![
            ("plain_signer".to_string(), true, false),
            ("writable_pda".to_string(), false, true),
            ("readonly_signer".to_string(), true, false),
            ("created".to_string(), false, true),
            ("program".to_string(), false, false),
        ]
    );
}
