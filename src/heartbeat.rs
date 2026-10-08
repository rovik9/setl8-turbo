//! How queued payouts get paid: the vault's **permissionless heartbeat**.
//!
//! This module is documentation only. It has no code and this crate has **no
//! builders** for these three instructions: they are not sector-program calls,
//! anyone (usually a keeper bot) may run them, and a keeper builds them from
//! the vault's own IDL. They are described here because a sector program's
//! `request_payout` only queues a claim, and everyone involved needs to know
//! what happens to it next. Account lists were read from the vault's
//! `#[derive(Accounts)]` structs (vault commit `be97396`).
//!
//! # The payout queue
//!
//! * `request_payout` that returns `PayoutOutcome::Paid` creates one
//!   `PayoutClaim` PDA under the **vault** program, at
//!   [`derive_payout_claim`](crate::derive_payout_claim)`(vault, trader_state,
//!   request_id)` (seed [`PAYOUT_CLAIM_SEED`](crate::PAYOUT_CLAIM_SEED)). It
//!   records the trader's wallet, the `TraderState`, the product, the request
//!   id, the amount **still owed**, the cycle it was created in and the last
//!   cycle that processed it. The vault also keeps two running totals in
//!   `VaultState`: the number of open claims and the sum still owed.
//! * No tokens move at that point. A claim stays owed, at equal priority with
//!   every other claim, until paid; it never expires or jumps the queue.
//!
//! # One cycle: `begin_heartbeat`, `settle_claims`, `finalize_heartbeat`
//!
//! **1. `begin_heartbeat`** (no arguments). Opens a cycle and freezes its
//! inputs: the total owed (`open_claims_total`) and the total available
//! (USDC pool + USDT pool balances). Only one cycle can be open at a time, and
//! a new cycle may not start until a fixed minimum gap (5 days, set in the
//! vault program) has passed since the *start* of the previous one. The first
//! cycle may start immediately.
//!
//! | # | flags | account |
//! |---|-------|---------|
//! | 0 | signer | `caller` (anyone; pays only the transaction fee) |
//! | 1 | writable | `vault_state` |
//! | 2 | | `usdc_pool` |
//! | 3 | | `usdt_pool` |
//!
//! **2. `settle_claims`** (no arguments; one or more calls per cycle). Pays a
//! batch of claims. Every eligible claim in the cycle gets the **same ratio**,
//! `min(available, owed) / owed` from the frozen snapshot, so the result does
//! not depend on processing order. Each claim is paid
//! `floor(owed * ratio)` (so rounding dust stays owed), capped by what the
//! pools hold right now, **pro rata**:
//!
//! * the payment comes from the **larger** pool first (a tie goes to USDC) and
//!   is topped up from the other pool;
//! * it goes straight to the trader's **associated token accounts** for the
//!   vault's USDC and USDT mints;
//! * whatever is **not** paid stays owed on the claim and is **carried over**
//!   to the next cycle; a claim paid in full is closed and its rent goes to
//!   the caller;
//! * a claim created *during* a cycle waits for the next one (listing it, or a
//!   claim already processed this cycle, in a batch fails the whole
//!   transaction with `ClaimNotEligible` / `ClaimAlreadySettled`; a keeper
//!   must not include it);
//! * if a correctly-addressed destination account is unusable (missing,
//!   frozen, wrong owner), the claim is skipped for this cycle and stays owed;
//!   a wrongly-addressed destination, or a claim account with the wrong
//!   owner, address or layout (`InvalidClaim`), reverts the whole transaction.
//!
//! | # | flags | account |
//! |---|-------|---------|
//! | 0 | signer, writable | `caller` (anyone; receives the rent of closed claims) |
//! | 1 | writable | `vault_state` |
//! | 2 | | `usdc_mint` |
//! | 3 | | `usdt_mint` |
//! | 4 | writable | `usdc_pool` |
//! | 5 | writable | `usdt_pool` |
//! | 6 | | `token_program` (classic SPL Token) |
//!
//! followed by 1 to 6 **triples** (the batch limit is set in the vault
//! program), each `[payout_claim (writable), trader_usdc_ata (writable),
//! trader_usdt_ata (writable)]`. An empty batch, a length that is not a
//! multiple of three, or too many triples fails.
//!
//! **3. `finalize_heartbeat`** (no arguments). Closes the cycle once every claim
//! that was eligible at the start has been processed (paid or skipped), and
//! recomputes the pools' reserve floors. Fails if the cycle is incomplete.
//!
//! | # | flags | account |
//! |---|-------|---------|
//! | 0 | signer | `caller` (anyone; pays only the transaction fee) |
//! | 1 | writable | `vault_state` |
//! | 2 | | `usdc_pool` |
//! | 3 | | `usdt_pool` |
//!
//! # What a sector program should take from this
//!
//! * `PayoutOutcome::Paid` from `request_payout` means **queued**. Tell the
//!   trader their payout is *requested*, not paid.
//! * How much is paid, and when, is the vault's business: it depends on the
//!   pools and on every other open claim. A claim can take several cycles.
//! * The sector keeps its payout tally in step with accepted requests only;
//!   see [`payout_tally`](crate::payout_tally) (the vault does not enforce it
//!   yet).
