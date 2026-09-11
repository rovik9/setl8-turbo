# Changelog

All notable changes to `setl8-shared-interfaces` are documented here. Format
loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Versioning convention: this crate is consumed as a **git dependency pinned to a
tag** (e.g. `setl8-shared-interfaces = { git = "https://github.com/rovik9/setl8-turbo", tag = "v0.1.0" }`),
not published to a private Cargo registry. See README.md "Versioning" section
-- this is an explicitly flagged placeholder decision, not a final one. Tags
follow `vMAJOR.MINOR.PATCH`; bump MAJOR on any breaking change to an
instruction's discriminator, argument struct shape, or documented account
order, since a consumer pinned to an older tag will otherwise silently
CPI-fail or misencode data against a newer vault.

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
