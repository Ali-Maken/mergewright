use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

use super::*;

const NOW: i64 = 2_000_000_000;
/// 210 days: `7mo`, `7 months ago`.
const LINA_AGE: i64 = 18_144_000;
/// 300 days: `10mo`.
const TOM_AGE: i64 = 25_920_000;

const LONG_LINE: &str = "            self.store.persist(&self.token, Duration::from_secs(self.config.session_ttl_seconds), Persist::Atomically, &self.audit_log)?;";

fn tom() -> Who {
    Who {
        sha7: "5e6f7a8".into(),
        name: "Tom".into(),
        author_time: NOW - TOM_AGE,
        subject: "Add token refresh".into(),
    }
}

fn lina() -> Who {
    Who {
        sha7: "a1b2c3d".into(),
        name: "Lina".into(),
        author_time: NOW - LINA_AGE,
        subject: "Retry refresh once on 401".into(),
    }
}

fn row(line_no: u32, text: &str, who: Who, block: BlockPos) -> BlameRowView {
    BlameRowView {
        line_no,
        text: text.into(),
        who: Some(who),
        block,
        mark: RowMark::Plain,
    }
}

/// Pass 1b's mock-up, lines 18–27 (line 19 is Lina's, per 2b F3).
fn mockup() -> Vec<BlameRowView> {
    use BlockPos::*;
    vec![
        row(
            18,
            "    pub fn refresh_token(&mut self, client: &Client) -> Result<()> {",
            tom(),
            Single,
        ),
        row(19, "        let mut attempt = 0;", lina(), Single),
        row(20, "        loop {", tom(), First),
        row(
            21,
            "            let resp = client.refresh(&self.token)?;",
            tom(),
            Last,
        ),
        row(
            22,
            "            if resp.status() == 401 && attempt == 0 {",
            lina(),
            Single,
        ),
        row(23, "                attempt += 1;", lina(), First),
        row(24, "                continue;", lina(), Last),
        row(25, "            }", tom(), First),
        row(26, "            self.token = resp.token()?;", tom(), Middle),
        row(27, LONG_LINE, tom(), Last),
    ]
}

/// Index of line 22 in [`mockup`].
const ON_22: usize = 4;
/// Index of line 27 in [`mockup`].
const ON_27: usize = 9;

pub(crate) fn view(rows: &[BlameRowView], cursor: usize) -> BlameView<'_> {
    BlameView {
        path: "src/auth/session.rs",
        rev: BlameRev::Now,
        body: BlameBody::Lines {
            rows,
            cursor,
            top: 0,
        },
        ignore: None,
        find: None,
        heatmap: false,
        now: NOW,
        // Blame's default (mergewright-git `blame.rs`): the title says nothing.
        whitespace_ignored: true,
        default_ignored: true,
    }
}

fn draw(view: &BlameView, width: u16, height: u16) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("the test backend starts");
    terminal
        .draw(|frame| draw_blame(frame, frame.area(), view, crate::keymap::Keymap::shipped()))
        .expect("the blame place draws");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// The drawn line holding line number `n`.
fn line_of(lines: &[String], n: u32) -> &str {
    let needle = format!("│ {n:>2}  ");
    lines
        .iter()
        .find(|line| line.contains(&needle))
        .unwrap_or_else(|| panic!("no line {n} in {lines:#?}"))
}

// --- Trailer -------------------------------------------------------------

#[test]
fn the_trailer_is_on_the_cursor_row_only() {
    let rows = mockup();
    let lines = draw(&view(&rows, ON_22), 187, 20);
    let trailed: Vec<&String> = lines.iter().filter(|line| line.contains('◂')).collect();
    assert_eq!(trailed.len(), 1, "{lines:#?}");
    let cursor = line_of(&lines, 22);
    assert!(
        cursor.ends_with(
            "attempt == 0 {    ◂ Lina · 7 months ago · Retry refresh once on 401 · a1b2c3d"
        ),
        "{cursor}"
    );
}

#[test]
fn the_trailer_sheds_sha_then_subject() {
    let mut rows = mockup();
    // A longer line, so every width below stays in the ≥180 (46-cell) gutter.
    rows[ON_22]
        .text
        .push_str(" // retry the refresh once when the server says expired");
    let at = |width: u16| line_of(&draw(&view(&rows, ON_22), width, 20), 22).to_owned();
    // The trailer starts after the gutter, `│ 22  `, the code and the gap.
    let full = "◂ Lina · 7 months ago · Retry refresh once on 401 · a1b2c3d";
    let start = 46 + 6 + rows[ON_22].text.len() + 4;
    let exactly = |text: &str| u16::try_from(start + text.chars().count()).expect("fits");
    assert!(at(exactly(full)).ends_with(full));
    let unhashed = "◂ Lina · 7 months ago · Retry refresh once on 401";
    assert!(
        at(exactly(full) - 1).ends_with(unhashed),
        "{}",
        at(exactly(full) - 1)
    );
    let cut = "◂ Lina · 7 months ago · Retry refresh once on…";
    assert!(
        at(exactly(unhashed) - 1).ends_with(cut),
        "{}",
        at(exactly(unhashed) - 1)
    );
    let head = "◂ Lina · 7 months ago";
    // `head · Retry…` needs head + 9; one short drops the subject.
    let first_word = "◂ Lina · 7 months ago · Retry…";
    assert!(at(exactly(first_word)).ends_with(first_word));
    assert!(
        at(exactly(first_word) - 1).ends_with(head),
        "{}",
        at(exactly(first_word) - 1)
    );
    assert!(!at(exactly(head) - 1).contains('◂'));
}

#[test]
fn a_long_line_drops_the_trailer_and_the_gutter_names_him() {
    let mut rows = mockup();
    // Mid-block, so only the cursor rule puts his name in the gutter.
    rows[ON_27].block = BlockPos::Middle;
    let lines = draw(&view(&rows, ON_27), 187, 20);
    let cursor = line_of(&lines, 27);
    assert!(!cursor.contains('◂'), "{cursor}");
    assert!(cursor.starts_with(" ▸         Tom"), "{cursor}");
}

#[test]
fn mid_block_cursor_row_prints_its_name() {
    let rows = mockup();
    let lines = draw(&view(&rows, 8), 187, 20); // line 26, Middle
    assert!(
        line_of(&lines, 26).starts_with(" ▸         Tom "),
        "{lines:#?}"
    );
    // Off the cursor the same row is a bare bracket.
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert!(
        line_of(&lines, 26).starts_with(" │                                            │"),
        "{lines:#?}"
    );
}

#[test]
fn uncommitted_line_trails_not_committed_yet() {
    let mut rows = mockup();
    rows[ON_22].mark = RowMark::NotCommitted;
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert!(line_of(&lines, 22).ends_with("{    ◂ not committed yet"));
}

#[test]
fn skipped_line_trails_skipped_commit() {
    let mut rows = mockup();
    rows[ON_22].mark = RowMark::Skipped;
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert!(line_of(&lines, 22).contains("{    ◂ (skipped commit) Lina · 7 months ago"));
    rows[ON_22].mark = RowMark::Unblamable;
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert!(line_of(&lines, 22).ends_with("{    ◂ git can't say who"));
}

// --- Gutter ----------------------------------------------------------------

/// Where the `│` rule sits on line `n`: the gutter's width.
fn rule_at(lines: &[String], n: u32) -> usize {
    let line = line_of(lines, n);
    let needle = format!("│ {n:>2}  ");
    line[..line.find(&needle).expect("the rule")]
        .chars()
        .count()
}

#[test]
fn gutter_is_46_at_187() {
    let rows = mockup();
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert_eq!(rule_at(&lines, 20), 46);
    assert!(line_of(&lines, 20).starts_with(" ┌ 5e6f7a8 Tom   10mo  Add token refresh"));
}

#[test]
fn gutter_is_22_without_subject_at_140() {
    let rows = mockup();
    let lines = draw(&view(&rows, ON_22), 140, 20);
    assert_eq!(rule_at(&lines, 20), 22);
    assert!(line_of(&lines, 20).starts_with(" ┌ 5e6f7a8 Tom   10mo │"));
}

#[test]
fn gutter_is_10_with_initials_at_100() {
    let rows = mockup();
    let lines = draw(&view(&rows, ON_22), 100, 20);
    assert_eq!(rule_at(&lines, 20), 10);
    assert!(line_of(&lines, 20).starts_with(" ┌ TO 10mo│"), "{lines:#?}");
}

#[test]
fn one_row_block_is_a_dash() {
    let rows = mockup();
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert!(line_of(&lines, 19).starts_with(" ─ a1b2c3d Lina   7mo"));
}

#[test]
fn block_brackets_open_and_close() {
    let rows = mockup();
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert!(line_of(&lines, 25).starts_with(" ┌ 5e6f7a8"));
    assert!(line_of(&lines, 26).starts_with(" │   "));
    assert!(line_of(&lines, 27).starts_with(" └   "));
}

// --- Title and states --------------------------------------------------------

#[test]
fn title_says_now_as_of_before() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    assert_eq!(
        draw(&v, 187, 20)[0],
        " BLAME · src/auth/session.rs · as it is now"
    );
    v.rev = BlameRev::AsOf("a1b2c3d".into());
    assert_eq!(
        draw(&v, 187, 20)[0],
        " BLAME · src/auth/session.rs · as of a1b2c3d"
    );
    v.rev = BlameRev::Before("a1b2c3d".into());
    assert_eq!(
        draw(&v, 187, 20)[0],
        " BLAME · src/auth/session.rs · before a1b2c3d"
    );
}

#[test]
fn reading_title_says_esc_cancels() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    v.body = BlameBody::Reading;
    let lines = draw(&v, 187, 20);
    assert_eq!(lines[0], " blaming src/auth/session.rs… · esc cancels");
    assert!(lines[1..].iter().all(String::is_empty));
}

#[test]
fn failed_body_shows_gits_first_line() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    v.body = BlameBody::Failed("fatal: no such path 'src/x.rs' in HEAD");
    let lines = draw(&v, 187, 20);
    assert_eq!(
        lines[1],
        "⊗ git blame stopped: fatal: no such path 'src/x.rs' in HEAD · esc back"
    );
}

#[test]
fn ignore_line_says_skipping_or_showing() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    v.ignore = Some(IgnoreLine {
        count: 3,
        applied: true,
    });
    let lines = draw(&v, 187, 20);
    assert_eq!(
        lines[1],
        " skipping 3 commits listed in .git-blame-ignore-revs · i shows them"
    );
    assert!(lines[2].contains("│ 18  "));
    v.ignore = Some(IgnoreLine {
        count: 3,
        applied: false,
    });
    assert_eq!(
        draw(&v, 187, 20)[1],
        " showing every commit · i skips the 3 in .git-blame-ignore-revs"
    );
}

#[test]
fn no_ignore_line_without_the_file() {
    let rows = mockup();
    let lines = draw(&view(&rows, ON_22), 187, 20);
    assert!(lines[1].contains("│ 18  "), "{}", lines[1]);
}

#[test]
fn find_title_counts_matches() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    v.find = Some(FindTitle {
        query: "retry",
        at: 2,
        of: 5,
    });
    assert!(draw(&v, 187, 20)[0].ends_with("as it is now · find \"retry\" 2 of 5"));
    v.find = Some(FindTitle {
        query: "retry",
        at: 0,
        of: 0,
    });
    assert!(draw(&v, 187, 20)[0].ends_with("as it is now · find \"retry\" · no match"));
}

#[test]
fn at_24_rows_code_fills_the_rest() {
    // The frame gives the place 23 rows (its bottom row is the bar).
    let rows: Vec<BlameRowView> = (1..=60)
        .map(|n| row(n, "x", tom(), BlockPos::Middle))
        .collect();
    let mut v = view(&rows, 0);
    assert_eq!(
        draw(&v, 187, 23)
            .iter()
            .filter(|l| l.contains("│  "))
            .count(),
        22
    );
    v.ignore = Some(IgnoreLine {
        count: 1,
        applied: true,
    });
    assert_eq!(
        draw(&v, 187, 23)
            .iter()
            .filter(|l| l.contains("│  "))
            .count(),
        21
    );
}

#[test]
fn heatmap_changes_colour_only() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    let off = draw(&v, 187, 20);
    v.heatmap = true;
    assert_eq!(draw(&v, 187, 20), off);
}

// --- Mouse ---------------------------------------------------------------------

#[test]
fn blame_row_at_maps_y_to_row() {
    let rows = mockup();
    let area = Rect::new(0, 5, 187, 8);
    let mut v = view(&rows, ON_22);
    v.body = BlameBody::Lines {
        rows: &rows,
        cursor: ON_22,
        top: 3,
    };
    assert_eq!(blame_row_at(area, &v, 5), None, "the title");
    assert_eq!(blame_row_at(area, &v, 6), Some(3));
    assert_eq!(blame_row_at(area, &v, 12), Some(9));
    v.ignore = Some(IgnoreLine {
        count: 3,
        applied: true,
    });
    assert_eq!(blame_row_at(area, &v, 6), None, "the ignore-revs line");
    assert_eq!(blame_row_at(area, &v, 7), Some(3));
    // Past the last row, and outside the area.
    v.body = BlameBody::Lines {
        rows: &rows,
        cursor: ON_22,
        top: 8,
    };
    assert_eq!(blame_row_at(area, &v, 10), None);
    assert_eq!(blame_row_at(area, &v, 13), None);
    v.body = BlameBody::Reading;
    assert_eq!(blame_row_at(area, &v, 7), None);
}

// --- Snapshots -------------------------------------------------------------------

fn snapshot(v: &BlameView, width: u16, height: u16) -> String {
    draw(v, width, height).join(
        "
",
    )
}

/// A snapshot whose header carries its "Right:" line.
macro_rules! snap {
    ($name:literal, $right:literal, $text:expr) => {
        insta::with_settings!({ description => concat!("Right: ", $right) }, {
            insta::assert_snapshot!($name, $text);
        })
    };
}

#[test]
fn snapshot_187x50() {
    let rows = mockup();
    snap!(
        "blame_187x50",
        "46-cell gutter with subjects; ▸ on 22; the trailer 4 cells after { names Lina, 7 months ago, the subject and a1b2c3d; no other row trails.",
        snapshot(&view(&rows, ON_22), 187, 50)
    );
}

#[test]
fn snapshot_140x40() {
    let rows = mockup();
    snap!(
        "blame_140x40",
        "22-cell gutter: bracket, sha, name, age, no subject; the trailer carries the subject, its sha shed for room.",
        snapshot(&view(&rows, ON_22), 140, 40)
    );
}

#[test]
fn snapshot_100x30() {
    let rows = mockup();
    snap!(
        "blame_100x30",
        "10-cell gutter: bracket, initials, age; the trailer shed to name and age, the room left.",
        snapshot(&view(&rows, ON_22), 100, 30)
    );
}

#[test]
fn snapshot_187x50_long_line() {
    let rows = mockup();
    snap!(
        "blame_187x50_long_line",
        "cursor on the long line 27, its block's last row: no trailer; the gutter prints Tom beside ▸.",
        snapshot(&view(&rows, ON_27), 187, 50)
    );
}

// --- Shallow boundary (20's shallow ruling, Part B) ------------------------------

fn priya() -> Who {
    Who {
        sha7: "0a27627".into(),
        name: "Priya".into(),
        author_time: NOW - TOM_AGE,
        subject: "Import the session module".into(),
    }
}

/// Lines 17–19 from the shallow boundary, then Lina's 20: the cursor on 18,
/// the boundary block's middle row.
fn shallow_rows() -> Vec<BlameRowView> {
    let shallow = |line_no, text: &str, block| BlameRowView {
        line_no,
        text: text.into(),
        who: Some(priya()),
        block,
        mark: RowMark::Shallow,
    };
    vec![
        shallow(17, "    // Refresh the token.", BlockPos::First),
        shallow(
            18,
            "    pub fn refresh_token(&mut self) {",
            BlockPos::Middle,
        ),
        shallow(19, "        let mut attempt = 0;", BlockPos::Last),
        row(20, "        loop {", lina(), BlockPos::Single),
    ]
}

/// Index of line 18 in [`shallow_rows`].
const SHALLOW_18: usize = 1;

#[test]
fn a_shallow_row_never_draws_its_authors_name() {
    let rows = shallow_rows();
    for width in [187, 140, 100] {
        for cursor in [0, SHALLOW_18, 2] {
            let lines = draw(&view(&rows, cursor), width, 10);
            for n in 17..=19 {
                let line = line_of(&lines, n);
                assert!(!line.contains("Priya"), "{width}: {line}");
                assert!(!line.contains("PR "), "{width} initials: {line}");
                assert!(!line.contains("Import the session"), "{width}: {line}");
                assert!(!line.contains("10mo"), "{width} age: {line}");
            }
        }
    }
}

#[test]
fn shallow_gutter_words_at_each_width() {
    let rows = shallow_rows();
    let at_187 = draw(&view(&rows, SHALLOW_18), 187, 10);
    assert!(
        line_of(&at_187, 17).starts_with(" ┌ ^0a2762 shallow clone · can't say who"),
        "{}",
        line_of(&at_187, 17)
    );
    assert!(
        line_of(&at_187, 18).starts_with(" ▸ ^0a2762 shallow clone · can't say who"),
        "{}",
        line_of(&at_187, 18)
    );
    assert!(
        line_of(&at_187, 19).starts_with(" └ "),
        "{}",
        line_of(&at_187, 19)
    );
    assert!(!line_of(&at_187, 19).contains("shallow"));
    let at_140 = draw(&view(&rows, SHALLOW_18), 140, 10);
    assert!(
        line_of(&at_140, 18).starts_with(" ▸ ^0a2762 shallow "),
        "{}",
        line_of(&at_140, 18)
    );
    let gutter_140 = line_of(&at_140, 18).split('│').next().unwrap_or("");
    assert_eq!(gutter_140.trim_end(), " ▸ ^0a2762 shallow");
    let at_100 = draw(&view(&rows, SHALLOW_18), 100, 10);
    assert!(
        line_of(&at_100, 18).starts_with(" ▸ shallow│"),
        "{}",
        line_of(&at_100, 18)
    );
}

#[test]
fn shallow_trailer_sheds_to_cant_say_who() {
    let rows = shallow_rows();
    let lines = draw(&view(&rows, SHALLOW_18), 187, 10);
    assert!(
        line_of(&lines, 18).ends_with("{    ◂ can't say who · this clone starts at 0a27627"),
        "{}",
        line_of(&lines, 18)
    );
    let row = &rows[SHALLOW_18];
    assert_eq!(
        trailer(row, NOW, 30).as_deref(),
        Some("◂ can't say who"),
        "short of room"
    );
    assert_eq!(trailer(row, NOW, 10), None, "no room at all");
}

#[test]
fn snapshot_187x50_shallow() {
    let rows = shallow_rows();
    snap!(
        "blame_187x50_shallow",
        "row 18 reads `▸ ^0a2762 shallow clone · can't say who`, its trailer `◂ can't say who · this clone starts at 0a27627`; 17 opens the block with the same words, 19 closes it bare; Priya nowhere.",
        snapshot(&view(&rows, SHALLOW_18), 187, 50)
    );
}

/// Spell sweep (c), ticket m8/10c: the reading title and the failed line
/// name his `app.back`, never `esc`.
#[test]
fn blame_spells_its_back_key_from_his_keymap() {
    use crate::keymap::sweep::{assert_spelled, swept};
    let (keys, _) = swept();
    let mut reading = view(&[], 0);
    reading.body = BlameBody::Reading;
    assert_spelled(&title(&reading, &keys), &keys, &[&["app.back"]]);
    let mut failed = view(&[], 0);
    failed.body = BlameBody::Failed("no such path");
    let backend = TestBackend::new(80, 6);
    let mut terminal = Terminal::new(backend).expect("the test backend starts");
    terminal
        .draw(|frame| draw_blame(frame, frame.area(), &failed, &keys))
        .expect("the widget draws");
    let text = terminal.backend().to_string();
    let line = text
        .lines()
        .find(|line| line.contains("git blame stopped"))
        .unwrap_or_else(|| panic!("{text}"));
    assert_spelled(line, &keys, &[&["app.back"]]);
}

/// m8-polish-and-ship/17b decision 3: blame's default is whitespace
/// ignored, so the title says nothing then; after `w` (whitespace counted)
/// it ends `· whitespace counted`.
#[test]
fn blame_title_names_whitespace_only_when_counted() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    v.whitespace_ignored = true;
    let title = draw(&v, 187, 20)[0].clone();
    assert!(!title.contains("whitespace"), "{title:?}");
    assert!(title.trim_end().ends_with("as it is now"), "{title:?}");
    v.whitespace_ignored = false;
    let title = draw(&v, 187, 20)[0].clone();
    assert!(
        title
            .trim_end()
            .ends_with("as it is now \u{b7} whitespace counted"),
        "{title:?}"
    );
}

/// 21b-2 item 2: the title's tail follows the default the view is given,
/// not a literal of blame's own (graft fills it from mergewright-git).
#[test]
fn the_title_tail_follows_the_default_it_is_given() {
    let rows = mockup();
    let mut v = view(&rows, ON_22);
    v.default_ignored = false;
    v.whitespace_ignored = true;
    let title = draw(&v, 187, 20)[0].clone();
    assert!(
        title
            .trim_end()
            .ends_with("as it is now \u{b7} whitespace ignored"),
        "{title:?}"
    );
    v.whitespace_ignored = false;
    let title = draw(&v, 187, 20)[0].clone();
    assert!(!title.contains("whitespace"), "{title:?}");
}
