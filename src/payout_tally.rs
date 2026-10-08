//! The **payout tally**: a small account every sector program maintains so the
//! vault can reconcile what the sector asked it to pay against what the vault
//! itself recorded.
//!
//! # Status
//!
//! This is the **agreed contract**. As of vault commit `be97396` the vault does
//! **not yet read or enforce** the tally: nothing in the vault looks at this
//! account, and nothing is paused over it today. Sector programs should
//! implement it now so they are compliant when the vault starts checking.
//!
//! # Contract (read this before writing a sector program)
//!
//! * The tally is a PDA **owned by the sector program**, derived with
//!   [`derive_payout_tally`] (`[PAYOUT_TALLY_SEED]` under the sector's own
//!   program ID). The vault is to read it; only the sector writes it.
//! * Initialise it with [`PayoutTally::write_into`] (`0 / 0`) when the sector
//!   creates the account. An account that exists but holds no valid header (for
//!   example freshly allocated and all zero) does not parse and counts as a
//!   mismatch, so never leave it uninitialised.
//! * For **every** `request_payout` the vault accepts, i.e. whose return data
//!   (read with `get_return_data`, checked to come from the vault program) is
//!   `PayoutOutcome::Paid`, increase `requested_count` by exactly 1 and
//!   `requested_total` by exactly the `amount` passed, **in the same
//!   transaction**. The simplest correct way is to update the tally *after* the
//!   CPI, once the return data says `Paid`. Updating it *before* the CPI is
//!   allowed only if the sector then fails the whole transaction whenever the
//!   outcome is anything other than `Paid`: `request_payout` returns `Ok` for
//!   `Abandoned`, so an unconditional early update would count a request that
//!   was never queued.
//! * Do **not** update the tally for `PayoutOutcome::Abandoned` (no claim was
//!   created, so nothing was requested). Only `Paid` ("accepted and queued") is
//!   counted.
//! * Use checked arithmetic. If either addition would overflow `u64`, **fail
//!   the transaction**; never saturate or wrap.
//! * The tally is **permanent**: both fields only ever go up. Never decrease
//!   it, and never close, shrink or re-initialise the account, including during
//!   a program upgrade or migration. It counts *requests*, not what is still
//!   owed, so paying, shrinking or closing a claim does not change it.
//! * The vault is to compare `requested_count` and `requested_total` with its
//!   own records for the product and **pause that product on any mismatch**. A
//!   **missing** tally (no account at the PDA address, or one that holds no
//!   data and is not owned by the sector, such as an address that only
//!   received lamports) counts as `0 / 0`. An account that exists but does not
//!   parse (see [`PayoutTally::parse`]) is a mismatch.
//!
//! # Byte layout (version 1, little-endian, no Anchor discriminator)
//!
//! | bytes  | field             | value                                  |
//! |--------|-------------------|----------------------------------------|
//! | 0..8   | magic             | `b"SL8TALLY"`                          |
//! | 8      | version           | `1`                                    |
//! | 9..17  | `requested_count` | `u64` LE, number of accepted requests  |
//! | 17..25 | `requested_total` | `u64` LE, sum of their `amount`s       |
//!
//! The account may be **longer** than [`PAYOUT_TALLY_MIN_LEN`] (25 bytes): a
//! sector may append its own data after byte 25 and the vault ignores it.
//! Nothing here depends on Anchor. The layout has no 8-byte Anchor
//! discriminator, so an Anchor sector must hold the tally as an
//! `UncheckedAccount` / `AccountInfo` and use [`PayoutTally::parse`] and
//! [`PayoutTally::write_into`] by hand; `#[account]` would add a discriminator.

use std::fmt;

use solana_program::pubkey::Pubkey;

/// Seed of a sector program's payout-tally PDA: `[PAYOUT_TALLY_SEED]` under
/// the **sector** program's ID. See [`derive_payout_tally`].
pub const PAYOUT_TALLY_SEED: &[u8] = b"payout_tally";

/// First 8 bytes of every payout-tally account.
pub const PAYOUT_TALLY_MAGIC: [u8; 8] = *b"SL8TALLY";

/// The only layout version this crate reads and writes.
pub const PAYOUT_TALLY_VERSION: u8 = 1;

/// Smallest valid payout-tally account: 8 magic + 1 version + 8 count + 8
/// total. Longer accounts are fine.
pub const PAYOUT_TALLY_MIN_LEN: usize = 25;

const VERSION_OFFSET: usize = 8;
const COUNT_OFFSET: usize = 9;
const TOTAL_OFFSET: usize = 17;

/// Derives a sector program's payout-tally PDA under **its own** program ID.
/// Returns `(pubkey, bump)`. The account is owned by the sector program, not
/// the vault; the vault finds it by making the same derivation from the
/// registered `product_program_id`.
pub fn derive_payout_tally(product_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PAYOUT_TALLY_SEED], product_program_id)
}

/// Why a payout-tally account could not be read or written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TallyError {
    /// The buffer is shorter than [`PAYOUT_TALLY_MIN_LEN`].
    TooShort {
        /// Length that was supplied.
        len: usize,
    },
    /// The first 8 bytes are not `b"SL8TALLY"`.
    BadMagic,
    /// The version byte is not [`PAYOUT_TALLY_VERSION`].
    UnsupportedVersion(u8),
}

impl fmt::Display for TallyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TallyError::TooShort { len } => write!(
                f,
                "payout tally too short: {len} bytes, need at least {PAYOUT_TALLY_MIN_LEN}"
            ),
            TallyError::BadMagic => write!(f, "payout tally has the wrong magic bytes"),
            TallyError::UnsupportedVersion(v) => write!(
                f,
                "unsupported payout tally version {v} (expected {PAYOUT_TALLY_VERSION})"
            ),
        }
    }
}

impl std::error::Error for TallyError {}

/// The two numbers a sector program keeps in its payout tally. See the
/// [module docs](self) for the full contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PayoutTally {
    /// How many `request_payout` calls the vault accepted (returned `Paid`).
    pub requested_count: u64,
    /// Sum of the `amount` of those requests, in the vault's 6-decimal base
    /// units.
    pub requested_total: u64,
}

impl PayoutTally {
    /// Reads a tally from an account's data. Rejects, in this order: a buffer
    /// shorter than [`PAYOUT_TALLY_MIN_LEN`], wrong magic, and any version other
    /// than 1. Bytes after the first 25 are ignored.
    pub fn parse(data: &[u8]) -> Result<PayoutTally, TallyError> {
        if data.len() < PAYOUT_TALLY_MIN_LEN {
            return Err(TallyError::TooShort { len: data.len() });
        }
        if data[..PAYOUT_TALLY_MAGIC.len()] != PAYOUT_TALLY_MAGIC {
            return Err(TallyError::BadMagic);
        }
        let version = data[VERSION_OFFSET];
        if version != PAYOUT_TALLY_VERSION {
            return Err(TallyError::UnsupportedVersion(version));
        }
        Ok(PayoutTally {
            requested_count: read_u64(data, COUNT_OFFSET),
            requested_total: read_u64(data, TOTAL_OFFSET),
        })
    }

    /// Writes magic, version, count and total into the first 25 bytes of
    /// `out`, leaving any bytes after them untouched (they are the sector's
    /// own data). Fails with [`TallyError::TooShort`] if `out` is shorter than
    /// [`PAYOUT_TALLY_MIN_LEN`]; nothing is written in that case.
    ///
    /// Sectors should write the tally with this function so the encoding is
    /// byte-for-byte what [`PayoutTally::parse`] and the vault read.
    pub fn write_into(&self, out: &mut [u8]) -> Result<(), TallyError> {
        if out.len() < PAYOUT_TALLY_MIN_LEN {
            return Err(TallyError::TooShort { len: out.len() });
        }
        out[..PAYOUT_TALLY_MAGIC.len()].copy_from_slice(&PAYOUT_TALLY_MAGIC);
        out[VERSION_OFFSET] = PAYOUT_TALLY_VERSION;
        out[COUNT_OFFSET..COUNT_OFFSET + 8].copy_from_slice(&self.requested_count.to_le_bytes());
        out[TOTAL_OFFSET..TOTAL_OFFSET + 8].copy_from_slice(&self.requested_total.to_le_bytes());
        Ok(())
    }
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}
