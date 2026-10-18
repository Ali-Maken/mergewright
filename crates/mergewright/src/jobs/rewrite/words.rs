//! The words around a planned put-in, absorb or reword: the preview box's
//! title, rows and confirm verb, and the landing's status line. Pure: the
//! plan and the reads arrive as arguments.

use std::collections::BTreeMap;

use mergewright_core::oid::Oid;
use mergewright_ops::absorb::{AbsorbMap, StagedHunk, StaysWhy};
use mergewright_ops::rewrite::RewriteParams;

use super::{BoxWords, DoneWords, Gain, PUT_IT_IN, PUT_THEM_IN, short};

/// `a`'s box: the title naming the target, and the `gains` row.
pub(crate) fn amend_into_words(
    target: Oid,
    subject: &str,
    gains: &[Gain],
) -> (DoneWords, BoxWords) {
    let short = short(target);
    let files: Vec<String> = gains.iter().map(gain_words).collect();
    let mut row = format!("{short}  {subject}  gains");
    if !files.is_empty() {
        row = format!("{row}  {}", files.join(" \u{b7} "));
    }
    (
        DoneWords {
            what: format!("put into \"{subject}\""),
            also: None,
        },
        BoxWords {
            title: format!("put your staged changes into {short} \"{subject}\""),
            confirm: PUT_IT_IN,
            rows: vec![row],
        },
    )
}

/// The confirm verb of a reword's box.
pub const REWORD: &str = "reword";

/// A reword of `HEAD` with a line built on it: the box names the old
/// subject and the new one; the landing, the new one.
pub(crate) fn reword_words(params: &RewriteParams, message: &str) -> (DoneWords, BoxWords) {
    let new = message.lines().next().unwrap_or_default().trim();
    let (short, old) = params.todo.last().map_or((String::new(), ""), |row| {
        (short(row.commit), row.subject.as_str())
    });
    (
        DoneWords {
            what: format!("reworded \"{new}\""),
            also: None,
        },
        BoxWords {
            title: format!("reword {short} \"{old}\""),
            confirm: REWORD,
            rows: vec![format!("{short}  new message  {new}")],
        },
    )
}

/// `src/auth.rs +4 −1`; `+0`/`−0` left out; a binary file by its path alone.
fn gain_words(gain: &Gain) -> String {
    let Some((added, removed)) = gain.counts else {
        return gain.path.clone();
    };
    let mut words = gain.path.clone();
    if added > 0 {
        words.push_str(&format!(" +{added}"));
    }
    if removed > 0 {
        words.push_str(&format!(" \u{2212}{removed}"));
    }
    words
}

/// `path  lines  what` rows, the first two columns padded to their widest.
fn aligned(all: &[(String, String, String)]) -> Vec<String> {
    // An empty map has no widest cell: no rows, so no width is used.
    let path_width = all
        .iter()
        .map(|row| row.0.chars().count())
        .max()
        .unwrap_or(0);
    let lines_width = all
        .iter()
        .map(|row| row.1.chars().count())
        .max()
        .unwrap_or(0);
    all.iter()
        .map(|(path, lines, what)| {
            format!("{path:<path_width$}  {lines:<lines_width$}  {what}")
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// Absorb's box: one row per hunk — `→ <sha>  <subject>` or `stays staged ·
/// <why>` — in the map's order, placed first; the landing's counts.
pub(crate) fn absorb_words(
    map: &AbsorbMap,
    subjects: &BTreeMap<Oid, &str>,
) -> (DoneWords, BoxWords) {
    let placed: Vec<(String, String, String)> = map
        .placed
        .iter()
        .map(|(hunk, target)| {
            // Empty only if the map names a commit the plan has no row for;
            // the row still names it by its sha, so nothing is hidden.
            let subject = subjects.get(target).copied().unwrap_or_default();
            (
                hunk.path.display().to_string(),
                lines_words(hunk),
                format!("\u{2192} {}  {subject}", short(*target)),
            )
        })
        .collect();
    let stays: Vec<(String, String, String)> = map
        .stays
        .iter()
        .map(|(hunk, why)| {
            (
                hunk.path.display().to_string(),
                lines_words(hunk),
                format!("stays staged \u{b7} {}", stays_words(*why)),
            )
        })
        .collect();
    let rows = aligned(&placed.into_iter().chain(stays).collect::<Vec<_>>());
    let hunks = map.placed.len();
    let mut targets: Vec<Oid> = map.placed.iter().map(|(_, target)| *target).collect();
    targets.sort_unstable();
    targets.dedup();
    let staying = map.stays.len();
    (
        DoneWords {
            what: format!(
                "put {hunks} {} into {} {}",
                plural(hunks, "hunk", "hunks"),
                targets.len(),
                plural(targets.len(), "commit", "commits")
            ),
            also: (staying > 0)
                .then(|| format!("{staying} {} staged", plural(staying, "stays", "stay"))),
        },
        BoxWords {
            title: "put staged changes into the commits they belong to".to_owned(),
            confirm: PUT_THEM_IN,
            rows,
        },
    )
}

fn plural(count: usize, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 { one } else { many }
}

/// `lines 40–44`, `line 3`; a hunk that only removes lines is named by the
/// lines it removes from `HEAD`'s file.
pub(super) fn lines_words(hunk: &StagedHunk) -> String {
    let range = if hunk.new.is_empty() {
        &hunk.old
    } else {
        &hunk.new
    };
    let last = range.end.saturating_sub(1).max(range.start);
    if last == range.start {
        format!("line {}", range.start)
    } else {
        format!("lines {}\u{2013}{last}", range.start)
    }
}

/// The words after `stays staged ·`, one per [`StaysWhy`] (note R2; 06c and
/// 06e carries).
pub(crate) fn stays_words(why: StaysWhy) -> &'static str {
    match why {
        StaysWhy::NoCommitChangedIt => "no commit on your branch changed these lines",
        StaysWhy::TwoCommitsChangedIt => "more than one commit changed these lines",
        StaysWhy::NewFile => "a new file · no commit has it yet",
        StaysWhy::BeforeYourBranch => "these lines are older than your branch",
        StaysWhy::Renamed => "a renamed file stays whole",
        StaysWhy::Submodule => "a submodule change has no lines to put in",
        StaysWhy::OutsideYourCommits => "these lines are older than the commits absorb reaches",
        StaysWhy::AlreadyPushed => "the commit that changed these lines is already pushed",
    }
}
