//! Usability Q1 (m8/46): the words of a `⇢` row.

use super::*;

fn commit(held_by: HeldBy, under: usize, fate: Fate) -> LostCommit {
    LostCommit {
        run: graft_git::history_reads::LostRun {
            tip: oid(0x9a),
            under,
        },
        subject: Some("Add rate limiter".to_owned()),
        held_by,
        fate,
    }
}

fn branch(name: &str, gone: bool) -> HeldBy {
    HeldBy::Branch {
        name: name.to_owned(),
        gone,
    }
}

#[test]
fn replaced_words_say_undo_brings_it_back() {
    let sha = "9a9a9a9";
    assert_eq!(
        lost_words(&commit(branch("feature", false), 0, Fate::Replaced)),
        format!("{sha} \"Add rate limiter\" · was feature's tip · undo brings it back")
    );
    assert_eq!(
        lost_words(&commit(branch("spike", true), 0, Fate::Deleted)),
        format!("{sha} \"Add rate limiter\" · was spike's tip · undo brings it back"),
        "a deleted branch: no `is gone`, undo brings it back"
    );
    assert_eq!(
        lost_words(&commit(HeldBy::Head, 0, Fate::Replaced)),
        format!("{sha} \"Add rate limiter\" · was HEAD · undo brings it back")
    );
    assert_eq!(
        lost_words(&commit(branch("feature", false), 2, Fate::Replaced)),
        format!(
            "{sha} \"Add rate limiter\" · was feature's tip · undo brings it back · +2 under it"
        )
    );
}

#[test]
fn lost_words_unchanged() {
    let sha = "9a9a9a9";
    assert_eq!(
        lost_words(&commit(branch("main", false), 0, Fate::Lost)),
        format!("{sha} \"Add rate limiter\" · was main's tip · no branch holds it")
    );
    assert_eq!(
        lost_words(&commit(branch("spike", true), 0, Fate::Lost)),
        format!("{sha} \"Add rate limiter\" · was spike's tip · spike is gone")
    );
    assert_eq!(
        lost_words(&commit(HeldBy::Head, 3, Fate::Lost)),
        format!("{sha} \"Add rate limiter\" · was HEAD · +3 under it")
    );
}
