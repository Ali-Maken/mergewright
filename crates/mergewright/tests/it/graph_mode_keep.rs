//! `GRAPH MODE` in `?` through the real binary on a pty (ease-pass 06i):
//! `?` · `every` · `↓` · `⏎` switches to Every branch and keeps it in the
//! config file the run would read (no file there before), so the next
//! launch opens in it; `?` · `every` · `⏎` goes back and keeps that.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::path::{Path, PathBuf};

use mergewright_core::on_disk::CURRENT_DIR;

use crate::fold_by_default::wallet_release;
use crate::pty::{Isolated, Pty, Screen};

const DOWN: &[u8] = b"\x1b[B";

/// The file `load` reads under this run's `XDG_CONFIG_HOME`.
fn config_file(isolated: &Isolated) -> PathBuf {
    isolated.config().join(CURRENT_DIR).join("config.toml")
}

/// The journey's keys, each one step; a typed word is one step.
struct Steps<'a> {
    pty: &'a mut Pty,
    taken: usize,
}

impl Steps<'_> {
    fn take(&mut self, bytes: &[u8]) {
        self.pty.send(bytes);
        self.taken += 1;
    }
}

fn shot(shots: &mut mergewright_fixtures::Captures, name: &str, right: &str, screen: &Screen) {
    shots.check(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots/ease-pass")
            .join(name),
        format!(
            "ease-pass 06i, from the real binary on a pty, `wallet_release`: what right looks \
             like — {right} Look unjudged (rule 6); colour unverified (rule 5).\n\n{}\n",
            screen.text()
        ),
    );
}

/// `wallet_release` copied to `~/code/wallet` in `isolated`'s home, so the
/// top line reads the same on every machine.
fn wallet_in(isolated: &Isolated, scratch: &Path) -> PathBuf {
    let code = isolated.home().join("code");
    std::fs::create_dir_all(&code).expect("~/code");
    let repo = code.join("wallet");
    let copied = std::process::Command::new("cp")
        .arg("-R")
        .arg(scratch)
        .arg(&repo)
        .status()
        .expect("cp runs");
    assert!(copied.success(), "copied");
    repo
}

fn folded_and_rested(screen: &Screen) -> bool {
    screen.has("merged branches folded") && screen.has("Add the readme")
}

/// The cursor's row is `mark` (`○`/`●`) and `title`.
fn cursor_on(screen: &Screen, mark: &str, title: &str) -> bool {
    let cursor = format!("▸{mark}");
    screen
        .lines()
        .iter()
        .any(|l| l.contains(&cursor) && l.contains(title))
}

/// Quit from the graph (help closed first) and expect a clean exit.
fn quit(mut pty: Pty, help_open: bool) {
    if help_open {
        pty.send(b"\x1b");
        pty.wait_for("help closed", |s| !s.has("esc close"));
    }
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// `?` · `graph` on the folded graph at `columns`, then `↓` to Every branch:
/// `graph` also keeps three HERE rows (the renderer, the commit-graph offer's
/// two), listed first, so four `↓` reach it (ticket NOTES).
/// The scratch home is returned so it outlives the run.
fn typed_graph_on_every_branch(columns: u16) -> (Pty, Screen, Isolated) {
    let scratch = wallet_release();
    let isolated = Isolated::new();
    let repo = wallet_in(&isolated, scratch.path());
    let mut pty = Pty::spawn(&isolated, &repo, isolated.root.path(), (columns, 36));
    pty.wait_for("the folded graph", folded_and_rested);
    pty.send(b"?");
    pty.send(b"graph");
    let typed = pty.wait_for("the group", |s| s.has("Every branch") && s.has("› graph"));
    assert!(typed.has("Cycle the graph renderer"), "{}", typed.text());
    for _ in 0..4 {
        pty.send(DOWN);
    }
    let screen = pty.wait_for("the cursor on Every branch", |s| {
        cursor_on(s, "○", "Every branch")
    });
    (pty, screen, isolated)
}

#[test]
fn typed_graph_lists_the_group_after_here_at_120_and_90() {
    let mut shots = mergewright_fixtures::Captures::new();
    let (pty, wide, _home) = typed_graph_on_every_branch(120);
    assert!(wide.has("graph.merged_branches"), "{}", wide.text());
    shot(
        &mut shots,
        "help-graph-mode-120.txt",
        "`?` on the graph, typed `graph`: three `HERE` rows `graph` also finds, then the \
         `GRAPH MODE` group with `● Folded merges` (its detail, then `(on)`) and `○ Every \
         branch` under the cursor `▸`, each with `graph.merged_branches` at the right.",
        &wide,
    );
    quit(pty, true);
    let (pty, narrow, _home) = typed_graph_on_every_branch(90);
    assert!(!narrow.has("graph.merged_branches"), "{}", narrow.text());
    assert!(narrow.has("(on)"), "{}", narrow.text());
    shot(
        &mut shots,
        "help-graph-mode-90.txt",
        "as at 120, but 90 columns: no id column; `(on)` whole after Folded merges' detail.",
        &narrow,
    );
    quit(pty, true);
    shots.finish();
}

/// The counted journey: `?` · `every` · `↓` · `⏎`, no config file before
/// (`Loaded.path` is `None`): written where `load` reads; the next launch
/// opens in it; `?` · `every` · `⏎` goes back and keeps that.
#[test]
fn picking_a_graph_mode_in_help_switches_keeps_and_reopens_in_it() {
    let mut shots = mergewright_fixtures::Captures::new();
    let scratch = wallet_release();
    let isolated = Isolated::new();
    let file = config_file(&isolated);
    assert!(!file.exists(), "no config file: `Loaded.path` is `None`");
    let repo = wallet_in(&isolated, scratch.path());
    let mut pty = Pty::spawn(&isolated, &repo, isolated.root.path(), (120, 36));
    pty.wait_for("the folded graph", folded_and_rested);
    let mut steps = Steps {
        pty: &mut pty,
        taken: 0,
    };
    steps.take(b"?");
    steps.take(b"every");
    steps.take(DOWN);
    steps.pty.wait_for("the cursor on Every branch", |s| {
        cursor_on(s, "○", "Every branch")
    });
    steps.take(b"\r");
    let switched = steps.pty.wait_for("every branch, kept", |s| {
        s.has("9 commits  every branch shown") && s.has("kept for next launch")
    });
    assert!(steps.taken <= 4, "{} steps", steps.taken);
    shot(
        &mut shots,
        "graph-mode-switched-120.txt",
        "after `⏎` on Every branch: the palette gone, the top line ends `every branch \
         shown`, the bar reads `✓ Graph: every branch · kept for next launch`.",
        &switched,
    );
    let written = std::fs::read_to_string(&file).expect("the file is written");
    assert_eq!(written, "[graph]\nmerged_branches = \"shown\"\n");
    quit(pty, false);

    // The next launch opens in the kept mode, no key pressed.
    let mut pty = Pty::spawn(&isolated, &repo, isolated.root.path(), (120, 36));
    pty.wait_for("every branch, from the file", |s| {
        s.has("every branch shown")
    });
    // Back: `?` · `every` · `⏎`: Folded merges is the first row kept.
    pty.send(b"?");
    pty.send(b"every");
    pty.wait_for("the group alone", |s| cursor_on(s, "○", "Folded merges"));
    pty.send(b"\r");
    pty.wait_for("folded again, kept", |s| {
        folded_and_rested(s) && s.has("Graph: folded merges · kept for next launch")
    });
    assert_eq!(
        std::fs::read_to_string(&file).expect("read"),
        "[graph]\nmerged_branches = \"folded\"\n"
    );
    quit(pty, false);
    shots.finish();
}
