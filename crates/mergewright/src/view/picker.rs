//! The branch picker's view model (`C`, m7-power-tools/10): the picker's
//! state and its list, worded for `mergewright_ui::picker`.
//!
//! Pure. Which rows show and in what order is
//! `mergewright_ops::switch_list::filter_switch_rows`'s answer (through
//! `PickerState::rows`), so the box and `⏎` can never disagree.

use mergewright_ops::switch_list::{Relation, SwitchKind, SwitchRow};
use mergewright_term::sanitize_line;
use mergewright_ui::graph_screen::columns::relative_date;
use mergewright_ui::picker::{PickerBody, PickerKind, PickerRow, PickerView};

use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::picker::PickerState;

/// Terminal columns from which every column is drawn.
const WIDE_FROM: u16 = 120;

/// The box's contents, while the picker is shown; `None` otherwise (closed,
/// or hidden under the preview it opened).
#[must_use]
pub fn picker_view(state: &AppState) -> Option<PickerView> {
    if !crate::update::picker::shown(state) {
        return None;
    }
    let picker = state.picker.as_ref()?;
    let now = state.harness.clock.now_seconds();
    let query = sanitize_line(picker.query.text());
    let (body, footer) = match &picker.list {
        Loading::Idle | Loading::Pending(_) => (PickerBody::Reading, String::new()),
        // One line: git's words can run to several.
        Loading::Failed(why) => (PickerBody::Failed(one_line(why)), String::new()),
        Loading::Ready(list) => {
            let total = list.rows.len() + usize::from(list.previous.is_some());
            let rows = picker.rows();
            let footer = if picker.query.is_empty() {
                branches(total)
            } else {
                format!("{} of {}", rows.len(), branches(total))
            };
            let body = if total == 0 {
                PickerBody::NoBranches
            } else if rows.is_empty() {
                PickerBody::NoMatch(query.clone())
            } else {
                PickerBody::Rows {
                    selected: picker.selected.min(rows.len() - 1),
                    rows: rows.iter().map(|row| worded(picker, row, now)).collect(),
                }
            };
            (body, footer)
        }
    };
    Some(PickerView {
        query,
        body,
        footer,
        note: picker.note.as_deref().map(sanitize_line),
        wide: state.size.columns >= WIDE_FROM,
    })
}

fn branches(count: usize) -> String {
    if count == 1 {
        "1 branch".to_owned()
    } else {
        format!("{count} branches")
    }
}

fn worded(picker: &PickerState, row: &SwitchRow, now: i64) -> PickerRow {
    let kind = if row.current {
        PickerKind::Here
    } else if picker.is_previous(row) {
        PickerKind::Before
    } else if matches!(row.kind, SwitchKind::RemoteOnly { .. }) {
        PickerKind::Remote
    } else {
        PickerKind::Local
    };
    let relation = match (&row.kind, &row.relation) {
        (SwitchKind::RemoteOnly { .. }, _) => "no copy of yours yet".to_owned(),
        (SwitchKind::Local, Relation::Tracks(upstream)) => {
            clean(&format!("tracks {}", short(upstream.shorthand())))
        }
        (SwitchKind::Local, Relation::BuiltOn(upstream)) => {
            clean(&format!("built on {}", short(upstream.shorthand())))
        }
        (SwitchKind::Local, Relation::None) => String::new(),
    };
    PickerRow {
        kind,
        name: sanitize_line(&short(row.name.shorthand())),
        // The first name, as the prototype's rows read (`Sara`).
        who: sanitize_line(row.author.split_whitespace().next().unwrap_or("")),
        age: relative_date(row.when, now),
        subject: sanitize_line(&row.subject),
        relation,
    }
}

/// git's failure, its lines joined by a space and sanitized: the box has
/// one row for it.
fn one_line(why: &str) -> mergewright_term::SanitizedText {
    let joined: Vec<&str> = why
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    sanitize_line(&joined.join(" "))
}

/// A ref name is text from the repository: sanitized before it is drawn.
fn clean(text: &str) -> String {
    sanitize_line(text).as_str().to_owned()
}

fn short(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
