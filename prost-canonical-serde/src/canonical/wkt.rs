use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write as _;

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};

use super::CanonicalError;
use super::number::{f64_from_i64_exact, f64_from_u64_exact};
use super::wrappers::{Canonical, CanonicalValue, CanonicalVec};
use crate::{CanonicalDeserialize, CanonicalSerialize};

impl CanonicalSerialize for prost_types::Timestamp {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = format_timestamp(self).map_err(ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }
}

impl CanonicalDeserialize for prost_types::Timestamp {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_timestamp_string(&value).map_err(de::Error::custom)
    }
}

impl CanonicalSerialize for prost_types::Duration {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = format_duration(self).map_err(ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }
}

impl CanonicalDeserialize for prost_types::Duration {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_duration_string(&value).map_err(de::Error::custom)
    }
}

impl CanonicalSerialize for prost_types::FieldMask {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut parts = Vec::new();
        for path in &self.paths {
            if path.is_empty() {
                return Err(ser::Error::custom("field mask path is empty"));
            }
            let mut segments = Vec::new();
            for segment in path.split('.') {
                if segment.is_empty() {
                    return Err(ser::Error::custom("field mask segment is empty"));
                }
                let json_segment = snake_to_lower_camel(segment);
                let round_trip = lower_camel_to_snake(&json_segment);
                if round_trip != segment {
                    return Err(ser::Error::custom("field mask segment does not round trip"));
                }
                segments.push(json_segment);
            }
            parts.push(segments.join("."));
        }
        let joined = parts.join(",");
        serializer.serialize_str(&joined)
    }
}

impl CanonicalDeserialize for prost_types::FieldMask {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.is_empty() {
            return Ok(prost_types::FieldMask { paths: Vec::new() });
        }

        let mut paths = Vec::new();
        for path in value.split(',') {
            if path.is_empty() {
                return Err(de::Error::custom("field mask path is empty"));
            }
            let mut segments = Vec::new();
            for segment in path.split('.') {
                if segment.is_empty() {
                    return Err(de::Error::custom("field mask segment is empty"));
                }
                if segment.contains('_') {
                    return Err(de::Error::custom("field mask contains underscore"));
                }
                segments.push(lower_camel_to_snake(segment));
            }
            paths.push(segments.join("."));
        }

        Ok(prost_types::FieldMask { paths })
    }
}

impl CanonicalSerialize for prost_types::Struct {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.fields.len()))?;
        for (key, value) in &self.fields {
            let value = Canonical::new(value);
            map.serialize_entry(key, &value)?;
        }
        map.end()
    }
}

impl CanonicalDeserialize for prost_types::Struct {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = prost_types::Struct;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut fields = BTreeMap::new();
                while let Some((key, value)) =
                    map.next_entry::<String, CanonicalValue<prost_types::Value>>()?
                {
                    fields.insert(key, value.0);
                }
                Ok(prost_types::Struct { fields })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

impl CanonicalSerialize for prost_types::ListValue {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.values.len()))?;
        for value in &self.values {
            let value = Canonical::new(value);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}

impl CanonicalDeserialize for prost_types::ListValue {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = CanonicalVec::<prost_types::Value>::deserialize(deserializer)?.0;
        Ok(prost_types::ListValue { values })
    }
}

impl CanonicalSerialize for prost_types::Value {
    fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.kind {
            Some(prost_types::value::Kind::NullValue(_)) => serializer.serialize_unit(),
            Some(prost_types::value::Kind::NumberValue(number)) => {
                if number.is_finite() {
                    serializer.serialize_f64(*number)
                } else {
                    Err(ser::Error::custom("Value.number_value must be finite"))
                }
            }
            Some(prost_types::value::Kind::StringValue(value)) => serializer.serialize_str(value),
            Some(prost_types::value::Kind::BoolValue(value)) => serializer.serialize_bool(*value),
            Some(prost_types::value::Kind::StructValue(value)) => {
                Canonical::new(value).serialize(serializer)
            }
            Some(prost_types::value::Kind::ListValue(value)) => {
                Canonical::new(value).serialize(serializer)
            }
            None => Err(ser::Error::custom("Value.kind is missing")),
        }
    }
}

impl CanonicalDeserialize for prost_types::Value {
    fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = prost_types::Value;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("json value")
            }

            fn visit_unit<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::NullValue(0)),
                })
            }

            fn visit_none<Err>(self) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                self.visit_unit()
            }

            fn visit_bool<Err>(self, value: bool) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::BoolValue(value)),
                })
            }

            fn visit_i64<Err>(self, value: i64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                let value = f64_from_i64_exact(value).map_err(Err::custom)?;
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::NumberValue(value)),
                })
            }

            fn visit_u64<Err>(self, value: u64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                let value = f64_from_u64_exact(value).map_err(Err::custom)?;
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::NumberValue(value)),
                })
            }

            fn visit_f64<Err>(self, value: f64) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::NumberValue(value)),
                })
            }

            fn visit_str<Err>(self, value: &str) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::StringValue(value.to_string())),
                })
            }

            fn visit_string<Err>(self, value: String) -> Result<Self::Value, Err>
            where
                Err: de::Error,
            {
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::StringValue(value)),
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<CanonicalValue<prost_types::Value>>()? {
                    values.push(value.0);
                }
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::ListValue(
                        prost_types::ListValue { values },
                    )),
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut fields = BTreeMap::new();
                while let Some((key, value)) =
                    map.next_entry::<String, CanonicalValue<prost_types::Value>>()?
                {
                    fields.insert(key, value.0);
                }
                Ok(prost_types::Value {
                    kind: Some(prost_types::value::Kind::StructValue(prost_types::Struct {
                        fields,
                    })),
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl CanonicalSerialize for prost_types::Any {
    fn serialize_canonical<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(ser::Error::custom("unsupported Any type"))
    }
}

impl CanonicalDeserialize for prost_types::Any {
    fn deserialize_canonical<'de, D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        Err(D::Error::custom("unsupported Any type"))
    }
}

fn snake_to_lower_camel(value: &str) -> String {
    let mut result = String::new();
    let mut iter = value.split('_');
    if let Some(first) = iter.next() {
        let mut chars = first.chars();
        if let Some(first_char) = chars.next() {
            result.push(first_char.to_ascii_lowercase());
            result.push_str(chars.as_str());
        }
    }
    for part in iter {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first_char) = chars.next() {
            result.push(first_char.to_ascii_uppercase());
            result.push_str(chars.as_str());
        }
    }
    result
}

fn lower_camel_to_snake(value: &str) -> String {
    let mut result = String::new();
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Minimum allowed timestamp seconds for canonical JSON (0001-01-01T00:00:00Z).
const MIN_TIMESTAMP_SECONDS: i64 = -62_135_596_800;
/// Maximum allowed timestamp seconds for canonical JSON (9999-12-31T23:59:59Z).
const MAX_TIMESTAMP_SECONDS: i64 = 253_402_300_799;

/// Formats a timestamp using canonical protojson rules.
///
/// Chrono's RFC 3339 formatting does not enforce protobuf timestamp bounds or
/// the canonical fractional-second precision (0/3/6/9 digits with a `Z`
/// suffix), so we format explicitly here.
fn format_timestamp(value: &prost_types::Timestamp) -> Result<String, CanonicalError> {
    if value.seconds < MIN_TIMESTAMP_SECONDS || value.seconds > MAX_TIMESTAMP_SECONDS {
        return Err(CanonicalError::new("timestamp seconds out of range"));
    }
    let nanos = value.nanos;
    if !(0..1_000_000_000).contains(&nanos) {
        return Err(CanonicalError::new("timestamp nanos out of range"));
    }
    let nanos_u32 =
        u32::try_from(nanos).map_err(|_| CanonicalError::new("timestamp nanos out of range"))?;
    let datetime = Utc
        .timestamp_opt(value.seconds, nanos_u32)
        .single()
        .ok_or_else(|| CanonicalError::new("timestamp out of range"))?;

    let mut formatted = String::with_capacity(32);
    let year = datetime.year();
    let month = datetime.month();
    let day = datetime.day();
    let hour = datetime.hour();
    let minute = datetime.minute();
    let second = datetime.second();
    let nano = datetime.nanosecond();

    write!(
        &mut formatted,
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}"
    )
    .map_err(|_| CanonicalError::new("format timestamp failed"))?;

    if nano != 0 {
        let mut frac = String::with_capacity(9);
        write!(&mut frac, "{nano:09}")
            .map_err(|_| CanonicalError::new("format timestamp failed"))?;
        while frac.ends_with('0') {
            frac.pop();
        }
        formatted.push('.');
        formatted.push_str(&frac);
    }

    formatted.push('Z');
    Ok(formatted)
}

fn parse_timestamp_string(value: &str) -> Result<prost_types::Timestamp, CanonicalError> {
    validate_timestamp_format(value)?;
    let datetime =
        DateTime::parse_from_rfc3339(value).map_err(|err| CanonicalError::new(err.to_string()))?;
    let utc = datetime.with_timezone(&Utc);
    let seconds = utc.timestamp();
    if !(MIN_TIMESTAMP_SECONDS..=MAX_TIMESTAMP_SECONDS).contains(&seconds) {
        return Err(CanonicalError::new("timestamp seconds out of range"));
    }
    Ok(prost_types::Timestamp {
        seconds,
        nanos: i32::try_from(utc.nanosecond())
            .map_err(|_| CanonicalError::new("timestamp nanos out of range"))?,
    })
}

fn validate_timestamp_format(value: &str) -> Result<(), CanonicalError> {
    if value.contains('t') {
        return Err(CanonicalError::new("timestamp must use 'T'"));
    }
    if !value.contains('T') {
        return Err(CanonicalError::new("timestamp must include 'T'"));
    }
    if value.contains('z') {
        return Err(CanonicalError::new("timestamp must use 'Z'"));
    }

    Ok(())
}

fn format_duration(value: &prost_types::Duration) -> Result<String, CanonicalError> {
    if value.seconds < -315_576_000_000 || value.seconds > 315_576_000_000 {
        return Err(CanonicalError::new("duration seconds out of range"));
    }
    let nanos = value.nanos;
    if nanos <= -1_000_000_000 || nanos >= 1_000_000_000 {
        return Err(CanonicalError::new("duration nanos out of range"));
    }
    if (value.seconds < 0 && nanos > 0) || (value.seconds > 0 && nanos < 0) {
        return Err(CanonicalError::new(
            "duration seconds and nanos must have same sign",
        ));
    }
    if value.seconds == 0 && nanos == 0 {
        return Ok("0s".to_string());
    }

    let negative = value.seconds < 0 || nanos < 0;
    let seconds = value.seconds.abs();
    let nanos = nanos.abs();

    let mut result = String::new();
    if negative {
        result.push('-');
    }
    write!(&mut result, "{seconds}").map_err(|_| CanonicalError::new("format duration failed"))?;

    if nanos != 0 {
        result.push('.');
        write!(&mut result, "{nanos:09}")
            .map_err(|_| CanonicalError::new("format duration failed"))?;
        while result.ends_with('0') {
            result.pop();
        }
    }

    result.push('s');
    Ok(result)
}

fn parse_duration_string(value: &str) -> Result<prost_types::Duration, CanonicalError> {
    let Some(value) = value.strip_suffix('s') else {
        return Err(CanonicalError::new("duration must end with 's'"));
    };
    if value.is_empty() {
        return Err(CanonicalError::new("duration is empty"));
    }

    let (negative, value) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value),
    };

    let mut parts = value.splitn(2, '.');
    let seconds_part = parts.next().unwrap_or("0");
    let fraction_part = parts.next();

    let seconds = seconds_part
        .parse::<i64>()
        .map_err(|_| CanonicalError::new("invalid duration seconds"))?;

    let nanos = if let Some(fraction) = fraction_part {
        if fraction.len() > 9 {
            return Err(CanonicalError::new("invalid duration fractional"));
        }
        if fraction.is_empty() {
            0
        } else {
            let parsed = fraction
                .parse::<u32>()
                .map_err(|_| CanonicalError::new("invalid duration nanos"))?;
            let fraction_len = u32::try_from(fraction.len())
                .map_err(|_| CanonicalError::new("invalid duration nanos"))?;
            let scale_exp = 9_u32
                .checked_sub(fraction_len)
                .ok_or_else(|| CanonicalError::new("invalid duration nanos"))?;
            let scale = 10_u32
                .checked_pow(scale_exp)
                .ok_or_else(|| CanonicalError::new("invalid duration nanos"))?;
            let nanos = parsed
                .checked_mul(scale)
                .ok_or_else(|| CanonicalError::new("invalid duration nanos"))?;
            i32::try_from(nanos).map_err(|_| CanonicalError::new("invalid duration nanos"))?
        }
    } else {
        0
    };

    let (seconds, nanos) = if negative {
        (-seconds, -nanos)
    } else {
        (seconds, nanos)
    };

    if !(-315_576_000_000..=315_576_000_000).contains(&seconds) {
        return Err(CanonicalError::new("duration seconds out of range"));
    }
    if nanos <= -1_000_000_000 || nanos >= 1_000_000_000 {
        return Err(CanonicalError::new("duration nanos out of range"));
    }
    if (seconds < 0 && nanos > 0) || (seconds > 0 && nanos < 0) {
        return Err(CanonicalError::new(
            "duration seconds and nanos must have same sign",
        ));
    }

    Ok(prost_types::Duration { seconds, nanos })
}
