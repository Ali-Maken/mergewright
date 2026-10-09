//! The drift box's consequence words and their fit rule, and the landing's
//! way back after a forced run (m6-conflicts/77).
//!
//! Pure text: nothing here touches `AppState`. Moved out of `update::undo`
//! whole (m6-conflicts/80), because these words are the part that changes.

use graft_term::sanitize_line;

use crate::update::state::Status;

/// The drift line's wrapped lines with the consequence line under them,
/// fitted into what graft-ui has left, each line sanitized for the box.
pub(super) fn with_consequence(
    mut message: Vec<String>,
    drift: &graft_ops::undo::Drift,
    redo: bool,
    presses: usize,
) -> Vec<graft_term::SanitizedText> {
    let room = CONFIRM_LINES.saturating_sub(message.len());
    message.extend(wrapped(&drift_consequence(
        drift,
        redo,
        presses,
        CONFIRM_COLUMNS,
        room,
    )));
    message.iter().map(|line| sanitize_line(line)).collect()
}

/// The widest line the confirm box draws without re-wrapping it: the box
/// is 64 columns at its narrowest (`graft_ui::confirm::confirm_area`) and
/// wraps its message 4 columns inside that. A `…` where the drift's own
/// words should be is the one thing this box must not do.
const CONFIRM_COLUMNS: u16 = 60;

/// The most message lines graft-ui draws before it cuts the rest with a
/// `…` (`graft-ui/src/confirm.rs`, `MAX_MESSAGE_LINES`).
const CONFIRM_LINES: usize = 4;

/// The drift box's consequence line (m6-conflicts/77, Decided 6): what the
/// run replaces, that a backup keeps it, and the key that brings it back —
/// fitted to `width` in at most `lines` wrapped lines, so graft-ui's cap
/// never cuts it.
///
/// `presses` is how many `U`s the way back takes: 1 for `u`, the jump's
/// step count for the time machine. A redo names **no** key: `u` after a
/// redo takes back the operation at the root (`graft_ops::journal::
/// next_undo` steps over `Undo` and `Redo` entries), which writes that
/// operation's before-state, not the edit the redo replaced. The backup
/// keeps it, and the line says only that.
///
/// To fit, the one path is cut from the left (`…/fix.txt`) until the
/// sentence fits; if the drift line above left a single line, the sentence
/// also drops `a backup keeps it, and ` and keeps the key. When even that
/// cannot fit the short form is returned whole and graft-ui cuts it.
pub(crate) fn drift_consequence(
    drift: &graft_ops::undo::Drift,
    redo: bool,
    presses: usize,
    width: u16,
    lines: usize,
) -> String {
    let verb = if redo { "Redo" } else { "Undo" };
    let key = (!redo).then(|| way_back_key(presses));
    let replaced = &drift.replaced;
    if replaced.is_empty() {
        // Nothing the run writes differs from what the entry left: the
        // user's edits (if any) are on other paths and stay put.
        return if drift.worktree || drift.index.is_some() {
            format!("{verb} leaves your edits since as they are.")
        } else {
            match key {
                Some(key) => format!("A backup is taken first, and {key} brings this state back."),
                None => "A backup is taken first.".to_owned(),
            }
        };
    }
    let (named, it) = named_replaced(replaced);
    let sentence = |what: &str, long: bool| match (&key, long) {
        (Some(key), true) => {
            format!("{verb} replaces {what}; a backup keeps {it}, and {key} brings {it} back.")
        }
        (Some(key), false) => format!("{verb} replaces {what}; {key} brings {it} back."),
        (None, _) => format!("{verb} replaces {what}; a backup keeps {it}."),
    };
    let width = usize::from(width);
    let fits = |text: &str| {
        let wrapped = wrapped_at(text, width);
        wrapped.len() <= lines && wrapped.iter().all(|line| line.chars().count() <= width)
    };
    let shown = match &named {
        Named::Path(path) => left_cuts(path),
        Named::Whole(what) => vec![what.clone()],
    };
    let described = |shown: &str| match &named {
        Named::Path(_) => format!("your edit to {shown}"),
        Named::Whole(_) => shown.to_owned(),
    };
    for long in [true, false] {
        if let Some(found) = shown
            .iter()
            .map(|shown| sentence(&described(shown), long))
            .find(|text| fits(text))
        {
            return found;
        }
    }
    sentence(&described(shown.last().map_or("", String::as_str)), false)
}

/// The footer's width on the narrowest terminal Graft draws for (80
/// columns), less the one-space indent it draws a status with
/// (`view::frame::bars`). A landing is fitted to it here because the
/// graph's footer (graft-ui) cuts a long `Done` line from the end, which
/// takes the key first.
const STATUS_COLUMNS: usize = 79;

/// The status after a forced run (m6-conflicts/77, fitted in /80): `what`,
/// and when the run replaced something, the way back.
///
/// After `u` or the time machine the way back is a key, so it goes in
/// `keys`: `U brings back your edit to fix.txt`. After a redo no key brings
/// it back (see [`drift_consequence`]), so what the backup keeps is part of
/// what happened and goes in `what`: `redo: checkout fix; a backup keeps
/// your edit to fix.txt` (review m7: `keys` is the next key, as the hint bar
/// words it — `update::state::Status::Done`).
///
/// Fitted to [`STATUS_COLUMNS`] the way the box is to its own: the one path
/// is cut from the left, then dropped for `it`/`them` before the key is; a
/// `what` too long even then is cut with `…` so the key still shows
/// (review m5: the time machine's landing was 93 cells).
pub(super) fn landing(
    what: String,
    replaced: &graft_ops::undo::Replaced,
    redo: bool,
    presses: usize,
) -> Status {
    if replaced.is_empty() {
        return Status::done(what);
    }
    let (named, it) = named_replaced(replaced);
    let mut shown: Vec<String> = match &named {
        Named::Path(path) => left_cuts(path)
            .into_iter()
            .map(|cut| format!("your edit to {cut}"))
            .collect(),
        Named::Whole(whole) => vec![whole.clone()],
    };
    if redo {
        shown.push(it.to_owned());
        let told = |shown: &str| format!("{what}; a backup keeps {shown}");
        let found = shown
            .iter()
            .map(|shown| told(shown))
            .find(|told| done_width(told, None) <= STATUS_COLUMNS);
        return Status::done(found.unwrap_or_else(|| {
            let tail = format!("; a backup keeps {it}");
            let room = STATUS_COLUMNS.saturating_sub(done_width(&tail, None));
            format!("{}{tail}", cut_end(&what, room))
        }));
    }
    let key = way_back_key(presses);
    let mut keys: Vec<String> = shown
        .iter()
        .map(|shown| format!("{key} brings back {shown}"))
        .collect();
    keys.push(format!("{key} brings {it} back"));
    if let Some(found) = keys
        .iter()
        .find(|keys| done_width(&what, Some(keys)) <= STATUS_COLUMNS)
    {
        return Status::done_with_keys(what, found.clone());
    }
    let shortest = keys.pop().unwrap_or(key);
    let room = STATUS_COLUMNS.saturating_sub(done_width("", Some(&shortest)));
    Status::done_with_keys(cut_end(&what, room), shortest)
}

/// The cells `view::status::done_line` draws for `what` and `keys`:
/// `<what> ✓` and ` · <keys>`.
fn done_width(what: &str, keys: Option<&str>) -> usize {
    what.chars().count() + 2 + keys.map_or(0, |keys| 3 + keys.chars().count())
}

/// `text` in at most `room` characters, its end cut with `…` when it is
/// longer.
fn cut_end(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_owned();
    }
    let kept: String = text.chars().take(room.saturating_sub(1)).collect();
    format!("{kept}\u{2026}")
}

/// What a [`graft_ops::undo::Replaced`] is called: one path by name,
/// anything wider by what it is. With the pronoun that goes with it.
enum Named {
    /// The one path, as text.
    Path(String),
    /// `the whole working tree`, `the whole index` or `your edits to 3 files`.
    Whole(String),
}

/// A whole restore is named by what it restores (m6-conflicts/80, review
/// m8): an interrupted `stage` restores the index alone and writes no
/// file, so it is `the whole index`, never `the whole working tree`.
fn named_replaced(replaced: &graft_ops::undo::Replaced) -> (Named, &'static str) {
    let whole = match (replaced.whole_tree, replaced.whole_index) {
        (true, true) => Some("the whole working tree and index"),
        (true, false) => Some("the whole working tree"),
        (false, true) => Some("the whole index"),
        (false, false) => None,
    };
    if let Some(whole) = whole {
        return (Named::Whole(whole.to_owned()), "it");
    }
    match replaced.paths.as_slice() {
        [one] => (Named::Path(one.to_string_lossy().into_owned()), "it"),
        many => (
            Named::Whole(format!("your edits to {} files", many.len())),
            "them",
        ),
    }
}

/// `Done`, naming `keys` when there are any.
pub(super) fn done_with(what: String, keys: Option<String>) -> Status {
    match keys {
        Some(keys) => Status::done_with_keys(what, keys),
        None => Status::done(what),
    }
}

/// Whether a finished `Undone::Done` was `U`'s: its entry is a `Redo`.
pub(super) fn applied_is_redo(applied: &graft_ops::apply::Applied) -> bool {
    applied.entry.kind == graft_core::journal::OperationKind::Redo
}

/// `U`, or `U (3 times)` when the way back is several redos.
fn way_back_key(presses: usize) -> String {
    if presses > 1 {
        format!("U ({presses} times)")
    } else {
        "U".to_owned()
    }
}

/// `path` whole, then cut from the left at each `/` (`…/src/fix.txt`,
/// `…/fix.txt`), then the file name itself cut a character at a time —
/// longest first, so the first that fits keeps the most.
fn left_cuts(path: &str) -> Vec<String> {
    let mut cuts = vec![path.to_owned()];
    cuts.extend(
        path.match_indices('/')
            .map(|(at, _)| format!("\u{2026}{}", &path[at..])),
    );
    let name = path.rsplit('/').next().unwrap_or(path);
    let chars: Vec<char> = name.chars().collect();
    cuts.extend((1..chars.len()).map(|from| {
        let kept: String = chars[from..].iter().collect();
        format!("\u{2026}{kept}")
    }));
    cuts
}

/// `text` broken at spaces into lines of at most [`CONFIRM_COLUMNS`]. A
/// single word longer than that is left whole — the box clips it, and
/// cutting a path in half would be worse than a clipped one.
pub(super) fn wrapped(text: &str) -> Vec<String> {
    wrapped_at(text, usize::from(CONFIRM_COLUMNS))
}

/// [`wrapped`] at any width.
fn wrapped_at(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines
}

#[cfg(test)]
mod tests;
