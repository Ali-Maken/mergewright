//! The review's words: how a clashed file was solved, and the message's
//! subject as the merge will commit it.

use mergewright_term::sanitize_line;
use mergewright_ui::finish_summary::{HowSolved, PickCounts};

use super::{Choices, ResultIs, Side, Take};

/// How a clashed file was solved, from what the resolver recorded (ticket
/// decision 4). First match wins: a whole-file choice, then a file handed
/// to an editor, then the picks counted by kind; nothing recorded (solved
/// in the file, or a relaunch since) is `ByHand`.
#[must_use]
pub(crate) fn how_solved(choices: Option<&Choices>, result: ResultIs) -> HowSolved {
    let Some(choices) = choices else {
        return HowSolved::ByHand;
    };
    if let Some(side) = choices.whole {
        let word = if result == ResultIs::Deleted {
            "delete the file"
        } else {
            match side {
                Side::Ours => "◀ yours, whole file",
                Side::Theirs => "▶ theirs, whole file",
            }
        };
        return HowSolved::Whole(sanitize_line(word));
    }
    if choices.edited {
        return HowSolved::ByHand;
    }
    let count = |take: Take| {
        let n = choices
            .takes
            .iter()
            .filter(|landed| **landed == Some(take))
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let counts = PickCounts {
        yours: count(Take::Ours),
        theirs: count(Take::Theirs),
        both: count(Take::Both),
    };
    if counts == PickCounts::default() {
        HowSolved::ByHand
    } else {
        HowSolved::Picks(counts)
    }
}

/// The message's first line, trimmed: what the review's `Message` row shows.
#[must_use]
pub(crate) fn subject_of(message: &[u8]) -> String {
    String::from_utf8_lossy(message)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_owned()
}
