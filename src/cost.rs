//! ⭐ What something costs, as one plain quantity: [`Cost::cost`] gives it, [`Candidates`] picks the cheapest of several.
//!
//! The quantity is the implementer's ([`Quantity`]: anything that adds up and compares, such as `f64`
//! or `u64`, or the implementer's own unit type); what it means and how it is computed belong to the
//! implementer. Measured numbers only: what cannot be costed is an `Err`, never a guess.

use core::ops::Add;

/// A quantity of cost: it adds up and compares.
pub trait Quantity: Copy + PartialOrd + Add<Output = Self> {}

impl<T: Copy + PartialOrd + Add<Output = T>> Quantity for T {}

/// Something that can be costed.
pub trait Cost {
    /// Why it cannot be costed.
    type Error;
    /// The quantity it is costed in.
    type Unit: Quantity;

    /// What it costs.
    ///
    /// # Errors
    /// It cannot be costed (not measured, not known statically, …).
    fn cost(&self) -> Result<Self::Unit, Self::Error>;
}

/// Several ways of doing the same thing.
pub trait Candidates {
    /// The cheapest (the first of equals) when every one can be costed; otherwise `rule`, the
    /// caller's own choice — never a pick among only some of them.
    fn cheapest_or(&self, rule: usize) -> usize;
}

impl<T: Cost> Candidates for [T] {
    fn cheapest_or(&self, rule: usize) -> usize {
        let mut best: Option<(usize, T::Unit)> = None;
        for (i, c) in self.iter().enumerate() {
            let Ok(x) = c.cost() else { return rule };
            if best.is_none_or(|(_, m)| x < m) {
                best = Some((i, x));
            }
        }
        best.map_or(rule, |(i, _)| i)
    }
}
