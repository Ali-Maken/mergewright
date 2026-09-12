use super::*;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

#[test]
fn the_same_name_always_produces_the_same_identity() {
    assert_eq!(
        BranchIdentity::of_name(b"main"),
        BranchIdentity::of_name(b"main")
    );
}

#[test]
fn different_names_produce_different_identities() {
    assert_ne!(
        BranchIdentity::of_name(b"main"),
        BranchIdentity::of_name(b"develop")
    );
}

#[test]
fn a_name_that_is_not_valid_ref_syntax_still_hashes() {
    // A name recovered from a merge message is raw bytes, not a `RefName`:
    // it may contain characters `check-ref-format` would reject.
    let identity = BranchIdentity::of_name(b"feature/foo bar");
    assert_eq!(identity, BranchIdentity::of_name(b"feature/foo bar"));
}

#[test]
fn the_same_short_sha_always_produces_the_same_identity() {
    assert_eq!(
        BranchIdentity::of_short_sha(oid(1)),
        BranchIdentity::of_short_sha(oid(1))
    );
}

#[test]
fn short_shas_that_differ_in_their_leading_bytes_produce_different_identities() {
    assert_ne!(
        BranchIdentity::of_short_sha(oid(1)),
        BranchIdentity::of_short_sha(oid(2))
    );
}
