//! The journey count (m6-conflicts/21, UX 1.3): the note's `journey-counts`
//! block, read from `docs/design-notes/m6-conflicts.md` rather than restated
//! here, and the check that a journey's route sent no more inputs than its
//! row allows. Also the two things a journey reads or does through the
//! **drawn** frame: its landing facts ([`frame_sees`]) and a click on the
//! file list ([`click_file`]).
//!
//! A journey is a scenario whose name is a row id of the block, or a row id
//! followed by `@<variant>` (the same journey on another fixture, start or
//! size). Its counted route sits between `Step::Route(true)` and
//! `Step::Route(false)`.

use graft::update::AppState;
use graft_core::event::MouseButton;

use super::steps::{Scenario, Sees, Step};
use crate::app_harness::App;
use crate::repo_truth::keys_label;

/// Where the note lives, from the crate's own directory.
pub const NOTE: &str = "../../docs/design-notes/m6-conflicts.md";
/// The fence the walk parses (every one of them). `journey-counts-superseded`
/// is not it.
const FENCE: &str = "```journey-counts";

/// One row of the block: `id keys [token…] [# comment]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The journey's id.
    pub id: String,
    /// The keys the journey may take. `None` for `-`: the journey leaves
    /// the app, so there is nothing to count.
    pub keys: Option<usize>,
    /// For a whole row: the ids its comment adds up (`a + b + …`).
    pub parts: Vec<String>,
}

/// The note's text.
pub fn note_text() -> Result<String, String> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(NOTE);
    std::fs::read_to_string(&path).map_err(|why| format!("{}: {why}", path.display()))
}

/// The rows of every fence tagged exactly `journey-counts`, in order; an id
/// counted in two fences is an error.
///
/// Tokens after `keys` of the form `P6:n` are read and not asserted; any
/// other token is an error, so a typo cannot pass as a comment. A comment
/// whose `+`-separated parts are all ids of its own fence names the row's
/// parts.
pub fn parse_counts(note: &str) -> Result<Vec<Row>, String> {
    let starts: Vec<usize> = note
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim_end() == FENCE)
        .map(|(number, _)| number)
        .collect();
    if starts.is_empty() {
        return Err(format!("no {FENCE} fence in the note"));
    }
    // A note may keep more than one fence (M7 has pass 2a's and 2b's): each
    // is read, in order, and an id is a journey's only once across them.
    let mut rows: Vec<Row> = Vec::new();
    for start in starts {
        for row in parse_fence(note, start)? {
            if rows.iter().any(|seen| seen.id == row.id) {
                return Err(format!(
                    "journey {} is counted twice (second time in the fence at line {})",
                    row.id,
                    start + 1
                ));
            }
            rows.push(row);
        }
    }
    Ok(rows)
}

/// The rows of the fence opening at line index `start`. A comment's parts
/// are ids of this fence.
fn parse_fence(note: &str, start: usize) -> Result<Vec<Row>, String> {
    let mut raw = Vec::new();
    let mut closed = false;
    for (number, line) in note.lines().enumerate().skip(start + 1) {
        let line = line.trim();
        if line.starts_with("```") {
            closed = true;
            break;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        raw.push(parse_row(line).map_err(|why| format!("note line {}: {why}", number + 1))?);
    }
    if !closed {
        return Err(format!("the {FENCE} fence never closes"));
    }
    let ids: Vec<String> = raw.iter().map(|(id, _, _)| id.clone()).collect();
    let rows = raw
        .into_iter()
        .map(|(id, keys, comment)| {
            let named: Vec<String> = comment.split('+').map(|p| p.trim().to_owned()).collect();
            let parts = if named.len() > 1 && named.iter().all(|p| ids.contains(p)) {
                named
            } else {
                Vec::new()
            };
            Row { id, keys, parts }
        })
        .collect();
    Ok(rows)
}

/// One row, before its comment is read for parts: `(id, keys, comment)`.
fn parse_row(line: &str) -> Result<(String, Option<usize>, String), String> {
    let (body, comment) = match line.split_once('#') {
        Some((body, comment)) => (body, comment.trim()),
        None => (line, ""),
    };
    let mut tokens = body.split_whitespace();
    let id = tokens.next().ok_or("no id")?;
    let keys = match tokens.next() {
        Some("-") => None,
        Some(n) => Some(
            n.parse::<usize>()
                .map_err(|why| format!("{id}: keys {n:?}: {why}"))?,
        ),
        None => return Err(format!("{id} has no keys")),
    };
    for token in tokens {
        let approved = token
            .split_once(':')
            .is_some_and(|(p, n)| p.starts_with('P') && n.parse::<usize>().is_ok());
        if !approved {
            return Err(format!(
                "{id}: {token:?} is neither `Pn:keys` nor a comment"
            ));
        }
    }
    Ok((id.to_owned(), keys, comment.to_owned()))
}

/// The keys the note's block allows journey `id`. For a walk that counts
/// its own keys outside the scenario runner (`m6_stash_put_back`), so its
/// budget is read from the note, not restated.
#[allow(
    dead_code,
    reason = "used by the stash walks in tests/it; the m5_walk example does not include them"
)]
pub fn noted_keys(id: &str) -> Result<usize, String> {
    let rows = parse_counts(&note_text()?)?;
    rows.iter()
        .find(|row| row.id == id)
        .ok_or_else(|| format!("the note's journey-counts has no row {id}"))?
        .keys
        .ok_or_else(|| format!("{id}: the note gives no count (`-`)"))
}

/// The journey id a scenario walks: its name up to `@`, when that is a row
/// of the block.
pub fn journey_id<'a>(name: &'a str, rows: &[Row]) -> Option<&'a str> {
    let id = name.split_once('@').map_or(name, |(id, _)| id);
    rows.iter().any(|row| row.id == id).then_some(id)
}

/// The scenario's route is within its row's `keys` — or the scenario is not
/// a journey and counted nothing. The failure names both numbers and the
/// inputs sent.
pub fn within_the_note(scenario: &Scenario, inputs: Option<&Vec<String>>) -> Result<(), String> {
    let rows = parse_counts(&note_text()?)?;
    let Some(id) = journey_id(scenario.name, &rows) else {
        return match inputs {
            None => Ok(()),
            Some(_) => Err(format!(
                "{} counts a route, but the note's journey-counts has no row for it",
                scenario.name
            )),
        };
    };
    let inputs = inputs.ok_or_else(|| {
        format!(
            "{} is journey {id} but never starts its route (Step::Route)",
            scenario.name
        )
    })?;
    let row = rows
        .iter()
        .find(|row| row.id == id)
        .ok_or_else(|| format!("no row {id}"))?;
    let allowed = row
        .keys
        .ok_or_else(|| format!("{id}: the note gives no count (`-`) and the walk counted one"))?;
    if inputs.len() > allowed {
        return Err(format!(
            "{}: {} inputs sent, the note's journey-counts allows {allowed}: {}",
            scenario.name,
            inputs.len(),
            inputs.join(" ")
        ));
    }
    Ok(())
}

/// What one step sends, as UX 1.3 counts it: one label per input. Steps
/// that send nothing (waits, marks, truth reads, frame reads) are empty. A
/// step that sends a number of keys decided at run time (`CursorTo`,
/// `Resolve`, a menu pick) cannot sit inside a route.
pub fn inputs_of(step: &Step) -> Result<Vec<String>, String> {
    Ok(match step {
        Step::Key(key) | Step::KeyFor(_, key, _) | Step::HandOver(_, key) => {
            vec![keys_label(&[*key])]
        }
        Step::Type(text) => vec![format!("type {text:?}")],
        Step::TypeSecret(_) => vec!["type (secret)".to_owned()],
        Step::Cancel(_) => vec!["Esc".to_owned()],
        Step::Confirm => vec!["→".to_owned(), "⏎".to_owned()],
        Step::Choose(index) => {
            let mut keys = vec!["→".to_owned(); *index];
            keys.push("⏎".to_owned());
            keys
        }
        Step::Click(path) => vec![format!("click {path}")],
        Step::WaitFor(_)
        | Step::Claim
        | Step::Mark(_)
        | Step::See(..)
        | Step::Frame(..)
        | Step::Shell(_)
        | Step::Write(_)
        | Step::Settle(_)
        | Step::Route(_) => Vec::new(),
        Step::CursorTo(_)
        | Step::CursorToSubject(_)
        | Step::MenuPick(_)
        | Step::MenuPickOpen(_)
        | Step::DragBadgeOnto(_)
        | Step::Stage(_)
        | Step::Resolve(_)
        | Step::Relaunch
        | Step::Kill => {
            return Err(
                "a step whose key count is decided at run time cannot sit inside a route"
                    .to_owned(),
            );
        }
    })
}

/// The walk's three journey steps: [`Step::Route`] (counted by the walk
/// itself, nothing to do here), [`Step::Click`], [`Step::Frame`].
pub fn step(app: &App, state: AppState, step: &Step) -> Result<AppState, String> {
    match step {
        Step::Click(path) => click_file(app, &state, path),
        Step::Frame(what, sees) => {
            frame_sees(&state, sees).map_err(|why| format!("{what}: {why}"))?;
            Ok(state)
        }
        _ => Ok(state),
    }
}

/// The frame the view draws for `state` at its own size.
fn drawn(state: &AppState) -> Vec<String> {
    crate::captures::frame(state, state.size.columns, state.size.rows)
}

/// Every fact in `sees` holds on the drawn frame. A failure names the fact
/// and the frame's bottom row; with `TRIB_WALK_FRAMES=<dir>` the whole
/// frame is written there too, so a failing run can be read without
/// printing it.
pub fn frame_sees(state: &AppState, sees: &[Sees]) -> Result<(), String> {
    let frame = drawn(state);
    let bottom = frame
        .iter()
        .rev()
        .find(|row| !row.trim().is_empty())
        .map_or("", |row| row.trim_start());
    for fact in sees {
        let holds = match fact {
            Sees::Has(text) => frame.iter().any(|row| row.contains(text)),
            Sees::Lacks(text) => !frame.iter().any(|row| row.contains(text)),
            Sees::BottomStarts(text) => bottom.starts_with(text),
            Sees::Row(first, second) => frame.iter().any(|row| {
                row.find(first)
                    .is_some_and(|at| row[at + first.len()..].contains(second))
            }),
            Sees::Above(upper, lower) => {
                let at = |text: &str| frame.iter().position(|row| row.contains(text));
                matches!((at(upper), at(lower)), (Some(u), Some(l)) if u < l)
            }
        };
        if !holds {
            let dumped = dump(&frame);
            return Err(format!(
                "{fact:?} does not hold; bottom row {bottom:?}{dumped}"
            ));
        }
    }
    Ok(())
}

/// With `TRIB_WALK_FRAMES` set, write the frame to a fresh file there and
/// say where.
fn dump(frame: &[String]) -> String {
    let Some(dir) = std::env::var_os("TRIB_WALK_FRAMES") else {
        return String::new();
    };
    let dir = std::path::PathBuf::from(dir);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let path = dir.join(format!("frame-{nanos}.txt"));
    match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, frame.join("\n"))) {
        Ok(()) => format!("; frame in {}", path.display()),
        Err(why) => format!(
            "; the frame could not be written to {}: {why}",
            path.display()
        ),
    }
}

/// A left click on `path` in the resolver's file list, where the frame
/// draws it: the first row under the header that names it left of any `│`
/// (the strip's row below 180 columns; the list column at 180 and over,
/// whose rows the panes' `│` follows).
pub fn click_file(app: &App, state: &AppState, path: &str) -> Result<AppState, String> {
    let frame = drawn(state);
    for (y, row) in frame.iter().enumerate().skip(1) {
        let list = row.split('│').next().unwrap_or(row);
        let Some(byte) = whole_word(list, path).or_else(|| cut_name(list, path)) else {
            continue;
        };
        let column = list[..byte].chars().count();
        let (column, y) = (
            u16::try_from(column).map_err(|why| format!("column {column}: {why}"))?,
            u16::try_from(y).map_err(|why| format!("row {y}: {why}"))?,
        );
        return Ok(app.mouse_down(state, MouseButton::Left, column, y));
    }
    let dumped = dump(&frame);
    Err(format!("no list row names {path}{dumped}"))
}

/// The byte where `path` starts cut from the left, as the list draws a name
/// wider than its column (`…tes.txt`): a `…` followed by at least four
/// trailing characters of `path`, as a whole word. The user clicks the row
/// as drawn, so the walk does too.
fn cut_name(text: &str, path: &str) -> Option<usize> {
    text.match_indices('…').find_map(|(at, ellipsis)| {
        let after = &text[at + ellipsis.len()..];
        let tail: String = after
            .chars()
            .take_while(|next| next.is_alphanumeric() || matches!(next, '.' | '_' | '-' | '/'))
            .collect();
        (tail.chars().count() >= 4 && path.ends_with(&tail)).then_some(at)
    })
}

/// The byte where `word` starts in `text` with no path character right
/// after it (so `a.txt` is not found inside `a.txt.orig`).
fn whole_word(text: &str, word: &str) -> Option<usize> {
    text.match_indices(word)
        .find(|(at, _)| {
            text[at + word.len()..].chars().next().is_none_or(|next| {
                !(next.is_alphanumeric() || matches!(next, '.' | '_' | '-' | '/'))
            })
        })
        .map(|(at, _)| at)
}

#[cfg(test)]
mod tests {
    use super::{Row, parse_counts};

    const NOTE: &str = "text\n\
```journey-counts-superseded\n\
old-row   9\n\
```\n\
```journey-counts\n\
# id  keys\n\
one   2\n\
two   0    # JetBrains 0\n\
three 1  P6:1   # lazygit 1 (bare confirm, rejected)\n\
whole 3    # one + two + three\n\
free  5  # esc + 2×⇥ + c + ⏎ (ticket 39)\n\
```\n\
more text\n";

    /// Review m7-16b..21b, Minor 8: the fence itself says 2 for
    /// `menu-checkout`, so a regression to `. ⏎ y` fails the journey walk
    /// and not only `m6_stash_put_back::menu_checkout_is_three_keys`.
    #[test]
    fn menu_checkout_is_fenced_at_two() {
        assert_eq!(super::noted_keys("menu-checkout"), Ok(2));
    }

    #[test]
    fn the_block_is_read_and_the_superseded_one_is_not() {
        let rows = parse_counts(NOTE).unwrap();
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["one", "two", "three", "whole", "free"]);
        assert_eq!(
            rows[3],
            Row {
                id: "whole".to_owned(),
                keys: Some(3),
                parts: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            }
        );
        assert!(rows[4].parts.is_empty(), "esc + … names no journeys");
        assert_eq!(rows[2].keys, Some(1));
    }

    #[test]
    fn a_stray_token_is_an_error_not_a_comment() {
        let note = "```journey-counts\none 2 three\n```\n";
        assert!(parse_counts(note).unwrap_err().contains("three"));
    }

    #[test]
    fn no_block_is_an_error() {
        assert!(parse_counts("```journey-counts-superseded\na 1\n```\n").is_err());
    }

    /// The M7 note keeps two fences on purpose (pass 2a's and 2b's,
    /// `docs/design-notes/m7-power-tools.md`): both are read, in order.
    #[test]
    fn two_fences_are_read_as_one_block() {
        let twice = "```journey-counts\na 1\n```\ntext\n```journey-counts\nb 2\n```\n";
        let rows = parse_counts(twice).unwrap();
        let read: Vec<(&str, Option<usize>)> =
            rows.iter().map(|row| (row.id.as_str(), row.keys)).collect();
        assert_eq!(read, [("a", Some(1)), ("b", Some(2))]);
    }

    #[test]
    fn a_duplicate_id_across_fences_is_refused() {
        let twice = "```journey-counts\na 1\n```\n```journey-counts\nb 1\na 2\n```\n";
        let why = parse_counts(twice).unwrap_err();
        assert!(why.contains("journey a is counted twice"), "{why}");
    }
}
