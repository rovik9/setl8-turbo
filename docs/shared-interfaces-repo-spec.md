# setl8-shared-interfaces — Repo Spec

Build this repo first. Everything else (`setl8-vault`, `setl8-lev-trading`, and future product repos) depends on it.

## Purpose

This repo holds **CPI interface definitions only** — the shape of the conversation between the vault program and sector programs. It contains:

- Instruction argument/account structs for every cross-program call
- No business logic
- No state or account data definitions beyond what's needed to describe a call's shape
- No numbers (no fees, no caps, no challenge costs — those live on-chain in the vault's own accounts, not here)

Every consuming repo (`setl8-vault`, `setl8-lev-trading`, future product repos) pulls this as a **versioned dependency**, not a copied file. A signature change here that a consumer hasn't updated for fails to compile in the consumer's own repo — that's the whole point of pulling it out into its own package instead of copying interface code by hand.

## Instructions to define shapes for

- `register_product(product_program_id: Pubkey, fee_split_bps: u16)` — 2-of-2 signer context shape
- `reactivate_product(product_program_id: Pubkey)` — 2-of-2 signer context shape
- `update_product_config(product_id: Pubkey, challenge_sizes: Vec<ChallengeSize>, fee_split_bps: u16, max_payout_count: u64)` — 2-of-2 signer context shape
- `admin_withdraw_marketing_funds(pool: PoolSide, amount: u64)` — 2-of-2 signer context shape, destination fixed to SL8 wallet (builder corrected in v0.4.1 to match the vault: 7 accounts, no remaining accounts)
- `deposit_fee(amount: u64, product_id: Pubkey, challenge_id: u64)` — CPI-auth context shape (calling program identity check against registry)
- `request_payout(trader_wallet: Pubkey, amount: u64, product_id: Pubkey, challenge_id: u64, proposed_request_id: u64)` — CPI-auth context shape; note the `proposed_request_id` field exists specifically for the mutual-agreement request-ID check the vault performs. **Status (v0.4.0): the vault queues the payout** as a `PayoutClaim` instead of paying instantly; the call has 7 accounts (`sector_authority`, `product_registry` (writable), `trader_state` (writable), `vault_state` (writable), `payout_claim` (writable, PDA `[b"payout_claim", trader_state, proposed_request_id u64 LE]` under the vault), `payer` (signer, writable), `system_program`), moves no tokens, and returns `PayoutOutcome::Paid` ("accepted and queued") or `Abandoned` (no claim). Payment happens later through the permissionless heartbeat (`begin_heartbeat`, `settle_claims`, `finalize_heartbeat`), pro rata, with the unpaid remainder carried over. Sectors also maintain a sector-owned payout tally, which the vault reads in `reconcile_product` (vault `686bd71`) and pauses the product on a mismatch (see README)
- `flag_trader_failed(trader_wallet: Pubkey, product_id: Pubkey, challenge_id: u64)` — CPI-auth context shape

## CPI auth pattern (reference, not reimplemented here)

Every non-admin instruction above must be called with the calling program's actual on-chain identity checked against `ProductRegistry.product_program_id` — never a self-reported field. This repo just defines the shapes; the actual check lives in `setl8-vault`. Document this expectation clearly in this repo's README so any future consumer repo's author knows the auth contract they're building against, even though they can't see the check itself.

## Versioning

Decide and document (not yet chosen — flag to Rovik if starting build before this is picked):
- **Option A:** private Cargo registry, semver releases
- **Option B:** git dependency pinned to a tag/commit per consumer's `Cargo.toml`

Whichever is chosen, every interface change gets a version bump and a one-line changelog entry — this is the only mechanism catching drift now that repos are separate, so don't skip it.

## What does NOT belong here

- Account state structs for `ProductRegistry`, `TraderState`, `PayoutClaim` — those live in `setl8-vault`. (`BondPosition`, `BondCapTracker` and the claim account all live in the vault.) The one exception is the sector-owned payout tally, whose byte layout is a cross-program contract defined in this crate
- Fee percentages, challenge costs, payout caps — those are on-chain data, not code
- Any business logic (graduation checks, floor calculations, abandonment sweeps) — those live in `setl8-vault`
