use super::*;

/// The id of git's empty tree, the one SHA-1 every git install agrees on.
const EMPTY_TREE_HEX: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

#[test]
fn round_trips_a_sha1_id_through_hex() {
    let oid = Oid::from_hex(EMPTY_TREE_HEX).expect("40 hex digits is a SHA-1 id");

    assert_eq!(oid.as_bytes().len(), Oid::SHA1_BYTES);
    assert_eq!(oid.to_string(), EMPTY_TREE_HEX);
    assert_eq!(Oid::from_bytes(oid.as_bytes()), Ok(oid));
}

#[test]
fn round_trips_a_sha256_id_through_hex() {
    let hex = "a".repeat(Oid::MAX_BYTES * 2);
    let oid = Oid::from_hex(&hex).expect("64 hex digits is a SHA-256 id");

    assert_eq!(oid.as_bytes().len(), Oid::MAX_BYTES);
    assert_eq!(oid.to_string(), hex);
}

#[test]
fn reads_uppercase_hex_and_writes_lowercase() {
    let oid = Oid::from_hex(&EMPTY_TREE_HEX.to_uppercase()).expect("hex is case-insensitive");

    assert_eq!(oid.to_string(), EMPTY_TREE_HEX);
}

#[test]
fn rejects_an_id_of_the_wrong_length() {
    assert_eq!(
        Oid::from_bytes(&[0u8; 19]),
        Err(Error::OidLength { len: 19 })
    );
    assert_eq!(Oid::from_bytes(&[]), Err(Error::OidLength { len: 0 }));
}

#[test]
fn rejects_hex_that_is_not_an_id() {
    // An abbreviated id: valid hex, but resolving it needs the object
    // database, so the constructor refuses it.
    assert!(Oid::from_hex("4b825dc").is_err());
    // Full length, but not hex digits.
    assert!(Oid::from_hex(&"z".repeat(40)).is_err());
    // Odd length that halves to a valid one.
    assert!(Oid::from_hex(&"a".repeat(41)).is_err());
    assert!(Oid::from_hex("").is_err());
}

#[test]
fn two_ids_of_different_lengths_are_never_equal() {
    let short = Oid::from_bytes(&[0xab; Oid::SHA1_BYTES]).expect("20 bytes");
    let long = Oid::from_bytes(&[0xab; Oid::MAX_BYTES]).expect("32 bytes");

    assert_ne!(short, long);
}

#[test]
fn debug_shows_the_hex() {
    let oid = Oid::from_hex(EMPTY_TREE_HEX).expect("valid id");

    assert_eq!(format!("{oid:?}"), format!("Oid({EMPTY_TREE_HEX})"));
}
