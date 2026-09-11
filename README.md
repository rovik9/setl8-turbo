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
  `BondPosition`, `BondCapTracker`, etc. live in `setl8-vault`, not here.
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
| `admin_withdraw_marketing_funds` | 2-of-2 admin multisig, fixed SL8 wallet destination |
| `deposit_fee` | CPI-auth (calling sector program's identity) |
| `request_payout` | CPI-auth |
| `flag_trader_failed` | CPI-auth |

Full argument shapes and account ordering are documented as doc comments on
each builder function in `src/instructions/`.

## Open placeholders -- confirm before `setl8-vault` depends on this crate

These are explicit, flagged assumptions this v0.1.0 scaffold makes. None of
them are silently buried in code -- every one has a matching doc comment at
its point of use, but they're listed together here so they're easy to review
in one pass.

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

3. **CPI-auth identity mechanism.** For `deposit_fee`, `request_payout`, and
   `flag_trader_failed`, the `calling_program_identity` account is assumed to
   be Solana's **Instructions sysvar** (re-exported as
   `setl8_shared_interfaces::INSTRUCTIONS_SYSVAR_ID`), with `setl8-vault`
   authenticating its direct caller via instruction introspection and
   checking that against `ProductRegistry.product_program_id`. This was not
   specified in the original request. If `setl8-vault` instead uses a
   signer-PDA-based identity scheme, every CPI-auth-context builder's
   `calling_program_identity` account needs reconsidering.

4. **`remaining_accounts: &[AccountMeta]` passthrough design.** This crate
   only knows about signers, `product_registry`, and the CPI-auth identity
   account -- it deliberately doesn't invent `TraderState`/`BondPosition`/
   token-account/`system_program` accounts it wasn't told about. Every
   builder appends a caller-supplied `remaining_accounts` slice after its
   documented fixed accounts, so callers can pass whatever `setl8-vault`
   turns out to require without this crate needing a breaking version bump
   every time the vault's internal account layout changes. Confirm this is
   the shape you want once `setl8-vault`'s real account layout exists --
   the alternative is fully fixed, explicit account lists per instruction,
   which would need that layout decided first.

5. **`product_registry` writable/readonly flags.** Marked writable on every
   instruction, on the assumption each one mutates some field of it (new
   product entry, updated config, fee/payout/failure bookkeeping). These are
   conservative defaults, not confirmed against real vault semantics -- some
   may turn out to be read-only.

6. **No `VAULT_PROGRAM_ID` constant.** `setl8-vault` isn't deployed, so every
   builder takes `vault_program_id: Pubkey` as an explicit parameter instead
   of a hardcoded constant. TODO(vault-deploy): once `setl8-vault` has a real
   deployed address, consider adding a `pub const VAULT_PROGRAM_ID: Pubkey`
   to this crate for convenience.

7. **No admin wallet addresses hardcoded.** The SL8/Rov multisig signers and
   the marketing destination wallet are all `Pubkey` builder parameters, not
   constants -- so a wallet rotation never requires a new crate version. This
   also means this crate holds no secret material of any kind; it only ever
   deals in public keys supplied by the caller.

8. **`ChallengeSize` shape.** Defined as `{ size: u64, cost: u64 }` per "a
   size/cost pair" in the original spec. Confirm this is sufficient -- no
   currency/denomination field, no min/max bounds, nothing else.

9. **`update_product_config`'s `product_id` vs. `register_product`'s
   `product_program_id`.** Kept exactly as specified even though the field
   names differ for what looks like the same concept. See the doc comment on
   `UpdateProductConfigArgs` in `src/instructions/update_product_config.rs`.

## Versioning

**Not yet decided** between (a) a private Cargo registry with semver releases
or (b) a git dependency pinned to a tag/commit per consumer. This repo is
scaffolded assuming **(b), a git-tag-pinned dependency**, since it's the
simplest option with zero extra infra to start with -- e.g.:

```toml
setl8-shared-interfaces = { git = "https://github.com/rovik9/setl8-turbo", tag = "v0.1.0" }
```

This is a placeholder decision, not a final one -- confirm before
`setl8-vault` starts depending on this crate for real. See `CHANGELOG.md` for
the tagging convention this assumes (`vMAJOR.MINOR.PATCH`, MAJOR bump on any
breaking change to a discriminator, argument shape, or documented account
order).

## Building

```bash
cargo build
cargo test
```
