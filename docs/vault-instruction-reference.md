# Vault instruction reference

What a client needs to call (or understand) the `setl8-vault` instructions that
this crate has **no builder** for, plus the bond and claim rules around them.
Everything here was read from the vault's source at commit **`686bd71`**
(`programs/core-vault/src`) and is the vault's behaviour, not a proposal. If the
vault changes, this file needs updating.

Conventions:

* All amounts are **6-decimal base units**. USDC and USDT are both worth $1.
* Account tables list accounts in the order the vault declares them. `signer`
  and `writable` are the flags the transaction must carry.
* Instruction data is the 8-byte discriminator `sha256("global:<name>")[..8]`
  followed by the Borsh-encoded arguments (`u64` is little-endian; an enum is one
  byte, its variant index).
* "Classic SPL Token" means the original token program; Token-2022 is rejected.
* The two admin keys, and therefore every PDA derived from them, differ between
  the vault's real build and its `localnet` test build. **Never hardcode a vault
  PDA**; derive it for the build you are talking to.

For the instructions this crate *does* have builders for, the rustdoc on each
builder in `src/instructions/` is the reference. The `heartbeat` and
`payout_tally` modules give the narrative for the payout queue and the tally.

## PDA seeds

| Account | Program | Seeds |
|---|---|---|
| `VaultState` | vault | `[b"vault_state", SL8_ADMIN, ROV_ADMIN]` |
| Payout pool (per mint) | vault | `[b"pool", vault_state, mint]` (token authority = `VaultState`) |
| `ProductRegistry` | vault | `[b"product_registry", product_program_id]` |
| `TraderState` | vault | `[b"trader_state", product_program_id, trader_wallet, challenge_id u64 LE]` |
| `PayoutClaim`, kind 0 (trader) | vault | `[b"payout_claim", trader_state, request_id u64 LE]` |
| `PayoutClaim`, kind 1 (bond) | vault | `[b"bond_claim", depositor, deposit_index u64 LE]` |
| `BondPosition` | vault | `[b"bond", depositor, deposit_index u64 LE]` |
| `BondCapTracker` | vault | `[b"bond_cap", depositor]` |
| Payout tally | **sector** | `[b"payout_tally"]` under the sector program id |
| `sector_authority` | **sector** | `[b"setl8_sector_authority"]` under the sector program id |

`SL8_ADMIN` / `ROV_ADMIN` are the vault's compiled-in admin public keys. This
crate exports `derive_payout_claim`, `derive_payout_tally` and
`derive_sector_authority` for the three it can derive without them.

## Instruction index

| Instruction | Caller | In this crate |
|---|---|---|
| `begin_heartbeat` | anyone | docs only |
| `settle_claims` | anyone | docs only |
| `finalize_heartbeat` | anyone | docs only |
| `reconcile_product` | anyone | docs only |
| `deposit_bond` | the depositor | docs only |
| `request_bond_payout` | the depositor | docs only |
| `admin_withdraw_marketing_funds` | 2-of-2 admins | builder |
| `mark_abandoned` | anyone | builder |

Discriminators of the docs-only instructions:

| Instruction | Discriminator |
|---|---|
| `begin_heartbeat` | `[38, 124, 157, 86, 152, 121, 160, 149]` |
| `settle_claims` | `[58, 91, 9, 15, 201, 59, 179, 94]` |
| `finalize_heartbeat` | `[53, 156, 100, 192, 145, 238, 80, 255]` |
| `reconcile_product` | `[196, 97, 47, 23, 32, 65, 106, 50]` |
| `deposit_bond` | `[120, 89, 18, 253, 112, 125, 87, 255]` |
| `request_bond_payout` | `[5, 44, 214, 232, 231, 74, 97, 219]` |

## The payout queue and claim kinds

`request_payout` (sector) and `request_bond_payout` (depositor) never move
tokens. Each creates a **`PayoutClaim`** owed to a wallet, and a heartbeat cycle
pays it later, pro rata. A claim has a **kind**:

| Kind | Meaning | Created by | Address seed |
|---|---|---|---|
| `0` | trader payout | `request_payout` | `b"payout_claim"` |
| `1` | bond withdrawal | `request_bond_payout` | `b"bond_claim"` |

Settlement treats both kinds identically (same cycle ratio, same pools, same
destinations); only the address derivation differs. Any other `kind` value is
invalid and rejected.

`PayoutClaim` account data (Anchor account: 8-byte discriminator, then Borsh,
138 bytes in all):

| Field | Type | Kind 0 | Kind 1 |
|---|---|---|---|
| `trader_wallet` | pubkey | the trader paid | the depositor |
| `trader_state` | pubkey | the `TraderState` | the (closed) `BondPosition` key |
| `product_program_id` | pubkey | the product | all zeros |
| `request_id` | u64 | the payout request id | the `deposit_index` |
| `owed` | u64 | still unpaid | still unpaid |
| `created_in_cycle` | u64 | `cycle_id` when created | same |
| `last_settled_cycle` | u64 | last cycle that processed it, 0 = never | same |
| `kind` | u8 | 0 | 1 |
| `bump` | u8 | canonical bump | canonical bump |

Both kinds are paid to the claim's `trader_wallet`'s **associated token
accounts** for the vault's USDC *and* USDT mints. A bond depositor therefore
needs both ATAs to exist, exactly like a trader (see `settle_claims`).

## Heartbeat

A cycle is `begin_heartbeat`, then one or more `settle_claims`, then
`finalize_heartbeat`. All three are permissionless and take **no arguments**.
At most one cycle is open at a time and a new one may start no sooner than
**432,000 seconds (5 days) after the previous one started**; the first may start
immediately. The gap and the batch limit are fixed in the vault program.

### `begin_heartbeat`

Opens a cycle and freezes its inputs: the total owed (`open_claims_total`) and
the total available (USDC pool + USDT pool balances), plus how many claims are
eligible. Every claim in the cycle is then settled with the same ratio.

| # | flags | account |
|---|---|---|
| 0 | signer | `caller` (anyone; pays only the transaction fee) |
| 1 | writable | `vault_state` |
| 2 |  | `usdc_pool` (must equal `vault_state.usdc_pool`) |
| 3 |  | `usdt_pool` (must equal `vault_state.usdt_pool`) |

Fails with `CycleInProgress` if a cycle is open and `HeartbeatTooEarly` inside
the gap. **The vault does not call `reconcile_product` itself**: see the keeper
duty below.

### `settle_claims`

Pays one batch of claims against the open cycle. Every eligible claim gets the
cycle's ratio `min(available, owed) / owed` (from the frozen snapshot), paid as
`floor(claim.owed * ratio)`, capped by what the pools hold *now*, **larger pool
first** (a tie goes to USDC) and topped up from the other. The result goes
straight to the claim holder's ATAs. Unpaid remainder (rounding dust, or a
shortfall) **stays owed** and is carried to the next cycle. A claim paid in full
is closed and its rent goes to the caller.

| # | flags | account |
|---|---|---|
| 0 | signer, writable | `caller` (anyone; receives the rent of every claim this call closes) |
| 1 | writable | `vault_state` |
| 2 |  | `usdc_mint` |
| 3 |  | `usdt_mint` |
| 4 | writable | `usdc_pool` |
| 5 | writable | `usdt_pool` |
| 6 |  | `token_program` |

followed by **1 to 6 triples** of remaining accounts, in this order:

| triple slot | flags | account |
|---|---|---|
| `3n + 0` | writable | `payout_claim` (vault-owned, at its canonical address; either kind) |
| `3n + 1` | writable | `trader_usdc_ata` (must be the claim holder's USDC ATA) |
| `3n + 2` | writable | `trader_usdt_ata` (must be the claim holder's USDT ATA) |

* **`MAX_SETTLE_BATCH` = 6.** A full batch (distinct keys, one ComputeBudget
  instruction, two signatures) is about 1,106 bytes of the 1,232-byte limit and
  uses roughly 125,000 compute units for ordinary wallets and roughly 235,000
  for wallets whose ATA derivation is expensive (measured by the vault's tests;
  bond claims, alone or mixed with trader claims, measure the same). **Add a
  `SetComputeUnitLimit` of 400,000** to a full batch. Any single claim can always
  be settled alone.
* With no cycle open the call fails with `NoCycleInProgress`. An empty batch
  fails with `EmptyBatch`, a length that is not a multiple of 3 with
  `InvalidClaim`, and more than 6 triples with `BatchTooLarge`. The mint and pool
  accounts are checked against `vault_state` (`InvalidMint` /
  `InvalidTokenAccount`).
* A claim created *during* the cycle, or one already processed this cycle,
  fails the **whole transaction** (`ClaimNotEligible` / `ClaimAlreadySettled`):
  do not list it. A claim with a wrong owner, address, kind or layout, or one not
  passed writable, fails with `InvalidClaim`.
* A **wrongly addressed** ATA fails the whole transaction (`InvalidTokenAccount`).
  A correctly addressed but **unusable** ATA (missing, frozen, wrong owner or
  mint) makes the claim *skipped*: it counts as processed for the cycle but stays
  open and owed. This is what stops one trader blocking everyone else's payment.

### `finalize_heartbeat`

Ends the cycle once every claim that was eligible at the start has been
processed (paid or skipped), and recomputes each pool's reserve floor as **25%**
of its post-settlement balance. The floor only limits
`admin_withdraw_marketing_funds`; it never limits claim settlement.

| # | flags | account |
|---|---|---|
| 0 | signer | `caller` (anyone; pays only the transaction fee) |
| 1 | writable | `vault_state` |
| 2 |  | `usdc_pool` |
| 3 |  | `usdt_pool` |

Fails with `NoCycleInProgress` or `CycleIncomplete`. The 5-day gap is measured
from the *start* of the previous cycle, not its end.

## `reconcile_product`

Permissionless. Compares a sector's **payout tally** (see the `payout_tally`
module) with the vault's own books for the product and **pauses the product on
any mismatch**.

Arguments: `product_program_id: Pubkey` (32 bytes; data is 40 bytes in all).

| # | flags | account |
|---|---|---|
| 0 | signer | `caller` (anyone; pays only the transaction fee) |
| 1 | writable | `product_registry` (`[b"product_registry", product_program_id]`) |
| 2 |  | `payout_tally` (must be exactly `derive_payout_tally(product_program_id)`) |

* The product must be active; a paused product fails with `ProductAlreadyPaused`
  and is neither re-paused nor given a new reason.
* A `payout_tally` that is not the canonical address fails with `InvalidTally`,
  so nobody can pause a product by passing junk.
* The tally is read with this crate's own `PayoutTally::parse`. **Missing**
  (no data and not owned by the sector) counts as `0 / 0`. **Invalid** (owned by
  the sector but unparseable, including empty or all-zero; or holding data but
  owned by someone else) is always a mismatch.
* It compares `requested_count` with the registry's `total_requests_emitted` and
  `requested_total` with `total_requested_amount`. **Both** must be equal. The
  vault bumps those two at the same moment it accepts a `request_payout`
  (`Paid`); the stale/`Abandoned` path does not.
* On mismatch it pauses the product with reason `2` (reconciliation deficit) and
  **returns `Ok`** (an error would revert the pause). Un-pausing is the 2-of-2
  `reactivate_product`.

**Keeper duty:** run `reconcile_product` for **every registered, active
product before each `begin_heartbeat`**. The heartbeat instructions never do it
for you, so a skipped reconcile means a mismatching product keeps accepting
fees, resets and payout requests until someone reconciles it. Skip products that
are already paused (the call would fail with `ProductAlreadyPaused`).

What a pause does: `deposit_fee`, `deposit_reset` and `request_payout` fail with
`ProductNotActive` and the product's inactivity clocks freeze. It does **not**
stop claims that are already queued: `settle_claims` never reads the product
registry, so a paused product's queued claims are still paid.

## Bonds

A bond is a deposit of USDC or USDT that earns a fixed interest, paid **only at
maturity**. Interest does not accrue over time.

### Terms and rules

| | 6-month bond | 9-month bond |
|---|---|---|
| Term | 15,552,000 s (180 days) | 23,328,000 s (270 days) |
| Hard lock | 7,776,000 s (90 days, half the term) | 11,664,000 s (135 days) |
| Interest at maturity | 20% (2,000 bps) | 30% (3,000 bps) |
| `term` argument | `SixMonths` = 0 | `NineMonths` = 1 |

* **Before the hard lock ends** a withdrawal fails with `BondLocked`.
* **From the lock until maturity** the gross amount is the **principal only** (no
  interest).
* **At or after maturity** the gross amount is principal plus
  `floor(principal * interest_bps / 10_000)`. Both boundaries are inclusive on
  the later side (`now >= created_at + lock` is unlocked, `now >= created_at +
  term` is matured).
* The interest rate is copied into the position at deposit, so a later change to
  the interest constants cannot alter an open bond's interest. The lock and term
  durations are **not** stored: they are derived from the `term` variant, using
  the vault's constants, at withdrawal time.

### Caps (measured on principal, not on the fee)

| Cap | Value |
|---|---|
| Minimum principal | $50 (`50_000_000`), else `BondBelowMinimum` |
| Open principal per wallet, across all its open positions | $50,000 (`50_000_000_000`), else `BondWalletCapExceeded` |
| Open principal across the whole vault | $600,000 (`600_000_000_000`), else `BondGlobalCapExceeded` |

A withdrawal request closes the position and frees that wallet's and the
vault's cap room.

### Fees

| Fee | Rate | Rounding | Who pays |
|---|---|---|---|
| Deposit | **0.2%** of the principal | rounded **up** | the depositor, **on top of** the principal; 100% to the SL8 wallet |
| Withdrawal | **0.2%** of the gross amount | rounded **up** | **deducted** from the amount owed; no tokens move, so it simply stays in the pool |

Deposit worked example, $1,000 USDC: the depositor pays $1,002.000000 (principal
$1,000 + fee $2). The pool gets $500 (50% of the principal, rounded down) and the
SL8 wallet gets $502 ($500 of principal + the $2 fee).

Withdrawal worked examples, principal $1,000: a 6-month bond at maturity has
gross $1,200, fee $2.40 and **claim $1,197.60**; the same bond withdrawn after
the lock but before maturity has gross $1,000, fee $2 and **claim $998**; a
9-month bond at maturity has gross $1,300, fee $2.60 and **claim $1,297.40**.

### `deposit_bond`

Opens a bond. The depositor signs and pays `principal + fee` from their own
token account. The principal is split like a `deposit_fee` payment: half
(rounded down) to the payout pool of the same mint, the rest to the SL8 wallet;
the whole fee goes to the SL8 wallet too. Every check runs before any account is
created or any token moves.

Arguments (Borsh, 17 bytes; 25 with the discriminator): `deposit_index: u64`,
`principal: u64`, `term: BondTerm` (one byte, see the table above).
`deposit_index` must equal the wallet's next index (the first deposit uses `0`,
and it never repeats); otherwise `BondIndexMismatch`.

| # | flags | account |
|---|---|---|
| 0 | signer, writable | `depositor` (pays the principal and fee, and the rent of the position and, on the first deposit, the tracker) |
| 1 | writable | `vault_state` (global open-principal counter) |
| 2 | writable | `depositor_token_account` (the depositor's own token account for `mint`) |
| 3 |  | `mint` (the vault's USDC or USDT mint) |
| 4 | writable | `pool_token_account` (the vault's pool for `mint`) |
| 5 | writable | `sl8_token_account` (a `mint` token account owned by the SL8 wallet) |
| 6 | writable | `bond_position` (`[b"bond", depositor, deposit_index]`, must not already be an initialised account; a lamport-only, system-owned address is fine) |
| 7 | writable | `bond_cap_tracker` (`[b"bond_cap", depositor]`; created by the first deposit) |
| 8 |  | `token_program` (classic SPL Token) |
| 9 |  | `system_program` |

If the depositor's token account holds less than `principal + fee` the call
fails with `InsufficientTokenBalance`.

`BondPosition` data (Anchor, 100 bytes): `depositor`, `deposit_index` u64,
`mint`, `principal` u64, `term` u8, `interest_bps` u16, `created_at` i64 (unix
seconds), `bump`. `BondCapTracker` data (Anchor, 57 bytes): `depositor`,
`open_principal_total` u64, `next_deposit_index` u64, `bump`.

### `request_bond_payout`

Withdraws a bond. Only the depositor may call it. It closes the `BondPosition`
(rent back to the depositor, so it can never be withdrawn twice), reduces the
wallet's open principal, and queues what the bond is worth as a **kind 1
`PayoutClaim`** that the heartbeat settles with the same ratio as every trader
claim. **No tokens move now.** The claim amount is the gross amount (per the
rules above) minus the 0.2% withdrawal fee; if that is zero the call fails with
`ZeroAmount`. Before the lock ends it fails with `BondLocked`.

Arguments (Borsh, 8 bytes; 16 with the discriminator): `deposit_index: u64`.

| # | flags | account |
|---|---|---|
| 0 | signer, writable | `depositor` (receives the closed position's rent; pays the new claim's rent) |
| 1 | writable | `vault_state` (global counters and the current cycle id) |
| 2 | writable | `bond_position` (the position being withdrawn; closed by the call) |
| 3 | writable | `bond_cap_tracker` (the depositor's tracker, which must exist) |
| 4 | writable | `payout_claim` (`[b"bond_claim", depositor, deposit_index]`, must not already be an initialised account; a lamport-only, system-owned address is fine) |
| 5 |  | `system_program` |

A missing, closed, forged, foreign or mismatched position or tracker all fail
with `InvalidBondPosition`. All validation runs before any state change or
account creation (the claim is created, then the tracker is written and the
position closed, all atomically). Because
the position is closed by the request, the `deposit_index` (and so the claim
address) is used exactly once; the tracker's `next_deposit_index` never goes
back down.

The claim is paid to the depositor's USDC and USDT ATAs by `settle_claims`; if
either does not exist the claim is skipped each cycle until it does.

## `admin_withdraw_marketing_funds`

A **documented exception to "no admin key on money"**: both admins together may
take up to 75% of one pool's live balance to the SL8 wallet's token account, with
no deduction for open claims, bond liabilities or the current cycle. It has a
builder in this crate; see its rustdoc for the account list, the reserve rule
(`reserve = max(stored_floor, ceil(live * 25%))`), the errors, and the 17-byte
data layout (`pool` byte, then `amount` u64 LE).
