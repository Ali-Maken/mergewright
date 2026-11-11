//! What the commit screen's character grid says (ease-pass/08): the
//! placeholders, the counter, the summary, the two file groups, the three
//! bars and the secrets note — each in the mock's words.

use mergewright_term::sanitize_line;

use super::fixture::{buffer, draw, grid, last_line, theme, view};
use super::*;
use crate::theme::Token;

#[test]
fn an_empty_screen_shows_both_placeholders_and_the_header() {
    let drawn = draw(&view("", CommitFocus::Subject), 120, 30);
    assert!(drawn.contains("What did you change? (one line)"), "{drawn}");
    assert!(
        drawn.contains("More detail, if it needs it (optional)"),
        "{drawn}"
    );
    let first = drawn.lines().next().unwrap_or_default();
    assert_eq!(first, " committing to wallet-release · no upstream set");
    // Nothing typed: no counter yet.
    assert!(!drawn.contains("0/72"), "{drawn}");
}

#[test]
fn the_counter_reads_typed_over_72_and_warns_past_it() {
    let drawn = draw(&view("fix toc!", CommitFocus::Subject), 120, 30);
    assert!(drawn.contains("8/72"), "{drawn}");
    assert!(!drawn.contains("What did you change?"), "{drawn}");

    let long = "x".repeat(73);
    let buffer = buffer(&view(&long, CommitFocus::Subject), 120, 30);
    let drawn = grid(&buffer);
    let row = drawn
        .lines()
        .position(|line| line.contains("73/72"))
        .expect("the counter is drawn");
    let line: Vec<&str> = drawn.lines().collect();
    let column = line[row].chars().take_while(|c| *c != '7').count();
    let cell = &buffer[(
        u16::try_from(column).unwrap_or(0),
        u16::try_from(row).unwrap_or(0),
    )];
    assert_eq!(
        cell.style().fg,
        Some(crate::paint::color(theme().token(Token::Warning))),
        "past 72 the counter is in the warning colour"
    );
}

#[test]
fn the_summary_says_what_goes_in_in_the_mocks_words() {
    let drawn = draw(&view("fix toc", CommitFocus::Subject), 120, 30);
    assert!(
        drawn.contains(" 3 files go in, +60 −3, as a new commit on wallet-release. 1 stays out."),
        "{drawn}"
    );
    let nothing = CommitScreenView {
        summary: Summary::Nothing,
        ..view("fix toc", CommitFocus::Subject)
    };
    let drawn = draw(&nothing, 120, 30);
    assert!(
        drawn.contains("Nothing goes in. Put a file in with space."),
        "{drawn}"
    );
}

#[test]
fn summary_words_count_one_file_and_many() {
    let one = Summary::Goes {
        files: 1,
        lines: None,
        onto: Onto::NewCommit(sanitize_line("main")),
        out: 2,
    };
    assert_eq!(
        summary_line(&one),
        "1 file goes in, as a new commit on main. 2 stay out."
    );
    let amend = Summary::Goes {
        files: 2,
        lines: Some((3, 1)),
        onto: Onto::Amend,
        out: 0,
    };
    assert_eq!(
        summary_line(&amend),
        "2 files go in, +3 −1, into the last commit."
    );
    assert_eq!(
        summary_line(&Summary::MessageOnly { out: 1 }),
        "Only the message changes. 1 stays out."
    );
}

#[test]
fn files_go_under_their_two_headings_with_their_marks() {
    let drawn = draw(&view("fix toc", CommitFocus::Files), 120, 30);
    assert!(drawn.contains(" Going in (3)"), "{drawn}");
    assert!(drawn.contains(" Staying out (1)"), "{drawn}");
    assert!(drawn.contains(" ▸✓ M src/receipt.rs"), "{drawn}");
    assert!(drawn.contains("  ✓ A src/refund.rs"), "{drawn}");
    assert!(
        drawn.contains("  ! A .env.local  looks like a secrets file"),
        "{drawn}"
    );
    assert!(drawn.contains("+12 −3"), "{drawn}");
}

#[test]
fn an_empty_group_says_so() {
    let mut out = view("fix toc", CommitFocus::Subject);
    for file in &mut out.files {
        file.group = FileGroup::StayingOut;
    }
    let drawn = draw(&out, 120, 30);
    assert!(drawn.contains(" Going in (0)"), "{drawn}");
    assert!(drawn.contains("\n   nothing yet "), "{drawn}");
    let drawn = draw(
        &view("fix toc", CommitFocus::Subject).clone_with_all_in(),
        120,
        30,
    );
    assert!(drawn.contains(" Staying out (0)"), "{drawn}");
    assert!(drawn.contains("\n   nothing "), "{drawn}");
}

impl CommitScreenView<'_> {
    fn clone_with_all_in(mut self) -> Self {
        for file in &mut self.files {
            file.group = FileGroup::GoingIn;
        }
        self
    }
}

#[test]
fn the_file_cursor_is_marked_away_from_the_files() {
    let drawn = draw(&view("fix toc", CommitFocus::Subject), 120, 30);
    assert!(drawn.contains(" ›✓ M src/receipt.rs"), "{drawn}");
    assert!(
        drawn.contains("│ ▸fix toc"),
        "the subject has the focus mark: {drawn}"
    );
}

#[test]
fn the_secrets_files_diff_starts_with_why_it_is_left_out() {
    let mut on_secret = view("fix toc", CommitFocus::Files);
    on_secret.cursor = 3;
    let drawn = draw(&on_secret, 120, 30);
    assert!(
        drawn.contains("! left out: looks like a secrets file. space puts it in."),
        "{drawn}"
    );
}

#[test]
fn the_subject_bar_is_the_mocks() {
    let drawn = draw(&view("fix toc", CommitFocus::Subject), 120, 30);
    assert_eq!(
        last_line(&drawn),
        " ⏎ commit 3 files  ⇥ more detail  ↑/↓ see a file's changes  esc cancel, your message is kept"
    );
    let drawn = draw(&view("", CommitFocus::Subject), 120, 30);
    assert!(
        last_line(&drawn).starts_with(" ⏎ type a message first  ⇥ more detail"),
        "{drawn}"
    );
    let no_file = CommitScreenView {
        refusal: Some(Refusal::NoFile),
        ..view("fix toc", CommitFocus::Subject)
    };
    let drawn = draw(&no_file, 120, 30);
    assert!(
        last_line(&drawn).starts_with(" ⏎ put a file in first"),
        "{drawn}"
    );
}

#[test]
fn the_body_bar_is_the_mocks() {
    let drawn = draw(&view("fix toc", CommitFocus::Body), 120, 30);
    assert_eq!(last_line(&drawn), " esc back to the message line  ⇥ files");
}

#[test]
fn the_files_bar_is_the_mocks() {
    let drawn = draw(&view("fix toc", CommitFocus::Files), 120, 30);
    assert_eq!(
        last_line(&drawn),
        " space leave it out  j/k move  ⏎ commit  esc back to the message"
    );
    let mut on_secret = view("fix toc", CommitFocus::Files);
    on_secret.cursor = 3;
    let drawn = draw(&on_secret, 120, 30);
    assert!(last_line(&drawn).starts_with(" space put it in"), "{drawn}");
}

#[test]
fn a_failure_takes_the_bar_row() {
    let failed = CommitScreenView {
        status: Some(crate::graph_screen::ProblemLine {
            kind: crate::graph_screen::ProblemKind::Failure,
            reason: sanitize_line("pre-commit hook said no"),
            keys: None,
        }),
        ..view("fix toc", CommitFocus::Subject)
    };
    let drawn = draw(&failed, 120, 30);
    assert_eq!(
        last_line(&drawn),
        " ⊗ pre-commit hook said no · esc dismisses"
    );
    assert!(drawn.contains("fix toc"), "the message is kept: {drawn}");
}

#[test]
fn the_header_names_the_options_and_the_amend() {
    let mut header = view("fix toc", CommitFocus::Subject);
    header.header.options = vec!["signing off", "skipping hooks"];
    let drawn = draw(&header, 120, 30);
    assert_eq!(
        drawn.lines().next().unwrap_or_default(),
        " committing to wallet-release · no upstream set · signing off · skipping hooks"
    );
    header.header.amending = Some(sanitize_line("a1b2c3d Fix the toc"));
    let drawn = draw(&header, 120, 30);
    assert!(
        drawn.starts_with(" amending a1b2c3d Fix the toc · no upstream set"),
        "{drawn}"
    );
}

/// 08s m12: the header starts with the words the graph's top line starts
/// with — `◆ <repo>  <~/path>` — then where the commit goes; short of room
/// the path is cut first, from the left, as the top line cuts it.
#[test]
fn the_header_names_the_repository_first() {
    let mut screen = view("fix toc", CommitFocus::Subject);
    screen.header.location = Some(crate::graph_screen::Location {
        repo: sanitize_line("wallet"),
        path: sanitize_line("~/work/clients/acme/wallet"),
        branch: sanitize_line("wallet-release"),
        practice: false,
    });
    let header_at = |columns| {
        draw(&screen, columns, 30)
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(
        header_at(120),
        " ◆ wallet  ~/work/clients/acme/wallet  committing to wallet-release · no upstream set"
    );
    assert_eq!(
        header_at(80),
        " ◆ wallet  …clients/acme/wallet  committing to wallet-release · no upstream set"
    );
    // No room for any of the path: the name, then the commit's words.
    let narrow = header_at(50);
    assert!(
        narrow.starts_with(" ◆ wallet  committing to wallet-release"),
        "{narrow}"
    );
    assert!(!narrow.contains('/'), "{narrow}");
}

#[test]
fn under_100_columns_the_diff_is_hidden() {
    let mut on_secret = view("fix toc", CommitFocus::Files);
    on_secret.cursor = 3;
    let drawn = draw(&on_secret, 90, 30);
    assert!(!drawn.contains("left out:"), "{drawn}");
    assert!(!drawn.contains('┬'), "{drawn}");
    assert!(drawn.contains(" Going in (3)"), "{drawn}");
    let wide = draw(&on_secret, 120, 30);
    assert!(wide.contains('┬'), "{wide}");
}

#[test]
fn no_option_row_is_drawn() {
    let drawn = draw(&view("fix toc", CommitFocus::Subject), 120, 30);
    for gone in ["[ ]", "[x]", "[ Commit ]", "sign-off", "no-verify"] {
        assert!(!drawn.contains(gone), "{gone}: {drawn}");
    }
}

#[test]
fn tab_walks_subject_body_files_and_back() {
    assert_eq!(CommitFocus::Subject.next(), CommitFocus::Body);
    assert_eq!(CommitFocus::Body.next(), CommitFocus::Files);
    assert_eq!(CommitFocus::Files.next(), CommitFocus::Subject);
    assert_eq!(CommitFocus::Subject.previous(), CommitFocus::Files);
    assert_eq!(CommitFocus::Files.previous(), CommitFocus::Body);
    assert_eq!(CommitFocus::Body.previous(), CommitFocus::Subject);
}

#[test]
fn a_tiny_area_draws_without_panicking() {
    let mut located = view("fix toc", CommitFocus::Files);
    located.header.location = Some(crate::graph_screen::Location {
        repo: sanitize_line("wallet"),
        path: sanitize_line("~/work/wallet"),
        branch: sanitize_line("main"),
        practice: false,
    });
    for (columns, rows) in [(1, 1), (10, 2), (40, 5), (120, 8), (120, 9)] {
        let _drawn: String = draw(&view("fix toc", CommitFocus::Files), columns, rows);
        let _drawn: String = draw(&located, columns, rows);
    }
}

/// A bar too wide for its row drops whole parts from its end: no key's
/// words are cut mid-word (the 90-column capture).
#[test]
fn a_bar_too_wide_keeps_whole_parts() {
    let line = "⏎ commit 3 files  ⇥ more detail  esc cancel, your message is kept";
    assert_eq!(
        words::whole_parts(line, 40),
        "⏎ commit 3 files  ⇥ more detail"
    );
    assert_eq!(words::whole_parts(line, 200), line);
    assert_eq!(words::whole_parts(line, 5), "");
}

/// 08s m4: with no diff on screen the subject bar leaves out `↑/↓ see a
/// file's changes`; short of room its parts go by rank, lowest first —
/// `↑/↓`, `⇥ more detail`, `esc cancel…`, then `⏎` — whole parts only.
#[test]
fn the_narrow_bar_drops_by_rank() {
    let subject = view("fix toc", CommitFocus::Subject);
    let bar_at = |columns| last_line(&draw(&subject, columns, 30)).trim().to_owned();
    assert_eq!(
        bar_at(120),
        "⏎ commit 3 files  ⇥ more detail  ↑/↓ see a file's changes  esc cancel, your message is kept"
    );
    assert_eq!(
        bar_at(90),
        "⏎ commit 3 files  ⇥ more detail  esc cancel, your message is kept"
    );
    // The two are 50 cells, after the bar's leading space.
    assert_eq!(
        bar_at(51),
        "⏎ commit 3 files  esc cancel, your message is kept"
    );
    assert_eq!(bar_at(50), "⏎ commit 3 files");
    assert_eq!(bar_at(10), "");
}

/// Part B (ease-pass 26): a failed diff body draws the mark, both tiers.
#[test]
fn a_failed_diff_body_draws_the_failure_mark() {
    for ascii in [false, true] {
        let mut shown = view("fix", CommitFocus::Subject);
        shown.diff = BodyView::Failed(sanitize_line("git diff: bad object"));
        let theme = theme();
        let text = crate::failure::frame_text(160, 40, |frame| {
            frame.render_widget(
                &CommitScreen {
                    view: &shown,
                    theme: &theme,
                    keys: crate::keymap::Keymap::shipped(),
                    ascii,
                },
                frame.area(),
            );
        });
        crate::failure::assert_marked(&text, "git diff: bad object", ascii);
    }
}
