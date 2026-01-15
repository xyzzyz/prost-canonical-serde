use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use core::fmt;
use serde::{Deserialize, Deserializer, Serializer, de};

use super::number::{
    f32_from_f64, f32_from_i64_exact, f32_from_u64_exact, f64_from_i64_exact, f64_from_u64_exact,
    i32_from_f64, i32_from_str, i64_from_f64, i64_from_str, parse_float, serialize_float32,
    serialize_float64, u32_from_f64, u32_from_str, u64_from_f64, u64_from_str,
};
use crate::{CanonicalDeserialize, CanonicalSerialize};

impl CanonicalSerialize for bool {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(*self)
    }
}

impl CanonicalDeserialize for bool {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        bool::deserialize(deserializer)
    }
}

impl CanonicalSerialize for i32 {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(*self)
    }
}

impl CanonicalDeserialize for i32 {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = i32;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("i32 or string")
            }

            fn visit_i32<Err>(self, value: i32) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(value)
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                i32::try_from(value).map_err(|_| Err::custom("i32 out of range"))
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                i32::try_from(value).map_err(|_| Err::custom("i32 out of range"))
            }

            fn visit_f64<Err>(self, value: f64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                i32_from_f64(value).map_err(Err::custom)
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                i32_from_str(value).map_err(Err::custom)
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl CanonicalSerialize for u32 {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(*self)
    }
}

impl CanonicalDeserialize for u32 {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = u32;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("u32 or string")
            }

            fn visit_u32<Err>(self, value: u32) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(value)
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                u32::try_from(value).map_err(|_| Err::custom("u32 out of range"))
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                if value < 0 {
                    return Err(Err::custom("u32 out of range"));
                }
                u32::try_from(value).map_err(|_| Err::custom("u32 out of range"))
            }

            fn visit_f64<Err>(self, value: f64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                u32_from_f64(value).map_err(Err::custom)
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                u32_from_str(value).map_err(Err::custom)
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl CanonicalSerialize for i64 {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl CanonicalDeserialize for i64 {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = i64;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("i64 or string")
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(value)
            }

            fn visit_f64<Err>(self, value: f64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                i64_from_f64(value).map_err(Err::custom)
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                i64::try_from(value).map_err(|_| Err::custom("i64 out of range"))
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                i64_from_str(value).map_err(Err::custom)
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl CanonicalSerialize for u64 {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl CanonicalDeserialize for u64 {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = u64;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("u64 or string")
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(value)
            }

            fn visit_f64<Err>(self, value: f64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                u64_from_f64(value).map_err(Err::custom)
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                if value < 0 {
                    return Err(Err::custom("u64 out of range"));
                }
                u64::try_from(value).map_err(|_| Err::custom("u64 out of range"))
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                u64_from_str(value).map_err(Err::custom)
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl CanonicalSerialize for f32 {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_float32(*self, serializer)
    }
}

impl CanonicalDeserialize for f32 {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = f32;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("f32 or string")
            }

            fn visit_f64<Err>(self, value: f64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                if !value.is_finite() {
                    return Err(Err::custom("float must be finite"));
                }
                f32_from_f64(value).map_err(Err::custom)
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                f32_from_i64_exact(value).map_err(Err::custom)
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                f32_from_u64_exact(value).map_err(Err::custom)
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                let parsed = parse_float(value).map_err(Err::custom)?;
                f32_from_f64(parsed).map_err(Err::custom)
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl CanonicalSerialize for f64 {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_float64(*self, serializer)
    }
}

impl CanonicalDeserialize for f64 {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = f64;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("f64 or string")
            }

            fn visit_f64<Err>(self, value: f64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                if !value.is_finite() {
                    return Err(Err::custom("float must be finite"));
                }
                Ok(value)
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                f64_from_i64_exact(value).map_err(Err::custom)
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                f64_from_u64_exact(value).map_err(Err::custom)
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                parse_float(value).map_err(Err::custom)
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl CanonicalSerialize for String {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl CanonicalDeserialize for String {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
    }
}

impl CanonicalSerialize for Vec<u8> {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = BASE64_STANDARD.encode(self);
        serializer.serialize_str(&encoded)
    }
}

impl CanonicalDeserialize for Vec<u8> {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        BASE64_STANDARD
            .decode(value.as_bytes())
            .map_err(de::Error::custom)
    }
}
