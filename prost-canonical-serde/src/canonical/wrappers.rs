use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{CanonicalDeserialize, CanonicalSerialize};

/// Wraps a value to serialize it using canonical protobuf JSON rules.
pub struct Canonical<'a, T: CanonicalSerialize + ?Sized> {
    value: &'a T,
}

impl<'a, T: CanonicalSerialize + ?Sized> Canonical<'a, T> {
    pub fn new(value: &'a T) -> Self {
        Self { value }
    }
}

impl<T: CanonicalSerialize + ?Sized> Serialize for Canonical<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize_canonical(serializer)
    }
}

/// Wraps a value for canonical protobuf JSON deserialization.
pub struct CanonicalValue<T>(pub T);

impl<'de, T: CanonicalDeserialize> Deserialize<'de> for CanonicalValue<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize_canonical(deserializer).map(CanonicalValue)
    }
}

impl<T: CanonicalSerialize> CanonicalSerialize for Box<T> {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_ref().serialize_canonical(serializer)
    }
}

impl<T: CanonicalDeserialize> CanonicalDeserialize for Box<T> {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize_canonical(deserializer).map(Box::new)
    }
}

/// Wraps an `Option` for canonical protobuf JSON deserialization.
pub struct CanonicalOption<T>(pub Option<T>);

impl<'de, T: CanonicalDeserialize> Deserialize<'de> for CanonicalOption<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<CanonicalValue<T>>::deserialize(deserializer)?;
        Ok(CanonicalOption(value.map(|value| value.0)))
    }
}

/// Wraps a slice to serialize as a canonical JSON array.
pub struct CanonicalSeq<'a, T: CanonicalSerialize> {
    values: &'a [T],
}

impl<'a, T: CanonicalSerialize> CanonicalSeq<'a, T> {
    pub fn new(values: &'a [T]) -> Self {
        Self { values }
    }
}

impl<T: CanonicalSerialize> Serialize for CanonicalSeq<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.values.len()))?;
        for value in self.values {
            let value = Canonical::new(value);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}

/// Wraps a vector for canonical protobuf JSON deserialization.
pub struct CanonicalVec<T>(pub Vec<T>);

impl<'de, T: CanonicalDeserialize> Deserialize<'de> for CanonicalVec<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<T>);

        impl<'de, T: CanonicalDeserialize> de::Visitor<'de> for Visitor<T> {
            type Value = CanonicalVec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<CanonicalValue<T>>()? {
                    values.push(value.0);
                }
                Ok(CanonicalVec(values))
            }

            fn visit_unit<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(CanonicalVec(Vec::new()))
            }

            fn visit_none<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_unit()
            }
        }

        deserializer.deserialize_any(Visitor(PhantomData))
    }
}
