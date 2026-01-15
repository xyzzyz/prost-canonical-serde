use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use core::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[cfg(feature = "std")]
use std::collections::HashMap;

use super::CanonicalError;
use super::enums::{CanonicalEnum, CanonicalEnumValue};
use super::wrappers::CanonicalValue;
use crate::ProstEnum;

/// Key conversion helper for canonical protobuf JSON maps.
#[expect(
    clippy::missing_errors_doc,
    reason = "Implementations describe key parsing failures in their error strings."
)]
pub trait CanonicalMapKey: Sized {
    /// Returns `CanonicalError` if the input does not represent a valid key.
    fn from_key(value: &str) -> Result<Self, CanonicalError>;
}

impl CanonicalMapKey for String {
    fn from_key(value: &str) -> Result<Self, CanonicalError> {
        Ok(value.to_string())
    }
}

impl CanonicalMapKey for bool {
    fn from_key(value: &str) -> Result<Self, CanonicalError> {
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(CanonicalError::new("invalid bool map key")),
        }
    }
}

impl CanonicalMapKey for i32 {
    fn from_key(value: &str) -> Result<Self, CanonicalError> {
        value
            .parse()
            .map_err(|_| CanonicalError::new("invalid i32 map key"))
    }
}

impl CanonicalMapKey for i64 {
    fn from_key(value: &str) -> Result<Self, CanonicalError> {
        value
            .parse()
            .map_err(|_| CanonicalError::new("invalid i64 map key"))
    }
}

impl CanonicalMapKey for u32 {
    fn from_key(value: &str) -> Result<Self, CanonicalError> {
        value
            .parse()
            .map_err(|_| CanonicalError::new("invalid u32 map key"))
    }
}

impl CanonicalMapKey for u64 {
    fn from_key(value: &str) -> Result<Self, CanonicalError> {
        value
            .parse()
            .map_err(|_| CanonicalError::new("invalid u64 map key"))
    }
}

/// Map type abstraction to handle both hash and btree maps.
pub trait CanonicalMapType: Default {
    type Key: CanonicalMapKey;
    type Value;

    fn insert(&mut self, key: Self::Key, value: Self::Value);
}

#[cfg(feature = "std")]
impl<K, V, S> CanonicalMapType for HashMap<K, V, S>
where
    K: CanonicalMapKey + Eq + core::hash::Hash,
    S: core::hash::BuildHasher + Default,
{
    type Key = K;
    type Value = V;

    fn insert(&mut self, key: Self::Key, value: Self::Value) {
        HashMap::insert(self, key, value);
    }
}

impl<K, V> CanonicalMapType for BTreeMap<K, V>
where
    K: CanonicalMapKey + Ord,
{
    type Key = K;
    type Value = V;

    fn insert(&mut self, key: Self::Key, value: Self::Value) {
        BTreeMap::insert(self, key, value);
    }
}

/// Wraps a map reference with canonical JSON serialization.
pub struct CanonicalMapRef<'a, M> {
    values: &'a M,
}

impl<'a, M> CanonicalMapRef<'a, M> {
    pub fn new(values: &'a M) -> Self {
        Self { values }
    }
}

impl<M, K, V> Serialize for CanonicalMapRef<'_, M>
where
    for<'b> &'b M: core::iter::IntoIterator<Item = (&'b K, &'b V)>,
    K: CanonicalMapKey + ToString,
    V: crate::CanonicalSerialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;
        for (key, value) in self.values {
            let value = super::wrappers::Canonical::new(value);
            map.serialize_entry(&key.to_string(), &value)?;
        }
        map.end()
    }
}

/// Wraps a map for canonical protobuf JSON deserialization.
pub struct CanonicalMap<M>(pub M);

impl<'de, M> Deserialize<'de> for CanonicalMap<M>
where
    M: CanonicalMapType,
    M::Value: crate::CanonicalDeserialize,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<M>(PhantomData<M>);

        impl<'de, M> de::Visitor<'de> for Visitor<M>
        where
            M: CanonicalMapType,
            M::Value: crate::CanonicalDeserialize,
        {
            type Value = CanonicalMap<M>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut values = M::default();
                while let Some(key) = map.next_key::<String>()? {
                    let key = M::Key::from_key(&key).map_err(de::Error::custom)?;
                    let value = map.next_value::<CanonicalValue<M::Value>>()?.0;
                    values.insert(key, value);
                }
                Ok(CanonicalMap(values))
            }

            fn visit_unit<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(CanonicalMap(M::default()))
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

/// Wraps a map reference with enum values for canonical JSON serialization.
pub struct CanonicalEnumMapRef<'a, E, M> {
    values: &'a M,
    _marker: PhantomData<E>,
}

impl<'a, E, M> CanonicalEnumMapRef<'a, E, M> {
    pub fn new(values: &'a M) -> Self {
        Self {
            values,
            _marker: PhantomData,
        }
    }
}

impl<E, M, K> Serialize for CanonicalEnumMapRef<'_, E, M>
where
    for<'b> &'b M: core::iter::IntoIterator<Item = (&'b K, &'b i32)>,
    K: CanonicalMapKey + ToString,
    E: ProstEnum + 'static,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;
        for (key, value) in self.values {
            let value = CanonicalEnum::<E>::new(*value);
            map.serialize_entry(&key.to_string(), &value)?;
        }
        map.end()
    }
}

/// Wraps a map with enum values for canonical JSON deserialization.
pub struct CanonicalEnumMap<E, M>(pub M, PhantomData<E>);

impl<'de, E, M> Deserialize<'de> for CanonicalEnumMap<E, M>
where
    E: ProstEnum + 'static,
    M: CanonicalMapType<Value = i32>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<E, M>(PhantomData<(E, M)>);

        impl<'de, E, M> de::Visitor<'de> for Visitor<E, M>
        where
            E: ProstEnum + 'static,
            M: CanonicalMapType<Value = i32>,
        {
            type Value = CanonicalEnumMap<E, M>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut values = M::default();
                while let Some(key) = map.next_key::<String>()? {
                    let key = M::Key::from_key(&key).map_err(de::Error::custom)?;
                    let value = map.next_value::<CanonicalEnumValue<E>>()?.0;
                    values.insert(key, value);
                }
                Ok(CanonicalEnumMap(values, PhantomData))
            }

            fn visit_unit<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(CanonicalEnumMap(M::default(), PhantomData))
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
