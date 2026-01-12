//! Canonical JSON helpers and adapters for prost types.
//!
//! Most users should derive `CanonicalSerialize` and `CanonicalDeserialize` on
//! prost-generated types and then use `serde_json` directly. This module exists
//! for advanced cases, such as wrapping values when manual control is needed.

mod enums;
mod error;
mod map;
mod number;
mod scalar;
mod wkt;
mod wrappers;

pub use enums::{
    CanonicalEnum, CanonicalEnumOption, CanonicalEnumSeq, CanonicalEnumValue, CanonicalEnumVec,
};
pub use error::CanonicalError;
pub use map::{
    CanonicalEnumMap, CanonicalEnumMapRef, CanonicalMap, CanonicalMapKey, CanonicalMapRef,
    CanonicalMapType,
};
pub use wrappers::{Canonical, CanonicalOption, CanonicalSeq, CanonicalValue, CanonicalVec};
