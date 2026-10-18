//! The commit screen's words: the three bars, the summary sentence, the
//! header line and a file row — each exactly the mock's (C4, C12), keys
//! spelled from the keymap, never typed into a literal.

use graft_core::event::KeyCode;

use super::{
    CommitFileRow, CommitFocus, CommitHeader, CommitScreenView, FileGroup, Onto, Refusal, Summary,
};
use crate::keymap::{ActionId, KeyChord, Keymap};

/// Between two `<key> <verb>` pairs on a bar.
const GAP: &str = "  ";

/// `space`, the files' in/out key. A text field's key is fixed (ADR-0055),
/// and it has no action of its own, so it is spelled from the chord.
pub(super) fn space() -> String {
    KeyChord::key(' ').label().0
}

/// `↑/↓`: in the subject they move the file cursor (C10). Fixed text-field
/// keys, spelled from their chords.
fn arrows() -> String {
    format!(
        "{}/{}",
        KeyChord::plain(KeyCode::Up).label().0,
        KeyChord::plain(KeyCode::Down).label().0
    )
}

/// `<keys> <verb>`, or nothing when the action is unbound.
fn pair(keys: &Keymap, action: &'static str, verb: &str) -> Option<String> {
    keys.key_verb(&[ActionId(action)], verb)
}

/// `1 file`, `3 files`.
fn files(count: usize) -> String {
    let noun = if count == 1 { "file" } else { "files" };
    format!("{count} {noun}")
}

/// What the `⏎` slot says: the refusal's words, or what `⏎` does.
fn enter_words(view: &CommitScreenView<'_>, counted: bool) -> String {
    match view.refusal {
        Some(Refusal::NoMessage) => "type a message first".to_owned(),
        Some(Refusal::NoFile) => "put a file in first".to_owned(),
        None => {
            let going_in = view
                .files
                .iter()
                .filter(|file| file.group == FileGroup::GoingIn)
                .count();
            if counted && going_in > 0 {
                format!("commit {}", files(going_in))
            } else {
                "commit".to_owned()
            }
        }
    }
}

/// The bar for whichever part has focus (C12, mock `:159`):
/// - subject: `⏎ commit 3 files  ⇥ more detail  ↑/↓ see a file's changes
///   esc cancel, your message is kept`;
/// - body: `esc back to the message line  ⇥ files`;
/// - files: `space leave it out  j/k move  ⏎ commit  esc back to the
///   message`.
///
/// While a commit runs: `committing…  esc cancel`.
#[must_use]
pub fn bar(view: &CommitScreenView<'_>, keys: &Keymap) -> String {
    if view.busy {
        return ["committing…".to_owned()]
            .into_iter()
            .chain(pair(keys, "app.back", "cancel"))
            .collect::<Vec<_>>()
            .join(GAP);
    }
    let parts: Vec<Option<String>> = match view.focus {
        CommitFocus::Subject => subject_parts(view, keys)
            .into_iter()
            .map(|(_, part)| part)
            .collect(),
        CommitFocus::Body => vec![
            pair(keys, "app.back", "back to the message line"),
            pair(keys, "commit.next_field", "files"),
        ],
        CommitFocus::Files => {
            let in_or_out = match view.files.get(view.cursor).map(|file| file.group) {
                Some(FileGroup::StayingOut) => Some("put it in"),
                Some(FileGroup::GoingIn) => Some("leave it out"),
                None => None,
            };
            vec![
                in_or_out.map(|verb| format!("{} {verb}", space())),
                keys.spell_keys(&[ActionId("app.down"), ActionId("app.up")])
                    .map(|move_keys| format!("{move_keys} move")),
                pair(keys, "app.open", &enter_words(view, false)),
                pair(keys, "app.back", "back to the message"),
            ]
        }
    };
    parts.into_iter().flatten().collect::<Vec<_>>().join(GAP)
}

/// The subject bar's parts by rank (08s m4): short of room, the lowest
/// goes first.
const SEE_CHANGES: u8 = 0;
const MORE_DETAIL: u8 = 1;
const CANCEL: u8 = 2;
const COMMIT: u8 = 3;

/// The subject's bar parts in drawn order, each with its rank.
fn subject_parts(view: &CommitScreenView<'_>, keys: &Keymap) -> [(u8, Option<String>); 4] {
    [
        (COMMIT, pair(keys, "app.open", &enter_words(view, true))),
        (MORE_DETAIL, pair(keys, "commit.next_field", "more detail")),
        (
            SEE_CHANGES,
            Some(format!("{} see a file's changes", arrows())),
        ),
        (
            CANCEL,
            pair(keys, "app.back", "cancel, your message is kept"),
        ),
    ]
}

/// The bar as drawn in `width` columns, whole parts only. The subject's
/// leaves out `↑/↓ see a file's changes` while no diff is on screen
/// (`diff_shown` false) and, short of room, drops its parts by rank, lowest
/// first: `↑/↓`, `⇥ more detail`, `esc cancel…`, `⏎` (08s m4). The other
/// bars drop from the right ([`whole_parts`]).
pub(super) fn fitted_bar(
    view: &CommitScreenView<'_>,
    keys: &Keymap,
    width: usize,
    diff_shown: bool,
) -> String {
    if view.busy || view.focus != CommitFocus::Subject {
        return whole_parts(&bar(view, keys), width);
    }
    let mut parts: Vec<(u8, String)> = subject_parts(view, keys)
        .into_iter()
        .filter(|(rank, _)| diff_shown || *rank != SEE_CHANGES)
        .filter_map(|(rank, part)| Some((rank, part?)))
        .collect();
    loop {
        let line = parts
            .iter()
            .map(|(_, part)| part.as_str())
            .collect::<Vec<_>>()
            .join(GAP);
        let lowest = (0..parts.len()).min_by_key(|at| parts[*at].0);
        match lowest {
            Some(at) if unicode_width::UnicodeWidthStr::width(line.as_str()) > width => {
                parts.remove(at);
            }
            _ => return line,
        }
    }
}

/// The sentence above the files (C4).
#[must_use]
pub fn summary_line(summary: &Summary) -> String {
    match summary {
        Summary::Goes {
            files: count,
            lines,
            onto,
            out,
        } => {
            let verb = if *count == 1 { "goes" } else { "go" };
            let mut line = format!("{} {verb} in", files(*count as usize));
            if let Some((added, removed)) = lines {
                line.push_str(&format!(", +{added} \u{2212}{removed}"));
            }
            match onto {
                Onto::NewCommit(branch) => {
                    line.push_str(&format!(", as a new commit on {}.", branch.as_str()));
                }
                Onto::Amend => line.push_str(", into the last commit."),
            }
            line.push_str(&stay_out(*out));
            line
        }
        Summary::MessageOnly { out } => {
            format!("Only the message changes.{}", stay_out(*out))
        }
        Summary::Nothing => format!("Nothing goes in. Put a file in with {}.", space()),
    }
}

/// ` 1 stays out.`, ` 2 stay out.`, or nothing.
fn stay_out(out: u32) -> String {
    match out {
        0 => String::new(),
        1 => " 1 stays out.".to_owned(),
        many => format!(" {many} stay out."),
    }
}

/// `committing to wallet-release · not pushed yet · signing off`, or
/// `amending a1b2c3d Fix the toc · …` for an amend (C1, Q8 a).
#[must_use]
pub(super) fn header_line(header: &CommitHeader) -> String {
    let mut line = match &header.amending {
        Some(commit) => format!("amending {}", commit.as_str()),
        None => format!("committing to {}", header.branch.as_str()),
    };
    for part in header
        .push
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .chain(header.options.iter().copied())
    {
        line.push_str(" \u{b7} ");
        line.push_str(part);
    }
    line
}

/// The mark before a file: `✓` going in, `!` a secrets file staying out,
/// `·` any other staying out (C5, C7).
pub(super) fn mark(file: &CommitFileRow) -> char {
    match file.group {
        FileGroup::GoingIn => '\u{2713}',
        FileGroup::StayingOut if file.secret => '!',
        FileGroup::StayingOut => '\u{b7}',
    }
}

/// Why a file stays out, said on its row: only a secrets file has one.
pub(super) const SECRET_WHY: &str = "looks like a secrets file";

/// `+12 −3`, `+40`, or nothing before the counts are read.
pub(super) fn counts(file: &CommitFileRow) -> String {
    match (file.added, file.removed) {
        (Some(added), Some(0)) => format!("+{added}"),
        (Some(added), Some(removed)) => format!("+{added} \u{2212}{removed}"),
        _ => String::new(),
    }
}

/// The diff's first line for a secrets file staying out (C7): `! left out:
/// looks like a secrets file. space puts it in.`
pub(super) fn secret_note() -> String {
    format!("! left out: {SECRET_WHY}. {} puts it in.", space())
}

/// The dim word after a staged part's counts (08s M3).
pub(super) const PART: &str = " part";

/// The note over a staged part's diff, in the secrets note's place (08s M3).
pub(super) const PART_NOTE: &str = "only the part you staged goes in; the rest stays uncommitted.";

/// `line`'s `GAP`-separated parts, as many whole ones from the left as fit
/// in `width` columns: a bar too wide loses its last keys, never half a word.
pub(super) fn whole_parts(line: &str, width: usize) -> String {
    let mut kept = String::new();
    for part in line.split(GAP) {
        let next = if kept.is_empty() {
            part.to_owned()
        } else {
            format!("{kept}{GAP}{part}")
        };
        if unicode_width::UnicodeWidthStr::width(next.as_str()) > width {
            break;
        }
        kept = next;
    }
    kept
}
