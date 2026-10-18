//! Word-level highlighting inside a paired diff line.
//!
//! `git`'s line pairing (`mergewright-git::diff::pairs`) says which old line goes
//! with which new line; it stops at that — byte offsets from `git` don't
//! survive sanitising, since `�`/`␛` change a line's length (design note,
//! "Proposed change: word and syntax spans computed on sanitised text in
//! `ui`"). This module runs [`similar`]'s word diff on the two *sanitised*
//! strings and hands back **char** ranges into each, the same currency
//! [`crate::syntax`] uses and the widget slices by.

use std::ops::Range;

use similar::{ChangeTag, TextDiff};

/// Word-changed ranges for a block of paired diff lines.
///
/// `removed[i]` is the ranges within `removed_lines[i]` that changed against
/// `added_lines[i]` (and symmetrically for `added`), where `removed_lines`
/// and `added_lines` are the caller's arguments to [`word_spans`]. A line
/// past the other side's length — no counterpart to diff against — gets an
/// empty `Vec`: nothing to compare means nothing to call "changed".
pub struct WordSpans {
    /// Per removed line, char ranges that differ from its paired added line.
    pub removed: Vec<Vec<Range<usize>>>,
    /// Per added line, char ranges that differ from its paired removed line.
    pub added: Vec<Vec<Range<usize>>>,
}

/// Word-diff `removed[i]` against `added[i]` for every `i` both sides have.
///
/// Extra lines on the longer side (`removed.len() != added.len()`) get an
/// empty `Vec`: there is no counterpart to diff against, so nothing is
/// "changed" within the line itself — the added/removed line tint already
/// says what happened.
///
/// A pair that shares no word at all (a full replacement, not an edit) also
/// gets no spans on either side: marking every word "changed" when nothing
/// was kept would say less than the plain removed/added tint already does.
#[must_use]
pub fn word_spans(removed: &[&str], added: &[&str]) -> WordSpans {
    let mut out_removed: Vec<Vec<Range<usize>>> = vec![Vec::new(); removed.len()];
    let mut out_added: Vec<Vec<Range<usize>>> = vec![Vec::new(); added.len()];

    for i in 0..removed.len().min(added.len()) {
        let (removed_spans, added_spans) = word_spans_for_pair(removed[i], added[i]);
        out_removed[i] = removed_spans;
        out_added[i] = added_spans;
    }

    WordSpans {
        removed: out_removed,
        added: out_added,
    }
}

/// One paired line: the changed char ranges on each side, or a pair of empty
/// `Vec`s when the two lines share no word.
fn word_spans_for_pair(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let diff = TextDiff::from_words(old, new);

    let mut removed = Vec::new();
    let mut added = Vec::new();
    let mut old_cursor = 0usize;
    let mut new_cursor = 0usize;
    let mut has_common_word = false;

    for change in diff.iter_all_changes() {
        let text = change.value();
        let len = text.chars().count();

        match change.tag() {
            ChangeTag::Equal => {
                has_common_word |= text.chars().any(|c| !c.is_whitespace());
                old_cursor += len;
                new_cursor += len;
            }
            ChangeTag::Delete => {
                push_merged(&mut removed, old_cursor..old_cursor + len);
                old_cursor += len;
            }
            ChangeTag::Insert => {
                push_merged(&mut added, new_cursor..new_cursor + len);
                new_cursor += len;
            }
        }
    }

    if has_common_word {
        (removed, added)
    } else {
        (Vec::new(), Vec::new())
    }
}

/// Push `range`, extending the last entry instead when it is contiguous —
/// `similar` reports one op per word token, so a multi-word change arrives as
/// several adjacent ranges that read as one changed run.
fn push_merged(spans: &mut Vec<Range<usize>>, range: Range<usize>) {
    if let Some(last) = spans.last_mut()
        && last.end == range.start
    {
        last.end = range.end;
        return;
    }
    spans.push(range);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_word_change_is_found() {
        let spans = word_spans(&["let x = 1;"], &["let y = 1;"]);
        assert_eq!(spans.removed, vec![vec![4..5]]);
        assert_eq!(spans.added, vec![vec![4..5]]);
    }

    #[test]
    fn identical_lines_have_no_spans() {
        let spans = word_spans(&["same"], &["same"]);
        assert_eq!(spans.removed, vec![Vec::<Range<usize>>::new()]);
        assert_eq!(spans.added, vec![Vec::<Range<usize>>::new()]);
    }

    #[test]
    fn an_added_line_with_no_removed_counterpart_gets_no_spans() {
        let spans = word_spans(&[], &["a"]);
        assert_eq!(spans.added, vec![Vec::<Range<usize>>::new()]);
        assert!(spans.removed.is_empty());
    }

    #[test]
    fn a_removed_line_with_no_added_counterpart_gets_no_spans() {
        let spans = word_spans(&["a"], &[]);
        assert_eq!(spans.removed, vec![Vec::<Range<usize>>::new()]);
        assert!(spans.added.is_empty());
    }

    #[test]
    fn a_full_replacement_with_no_shared_word_gets_no_spans() {
        let spans = word_spans(&["completely different"], &["totally other"]);
        assert_eq!(spans.removed, vec![Vec::<Range<usize>>::new()]);
        assert_eq!(spans.added, vec![Vec::<Range<usize>>::new()]);
    }

    #[test]
    fn contiguous_ranges_merge_into_one() {
        // `similar` reports one op per word token, so a multi-token change
        // (e.g. a word and the space after it, both deleted) arrives as
        // several adjacent ranges; `word_spans` reads as one changed run.
        let mut spans = Vec::new();
        push_merged(&mut spans, 4..7);
        push_merged(&mut spans, 7..8);
        push_merged(&mut spans, 8..11);
        assert_eq!(spans, vec![4..11]);
    }

    #[test]
    fn non_contiguous_ranges_stay_separate() {
        let mut spans = Vec::new();
        push_merged(&mut spans, 0..2);
        push_merged(&mut spans, 5..7);
        assert_eq!(spans, vec![0..2, 5..7]);
    }

    #[test]
    fn extra_lines_on_the_longer_side_never_panic() {
        let spans = word_spans(&["one", "two", "three"], &["one"]);
        assert_eq!(spans.removed.len(), 3);
        assert_eq!(spans.added.len(), 1);
        assert!(spans.removed[1].is_empty());
        assert!(spans.removed[2].is_empty());
    }
}
