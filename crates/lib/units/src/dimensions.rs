//! 型レベル次元と別名の共有定義。
use crate::{Dimension as RuntimeDimension, Rational};
#[path = "typed/aliases.rs"]
mod aliases;
#[path = "typed/private.rs"]
pub(crate) mod private;
pub use aliases::*;
/// A type-level physical dimension with an exact runtime counterpart.
///
/// The trait is sealed: dimensions are supplied through this module's named aliases so their
/// compile-time and runtime representations cannot disagree.
pub trait Dimension: private::Sealed {
    /// The runtime metadata represented by this type-level dimension.
    const DIM: RuntimeDimension;
}
