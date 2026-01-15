use alloc::string::String;
use alloc::vec::Vec;
use core::any::TypeId;
use core::fmt;
use core::marker::PhantomData;

use prost_types::NullValue;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::ProstEnum;

/// Wraps an optional enum number for canonical protobuf JSON deserialization.
pub struct CanonicalEnumOption<E>(pub Option<i32>, PhantomData<E>);

impl<'de, E: ProstEnum + 'static> Deserialize<'de> for CanonicalEnumOption<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<CanonicalEnumValue<E>>::deserialize(deserializer)?;
        Ok(CanonicalEnumOption(value.map(|value| value.0), PhantomData))
    }
}

/// Wraps an enum number for canonical protobuf JSON serialization.
pub struct CanonicalEnum<'a, E: ProstEnum> {
    value: i32,
    _marker: PhantomData<&'a E>,
}

impl<E: ProstEnum> CanonicalEnum<'_, E> {
    pub fn new(value: i32) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }
}

impl<E: ProstEnum + 'static> Serialize for CanonicalEnum<'_, E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // ProtoJSON: `null` is generally treated as "unset", but
        // `google.protobuf.NullValue` is the exception that represents a
        // sentinel null so `Struct`/`Value` can round-trip arbitrary JSON.
        if is_null_value_enum::<E>() && self.value == 0 {
            return serializer.serialize_unit();
        }
        if let Some(enum_value) = E::from_i32(self.value) {
            serializer.serialize_str(enum_value.as_str_name())
        } else {
            serializer.serialize_i32(self.value)
        }
    }
}

/// Wraps an enum number for canonical protobuf JSON deserialization.
pub struct CanonicalEnumValue<E>(pub i32, PhantomData<E>);

impl<'de, E: ProstEnum + 'static> Deserialize<'de> for CanonicalEnumValue<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<E>(PhantomData<E>);

        impl<E: ProstEnum + 'static> de::Visitor<'_> for Visitor<E> {
            type Value = CanonicalEnumValue<E>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("enum string or number")
            }

            fn visit_unit<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                if is_null_value_enum::<E>() {
                    return Ok(CanonicalEnumValue(0, PhantomData));
                }
                Err(Err::custom("invalid enum value"))
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                if is_null_value_enum::<E>() && value == "NULL_VALUE" {
                    return Ok(CanonicalEnumValue(0, PhantomData));
                }
                E::from_str_name(value)
                    .map(|enum_value| CanonicalEnumValue(enum_value.as_i32(), PhantomData))
                    .ok_or_else(|| Err::custom("invalid enum string"))
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_i32<Err>(self, value: i32) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(CanonicalEnumValue(value, PhantomData))
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                let value =
                    i32::try_from(value).map_err(|_| Err::custom("enum number out of range"))?;
                Ok(CanonicalEnumValue(value, PhantomData))
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                let value =
                    i32::try_from(value).map_err(|_| Err::custom("enum number out of range"))?;
                Ok(CanonicalEnumValue(value, PhantomData))
            }
        }

        deserializer.deserialize_any(Visitor(PhantomData))
    }
}

/// Wraps a slice of enum numbers for canonical JSON serialization.
pub struct CanonicalEnumSeq<'a, E: ProstEnum> {
    values: &'a [i32],
    _marker: PhantomData<E>,
}

impl<'a, E: ProstEnum> CanonicalEnumSeq<'a, E> {
    pub fn new(values: &'a [i32]) -> Self {
        Self {
            values,
            _marker: PhantomData,
        }
    }
}

impl<E: ProstEnum + 'static> Serialize for CanonicalEnumSeq<'_, E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.values.len()))?;
        for value in self.values {
            let value = CanonicalEnum::<E>::new(*value);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}

/// Wraps a vector of enum numbers for canonical JSON deserialization.
pub struct CanonicalEnumVec<E>(pub Vec<i32>, PhantomData<E>);

impl<'de, E: ProstEnum + 'static> Deserialize<'de> for CanonicalEnumVec<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<E>(PhantomData<E>);

        impl<'de, E: ProstEnum + 'static> de::Visitor<'de> for Visitor<E> {
            type Value = CanonicalEnumVec<E>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<CanonicalEnumValue<E>>()? {
                    values.push(value.0);
                }
                Ok(CanonicalEnumVec(values, PhantomData))
            }

            fn visit_unit<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(CanonicalEnumVec(Vec::new(), PhantomData))
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

/// Checks whether an enum is `google.protobuf.NullValue`.
fn is_null_value_enum<E: 'static>() -> bool {
    TypeId::of::<E>() == TypeId::of::<NullValue>()
}

impl ProstEnum for NullValue {
    fn from_i32(value: i32) -> Option<Self> {
        NullValue::try_from(value).ok()
    }

    fn from_str_name(value: &str) -> Option<Self> {
        NullValue::from_str_name(value)
    }

    fn as_str_name(&self) -> &'static str {
        self.as_str_name()
    }

    fn as_i32(&self) -> i32 {
        *self as i32
    }
}
