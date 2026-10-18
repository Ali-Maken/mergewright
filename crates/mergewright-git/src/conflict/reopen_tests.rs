//! `ConflictFile::reopen`: a taken block back to open (ease-pass/02). Its
//! own file so `tests.rs` stays at its listed size (PROMPT.md 6.2).

use proptest::prelude::*;

use super::{NoSuchBlock, Take, parse};

#[test]
fn reopen_refuses_an_open_block_and_an_index_past_the_end() {
    let input = b"a\n<<<<<<< x\n1\n=======\n2\n>>>>>>> y\nb\n<<<<<<< x\n3\n=======\n4\n>>>>>>> y\n";
    let mut file = parse(input).expect("parses");
    // Both blocks still open: nothing to reopen.
    assert_eq!(file.reopen(0), Err(NoSuchBlock(0)));
    assert_eq!(file.reopen(1), Err(NoSuchBlock(1)));
    // Past the last block.
    assert_eq!(file.reopen(2), Err(NoSuchBlock(2)));
    // A refused reopen changes nothing.
    assert_eq!(file.render(), input);

    file.take(1, Take::Theirs).expect("block 1 exists");
    assert_eq!(file.reopen(0), Err(NoSuchBlock(0)), "block 0 is still open");
    assert_eq!(file.reopen(1), Ok(()));
    assert_eq!(file.open(), 2);
    assert_eq!(file.reopen(1), Err(NoSuchBlock(1)), "open again: refused");
    assert_eq!(file.render(), input);
}

/// 64 cases in the fast tier, 1000 with `heavy-tests` (PROMPT.md §7.1).
fn tier() -> ProptestConfig {
    ProptestConfig::with_cases(mergewright_fixtures::proptest_cases())
}

fn take() -> impl Strategy<Value = Take> {
    prop_oneof![Just(Take::Ours), Just(Take::Theirs), Just(Take::Both)]
}

/// Content lines that can never read as a marker.
fn lines() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z ]{0,6}", 0..3)
}

fn body(side: &[String]) -> String {
    side.iter().fold(String::new(), |mut out, line| {
        out.push_str(line);
        out.push('\n');
        out
    })
}

/// One region's bytes: plain lines, or a block with an optional base and
/// optional labels, its markers LF- or CRLF-ended.
fn region() -> impl Strategy<Value = String> {
    let block = (
        lines(),
        prop::option::of(lines()),
        lines(),
        prop::option::of("[A-Za-z]{0,4}"),
        prop::option::of("[A-Za-z]{0,4}"),
        prop::bool::ANY,
    )
        .prop_map(|(ours, base, theirs, ours_label, theirs_label, crlf)| {
            let end = if crlf { "\r\n" } else { "\n" };
            let marker = |m: &str, label: &Option<String>| match label {
                Some(label) => format!("{m} {label}{end}"),
                None => format!("{m}{end}"),
            };
            let mut out = marker("<<<<<<<", &ours_label);
            out.push_str(&body(&ours));
            if let Some(base) = &base {
                out.push_str("|||||||");
                out.push_str(end);
                out.push_str(&body(base));
            }
            out.push_str("=======");
            out.push_str(end);
            out.push_str(&body(&theirs));
            out.push_str(&marker(">>>>>>>", &theirs_label));
            out
        });
    prop_oneof![lines().prop_map(|plain| body(&plain)), block]
}

proptest! {
    #![proptest_config(tier())]

    /// `parse → take(i, t) → reopen(i) → render` is the original bytes, for
    /// every block and every take; the file is the parsed one again.
    #[test]
    fn take_then_reopen_renders_the_original(
        regions in prop::collection::vec(region(), 0..6),
        t in take(),
    ) {
        let input: String = regions.concat();
        let parsed = parse(input.as_bytes()).expect("generated files parse");
        for i in 0..parsed.blocks().len() {
            let mut file = parsed.clone();
            file.take(i, t).expect("index in range");
            file.reopen(i).expect("a taken block reopens");
            prop_assert_eq!(file.render(), input.as_bytes());
            prop_assert_eq!(&file, &parsed);
        }
    }
}
