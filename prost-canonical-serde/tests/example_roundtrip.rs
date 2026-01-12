use chrono::{TimeZone, Timelike, Utc};
use prost_canonical_serde_example::demo::Example;
use prost_types::Timestamp;
use std::time::SystemTime;

#[test]
fn example_canonical_json_roundtrip() {
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

    let json = serde_json::to_value(&message).expect("serialize example");
    let count = json
        .get("count")
        .expect("count field")
        .as_str()
        .expect("count should be serialized as a string");
    assert_eq!(count, "42");
    let payload = json
        .get("payload")
        .expect("payload field")
        .as_str()
        .expect("payload should be serialized as a string");
    assert_eq!(payload, "AAEC/w==");
    let created_at = json
        .get("createdAt")
        .expect("createdAt field")
        .as_str()
        .expect("createdAt should be serialized as a string");
    assert_eq!(created_at, "2006-01-02T15:04:05.123456Z");

    let roundtrip: Example = serde_json::from_value(json).expect("deserialize example");
    assert_eq!(roundtrip, message);
}
