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
