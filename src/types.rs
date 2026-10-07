use borsh::{BorshDeserialize, BorshSerialize};

/// A single challenge-size tier a product offers: the position/account size
/// being evaluated, paired with the cost a trader pays to attempt a challenge
/// at that size.
///
/// Used as `Vec<ChallengeSize>` by both `register_product` and
/// `update_product_config` -- a product can offer several tiers at once (e.g.
/// a $10k and a $100k tier), each with its own cost. This crate does not
/// interpret, validate, or bound these values in any way (no min/max checks,
/// no currency assumptions) -- that's setl8-vault's business logic.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChallengeSize {
    /// The challenge's position/account size, in whatever base unit
    /// setl8-vault standardizes on (e.g. USDC base units). Opaque to this
    /// crate.
    pub size: u64,
    /// The cost a trader pays to attempt a challenge at this size, in the
    /// same base unit as `size`. Opaque to this crate.
    pub cost: u64,
}

/// Gives `ChallengeSize` an IDL type definition when a consumer builds with
/// `--features idl-build`.
///
/// Anchor's IDL generator requires every type reachable from an instruction
/// argument to implement `anchor_lang::IdlBuild` -- without this, a
/// consumer's `anchor build` fails to compile wherever `ChallengeSize` (or
/// `Vec<ChallengeSize>`) appears in an instruction argument struct, since
/// this crate deliberately doesn't derive Anchor's own `AnchorSerialize`
/// (that would make `anchor-lang` a hard dependency, which this crate avoids
/// by design -- see README.md). `IdlBuild`'s default `create_type()` returns
/// `None`, which compiles but silently drops the type from the generated
/// IDL/TS client, so this provides a real definition matching the struct
/// above field-for-field instead.
#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for ChallengeSize {
    fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
        use anchor_lang::idl::types::{
            IdlDefinedFields, IdlField, IdlType, IdlTypeDef, IdlTypeDefTy,
        };

        Some(IdlTypeDef {
            name: "ChallengeSize".into(),
            docs: vec![],
            serialization: Default::default(),
            repr: None,
            generics: vec![],
            ty: IdlTypeDefTy::Struct {
                fields: Some(IdlDefinedFields::Named(vec![
                    IdlField {
                        name: "size".into(),
                        docs: vec![],
                        ty: IdlType::U64,
                    },
                    IdlField {
                        name: "cost".into(),
                        docs: vec![],
                        ty: IdlType::U64,
                    },
                ])),
            },
        })
    }
}

/// What `record_activity` did, returned by the vault via Solana return data
/// (a single `u8`). A sector program reads it with
/// `solana_program::program::get_return_data` right after the CPI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ActivityOutcome {
    /// Timestamp refreshed.
    Recorded = 0,
    /// Called again inside the vault's throttle window; nothing changed.
    Throttled = 1,
    /// The challenge was already past its inactivity window, so the vault
    /// flipped it to `Abandoned`. The sector program must treat it as dead
    /// (reject the order/phase event and close any open positions).
    Abandoned = 2,
}

impl ActivityOutcome {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Recorded),
            1 => Some(Self::Throttled),
            2 => Some(Self::Abandoned),
            _ => None,
        }
    }
}

/// What `request_payout` did, returned via Solana return data (a single
/// `u8`). The call can succeed without paying anything: if the challenge is
/// stale, the vault records `Abandoned` and returns `Abandoned`, because an
/// error would revert that state change. A sector program MUST check this
/// before telling the trader they were paid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PayoutOutcome {
    /// Payout accepted and recorded.
    Paid = 0,
    /// Challenge was past its inactivity window; it is now `Abandoned` and
    /// nothing was paid.
    Abandoned = 1,
}

impl PayoutOutcome {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Paid),
            1 => Some(Self::Abandoned),
            _ => None,
        }
    }
}
