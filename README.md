# setl8-shared-interfaces

Shared CPI interface definitions for cross-program calls between the Setl8
vault program and sector programs (lev-trading, and future
options/predictions/etc).

## What this crate is

- Instruction argument structs (the Borsh-serialized data payload per
  instruction)
- Instruction builder functions that construct a raw
  `solana_program::instruction::Instruction`
- Documented account ordering per instruction, including which account
  carries the CPI-auth identity check on the vault side
- This `CHANGELOG.md` and its versioning convention

## What this crate is *not*

- No account/state struct definitions. `ProductRegistry`, `TraderState`,
  `PayoutClaim`, `BondPosition`, `BondCapTracker`, etc. live in `setl8-vault`,
  not here. The one exception is the sector-owned **payout tally**, whose byte
  layout is a cross-program contract; see "Payout tally" below.
- No business logic. No floor math, no graduation checks, no abandonment
  sweeps.
- No protocol numbers. No fees, caps, or challenge costs baked in.

Sector programs and the vault program each depend on this crate as an
external versioned dependency, not a copied file, so a signature change here
fails to compile in a consumer that hasn't updated to match.

## Instructions

| Instruction | Auth context |
|---|---|
| `register_product` | 2-of-2 admin multisig (SL8 + Rov) |
| `reactivate_product` | 2-of-2 admin multisig |
| `update_product_config` | 2-of-2 admin multisig |
| `admin_withdraw_marketing_funds` | 2-of-2 admin multisig; takes `(pool, amount)`, up to 75% of one pool, fixed SL8 wallet destination |
| `pause_product` | 2-of-2 admin multisig |
| `deposit_fee` | PDA-signer (`sector_authority`) |
| `deposit_reset` | PDA-signer (`sector_authority`) |
| `record_activity` | PDA-signer (`sector_authority`) |
| `mark_abandoned` | none (permissionless; any signer) |
| `request_payout` | PDA-signer (`sector_authority`) |
| `flag_trader_failed` | PDA-signer (`sector_authority`) |

Full argument shapes and account ordering are documented as doc comments on
each builder function in `src/instructions/`.

Vault instructions this crate has **no builders** for (the heartbeat, tally
reconciliation and the bond vault) are documented with exact account lists,
flags, arguments, PDA seeds and rules in
[`docs/vault-instruction-reference.md`](docs/vault-instruction-reference.md):
`begin_heartbeat`, `settle_claims`, `finalize_heartbeat`, `reconcile_product`,
`deposit_bond`, `request_bond_payout`, the claim kinds, and the bond terms,
caps and fees.

## Open placeholders -- confirm before `setl8-vault` depends on this crate

These are explicit, flagged assumptions this crate makes. None of them are
silently buried in code -- every one has a matching doc comment at its point
of use, but they're listed together here so they're easy to review in one
pass. (Two items resolved in v0.2.0 -- the CPI-auth mechanism and a naming
inconsistency -- have been removed from this list; see `CHANGELOG.md`.)

1. **Discriminator scheme.** Every instruction uses an 8-byte Anchor-style
   discriminator: `sha256("global:<instruction_name>")[..8]`. This is
   grounded (not a blind guess) -- the actual `setl8-vault` repo on this
   machine already depends on `anchor-lang 0.32.1` -- but it's still an
   assumption about how the vault's `#[program]` module names its
   instructions and that it doesn't override Anchor's default discriminator.
   **If that changes, every `*_DISCRIMINATOR` constant in
   `src/instructions/*.rs` needs regenerating.**

2. **Borsh version pin (`0.10`).** Chosen to match `anchor-lang 0.32.1`'s own
   `borsh ^0.10.3` dependency, since `AnchorSerialize`/`AnchorDeserialize` on
   the vault side are thin wrappers over these same borsh 0.10 traits. If
   `setl8-vault` upgrades Anchor to a release that moves to borsh 1.x, this
   crate's `Cargo.toml` pin needs to move with it or serialized instruction
   data stops round-tripping.

3. **`remaining_accounts: &[AccountMeta]` passthrough design.** This crate
   only fixes `sector_authority` and `product_registry`; it does
   not derive or hardcode the vault's PDAs, mints, pools or wallets, because
   those depend on the vault's deployment. Every builder appends a
   caller-supplied `remaining_accounts` slice after its fixed accounts. The
   vault's current layout for the token-moving instructions (`deposit_fee`,
   `deposit_reset`, `request_payout`) is documented position by position in
   each builder's doc comment (see "Token movement" below); the vault
   validates the accounts with its own constraints (seeds, `VaultState`
   fields, owner/mint checks, program ids), so a wrong address is rejected
   rather than silently accepted. The open slice also lets the vault append
   accounts without a breaking crate version (the docs here then need
   updating).

4. **`product_registry` writable/readonly flags.** Marked writable on every
   registry-touching builder. Checked against the vault: `deposit_fee`,
   `deposit_reset` and `request_payout` declare it `mut`; `flag_trader_failed`
   only reads it; v0.4.0 corrects the builder to read-only (previously it took
   a needless write lock and forced the sector's outer transaction to mark the
   registry writable).

5. **No `VAULT_PROGRAM_ID` constant.** `setl8-vault` isn't deployed, so every
   builder takes `vault_program_id: Pubkey` as an explicit parameter instead
   of a hardcoded constant. TODO(vault-deploy): once `setl8-vault` has a real
   deployed address, consider adding a `pub const VAULT_PROGRAM_ID: Pubkey`
   to this crate for convenience.

6. **No admin wallet addresses hardcoded.** The SL8/Rov multisig signers and
   the marketing destination wallet are all `Pubkey` builder parameters, not
   constants -- so a wallet rotation never requires a new crate version. This
   also means this crate holds no secret material of any kind; it only ever
   deals in public keys supplied by the caller.

7. **`ChallengeSize` shape.** Defined as `{ size: u64, cost: u64 }` per "a
   size/cost pair" in the original spec. Confirm this is sufficient -- no
   currency/denomination field, no min/max bounds, nothing else.

## Token movement (vault Module 2b)

The vault moves the tokens; sector programs do not.

- **`deposit_fee` / `deposit_reset`**: the **trader pays the vault directly**
  from their own token account (USDC or USDT, classic SPL Token, 6 decimals).
  The trader must **sign the transaction**, and the sector's CPI only works
  when that signature is in the outer transaction. The payment splits as
  `pool = floor(amount * fee_split_bps / 10_000)` to the payout pool of the
  same mint, the exact remainder to the SL8 wallet's token account.
- **`request_payout`** (changed in v0.4.0, see below): no longer pays. It
  **queues** a claim; tokens move later through the vault's heartbeat.

The exact account order and flags for each are in the doc comments on
`deposit_fee`, `deposit_reset` and `request_payout` in `src/instructions/`.

## Queued payouts and the heartbeat

`request_payout` records a `PayoutClaim` owed to the trader and **moves no
tokens**. The vault's account list is 7 accounts: `sector_authority`,
`product_registry` (writable), then `remaining_accounts` =
`[trader_state (writable), vault_state (writable), payout_claim (writable),
payer (signer, writable), system_program]`. `payout_claim` is the PDA
`[b"payout_claim", trader_state, proposed_request_id u64 LE]` under the
**vault's** program ID (`derive_payout_claim`); `payer` pays its rent. Only
`sector_authority` and `payer` sign. There are no token accounts in this
instruction.

The return data is a `PayoutOutcome`, and a successful CPI alone means
nothing:

- `Paid` = **accepted and queued**, no tokens moved. Tell the trader the payout
  is *requested*, not paid.
- `Abandoned` = the challenge was past its inactivity window; no claim was
  created.

Payment happens later through three permissionless vault instructions that
anyone may run (this crate documents them in the `heartbeat` module and has no
builders for them): `begin_heartbeat` snapshots the total owed and the total in
both pools, `settle_claims` pays batches of claims **pro rata** (the same ratio
for every claim; larger pool first, straight to the trader's associated token
accounts), and `finalize_heartbeat` closes the cycle. Whatever is not paid
stays owed and is carried over to the next cycle.

## Payout tally

Every sector program keeps a small account, its **payout tally**, that the vault
reads to reconcile what the sector asked to be paid against the vault's own
records. **The vault enforces it:** its permissionless `reconcile_product`
instruction (vault commit `686bd71`) compares the tally with the product's
books and pauses the product on any mismatch. The heartbeat does not call it; a
keeper runs it for every product before each `begin_heartbeat`.

- PDA: `derive_payout_tally(&sector_program_id)`, seed `b"payout_tally"`, owned
  by the **sector** program.
- Layout, little-endian, no Anchor discriminator, at least 25 bytes (a sector
  may append its own data after byte 25):

  | bytes | field |
  |---|---|
  | 0..8 | magic `b"SL8TALLY"` |
  | 8 | version `1` |
  | 9..17 | `requested_count` (`u64`) |
  | 17..25 | `requested_total` (`u64`, sum of `amount`) |

- Rules: initialise it with `write_into` (0/0) when you create it. In the
  **same transaction** as every `request_payout` whose return data is `Paid`,
  add 1 to the count and the request's `amount` to the total (checked
  arithmetic; fail the transaction on overflow). Update it *after* the CPI,
  once you have read `Paid`; never for `Abandoned` (which returns `Ok`). It only
  ever goes up and the account is never closed or re-initialised. A missing
  account counts as 0/0; an account that does not parse (including an
  allocated, all-zero one) is a mismatch. Anchor sectors hold it as an
  `UncheckedAccount`, since the layout has no Anchor discriminator.
- Use `PayoutTally::parse` and `PayoutTally::write_into` so the encoding is
  identical to what the vault reads. The full contract is in the
  `payout_tally` module docs.

## Optional `idl-build` feature

This crate has no `anchor-lang` dependency by default. A consumer that runs
`anchor build`/`anchor idl build` needs `ChallengeSize` to implement
`anchor_lang::IdlBuild` (Anchor's IDL generator requires this for every type
reachable from an instruction argument), so enable it only when generating an
IDL:

```toml
setl8-shared-interfaces = { git = "https://github.com/rovik9/setl8-turbo", tag = "v0.4.1", features = ["idl-build"] }
```

With default features (no `features = [...]`), `anchor-lang` does not appear
in the dependency tree at all -- confirmed via `cargo tree`.

## Versioning

**Not yet decided** between (a) a private Cargo registry with semver releases
or (b) a git dependency pinned to a tag/commit per consumer. This repo is
scaffolded assuming **(b), a git-tag-pinned dependency**, since it's the
simplest option with zero extra infra to start with -- e.g.:

```toml
setl8-shared-interfaces = { git = "https://github.com/rovik9/setl8-turbo", tag = "v0.4.1" }
```

This is a placeholder decision, not a final one -- confirm before
`setl8-vault` starts depending on this crate for real. See `CHANGELOG.md` for
the tagging convention this assumes (`vMAJOR.MINOR.PATCH`; while pre-1.0,
MINOR is the breaking unit per standard Cargo semver, so bump MINOR on any
breaking change to a discriminator, argument shape, or documented account
order).

## Building

```bash
cargo build
cargo test
```

When the vault checkout sits next to this repo (`../setl8-vault`), some tests
also parse the vault's `#[derive(Accounts)]` source and fail if a builder drifts
from it; they are skipped otherwise. For a release check run
`SETL8_REQUIRE_VAULT_SRC=1 cargo test` (optionally with `SETL8_VAULT_SRC=<path
to programs/core-vault/src>`) so a missing checkout is a failure, not a skip.
