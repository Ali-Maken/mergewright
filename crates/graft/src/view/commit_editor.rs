//! Turning [`AppState`]'s commit screen into the view model
//! [`graft_ui::commit_editor::CommitScreen`] draws (ease-pass/08, the
//! commit mock).
//!
//! Every string that came out of the repository — the branch name, the
//! paths, the short SHA, the failure — goes through
//! `graft_term::sanitize_line` here, because the widget takes
//! [`SanitizedText`] and nothing else.

use graft_term::{SanitizedText, sanitize_line};
use graft_ui::commit_editor::{
    BODY_LIMIT, CommitFileRow, CommitFocus, CommitHeader, CommitScreenView, FileGroup, Onto,
    SUBJECT_LIMIT, Summary,
};
use graft_ui::detail::BodyView;
use graft_ui::text_input::{AreaView, FieldView};

use crate::update::AppState;
use crate::update::commit_editor::{
    CommitEditor, CommitOption, head_oid, kind_of, push_words, short_sha, shown_section,
};

/// The screen as it draws now, or `None` when none is open.
#[must_use]
pub fn commit_screen_view(state: &AppState) -> Option<CommitScreenView<'static>> {
    let editor = state.commit_editor.as_ref()?;
    let order = crate::update::commit_editor::display_order(&editor.files);
    let files: Vec<CommitFileRow> = order.iter().map(|at| row(state, editor, *at)).collect();
    let cursor = order
        .iter()
        .position(|at| *at == editor.file_cursor)
        .unwrap_or(0);
    Some(CommitScreenView {
        header: header(state, editor),
        subject: FieldView {
            label: "",
            text: sanitize_line(editor.subject.text()),
            cursor: (editor.focus == CommitFocus::Subject).then(|| editor.subject.cursor()),
            guide_at: None,
            counter: Some((editor.subject.len(), SUBJECT_LIMIT)),
            focused: editor.focus == CommitFocus::Subject,
            secret: None,
        },
        body: AreaView {
            label: "",
            lines: editor
                .body
                .lines()
                .iter()
                .map(|line| sanitize_line(line))
                .collect(),
            cursor: (editor.focus == CommitFocus::Body).then(|| editor.body.cursor()),
            guide_at: Some(BODY_LIMIT),
            focused: editor.focus == CommitFocus::Body,
        },
        summary: summary(state, editor, &files),
        files,
        cursor,
        focus: editor.focus,
        diff: diff(editor),
        refusal: editor.refusal(),
        // A hook's refusal is the commonest failure here and the screen is
        // what is up when it lands, so the screen draws it, until `esc`
        // dismisses it (m3/05 review, Major 1). `shown_status` like every
        // status reader (m6-conflicts/78).
        status: super::status::problem(state.shown_status()),
        busy: state.operating.is_some(),
        live: crate::update::commit_editor::live_words(state).map(|words| sanitize_line(&words)),
    })
}

/// One file as the list draws it, its counts from the line-count read when
/// it has landed.
fn row(state: &AppState, editor: &CommitEditor, at: usize) -> CommitFileRow {
    let choice = &editor.files[at];
    let counted = editor
        .counts
        .as_deref()
        .and_then(|list| crate::update::commit_editor::counted(list, &choice.path));
    let entry = state.working_tree.as_deref().and_then(|status| {
        status
            .entries
            .iter()
            .find(|entry| entry.path == choice.path)
    });
    CommitFileRow {
        path: sanitize_line(&choice.path.to_string_lossy()),
        kind: entry.map_or('M', kind_of),
        group: if choice.goes_in {
            FileGroup::GoingIn
        } else {
            FileGroup::StayingOut
        },
        added: counted.and_then(|entry| entry.added),
        removed: counted.and_then(|entry| entry.removed),
        secret: matches!(
            choice.why_out,
            Some(graft_ops::operation::OutWhy::LooksLikeSecrets)
        ),
        // Only a part goes in when there is a rest left behind: a file
        // staged whole is all of it (08s M3).
        part_only: shown_section(&choice.part).is_some()
            && entry.is_some_and(|entry| entry.unstaged.is_some()),
    }
}

/// The sentence above the files (C4).
fn summary(state: &AppState, editor: &CommitEditor, files: &[CommitFileRow]) -> Summary {
    let count = |group| {
        u32::try_from(files.iter().filter(|file| file.group == group).count()).unwrap_or(u32::MAX)
    };
    let going = count(FileGroup::GoingIn);
    let out = count(FileGroup::StayingOut);
    if going == 0 {
        return if editor.message_only() {
            Summary::MessageOnly { out }
        } else {
            Summary::Nothing
        };
    }
    // Every file going in has its counts, or the sum would understate.
    let lines = files
        .iter()
        .filter(|file| file.group == FileGroup::GoingIn)
        .try_fold((0u32, 0u32), |(added, removed), file| {
            Some((
                added.saturating_add(file.added?),
                removed.saturating_add(file.removed.unwrap_or(0)),
            ))
        });
    let onto = if editor.amend {
        Onto::Amend
    } else {
        Onto::NewCommit(branch(state))
    };
    Summary::Goes {
        files: going,
        lines,
        onto,
        out,
    }
}

/// The header: branch, amend, push state, options on (Q8 a).
fn header(state: &AppState, editor: &CommitEditor) -> CommitHeader {
    CommitHeader {
        branch: branch(state),
        amending: amending(state, editor).map(|words| sanitize_line(&words)),
        push: push_words(state, true).map(|words| sanitize_line(&words)),
        options: CommitOption::HEADER_ORDER
            .iter()
            .filter(|option| editor.option(**option))
            .filter_map(|option| option.header_word())
            .collect(),
    }
}

/// `<sha7> <subject>` of the commit `⏎` rewrites while amend is on: what
/// `a` read, or — amend turned on from `c` (08s M2) — `HEAD` from the refs
/// on hand and its subject from its loaded row, no new read. `None` while
/// amend is off: the header says `committing to <branch>`.
fn amending(state: &AppState, editor: &CommitEditor) -> Option<String> {
    if !editor.amend {
        return None;
    }
    if let Some(short) = &editor.head_short {
        return Some(short.clone());
    }
    let Some(head) = head_oid(state) else {
        return Some("HEAD".to_owned());
    };
    let short = short_sha(head);
    let history = state.history.as_ref();
    let row = history
        .and_then(|history| history.window.as_ref())
        .and_then(|window| window.rows.iter().find(|row| row.commit.id == head));
    Some(match row {
        Some(row) => format!(
            "{short} {}",
            String::from_utf8_lossy(row.commit.subject()).trim()
        ),
        None => short,
    })
}

/// The branch `HEAD` is on, or `HEAD`'s short SHA when detached.
fn branch(state: &AppState) -> SanitizedText {
    let name = state.repository.as_ref().map(|facts| match &facts.head {
        crate::repository::Head::Branch(name) | crate::repository::Head::Unborn(name) => {
            String::from_utf8_lossy(name).into_owned()
        }
        crate::repository::Head::Detached(oid) => oid.to_string().chars().take(7).collect(),
        crate::repository::Head::Unreadable(_) => "HEAD".to_owned(),
    });
    sanitize_line(name.as_deref().unwrap_or("HEAD"))
}

/// The diff of the file under the cursor.
fn diff(editor: &CommitEditor) -> BodyView {
    if editor.files.is_empty() {
        return BodyView::Empty(sanitize_line("no changes"));
    }
    match &editor.diff.view {
        Some(Ok(view)) => BodyView::Diff(std::sync::Arc::clone(view)),
        Some(Err(why)) => BodyView::Failed(sanitize_line(why)),
        None => BodyView::Loading(sanitize_line("reading…")),
    }
}
