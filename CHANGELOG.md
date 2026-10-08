# Changelog

All notable changes to `setl8-shared-interfaces` are documented here. Format
loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Versioning convention: this crate is consumed as a **git dependency pinned to a
tag** (e.g. `setl8-shared-interfaces = { git = "https://github.com/rovik9/setl8-turbo", tag = "v0.1.0" }`),
not published to a private Cargo registry. See README.md "Versioning" section
-- this is an explicitly flagged placeholder decision, not a final one. Tags
follow `vMAJOR.MINOR.PATCH`. While this crate is pre-1.0 (per standard Cargo
semver, where MINOR is the breaking unit below 1.0.0), bump MINOR on any
breaking change to an instruction's discriminator, argument struct shape, or
documented account order, since a consumer pinned to an older tag will
otherwise silently CPI-fail or misencode data against a newer vault. Once at
1.0.0, breaking changes bump MAJOR instead.

## [0.4.1] - 2026-10-09

**Fix: `admin_withdraw_marketing_funds` builder now matches the vault. This is a
BREAKING builder change** (the old signature and account list are removed), and
docs for the vault instructions added since v0.4.0. Pin consumers to `v0.4.1`.

- **BREAKING: `admin_withdraw_marketing_funds`.** The old builder took
  `marketing_funds_source` and `destination_wallet`, built 4 accounts plus
  `remaining_accounts`, and its args had only `amount`. None of that matched the
  vault, so the instruction could not have succeeded on-chain. The builder is now
  `admin_withdraw_marketing_funds(vault_program_id, sl8_admin, rov_admin,
  vault_state, mint, pool_token_account, sl8_token_account, token_program,
  args)` with **no `remaining_accounts` parameter**, building exactly the
  vault's 7 accounts (`sl8_admin` and `rov_admin` read-only signers,
  `vault_state` writable, `mint` read-only, `pool_token_account` and
  `sl8_token_account` writable, `token_program` read-only). The args are now
  `AdminWithdrawMarketingFundsArgs { pool: PoolSide, amount: u64 }`, 17 bytes of
  instruction data. The discriminator is unchanged
  (`[149, 0, 251, 20, 103, 248, 17, 186]`, recomputed in a test). Callers must
  update every call site.
- **New `PoolSide` enum** (`Usdc` = 0, `Usdt` = 1; one Borsh byte), exported
  from the crate root.
- **New docs: `docs/vault-instruction-reference.md`**, linked from the README,
  with exact account lists, flags, arguments, PDA seeds and rules for
  `begin_heartbeat`, `settle_claims` (the `[payout_claim, trader_usdc_ata,
  trader_usdt_ata]` triples, `MAX_SETTLE_BATCH` = 6, 400,000 compute units for a
  full batch), `finalize_heartbeat`, `reconcile_product` (and the keeper duty to
  run it for every product before each `begin_heartbeat`), `deposit_bond` and
  `request_bond_payout`, the claim kinds (0 trader, 1 bond, with the bond-claim
  seed), and the bond terms, caps, 0.2% deposit (on top) and withdrawal
  (deducted) fees and maturity-only interest. The `heartbeat` module docs were
  updated to match. No builders were added for these instructions.
- **Docs corrected:** the vault now **reads and enforces the payout tally**
  (`reconcile_product`, vault `686bd71`); v0.4.0 said it did not yet.
  `BondPosition` / `BondCapTracker` are described as living in the vault, not as
  planned.
- **Tests:** discriminators of every builder are recomputed from the instruction
  names; exact data bytes for both pools; account order and flags; and new
  cross-checks that, when the vault checkout is next to this repo, parse the
  vault's `#[derive(Accounts)]` source and compare it with the builders' fixed
  accounts and the documented `remaining_accounts` layouts (names, order, signer
  and writable flags) for `admin_withdraw_marketing_funds`, `flag_trader_failed`,
  `deposit_fee`, `deposit_reset` and `request_payout`. They are **skipped** (with
  a message) when the vault source is absent; set `SETL8_REQUIRE_VAULT_SRC=1`
  (and optionally `SETL8_VAULT_SRC=<path>`) to make absence a failure, e.g. for
  a release check. The existing `admin_withdraw_marketing_funds_shape` test was
  rewritten for the new signature (stricter than before); every other existing
  test is unchanged.
- **Versioning note:** this breaking builder change ships as a PATCH number
  (0.4.1) at the maintainer's instruction; under this file's own convention
  (pre-1.0 MINOR = breaking) it would be 0.5.0. Consumers pin exact git tags,
  so they are unaffected until they move the pin, but anyone using a looser
  `0.4` range should pin `v0.4.1` explicitly and update their call sites.

## [0.4.0] - 2026-10-08

**Minor (pre-1.0 breaking unit): new public API, and `request_payout` is now a
queue.** Pin consumers to `v0.4.0`. The builder signatures, discriminators and
args structs are unchanged, but the **`request_payout` `remaining_accounts`
layout changed from 11 accounts to 7** (the vault no longer pays instantly), so
a sector caller built against the v0.3.2 docs must be updated. One builder
flag also changed (`flag_trader_failed`, below).

- **New: payout tally** (`payout_tally` module). A sector-owned account whose
  fixed layout the vault reads to reconcile requests: `PAYOUT_TALLY_SEED`
  (`b"payout_tally"`), `derive_payout_tally`, `PayoutTally { requested_count,
  requested_total }` with `parse` / `write_into`, `TallyError`,
  `PAYOUT_TALLY_MIN_LEN` (25), `PAYOUT_TALLY_MAGIC` (`b"SL8TALLY"`) and
  `PAYOUT_TALLY_VERSION` (1). Layout: magic (0..8), version (8), count u64 LE
  (9..17), total u64 LE (17..25); longer accounts are fine. Contract: bump it
  by 1 / `amount` in the same transaction as every `request_payout` that
  returns `Paid` (update after the CPI, once return data says `Paid`); never on
  `Abandoned`; it never decreases and is never closed; the vault is specified to
  pause the product on any mismatch, treat a missing account as 0/0 and an
  unparseable one as a mismatch. **The vault does not read the tally yet (as of
  `be97396`)**; this is the agreed contract for sectors to implement now.
- **New: `PAYOUT_CLAIM_SEED` and `derive_payout_claim(vault_program_id,
  trader_state, request_id)`** for the vault's `PayoutClaim` PDA.
- **Docs: `request_payout` is now a queue** (the vault queues claims instead of
  paying instantly). The builder docs state the vault's 7-account layout:
  `sector_authority`, `product_registry` (writable), then
  `trader_state` (writable), `vault_state` (**now writable**),
  `payout_claim` (writable), `payer` (signer, writable) and `system_program`.
  `PayoutOutcome::Paid` now means "accepted and queued; no tokens moved";
  `Abandoned` means no claim was created. All pool/destination token accounts
  were removed from the `request_payout` docs.
- **New `heartbeat` module (docs only)** describing `begin_heartbeat`,
  `settle_claims` and `finalize_heartbeat`: permissionless, pro rata, larger
  pool first, unpaid remainder carried over. No builders were added for them.
- **Fix: `flag_trader_failed` marks `product_registry` read-only.** The vault
  declares it read-only; v0.3.x marked it writable, which took a needless write
  lock and forced the sector's outer transaction to mark it writable. A
  read-only meta is accepted wherever a writable one was, so existing callers
  keep working.
- **Docs:** `register_product` now says its `system_program` remaining account
  is required (an empty slice fails); `record_activity` lists its single
  remaining account; `BondPosition` / `BondCapTracker` are described as
  planned, not present in the vault source.
- New tests: tally roundtrip, golden bytes, every rejection, boundaries, PDA
  golden vectors, the queued `request_payout` layout, and the
  `flag_trader_failed` flags. Existing tests are unchanged.

## [0.3.2] - 2026-10-08

**docs: token-movement account layout; no wire changes** (PATCH). Pin
consumers to `v0.3.2`. Documentation and tests only: no discriminator, args
struct, account order, signer/writable flag or public API item changed.

- **Exact `remaining_accounts` layout** documented on `deposit_fee` (12
  accounts), `deposit_reset` (13) and `request_payout` (11), position by
  position with signer/writable flags, checked against the vault's `Accounts`
  structs.
- **Trader pays the vault directly** (`deposit_fee`, `deposit_reset`): from
  their own USDC/USDT token account (classic SPL Token, 6 decimals), so the
  trader must sign the outer transaction. The sector no longer collects the
  fee. Split rule: `pool = floor(amount * fee_split_bps / 10_000)` to the
  same-mint payout pool, remainder to the SL8 wallet's token account.
- **`request_payout`** pays from the larger pool only (tie -> USDC), fails with
  `InsufficientPoolBalance` if that pool is short, needs both trader token
  accounts, and requires the sector to read `PayoutOutcome` return data before
  reporting a payout (a stale challenge on an otherwise-valid call returns `Ok`
  without paying).
- Removed stale "(assumed)", "confirm against setl8-vault" and "not known to
  this crate by design" wording (also on `flag_trader_failed`, the module docs
  and README). The reason this crate hardcodes no vault PDAs is kept.
- Noted that the vault only reads `product_registry` in `flag_trader_failed`
  although the builder marks it writable (left unchanged in a patch release).
  The vault's current layout is documented, not frozen: if it appends
  accounts, these docs need updating.
- New tests pin each documented layout against the builders.

## [0.3.1] - 2026-10-08

**Bug fix, no wire-format change** (PATCH). Pin consumers to `v0.3.1`.

- **`register_product`: `sl8_admin` is now marked writable** (`[signer,
  writable]`). The vault makes `sl8_admin` the payer that funds the new
  `ProductRegistry` PDA, so it must be writable. v0.3.0 built it read-only,
  which only worked when `sl8_admin` was also the transaction fee payer; with
  any other fee payer the runtime rejects the instruction with
  `PrivilegeEscalation` ("writable privilege escalated"). Found while writing
  LiteSVM integration tests against the vault. Instruction data, discriminator,
  argument layout and account order are unchanged; `rov_admin` stays
  signer + read-only.
- New unit test pins the signer/writable flags of `sl8_admin` and `rov_admin`.
- Existing `register_product_shape` test updated for the new `sl8_admin` flag.

## [0.3.0] - 2026-10-08

**Breaking** (pre-1.0, so MINOR bump). Adds the lifecycle instructions agreed
in the Oct 4-8 design session. Pin consumers to `v0.3.0`.

- **New `record_activity`** (CPI-auth). Sector program reports an order action
  or phase pass so the vault can refresh the challenge's inactivity clock.
  Vault throttles to once per day per challenge.
- **New `mark_abandoned`** (permissionless). Flips an `Active` challenge to
  `Abandoned` once it is past the inactivity window. No heartbeat sweep.
- **New `deposit_reset`** (CPI-auth). Phase-specific reset of a `Failed`
  record at a reduced price. The vault copies `payout_count` and account size
  from the previous record itself, so a sector bug cannot inflate the payout
  cap. `Failed` only, once per record; `Abandoned` is not resettable.
- **New `pause_product`** (2-of-2 admin). Manual planned-upgrade pause,
  mirrors `reactivate_product`. Pauses freeze trader inactivity clocks.
- **New return-data enums `ActivityOutcome` and `PayoutOutcome`.**
  `request_payout` and `record_activity` return `Ok` with these (via Solana
  return data) when they flip a stale challenge to `Abandoned`, because an
  error would revert the status write. Sector programs MUST read them.
- **`DepositFeeArgs` gains `trader_wallet` and `account_size`** (appended).
  The vault keys `TraderState` by wallet + product + challenge and validates
  `(account_size, amount)` against the registry's tiers.
- **`RegisterProductArgs` / `UpdateProductConfigArgs` gain
  `reset_price_bps: Vec<u16>`** (appended): reset price per 0-based phase, in
  basis points of account size.

## [0.2.1] - 2026-09-11

**Non-breaking, opt-in.** Fixes the IDL-generation gap flagged in v0.1.0/
v0.2.0: `ChallengeSize` couldn't satisfy Anchor's `IdlBuild` trait, since this
crate has no `anchor-lang` dependency by design, so a consumer's
`anchor build` failed to compile wherever `ChallengeSize` (or
`Vec<ChallengeSize>`) appeared in an instruction argument struct.

- Added `anchor-lang` as an **optional** dependency, gated behind a new
  `idl-build` feature (`idl-build = ["dep:anchor-lang", "anchor-lang/idl-build"]`).
  With default features, nothing changes -- `anchor-lang` does not appear in
  the dependency tree at all (verified via `cargo tree`).
- `ChallengeSize` now implements `anchor_lang::IdlBuild` when built with
  `--features idl-build`, with a real `create_type()` (not the trait's
  no-op default) so the type is actually included in the generated IDL/TS
  client, not silently dropped.
- Verified against `anchor-lang 0.32.1`'s and `anchor-lang-idl-spec 0.1.0`'s
  actual source (downloaded from crates.io, not assumed from memory) for the
  exact trait and `IdlTypeDef`/`IdlField`/`IdlType` shapes required.
- `cargo build`, `cargo test`, and `cargo clippy --all-targets` all pass
  clean both with and without `--features idl-build`.

## [0.2.0] - 2026-09-11

**Breaking.** Two corrections to the v0.1.0 scaffold, ahead of any real
consumer:

- **CPI-auth mechanism replaced: Instructions-sysvar -> PDA-signer.** v0.1.0
  assumed `setl8-vault` authenticates its CPI caller by reading the
  Instructions sysvar. That mechanism doesn't actually prove CPI-caller
  identity -- the sysvar exposes sibling top-level instructions, not the
  immediate CPI caller, and Solana has no free field for that. Replaced with
  the standard PDA-signer pattern: every sector program derives a
  `sector_authority` PDA under its own program ID using the new
  `SECTOR_AUTHORITY_SEED` constant (`b"setl8_sector_authority"`, see
  `derive_sector_authority` in `src/lib.rs`) and signs its CPI into the vault
  via `invoke_signed`. `deposit_fee`, `request_payout`, and
  `flag_trader_failed` all now take `sector_authority: Pubkey` as a
  `[signer]` account instead of the removed `calling_program_identity`.
  `INSTRUCTIONS_SYSVAR_ID` is gone entirely.
- **Naming unified: `product_id` -> `product_program_id`.** Every instruction
  argument struct now uses `product_program_id` consistently for the sector
  program's on-chain program ID. `UpdateProductConfigArgs` was the only
  holdout (was `product_id` in v0.1.0).

## [0.1.0] - 2026-09-11

Initial scaffold. Defines instruction shapes for the 7 vault instructions
sector programs (and the SL8/Rov admin multisig) call via CPI:

- `register_product`
- `reactivate_product`
- `update_product_config`
- `admin_withdraw_marketing_funds`
- `deposit_fee`
- `request_payout`
- `flag_trader_failed`

No consumer depends on this crate yet -- setl8-vault is not deployed. See
README.md "Open placeholders" for assumptions that need confirmation before
that happens (discriminator scheme, borsh version pin, CPI-auth identity
mechanism, `remaining_accounts` design, versioning strategy).
