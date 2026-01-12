use chrono::{TimeZone, Timelike, Utc};
use prost_canonical_serde_example::demo::Example;
use prost_types::Timestamp;
use std::time::SystemTime;

let chrono_dt = Utc
    .with_ymd_and_hms(2006, 1, 2, 15, 4, 5)
    .unwrap()
    .with_nanosecond(123_456_000)
    .unwrap();
let created_at = Timestamp::from(SystemTime::from(chrono_dt));

let message = Example {
    name: "demo".to_string(),
    count: 42,
    payload: vec![0, 1, 2, 255],
    created_at: Some(created_at),
};

let json = serde_json::to_string(&message).unwrap();
// Canonical proto JSON encodes int64/uint64 as strings, bytes as base64,
// and well-known types like Timestamp in RFC 3339 form.
let expected =
    r#"{"name":"demo","count":"42","payload":"AAEC/w==","createdAt":"2006-01-02T15:04:05.123456Z"}"#;
assert_eq!(json, expected);

// A normal serde struct would typically emit count as a number, not a string.
// The canonical derives handle this conversion for you.
let roundtrip: Example = serde_json::from_str(&json).unwrap();
assert_eq!(roundtrip, message);
