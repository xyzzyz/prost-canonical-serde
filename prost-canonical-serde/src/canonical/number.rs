use serde::Serializer;

use super::CanonicalError;

pub(crate) fn serialize_float64<S>(value: f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if value.is_finite() {
        serializer.collect_str(&value)
    } else if value.is_nan() {
        serializer.serialize_str("NaN")
    } else if value.is_sign_positive() {
        serializer.serialize_str("Infinity")
    } else {
        serializer.serialize_str("-Infinity")
    }
}

pub(crate) fn serialize_float32<S>(value: f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if value.is_finite() {
        serializer.collect_str(&value)
    } else if value.is_nan() {
        serializer.serialize_str("NaN")
    } else if value.is_sign_positive() {
        serializer.serialize_str("Infinity")
    } else {
        serializer.serialize_str("-Infinity")
    }
}

pub(crate) fn parse_float(value: &str) -> Result<f64, CanonicalError> {
    match value {
        "NaN" => Ok(f64::NAN),
        "Infinity" => Ok(f64::INFINITY),
        "-Infinity" => Ok(f64::NEG_INFINITY),
        _ => {
            let parsed = value
                .parse::<f64>()
                .map_err(|_| CanonicalError::new("invalid f64 string"))?;
            if !parsed.is_finite() {
                return Err(CanonicalError::new("float out of range"));
            }
            Ok(parsed)
        }
    }
}

fn is_integral(value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }
    value % 1.0 == 0.0
    // because there's no floating point math in core, we need to do some hacky stuff
    // TODO(amichalik): simplify this once https://github.com/rust-lang/rust/issues/137578 is stabilized
}

pub(crate) fn i32_from_str(value: &str) -> Result<i32, CanonicalError> {
    if let Ok(parsed) = value.parse::<i32>() {
        return Ok(parsed);
    }
    let parsed = parse_float(value).map_err(|_| CanonicalError::new("invalid i32 string"))?;
    if !is_integral(parsed) {
        return Err(CanonicalError::new("invalid i32 string"));
    }
    if parsed < f64::from(i32::MIN) || parsed > f64::from(i32::MAX) {
        return Err(CanonicalError::new("i32 out of range"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Range checks ensure the cast preserves the i32 value."
    )]
    Ok(parsed as i32)
}

pub(crate) fn u32_from_str(value: &str) -> Result<u32, CanonicalError> {
    value
        .parse::<u32>()
        .map_err(|_| CanonicalError::new("invalid u32 string"))
}

/// Minimum i64 that round-trips exactly through canonical JSON f64 values.
const MIN_SAFE_I64: i64 = -9_007_199_254_740_992;
/// Maximum i64 that round-trips exactly through canonical JSON f64 values.
const MAX_SAFE_I64: i64 = 9_007_199_254_740_992;
/// Maximum u64 that round-trips exactly through canonical JSON f64 values.
const MAX_SAFE_U64: u64 = 18_014_398_509_481_984;
/// Maximum i64 that round-trips exactly through canonical JSON f32 values.
const MAX_SAFE_I64_F32: i64 = 16_777_216;
/// Maximum u64 that round-trips exactly through canonical JSON f32 values.
const MAX_SAFE_U64_F32: u64 = 16_777_216;

pub(crate) fn f64_from_i64_exact(value: i64) -> Result<f64, CanonicalError> {
    if !(MIN_SAFE_I64..=MAX_SAFE_I64).contains(&value) {
        return Err(CanonicalError::new("integer out of range for f64"));
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "Range checks ensure the cast preserves the integer value."
    )]
    Ok(value as f64)
}

pub(crate) fn f64_from_u64_exact(value: u64) -> Result<f64, CanonicalError> {
    if value > MAX_SAFE_U64 {
        return Err(CanonicalError::new("integer out of range for f64"));
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "Range checks ensure the cast preserves the integer value."
    )]
    Ok(value as f64)
}

pub(crate) fn i32_from_f64(value: f64) -> Result<i32, CanonicalError> {
    if !is_integral(value) {
        return Err(CanonicalError::new("invalid i32"));
    }
    if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
        return Err(CanonicalError::new("i32 out of range"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Range checks ensure the cast preserves the i32 value."
    )]
    Ok(value as i32)
}

pub(crate) fn u32_from_f64(value: f64) -> Result<u32, CanonicalError> {
    if !is_integral(value) {
        return Err(CanonicalError::new("invalid u32"));
    }
    if value < 0.0 || value > f64::from(u32::MAX) {
        return Err(CanonicalError::new("u32 out of range"));
    }
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "Range checks ensure the cast preserves the u32 value."
    )]
    Ok(value as u32)
}

pub(crate) fn f32_from_i64_exact(value: i64) -> Result<f32, CanonicalError> {
    if value.abs() > MAX_SAFE_I64_F32 {
        return Err(CanonicalError::new("integer out of range for f32"));
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "Range checks ensure the cast preserves the integer value."
    )]
    Ok(value as f32)
}

pub(crate) fn f32_from_u64_exact(value: u64) -> Result<f32, CanonicalError> {
    if value > MAX_SAFE_U64_F32 {
        return Err(CanonicalError::new("integer out of range for f32"));
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "Range checks ensure the cast preserves the integer value."
    )]
    Ok(value as f32)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Range checks ensure the cast preserves the canonical integer value."
)]
pub(crate) fn i64_from_f64(value: f64) -> Result<i64, CanonicalError> {
    const MIN_SAFE_INT: f64 = -9_007_199_254_740_992.0;
    const MAX_SAFE_INT: f64 = 9_007_199_254_740_992.0;

    if !is_integral(value) {
        return Err(CanonicalError::new("invalid i64"));
    }
    if !(MIN_SAFE_INT..=MAX_SAFE_INT).contains(&value) {
        return Err(CanonicalError::new("i64 out of range"));
    }
    Ok(value as i64)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Range checks ensure the cast preserves the canonical unsigned value."
)]
pub(crate) fn u64_from_f64(value: f64) -> Result<u64, CanonicalError> {
    const MAX_SAFE_UINT: f64 = 18_014_398_509_481_984.0;

    if !is_integral(value) {
        return Err(CanonicalError::new("invalid u64"));
    }
    if !(0.0..=MAX_SAFE_UINT).contains(&value) {
        return Err(CanonicalError::new("u64 out of range"));
    }
    Ok(value as u64)
}

pub(crate) fn i64_from_str(value: &str) -> Result<i64, CanonicalError> {
    value
        .parse::<i64>()
        .map_err(|_| CanonicalError::new("invalid i64 string"))
}

pub(crate) fn u64_from_str(value: &str) -> Result<u64, CanonicalError> {
    value
        .parse::<u64>()
        .map_err(|_| CanonicalError::new("invalid u64 string"))
}

pub(crate) fn f32_from_f64(value: f64) -> Result<f32, CanonicalError> {
    if value.is_nan() {
        return Ok(f32::NAN);
    }
    if value.is_infinite() {
        return Ok(if value.is_sign_positive() {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        });
    }
    if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return Err(CanonicalError::new("float out of range"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Range checks ensure the cast preserves the f32 value."
    )]
    Ok(value as f32)
}
