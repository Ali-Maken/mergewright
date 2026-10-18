//! The merge preview's pure pieces: names and first names. Its keys and
//! box are walked through the real app in `tests/it/merge_preview.rs`.

use graft_core::oid::Oid;
use graft_ops::operation::{FastForward, MergeParams, Source, Tip};

use super::{first_name, names};

fn params(source: Source) -> MergeParams {
    MergeParams {
        source,
        into: Tip {
            name: "refs/heads/wallet-release".to_owned(),
            oid: Oid::from_hex("0101010101010101010101010101010101010101").expect("an id"),
        },
        source_tip: Oid::from_hex("0202020202020202020202020202020202020202").expect("an id"),
        ff: FastForward::Default,
    }
}

#[test]
fn a_branch_merge_names_both_sides_short() {
    let (from, into) = names(&params(Source::Branch(
        "refs/heads/sara/fix-toc".to_owned(),
    )));
    assert_eq!(from, "sara/fix-toc");
    assert_eq!(into, "wallet-release");
}

#[test]
fn a_commit_merge_names_the_commit_by_its_short_id() {
    let oid = Oid::from_hex("0303030303030303030303030303030303030303").expect("an id");
    let (from, _) = names(&params(Source::Commit(oid)));
    assert_eq!(from, "0303030");
}

#[test]
fn who_is_the_first_word_of_the_author_name() {
    assert_eq!(first_name(b"Sara Ahmed"), "Sara");
    assert_eq!(first_name(b"  Omar "), "Omar");
    assert_eq!(first_name(b""), "");
}
