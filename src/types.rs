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
        use anchor_lang::idl::types::{IdlDefinedFields, IdlField, IdlType, IdlTypeDef, IdlTypeDefTy};

        Some(IdlTypeDef {
            name: "ChallengeSize".into(),
            docs: vec![],
            serialization: Default::default(),
            repr: None,
            generics: vec![],
            ty: IdlTypeDefTy::Struct {
                fields: Some(IdlDefinedFields::Named(vec![
                    IdlField { name: "size".into(), docs: vec![], ty: IdlType::U64 },
                    IdlField { name: "cost".into(), docs: vec![], ty: IdlType::U64 },
                ])),
            },
        })
    }
}
