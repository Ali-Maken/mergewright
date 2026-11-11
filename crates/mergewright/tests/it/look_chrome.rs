//! m8-polish-and-ship/06 (ADR-0058 §4, DESIGN.md §1.0): the graph screen's
//! chrome like the mock-up — the header names the repository, its `~` path
//! and the branch; list rows keep a two-cell gutter and a right margin; from
//! 30 rows a blank row sits under the header and a rule over the bar.
//!
//! Two things are checked against the real app (real repository, real
//! update loop): the geometry the mouse uses agrees with what is drawn at
//! every designed size, and the frames are captured for the user's eye.
//! Colour is **unverified** (CLAUDE.md rule 5): the captures are grids.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright_core::event::MouseButton;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, radar_has_answered, settled_frame};

mod fill;

/// The sizes the ticket names: the everyday editor terminal, the mock-up's
/// own, and a wide one.
const SIZES: [(u16, u16); 3] = [(80, 24), (120, 36), (187, 50)];

/// Long after the fixture's commits, so their ages read the same every run.
pub(crate) const NOW: i64 = 1_800_000_000;

/// A feature-branch history, opened and settled at `columns`x`rows`.
pub(crate) fn opened(columns: u16, rows: u16) -> (mergewright_fixtures::Scratch, App, AppState) {
    opened_in(
        mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::FeatureBranches),
        columns,
        rows,
    )
}

/// The feature-branch corpus plus `feature/two-next`: one commit on
/// `feature/two`'s tip that no merge brought in, dated a second after it.
///
/// `feature/two` is merged into `main`, so its rows fold by default (ease-pass
/// 06). The cursor-row captures are about a row with a branch badge and three
/// lanes, not about a merged branch, so they keep `feature/two  two: finish`
/// on screen this way (06 D3, route 2: a new ref, the corpus refs untouched).
pub(crate) fn with_feature_two_kept() -> mergewright_fixtures::Scratch {
    let scratch =
        mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::FeatureBranches);
    let path = scratch.path();
    let tip_date: i64 = git(path, &["log", "-1", "--format=%ct", "feature/two"])
        .trim()
        .parse()
        .expect("a commit time");
    let date = format!("@{} +0000", tip_date + 1);
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args([
            "-c",
            "user.name=Test Maker",
            "-c",
            "user.email=test@example.invalid",
            "commit-tree",
            "feature/two^{tree}",
            "-p",
            "feature/two",
            "-m",
            "two: follow-up",
        ])
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .unwrap_or_else(|why| panic!("git commit-tree: {why}"));
    assert!(
        out.status.success(),
        "git commit-tree: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tip = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    git(path, &["branch", "feature/two-next", &tip]);
    scratch
}

/// `scratch`, opened and settled at `columns`x`rows`.
pub(crate) fn opened_in(
    scratch: mergewright_fixtures::Scratch,
    columns: u16,
    rows: u16,
) -> (mergewright_fixtures::Scratch, App, AppState) {
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, columns, rows);
    let state = settled_frame(&app, state);
    // Rows, refs (the rows' branch badges) and the radar (its marks) each
    // land on their own job; a frame taken before all three is a frame that
    // differs from the next run's.
    let state = app.pump(state, "the first rows, the refs and the radar", |state| {
        state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
            && state.refs.is_some()
            && radar_has_answered(state)
    });
    let state = settled_frame(&app, state);
    // `settled_frame` reads a 120-column frame, whose header may have no
    // room for the walk's `sorting…`; read this size's own.
    let state = app.pump(state, "nothing loading at this size", |state| {
        frame_at(state, columns, rows, Some(Clock::Fixed(NOW)))
            .iter()
            .all(|row| !row.contains("reading\u{2026}") && !row.contains("sorting\u{2026}"))
    });
    (scratch, app, state)
}

/// The selected row's subject.
fn selected_subject(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .map(|row| {
            String::from_utf8_lossy(&row.commit.message)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
}

/// Clicks every commit row drawn in `state`'s frame and checks the click
/// selects the commit whose subject that row draws; how many it checked.
fn clicks_select_what_is_drawn(app: &App, state: &AppState, columns: u16, rows: u16) -> usize {
    let drawn = frame_at(state, columns, rows, Some(Clock::Fixed(NOW)));
    let mut checked = 0;
    for (y, whole) in drawn.iter().enumerate().skip(1) {
        let line = &crate::row_reads::graph_side(whole, columns);
        if !crate::row_reads::is_commit_row(line) {
            continue;
        }
        let row = u16::try_from(y).expect("a screen is shorter than u16::MAX");
        // Inside the subject column, clear of the gutter and the lanes.
        let clicked = app.mouse_down(state, MouseButton::Left, 30, row);
        let selected = selected_subject(&clicked);
        // At 80 columns a subject may be cut to `Merge branch 'f…`: the
        // row reads the selected subject or a cut of it, and the redrawn
        // frame puts the cursor on the clicked row.
        let reads = |subject: &str| {
            line.contains(subject)
                || (1..subject.len()).any(|cut| {
                    subject.is_char_boundary(cut)
                        && line.contains(&format!("{}\u{2026}", &subject[..cut]))
                })
        };
        // A row under the old cursor's second line (07b) is redrawn a line up.
        let redrawn = frame_at(&clicked, columns, rows, Some(Clock::Fixed(NOW)));
        let cursor = [y, y - 1]
            .into_iter()
            .any(|at| redrawn[at].starts_with('\u{25b8}'));
        assert!(
            selected.as_deref().is_some_and(reads) && cursor,
            "{columns}x{rows}: a click on row {row} ({line:?}) selected {selected:?}\n{}",
            drawn.join("\n")
        );
        checked += 1;
    }
    checked
}

/// Acceptance (geometry sweep): at 24×80, 36×120 and 50×187 a click on
/// every drawn graph row selects the commit drawn on that row — the mouse's
/// row math (`update::geometry::rows_above_graph`, `graph_pane_rows`) and
/// the widget's carve (`ScreenRows`, gap and rule included) agree.
///
/// Swept twice (ease-pass 06e): as the fixture opens, both merges folded
/// (six rows), and with `feature/one`'s merge opened by `space` (its two
/// commits drawn between rows the fold had made neighbours) — click
/// mapping across hidden rows is the risk the fold brings.
#[test]
fn every_drawn_row_is_the_row_a_click_selects_at_every_designed_size() {
    for (columns, rows) in SIZES {
        let (_scratch, app, state) = opened(columns, rows);
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
        let checked = clicks_select_what_is_drawn(&app, &state, columns, rows);
        assert_eq!(
            checked,
            6,
            "{columns}x{rows}: folded, six commit rows drawn\n{}",
            drawn.join("\n")
        );
        let merge = drawn
            .iter()
            .position(|line| {
                line.contains("Merge branch 'feature/one'") || line.contains("from feature/one")
            })
            .expect("feature/one's merge is drawn");
        let merge = u16::try_from(merge).expect("a screen is shorter than u16::MAX");
        let on_merge = app.mouse_down(&state, MouseButton::Left, 30, merge);
        let opened = app.key(&on_merge, mergewright_core::event::KeyCode::Char(' '));
        let opened = app.pump(opened, "feature/one's commits drawn", |state| {
            let frame = frame_at(state, columns, rows, Some(Clock::Fixed(NOW)));
            ["one: start", "one: finish"]
                .iter()
                .all(|subject| frame.iter().any(|line| line.contains(subject)))
        });
        let opened = settled_frame(&app, opened);
        let checked = clicks_select_what_is_drawn(&app, &opened, columns, rows);
        let opened_frame = frame_at(&opened, columns, rows, Some(Clock::Fixed(NOW)));
        assert_eq!(
            checked,
            8,
            "{columns}x{rows}: one merge opened, eight commit rows drawn\n{}",
            opened_frame.join("\n")
        );
        drop(opened);
        // From 30 rows the second row is blank and the row over the bar is
        // the rule; below, neither is drawn.
        let spaced = mergewright_ui::graph_screen::spaced(rows);
        assert_eq!(drawn[1].trim().is_empty(), spaced, "{}", drawn.join("\n"));
        let over_the_bar = &drawn[drawn.len() - 2];
        assert_eq!(
            over_the_bar.trim_start().starts_with('\u{2500}'),
            spaced,
            "{columns}x{rows}: {over_the_bar:?}"
        );
        drop(state);
        app.close();
    }
}

/// The frame's buffer, cells and colours, drawn with `ui.theme = theme` —
/// [`frame_at`] with the theme chosen and the attributes kept.
fn buffer_with(
    state: &AppState,
    theme: mergewright::config::ThemeChoice,
    columns: u16,
    rows: u16,
) -> ratatui::buffer::Buffer {
    let mut loaded = mergewright::config::Loaded::default();
    loaded.config.ui.theme = theme;
    let session = mergewright::session::Session::settle(&loaded, crate::captures::capabilities());
    let mut state = state.clone();
    state.size = mergewright::update::Size { columns, rows };
    state.harness.clock = Clock::Fixed(NOW);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(columns, rows))
        .expect("test backend opens");
    terminal
        .draw(|frame| mergewright::view::view(&session, &state, frame))
        .expect("drawing into a test backend cannot fail");
    terminal.backend().buffer().clone()
}

/// m8-polish-and-ship/07, J23 (0 steps) and the ticket's two captures: the
/// cursor on `feature/two  two: finish` — a branch badge and three lanes.
/// The cursor row carries `▸` in the gutter, its lanes keep the ground, the
/// fill runs from the marker slot to the margin, and every character on the
/// fill clears 4.5:1 against it (measured from the real app's buffer, the
/// worst ratio written into the caption). Colour unverified (rule 5): the
/// grid is the same in both themes; the ratios are the tests' claim.
#[test]
fn capture_the_cursor_row_in_dark_and_light() {
    use mergewright_ui::theme::{Theme, ThemeKind, Token, TokenColor};

    let (columns, rows) = (120, 36);
    let (_scratch, app, state) = opened_in(with_feature_two_kept(), columns, rows);
    let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
    let y = drawn
        .iter()
        .position(|line| line.contains("two: finish"))
        .expect("the row is drawn");
    let row = u16::try_from(y).expect("short screen");
    let state = app.mouse_down(&state, MouseButton::Left, 30, row);
    assert_eq!(selected_subject(&state).as_deref(), Some("two: finish"));
    let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
    // Looked for again: the old cursor's second line went with it (07b).
    let line = drawn
        .iter()
        .find(|l| l.contains("two: finish"))
        .expect("the row is drawn");
    assert!(
        // The lanes by role (ease-pass 07): main the trunk `║`, feature/three
        // another branch `┊`, then the cursor row's own node.
        line.starts_with("\u{25b8} \u{2551} \u{250a} \u{25cf}"),
        "{line:?}"
    );
    assert!(!line.contains('\u{258f}'), "no ▏ for the cursor: {line:?}");
    // J23: the branch is read off the `▸` row itself, no key.
    assert!(line.contains("feature/two"), "its branch: {line:?}");
    crate::m8_walk::within_the_note::<&str>("cursor-row-branch", &[]);

    for (name, choice, kind) in [
        (
            "dark",
            mergewright::config::ThemeChoice::Dark,
            ThemeKind::Dark,
        ),
        (
            "light",
            mergewright::config::ThemeChoice::Light,
            ThemeKind::Light,
        ),
    ] {
        let buffer = buffer_with(&state, choice, columns, rows);
        let theme = Theme::new(kind, &crate::captures::capabilities());
        let TokenColor::Rgb(selection) = theme.token(Token::Selection) else {
            panic!("the capture terminal is truecolor");
        };
        let worst = fill::worst_on_the_fill(&buffer, row, selection, name);
        assert!(worst >= 4.5, "{name}: worst text on the fill {worst:.2}:1");
        let caption = format!(
            "m8-polish-and-ship/07, {name} theme, 120x36, cursor on `feature/two  two: finish`: \
             what right looks like — `▸` in the gutter's first cell; the three lanes `│ │ ●` \
             on the ground like every other row; no `▏` in the marker slot; the fill runs from \
             the marker slot to one cell short of the edge; the badge reads as text in its lane's \
             hue family. Compare beside docs/shots/m8-proto-4a-dark-a-b.txt (gutter `▸`, lanes, \
             columns line up). Measured from the app's buffer: worst character on the fill \
             {worst:.2}:1 (floor 4.5). Colour unverified (rule 5): this grid is the same in \
             both themes; legibility is your eye.\n\n"
        );
        check_shot(
            &format!("m8-look-cursor-row-{name}-{columns}x{rows}.txt"),
            &format!("{caption}{}\n", drawn.join("\n")),
        );
    }
    drop(state);
    app.close();
}

/// J22 (0 steps) and the ticket's captures: the first row names the
/// repository, its `~` path and the branch, and never the product.
#[test]
fn capture_the_chrome_at_every_designed_size() {
    for (columns, rows) in SIZES {
        let (_scratch, app, state) = opened(columns, rows);
        let drawn = frame_at(&state, columns, rows, Some(Clock::Fixed(NOW)));
        let header = &drawn[0];
        assert!(
            header.starts_with(" \u{25c6} api  "),
            "the repository's name leads: {header:?}"
        );
        if columns >= 120 {
            assert!(
                header.contains("~/code/api \u{b7} main"),
                "{columns}x{rows}: the `~` path and the branch: {header:?}"
            );
        }
        assert!(
            !header.contains(mergewright_core::on_disk::TITLE),
            "no product name: {header:?}"
        );
        // J22: read off the header, no key.
        crate::m8_walk::within_the_note::<&str>("which-repo-branch", &[]);
        let caption = format!(
            "m8-polish-and-ship/06 at {columns}x{rows}: what right looks like — the first row \
             is ` ◆ <repo>  ~/<path> · <branch>` with the walk state right-aligned; list rows \
             start two cells in and stop one short of the edge; the bar joins its hints with \
             ` · ` and ends `? keys`; from 30 rows a blank row sits under the header and a `─` \
             rule over the bar. Colour unverified (rule 5).\n\n"
        );
        check_shot(
            &format!("m8-look-chrome-{columns}x{rows}.txt"),
            &format!("{caption}{}\n", drawn.join("\n")),
        );
        drop(state);
        app.close();
    }
}
