//! The mock's screen as a view model, and its drawn grid: what every
//! commit-screen test file starts from.

use std::sync::Arc;

use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use super::*;
use crate::detail::{BodyView, DiffView, FileStatusView};
use crate::test_support::terminal as test_capabilities;
use crate::text_input::{AreaView, FieldView};
use crate::theme::ThemeKind;

pub(super) fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

pub(super) fn file(
    path: &str,
    kind: char,
    group: FileGroup,
    counts: (u32, u32),
    secret: bool,
) -> CommitFileRow {
    CommitFileRow {
        path: sanitize_line(path),
        kind,
        group,
        added: Some(counts.0),
        removed: Some(counts.1),
        secret,
        part_only: false,
    }
}

/// The mock's four files: three going in, `.env.local` left out.
pub(super) fn files() -> Vec<CommitFileRow> {
    vec![
        file("src/receipt.rs", 'M', FileGroup::GoingIn, (12, 3), false),
        file("src/refund.rs", 'A', FileGroup::GoingIn, (40, 0), false),
        file("tests/receipt.rs", 'M', FileGroup::GoingIn, (8, 0), false),
        file(".env.local", 'A', FileGroup::StayingOut, (1, 0), true),
    ]
}

pub(super) fn view<'a>(subject: &str, focus: CommitFocus) -> CommitScreenView<'a> {
    let typed = subject.chars().count();
    CommitScreenView {
        header: CommitHeader {
            location: None,
            branch: sanitize_line("wallet-release"),
            amending: None,
            push: Some(sanitize_line("not pushed yet")),
            options: Vec::new(),
        },
        subject: FieldView {
            label: "",
            text: sanitize_line(subject),
            cursor: (focus == CommitFocus::Subject).then_some(typed),
            guide_at: None,
            counter: Some((typed, SUBJECT_LIMIT)),
            focused: focus == CommitFocus::Subject,
            secret: None,
        },
        body: AreaView {
            label: "",
            lines: vec![sanitize_line("")],
            cursor: (focus == CommitFocus::Body).then_some((0, 0)),
            guide_at: Some(BODY_LIMIT),
            focused: focus == CommitFocus::Body,
        },
        summary: Summary::Goes {
            files: 3,
            lines: Some((60, 3)),
            onto: Onto::NewCommit(sanitize_line("wallet-release")),
            out: 1,
        },
        files: files(),
        cursor: 0,
        focus,
        diff: BodyView::Diff(Arc::new(DiffView {
            path: sanitize_line("src/receipt.rs"),
            old_path: None,
            status: FileStatusView::Modified,
            hunks: Vec::new(),
            binary: None,
            truncated: None,
            untracked: false,
        })),
        refusal: (typed == 0).then_some(Refusal::NoMessage),
        status: None,
        busy: false,
        live: None,
    }
}

pub(super) fn buffer(view: &CommitScreenView<'_>, columns: u16, rows: u16) -> Buffer {
    let theme = theme();
    let mut terminal = Terminal::new(TestBackend::new(columns, rows))
        .unwrap_or_else(|why| panic!("test backends always open: {why}"));
    terminal
        .draw(|frame| {
            frame.render_widget(
                &CommitScreen {
                    view,
                    theme: &theme,
                    keys: Keymap::shipped(),
                },
                frame.area(),
            );
        })
        .unwrap_or_else(|why| panic!("test backends always draw: {why}"));
    terminal.backend().buffer().clone()
}

pub(super) fn grid(buffer: &Buffer) -> String {
    let area = buffer.area;
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn draw(view: &CommitScreenView<'_>, columns: u16, rows: u16) -> String {
    grid(&buffer(view, columns, rows))
}

pub(super) fn last_line(drawn: &str) -> &str {
    drawn.lines().last().unwrap_or_default()
}
