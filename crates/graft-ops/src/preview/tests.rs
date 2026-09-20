//! Unit tests for the pure parts of preview planning: the ghost id, the
//! forecast fold and the identity parser. Everything that needs a
//! repository is in `tests/it/history_ops.rs`, against real git.

use graft_core::commit::Timestamp;

use super::{ConflictForecast, fold, ghost_oid, parse_ident, parse_offset, sha1};

#[test]
fn sha1_matches_the_standard_vectors() {
    // FIPS 180-4 / RFC 3174 worked examples.
    assert_eq!(
        hex(&sha1(b"abc")),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    assert_eq!(
        hex(&sha1(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
    );
    // One million 'a's: the vector that catches a broken length field.
    assert_eq!(
        hex(&sha1(&vec![b'a'; 1_000_000])),
        "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
    );
}

/// Lowercase hex, without a formatter and so without a `Result` to ignore.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

#[test]
fn ghost_oids_are_deterministic_and_distinct() {
    let first = ghost_oid("Merge branch 'feat' into main", 0);
    assert_eq!(first, ghost_oid("Merge branch 'feat' into main", 0));
    // A different position in the same plan is a different commit.
    assert_ne!(first, ghost_oid("Merge branch 'feat' into main", 1));
    // As is a different subject.
    assert_ne!(first, ghost_oid("Merge branch 'other' into main", 0));
    assert_eq!(first.as_bytes().len(), 20);
}

#[test]
fn a_hole_in_the_forecast_beats_both_clean_and_likely() {
    let hole = ConflictForecast::NotChecked("git 2.38+ needed".to_owned());
    let likely = ConflictForecast::Likely {
        paths: vec!["src/a.rs".into()],
    };
    assert_eq!(
        fold(ConflictForecast::Clean, hole.clone()),
        hole,
        "absence of evidence is not a clean bill (PROMPT.md §1.4)"
    );
    assert_eq!(fold(likely.clone(), hole.clone()), hole);
    assert_eq!(fold(hole.clone(), likely.clone()), hole);
    assert_eq!(fold(ConflictForecast::Clean, likely.clone()), likely);
    assert_eq!(
        fold(ConflictForecast::Clean, ConflictForecast::Clean),
        ConflictForecast::Clean
    );
}

#[test]
fn folded_paths_keep_their_order_and_do_not_repeat() {
    let first = ConflictForecast::Likely {
        paths: vec!["src/a.rs".into(), "src/b.rs".into()],
    };
    let second = ConflictForecast::Likely {
        paths: vec!["src/b.rs".into(), "src/c.rs".into()],
    };
    assert_eq!(
        fold(first, second),
        ConflictForecast::Likely {
            paths: vec!["src/a.rs".into(), "src/b.rs".into(), "src/c.rs".into()],
        }
    );
}

#[test]
fn an_ident_line_parses_into_name_email_and_time() {
    let parsed = parse_ident(b"A U Thor <a@example.com> 1758377000 +0500\n").unwrap();
    assert_eq!(parsed.name, b"A U Thor".to_vec());
    assert_eq!(parsed.email, b"a@example.com".to_vec());
    assert_eq!(
        parsed.when,
        Timestamp {
            seconds: 1_758_377_000,
            offset_seconds: 5 * 3600,
        }
    );
    // A name git stores in Latin-1 survives: bytes in, bytes out.
    let latin1 = [b"Jos\xe9 <j@example.com> 1 -0130".as_slice()].concat();
    let parsed = parse_ident(&latin1).unwrap();
    assert_eq!(parsed.name, b"Jos\xe9".to_vec());
    assert_eq!(parsed.when.offset_seconds, -(3600 + 30 * 60));
}

#[test]
fn a_malformed_ident_is_none_rather_than_a_guess() {
    assert!(parse_ident(b"").is_none());
    assert!(parse_ident(b"A U Thor <a@example.com>").is_none());
    assert!(parse_ident(b"A U Thor a@example.com 1 +0000").is_none());
    assert!(parse_ident(b"A U Thor <a@example.com> notanumber +0000").is_none());
    assert_eq!(parse_offset("+0000"), Some(0));
    assert!(parse_offset("0000").is_none());
    assert!(parse_offset("+00").is_none());
}
