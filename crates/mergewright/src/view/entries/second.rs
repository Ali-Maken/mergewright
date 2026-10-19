//! The cursor row's second line (ease-pass/07b, mock G9), in words:
//! `<who> · <N hours> ago · made on <branch>[ · merged N from <b>][ · also
//! in A, B] · <sha7>`; on the uncommitted row `not committed yet · N files ·
//! on <branch>`.
//!
//! `also in` draws only from a "contained in" answer already held for the
//! commit: the read is the detail pane's, and a rest with the pane hidden
//! does not ask it (settle test 1, `event_loop::tests::settle::contains`).
//! This adds no read and no trigger. Never `reading…`.

use mergewright_git::status::WorkingTreeStatus;
use mergewright_graph::row::Row;
use mergewright_term::sanitize_line;
use mergewright_ui::detail::ContainedIn;
use mergewright_ui::graph_screen::columns::{relative_date_long, short_sha};
use mergewright_ui::graph_screen::{FoldCount, GraphEntry, SecondLine};

use crate::history::History;
use crate::update::AppState;

const SEP: &str = " \u{b7} ";

/// Gives the cursor's entry its second line: the uncommitted row's while
/// the cursor is on it, else the cursor commit row's (`rows` drawn from
/// `entries[first..]`). A Ghost Preview's rows draw none.
pub(super) fn attach(
    state: &AppState,
    history: &History,
    uncommitted: Option<&WorkingTreeStatus>,
    (rows, first): (&[Row], usize),
    entries: &mut [GraphEntry<'_>],
    now: i64,
) {
    if state.preview.is_some() {
        return;
    }
    if state.on_working_tree {
        if let (Some(status), Some(entry)) = (uncommitted, entries.first_mut()) {
            entry.second = Some(uncommitted_second_line(state, status));
        }
    } else if let Some(at) = history.cursor.checked_sub(history.top)
        && let Some(row) = rows.get(at)
    {
        let line = commit_second_line(
            state,
            history,
            (row, history.cursor),
            &entries[first + at],
            now,
        );
        entries[first + at].second = Some(line);
    }
}

/// The second line of the commit row at `index`, drawn as `entry`.
fn commit_second_line(
    state: &AppState,
    history: &History,
    (row, index): (&Row, usize),
    entry: &GraphEntry<'_>,
    now: i64,
) -> SecondLine {
    let mut parts: Vec<String> = Vec::new();
    if let Some(who) = &entry.who {
        parts.push(who.name.as_str().to_owned());
    }
    parts.push(relative_date_long(row.commit.author.when.seconds, now));
    let made = made_on_name(state, history, row, index);
    if let Some(name) = &made {
        parts.push(format!("made on {name}"));
    }
    if let Some(fold) = &entry.fold
        && let FoldCount::Known(count) = fold.count
        && let Some(from) = &fold.from
    {
        // By its last segment, as the branch column names a merged side
        // (ease-pass 07): `merged 3 from fix-toc`.
        let last = from.as_str().rsplit('/').next().unwrap_or(from.as_str());
        parts.push(format!("merged {count} from {last}"));
    }
    if let Some(ContainedIn::Known { branches, .. }) = state.detail.contained.get(&row.commit.id) {
        let also: Vec<&str> = branches
            .iter()
            .map(mergewright_term::SanitizedText::as_str)
            .filter(|branch| made.as_deref() != Some(*branch))
            .collect();
        if !also.is_empty() {
            parts.push(format!("also in {}", also.join(", ")));
        }
    }
    SecondLine {
        words: sanitize_line(&parts.join(SEP)),
        sha7: sanitize_line(&short_sha(&row.commit.id.to_string(), 7)),
    }
}

/// The branch a row was made on, as the branch column names it; a row
/// inside an opened merge: the branch the merge's subject names.
fn made_on_name(state: &AppState, history: &History, row: &Row, index: usize) -> Option<String> {
    let name = if let Some(inside) = crate::view::rows::inside_merge(history, index) {
        inside.label?
    } else {
        let made = mergewright_graph::made_on(row)?;
        super::branch_name(state, history, made)?
    };
    Some(String::from_utf8_lossy(&name).into_owned())
}

/// The uncommitted row's second line: `not committed yet · N files · on
/// <branch>` (no `on` while `HEAD` is detached).
fn uncommitted_second_line(state: &AppState, status: &WorkingTreeStatus) -> SecondLine {
    let files = status.entries.len();
    let mut words = format!(
        "not committed yet{SEP}{files} file{}",
        if files == 1 { "" } else { "s" }
    );
    let refs = state.refs.as_deref();
    let branch = refs.and_then(|refs| {
        refs.on(refs.head()?).iter().find_map(|label| match label {
            crate::refs::RefLabel::Branch {
                name,
                current: true,
                ..
            } => Some(String::from_utf8_lossy(name).into_owned()),
            _ => None,
        })
    });
    if let Some(branch) = branch {
        words.push_str(&format!("{SEP}on {branch}"));
    }
    SecondLine {
        words: sanitize_line(&words),
        sha7: sanitize_line(""),
    }
}
