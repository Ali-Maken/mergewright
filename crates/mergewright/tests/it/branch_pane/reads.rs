//! What the pane says once its read lands, through the real app: the
//! heading, the four marked lines, the route, and the branches it leaves
//! out (ease-pass 09 acceptance).

use mergewright::update::AppState;
use mergewright::update::branch_pane::{BranchNames, candidates, identity_of};
use mergewright_core::event::KeyCode;

use super::{
    ALI, MORE_RELEASE, README, START_RELEASE, TOC, commit, cursor_on, drawn, opened, pane, read,
    repo_of, sara_on_wallet, with_sibling,
};
use crate::app_harness::App;

const OMAR: &str = "omar: try the queue";
const LOOSE: &str = "a commit no branch holds";

/// The pane's route: its lines after `built on`, trimmed and joined.
fn route(lines: &[String]) -> String {
    lines
        .iter()
        .skip_while(|line| *line != "built on")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The pane's answer lines: those after the heading that start with a mark.
fn marked(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .skip(1)
        .take_while(|line| *line != "built on")
        .filter(|line| line.starts_with(['!', '\u{2713}', '\u{b7}']))
        .collect()
}

/// Opened at 120, the cursor moved onto `subject` with `j` alone, the read
/// landed: the pane's lines.
fn pane_on(repo: &std::path::Path, subject: &str) -> (App, AppState, Vec<String>) {
    let (app, state) = opened(repo, 120);
    let state = cursor_on(&app, state, subject);
    let state = read(&app, state, 120);
    let lines = pane(&drawn(&state, 120), 120);
    (app, state, lines)
}

/// Sara's row at 120: her branch's name, four marked answers, and the route
/// `main ─◆ wallet-release ─◆ sara/fix-toc` (01r R1: wallet-release has no
/// commit after hers was cut, so it is her base). Reached with `j` alone:
/// a teammate's status costs no key beyond moving to his row.
#[test]
fn saras_row_reads_her_heading_four_answers_and_her_route() {
    let repo = sara_on_wallet();
    let (app, _state, lines) = pane_on(repo.path(), TOC);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("sara/fix-toc"),
        "{lines:#?}"
    );
    assert_eq!(marked(&lines).len(), 4, "{lines:#?}");
    assert_eq!(
        route(&lines),
        "main \u{2500}\u{25c6} wallet-release \u{2500}\u{25c6} sara/fix-toc",
        "{lines:#?}"
    );
    app.close();
}

/// A sibling cut where sara cut hers is no base of hers: the route never
/// names `ali/x`, and `ali/x`'s lane beside her row draws `┊` (Other).
#[test]
fn a_sibling_branch_is_never_on_the_route_and_its_lane_is_other() {
    let repo = with_sibling();
    let (app, state, lines) = pane_on(repo.path(), TOC);
    assert!(!route(&lines).contains("ali/x"), "{lines:#?}");
    let frame: Vec<String> = drawn(&state, 120)
        .iter()
        .map(|line| crate::row_reads::graph_side(line, 120))
        .collect();
    let row = |subject: &str| -> Vec<char> {
        frame
            .iter()
            .find(|line| line.contains(subject))
            .unwrap_or_else(|| panic!("{subject:?} drawn\n{}", frame.join("\n")))
            .chars()
            .collect()
    };
    let ali = row(ALI);
    let column = ali
        .iter()
        .position(|cell| "\u{25cf}\u{25c6}\u{25c9}\u{25cb}".contains(*cell))
        .expect("ali's node");
    let toc = row(TOC);
    let below = row(START_RELEASE);
    let cells = [toc.get(column), below.get(column)];
    assert!(
        cells.iter().all(|cell| cell != &Some(&'\u{2502}')),
        "ali/x's lane is never `│`: {cells:?}\n{}",
        frame.join("\n")
    );
    assert!(
        cells.contains(&Some(&'\u{250a}')),
        "ali/x's lane draws `┊`: {cells:?}\n{}",
        frame.join("\n")
    );
    app.close();
}

/// `HEAD`'s branch says `(you're here)`; a commit no branch holds says
/// `<sha7> · not on a branch`.
#[test]
fn heads_branch_is_here_and_a_loose_commit_is_on_no_branch() {
    let repo = with_sibling();
    let (app, _state, lines) = pane_on(repo.path(), MORE_RELEASE);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("wallet-release  (you're here)"),
        "{lines:#?}"
    );
    app.close();

    let repo = repo_of(&[
        &commit(README),
        &["switch", "--quiet", "--detach"],
        &commit(LOOSE),
    ]);
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, LOOSE);
    let state = app.pump(state, "the loose commit's pane", |state| {
        pane(&drawn(state, 120), 120)
            .iter()
            .any(|line| line.ends_with(" \u{b7} not on a branch"))
    });
    let lines = pane(&drawn(&state, 120), 120);
    let sha = lines.first().expect("a heading");
    assert_eq!(sha.len(), "abc1234 · not on a branch".len(), "{lines:#?}");
    assert!(
        sha.chars().take(7).all(|c| c.is_ascii_hexdigit()),
        "{lines:#?}"
    );
    app.close();
}

/// Inside an opened merge the heading is the merge row's branch: the rows
/// share its focus (`view::entries::focus`, G4).
#[test]
fn inside_an_opened_merge_the_heading_is_the_merges_branch() {
    let repo = repo_of(&[
        &commit(README),
        &["switch", "--quiet", "-c", "wallet-release"],
        &commit(START_RELEASE),
        &["switch", "--quiet", "-c", "sara/fix-toc"],
        &commit(TOC),
        &["switch", "--quiet", "wallet-release"],
        &[
            "merge",
            "--quiet",
            "--no-ff",
            "-m",
            "Merge branch 'sara/fix-toc' into wallet-release",
            "sara/fix-toc",
        ],
        &["branch", "--quiet", "-D", "sara/fix-toc"],
    ]);
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, "Merge branch 'sara/fix-toc'");
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the merged commit's row", |state| {
        drawn(state, 120).iter().any(|line| line.contains(TOC))
    });
    let state = cursor_on(&app, state, TOC);
    let state = read(&app, state, 120);
    let lines = pane(&drawn(&state, 120), 120);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("wallet-release  (you're here)"),
        "{lines:#?}"
    );
    app.close();
}

/// `⏎` puts the detail pane where the branch pane was; `esc` brings the
/// branch pane back.
#[test]
fn enter_swaps_in_the_detail_pane_and_esc_swaps_back() {
    let repo = sara_on_wallet();
    let (app, state, before) = pane_on(repo.path(), TOC);
    let detail = app.key(&state, KeyCode::Enter);
    let detail = app.pump(detail, "the detail pane", |state| state.detail.shown);
    assert!(
        !drawn(&detail, 120)
            .iter()
            .any(|line| line.contains(" built on")),
        "the branch pane gave way to the detail pane"
    );
    let back = app.key(&detail, KeyCode::Escape);
    let back = read(&app, back, 120);
    assert!(!back.detail.shown);
    assert_eq!(pane(&drawn(&back, 120), 120), before);
    app.close();
}

/// Y: on `HEAD`'s branch, with `user.email` set to the fixture author, the
/// who line starts `· you,` — the who column's word (G8), through the
/// repository's mailmap.
#[test]
fn the_who_line_says_you_for_his_own_branch() {
    let repo = repo_of(&[
        &[
            "config",
            "user.email",
            mergewright_fixtures::git::IDENTITY_EMAIL,
        ],
        &commit(README),
        &["switch", "--quiet", "-c", "wallet-release"],
        &commit(START_RELEASE),
        &[
            "-c",
            "user.name=Sara Khan",
            "-c",
            "user.email=sara@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            TOC,
        ],
        &commit(MORE_RELEASE),
    ]);
    let (app, _state, lines) = pane_on(repo.path(), MORE_RELEASE);
    let who = marked(&lines);
    let who = who.last().expect("a who line");
    assert!(who.starts_with("\u{b7} you,"), "{lines:#?}");
    app.close();
}

/// `main`, then `omar/x` with a commit of its own, kept only as
/// `origin/omar/x` (he never checked it out here); `sara` cut from it when
/// `cut_sara`. Omar has no commit after sara's cut (01r R1).
fn omar_remote_only(cut_sara: bool) -> tempfile::TempDir {
    let mut steps: Vec<Vec<&str>> = vec![
        commit(README),
        vec!["switch", "--quiet", "-c", "omar/x"],
        commit(OMAR),
    ];
    if cut_sara {
        steps.push(vec!["switch", "--quiet", "-c", "sara/fix-toc"]);
        steps.push(commit(TOC));
    }
    steps.push(vec!["switch", "--quiet", "main"]);
    steps.push(vec!["update-ref", "refs/remotes/origin/omar/x", "omar/x"]);
    steps.push(vec!["branch", "--quiet", "-D", "omar/x"]);
    let steps: Vec<&[&str]> = steps.iter().map(Vec::as_slice).collect();
    repo_of(&steps)
}

/// R: a remote-only branch is a branch. Its identity is its lane's
/// (settle 2 extended); on its row the heading is the column's name for it,
/// and line 1 reads `✓ pushed` — never `not on a branch`, never `not
/// pushed yet`.
#[test]
fn a_remote_only_branch_reads_as_pushed_under_its_own_name() {
    let repo = omar_remote_only(false);
    let (app, state) = opened(repo.path(), 120);
    let history = state.history.as_ref().expect("a walk");
    let walk = history.walk.as_ref().expect("walk facts");
    let refs = state.refs.as_deref().expect("refs read");
    let all = candidates(refs, &BranchNames::of(walk));
    let omar = all
        .iter()
        .find(|candidate| candidate.name == "refs/remotes/origin/omar/x")
        .unwrap_or_else(|| panic!("origin/omar/x is a candidate: {all:?}"));
    let row = history
        .window
        .as_ref()
        .and_then(|window| window.rows.iter().find(|row| row.commit.id == omar.tip))
        .expect("omar's tip on screen");
    assert_eq!(Some(identity_of(omar)), mergewright_graph::made_on(row));
    drop(state);
    app.close();

    let (app, _state, lines) = pane_on(repo.path(), OMAR);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("origin/omar/x"),
        "{lines:#?}"
    );
    assert_eq!(
        marked(&lines).first().map(|line| line.as_str()),
        Some("\u{2713} pushed"),
        "{lines:#?}"
    );
    let all = lines.join("\n");
    assert!(
        !all.contains("not on a branch") && !all.contains("no upstream set"),
        "{all}"
    );
    app.close();
}

/// R: sara cut from a remote-only branch has it on her route.
#[test]
fn a_remote_only_base_is_on_the_route() {
    let repo = omar_remote_only(true);
    let (app, _state, lines) = pane_on(repo.path(), TOC);
    assert!(route(&lines).contains("origin/omar/x"), "{lines:#?}");
    app.close();
}
