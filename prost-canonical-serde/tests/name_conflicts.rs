use prost_canonical_serde_example::conflict::NameConflicts;
use prost_canonical_serde_example::conflict::name_conflicts::Choice;

#[test]
fn name_conflicts_roundtrip() {
    let message = NameConflicts {
        key: "alpha".to_string(),
        value: "bravo".to_string(),
        map: 7,
        choice: Some(Choice::ValueChoice(42)),
    };

    let json = serde_json::to_string(&message).expect("serialize name conflicts");
    let decoded: NameConflicts = serde_json::from_str(&json).expect("deserialize name conflicts");
    assert_eq!(message, decoded);
}
