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
