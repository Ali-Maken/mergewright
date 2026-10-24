//! Part B (ease-pass 26): each failed body drawn at both tiers through
//! [`assert_marked`](super::assert_marked). They sit here, not beside the
//! widgets' own tests, so those files stay under their line ratchet; the
//! fixtures are the widgets' own (`pub(crate)` in their test modules).

use mergewright_term::sanitize::{sanitize, sanitize_line};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

mod blame_failed {
    use crate::blame::tests::*;
    use crate::blame::*;

    /// Part B (ease-pass 26): a failed blame draws the mark, both tiers.
    #[test]
    fn a_failed_blame_draws_the_failure_mark() {
        for ascii in [false, true] {
            let mut failed = view(&[], 0);
            failed.body = BlameBody::Failed("no such path");
            let text = crate::failure::frame_text(80, 6, |frame| {
                draw_blame_in_tier(
                    frame,
                    frame.area(),
                    &failed,
                    crate::keymap::Keymap::shipped(),
                    ascii,
                );
            });
            crate::failure::assert_marked(&text, "git blame stopped: no such path", ascii);
        }
    }
}

mod detail_pane_failed {
    use super::*;
    use crate::detail::{BodyView, ContainedIn, Subject};
    use crate::detail_pane::tests::{commit_view, compare, text};
    use crate::detail_pane::*;

    fn draw(
        view: &crate::detail::DetailView,
        layout: DetailLayout,
        columns: u16,
        rows: u16,
    ) -> ratatui::Terminal<ratatui::backend::TestBackend> {
        let theme = crate::theme::Theme::new(
            crate::theme::ThemeKind::Dark,
            &crate::test_support::terminal(
                mergewright_core::capabilities::ColorDepth::TrueColor,
                false,
            ),
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(columns, rows)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    DetailPane {
                        view,
                        layout: &layout,
                        theme: &theme,
                        keys: crate::keymap::Keymap::shipped(),
                    },
                    frame.area(),
                );
            })
            .expect("a test backend draws");
        terminal
    }

    /// Part B (ease-pass 26): a failed body, a failed compare and a failed
    /// contained-in each draw the mark, both tiers.
    #[test]
    fn failed_body_compare_and_contained_in_draw_the_failure_mark() {
        for ascii in [false, true] {
            let layout = DetailLayout::Right { columns: 80 };
            let mut view = commit_view();
            view.ascii = ascii;
            view.body = BodyView::Failed(sanitize_line("git diff-tree: fatal: bad object"));
            let shown = text(&draw(&view, layout, 80, 24)).join("\n");
            crate::failure::assert_marked(&shown, "git diff-tree: fatal: bad object", ascii);

            let mut view = commit_view();
            view.ascii = ascii;
            view.header.contained = ContainedIn::Failed(sanitize_line("no refs"));
            let shown = text(&draw(&view, layout, 80, 24)).join("\n");
            crate::failure::assert_marked(&shown, "Contained in: no refs", ascii);

            let mut view = commit_view();
            view.ascii = ascii;
            view.header.subject = Subject::Compare(compare(
                "x",
                "main",
                crate::detail::CompareState::Failed(sanitize_line("bad object")),
            ));
            let shown = text(&draw(&view, layout, 80, 24)).join("\n");
            crate::failure::assert_marked(&shown, "couldn't compare: bad object", ascii);
        }
    }
}

mod finish_summary_failed {
    use super::*;
    use crate::finish_summary::tests::*;
    use crate::finish_summary::*;

    /// B3: a notice that is a problem leads with the failure mark, in its ink;
    /// a plain notice draws none. Both tiers.
    #[test]
    fn a_problem_notice_is_marked() {
        let theme = theme();
        let render = |shown: &ReviewView<'_>, ascii: bool| {
            let area = Rect::new(0, 0, 120, 40);
            let mut buf = Buffer::empty(area);
            (&FinishScreen {
                view: shown,
                theme: &theme,
                ascii,
            })
                .render(area, &mut buf);
            buf
        };
        let text = |buf: &Buffer| {
            (0..40)
                .map(|y| (0..120).map(|x| buf[(x, y)].symbol()).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        };
        for ascii in [false, true] {
            let mut problem = view();
            problem.notice = Some((sanitize_line("could not write"), true));
            let buf = render(&problem, ascii);
            crate::failure::assert_marked(&text(&buf), "could not write", ascii);
            if !ascii {
                let mark = buf
                    .content()
                    .iter()
                    .find(|cell| cell.symbol() == "⊗")
                    .expect("the mark is drawn");
                assert_eq!(
                    mark.style().fg,
                    crate::paint::foreground(&theme, crate::failure::FAILED_INK).fg
                );
            }
            let mut plain = view();
            plain.notice = Some((sanitize_line("pushed"), false));
            let shown = text(&render(&plain, ascii));
            assert!(shown.contains("pushed") && !shown.contains("⊗") && !shown.contains("(x)"));
        }
    }
}

mod history_failed {
    use crate::history::tests::*;
    use crate::history::*;

    /// Part B (ease-pass 26): a failed trace and a failed file read each draw the mark.
    #[test]
    fn failed_trace_and_failed_file_draw_the_failure_mark() {
        let theme = theme();
        for ascii in [false, true] {
            let base = view(&[], &[], &theme, HistoryFocus::List);
            let trace = HistoryView {
                list: HistoryList::Failed("shallow clone"),
                ..base.clone()
            };
            let text = crate::failure::frame_text(120, 8, |frame| {
                draw_history_in_tier(
                    frame,
                    frame.area(),
                    &trace,
                    crate::keymap::Keymap::shipped(),
                    ascii,
                );
            });
            crate::failure::assert_marked(&text, "couldn't trace: shallow clone", ascii);
            let file = HistoryView {
                list: HistoryList::Rows {
                    rows: &[],
                    selected: None,
                    top: 0,
                    end: None,
                },
                file: FilePane::Failed("bad object"),
                focus: HistoryFocus::File,
                ..base
            };
            let text = crate::failure::frame_text(120, 8, |frame| {
                draw_history_in_tier(
                    frame,
                    frame.area(),
                    &file,
                    crate::keymap::Keymap::shipped(),
                    ascii,
                );
            });
            crate::failure::assert_marked(&text, "bad object", ascii);
        }
    }
}

mod journal_screen_failed {
    use super::*;
    use crate::journal_screen::tests::*;
    use crate::journal_screen::*;

    /// Part B (ease-pass 26): a failed read and an expanded row's failure line draw the mark.
    #[test]
    fn a_failed_read_and_a_row_failure_draw_the_failure_mark() {
        for ascii in [false, true] {
            let mut failed = view(Vec::new(), JournalStatus::Failed(sanitize("bad reflog")));
            failed.ascii = ascii;
            let text = draw(&failed, 100, 12).join("\n");
            crate::failure::assert_marked(
                &text,
                "the journal could not be read: bad reflog",
                ascii,
            );
            let mut expanded = row("1m", "git commit", RowMark::None);
            expanded.expanded = true;
            expanded.failure = Some(sanitize("hook refused"));
            let mut shown = view(vec![expanded], JournalStatus::Ready);
            shown.ascii = ascii;
            let text = draw(&shown, 100, 12).join("\n");
            crate::failure::assert_marked(&text, "hook refused", ascii);
        }
    }
}

mod rewrite_editor_failed {
    use super::*;
    use crate::rewrite_editor::tests::*;
    use crate::rewrite_editor::*;

    /// Part B (ease-pass 26): a failed AFTER and a failed CHANGES draw the
    /// mark; a refused AFTER does not.
    #[test]
    fn failed_after_and_failed_changes_draw_the_failure_mark() {
        for ascii in [false, true] {
            let after = RewriteEditorView {
                right: RightSide::After(AfterView::Failed(sanitize_line("no plan"))),
                ascii,
                ..view()
            };
            crate::failure::assert_marked(&screen(&draw(&after, 187, 50)), "no plan", ascii);
            let changes = RewriteEditorView {
                right: RightSide::Changes {
                    short: "5d4c3b2".to_owned(),
                    files: ChangesView::Failed(sanitize_line("diff-tree failed")),
                },
                ascii,
                ..view()
            };
            crate::failure::assert_marked(
                &screen(&draw(&changes, 187, 50)),
                "diff-tree failed",
                ascii,
            );
        }
        let refused = RewriteEditorView {
            right: RightSide::After(AfterView::Refused(sanitize_line("every commit dropped"))),
            ..view()
        };
        let shown = screen(&draw(&refused, 187, 50));
        assert!(shown.contains("every commit dropped") && !shown.contains('\u{2297}'));
    }
}

mod stash_screen_failed {
    use super::*;
    use crate::stash_screen::tests::*;
    use crate::stash_screen::*;

    /// Part B (ease-pass 26): a failed list and a failed notice draw the mark;
    /// a sticky notice (not a failure) does not.
    #[test]
    fn a_failed_list_and_a_failed_notice_draw_the_failure_mark() {
        for ascii in [false, true] {
            let mut shown = view(Vec::new(), StashStatus::Failed(sanitize("git said no")));
            shown.ascii = ascii;
            let text = grid(&shown, 100, 10).join("\n");
            crate::failure::assert_marked(
                &text,
                "the stash list could not be read: git said no",
                ascii,
            );
            let mut noticed = view(Vec::new(), StashStatus::Empty);
            noticed.ascii = ascii;
            noticed.notice = Some(sanitize("apply failed"));
            noticed.notice_is_failure = true;
            noticed.notice_failed = true;
            let text = grid(&noticed, 100, 10).join("\n");
            crate::failure::assert_marked(&text, "apply failed", ascii);
            noticed.notice_failed = false;
            let text = grid(&noticed, 100, 10).join("\n");
            assert!(
                text.contains("apply failed") && !text.contains("\u{2297}"),
                "{text}"
            );
        }
    }
}
