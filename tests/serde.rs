use fchashmap::FcHashMap;
use serde::{Deserialize, Serialize};

#[test]
fn test_serialize() {
    let mut m = FcHashMap::<&'static str, &'static str, 1024>::new();
    m.insert("hello", "world").unwrap();
    m.insert("foo", "bar").unwrap();

    let serialized = serde_json::to_string(&m).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized["hello"], "world");
    assert_eq!(deserialized["foo"], "bar");
}

#[test]
fn test_deserialize() {
    let json = r#"{"foo":"bar","hello":"world"}"#;
    let deserialized: FcHashMap<String, String, 16> = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.len(), 2);
    assert_eq!(deserialized["hello"], "world");
    assert_eq!(deserialized["foo"], "bar");
}
