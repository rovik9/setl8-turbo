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
