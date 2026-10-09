//! Outside a repository (m8-polish-and-ship/08, mock-ups §1.15): the real
//! binary on a pty, walking the design note's journeys `open-recent-outside`,
//! `open-second-recent` and `init-here`, counting keys against its fence.
//! Every run is isolated by `pty::Isolated` (no real config, no real recent
//! list).
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::path::{Path, PathBuf};

use crate::app_harness::git;
use crate::pty::{Isolated, Pty, Screen, count};

/// The design note whose `journey-counts` fence these walks are held to.
const NOTE: &str = "../../docs/design-notes/m8-polish-and-ship.md";

/// The alternate screen going up.
const ALT_SCREEN: &[u8] = b"\x1b[?1049h";
/// The capability probe's first query: kitty graphics, `a=q` on image 31
/// (`graft_term::detect::probe::full_round`'s first item).
const PROBE_FIRST: &[u8] = b"\x1b_Gi=31,";

fn noted_keys(id: &str) -> usize {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(NOTE);
    let note =
        std::fs::read_to_string(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    crate::m5_walk::parse_counts(&note)
        .unwrap_or_else(|why| panic!("the M8 fence does not parse: {why}"))
        .iter()
        .find(|row| row.id == id)
        .and_then(|row| row.keys)
        .unwrap_or_else(|| panic!("the M8 fence has no count for {id}"))
}

/// A journey's inputs, counted as the note's fence counts them: each key
/// one, and launching Graft one more where the row's keys start at `trib`
/// (`open-*`); `init-here` starts on the empty state (`i y`).
pub(crate) fn assert_within_the_note(id: &str, launch: bool, keys: &[&[u8]]) {
    let taken = usize::from(launch) + keys.len();
    let noted = noted_keys(id);
    assert!(
        taken <= noted,
        "{id}: took {taken} inputs, the note allows {noted}"
    );
}

/// A scratch tree: a plain folder to start in, and a ceiling above it.
struct Tree {
    root: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("scratch");
        std::fs::create_dir(root.path().join("code")).expect("plain folder");
        Self { root }
    }

    fn plain(&self) -> PathBuf {
        self.root
            .path()
            .join("code")
            .canonicalize()
            .expect("canonical")
    }

    /// A copy of the feature-branch corpus shape under the tree, named
    /// `name`; its canonical path and HEAD's subject.
    fn repo(&self, name: &str) -> (PathBuf, String) {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
        let target = self.root.path().join(name);
        let status = std::process::Command::new("cp")
            .arg("-R")
            .arg(scratch.path())
            .arg(&target)
            .status()
            .expect("cp runs");
        assert!(status.success(), "copied");
        let path = target.canonicalize().expect("canonical");
        let subject = git(&path, &["log", "-1", "--format=%s"]).trim().to_owned();
        (path, subject)
    }
}

fn on_the_empty_state(pty: &Pty) -> Screen {
    pty.wait_for("the empty state", |s| s.has("no repository here"))
}

/// The graph of the repository at `path`, its cursor on HEAD's row. Waits
/// for the cursor, not the subject anywhere: rows draw a frame before the
/// landing moves the cursor to `HEAD` ([`landed`]); under load that frame
/// failed on the top row (`M8/open-recent-outside-flake`).
fn on_the_graph_at_head(pty: &Pty, path: &Path, subject: &str) -> Screen {
    let name = path
        .file_name()
        .expect("named")
        .to_string_lossy()
        .into_owned();
    pty.wait_for(
        &format!("the repository's graph, the cursor on {subject:?}"),
        |s| {
            s.lines()[0].contains(&name)
                && s.lines()
                    .iter()
                    .any(|line| line.contains('\u{25b8}') && line.contains(subject))
        },
    )
}

pub(crate) fn quit_cleanly(mut pty: Pty) -> Vec<u8> {
    pty.send(b"q");
    let (status, stderr, output) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
    assert!(stderr.is_empty(), "nothing on stderr: {stderr:?}");
    output
}

/// The newest entry in the recent list.
fn newest_recent(isolated: &Isolated) -> PathBuf {
    let text =
        std::fs::read_to_string(isolated.recent_file()).expect("the recent list was written");
    let first = text.lines().next().expect("one line");
    PathBuf::from(first.split_once('\t').expect("secs\\tpath").1)
}

/// J14 `open-recent-outside` = 2: `trib ⏎` from a folder that is no
/// repository opens yesterday's repo at HEAD — and the terminal is taken
/// over once for both places: one alternate screen, one probe.
#[test]
fn open_recent_outside_opens_the_newest_at_head_on_one_terminal() {
    let tree = Tree::new();
    let (repo, subject) = tree.repo("payments-service");
    let (older, _) = tree.repo("web-dashboard");
    let isolated = Isolated::new();
    isolated.write_recent(&[(1_000_000_200, &repo), (1_000_000_100, &older)]);
    let mut pty = Pty::spawn(&isolated, &tree.plain(), tree.root.path(), (120, 36));

    let empty = on_the_empty_state(&pty);
    assert!(
        empty.has("\u{25b8} payments-service"),
        "newest preselected:\n{}",
        empty.text()
    );
    let keys: [&[u8]; 1] = [b"\r"];
    for key in keys {
        pty.send(key);
    }
    on_the_graph_at_head(&pty, &repo, &subject);
    assert_within_the_note("open-recent-outside", true, &keys);

    let output = quit_cleanly(pty);
    assert_eq!(
        count(&output, ALT_SCREEN),
        1,
        "the alternate screen is entered once"
    );
    assert_eq!(count(&output, PROBE_FIRST), 1, "the probe is asked once");
    assert_eq!(
        newest_recent(&isolated),
        repo,
        "opening it moved it to the top"
    );
}

/// J14 `open-second-recent` = 3: `trib j ⏎`.
#[test]
fn open_second_recent_is_j_then_enter() {
    let tree = Tree::new();
    let (newest, _) = tree.repo("payments-service");
    let (second, subject) = tree.repo("web-dashboard");
    let isolated = Isolated::new();
    isolated.write_recent(&[(1_000_000_200, &newest), (1_000_000_100, &second)]);
    let mut pty = Pty::spawn(&isolated, &tree.plain(), tree.root.path(), (120, 36));

    on_the_empty_state(&pty);
    let keys: [&[u8]; 2] = [b"j", b"\r"];
    pty.send(keys[0]);
    pty.wait_for("the cursor on the second row", |s| {
        s.has("\u{25b8} web-dashboard")
    });
    pty.send(keys[1]);
    on_the_graph_at_head(&pty, &second, &subject);
    assert_within_the_note("open-second-recent", true, &keys);
    quit_cleanly(pty);
    assert_eq!(newest_recent(&isolated), second);
}

/// J21 `init-here` = 2: `i y` on the empty state makes the folder a repository and lands
/// on its graph.
#[test]
fn init_here_is_i_then_y() {
    let tree = Tree::new();
    let isolated = Isolated::new();
    let folder = tree.plain();
    let mut pty = Pty::spawn(&isolated, &folder, tree.root.path(), (80, 24));

    on_the_empty_state(&pty);
    let keys: [&[u8]; 2] = [b"i", b"y"];
    pty.send(keys[0]);
    pty.wait_for("the init confirm", |s| {
        s.has("/code a Git repository?") && s.has("y init")
    });
    pty.send(keys[1]);
    pty.wait_for("the new repository's graph", |s| {
        s.lines()[0].contains("code") && !s.has("no repository here")
    });
    assert_within_the_note("init-here", false, &keys);
    assert!(
        folder.join(".git").is_dir(),
        "the folder became a repository"
    );
    let output = quit_cleanly(pty);
    assert_eq!(count(&output, ALT_SCREEN), 1);
    assert_eq!(
        newest_recent(&isolated),
        folder,
        "init's repository is remembered"
    );
}

/// J21's decline (Lands row "Init declined"): `i esc` leaves the empty
/// state as it was, nothing written. m8/09e: `n` no longer declines — the
/// box is a confirm box, `esc` its one exit (§2.3c) — so `i n esc` is the
/// second case: `n` is swallowed, the box stays, `esc` closes it.
#[test]
fn init_declined_writes_nothing() {
    for decline in [&[&b"\x1b"[..]][..], &[b"n", b"\x1b"]] {
        let tree = Tree::new();
        let isolated = Isolated::new();
        let folder = tree.plain();
        let mut pty = Pty::spawn(&isolated, &folder, tree.root.path(), (80, 24));

        on_the_empty_state(&pty);
        pty.send(b"i");
        pty.wait_for("the init confirm", |s| s.has("a Git repository?"));
        for key in decline {
            pty.send(key);
        }
        pty.wait_for("the confirm gone", |s| {
            !s.has("a Git repository?") && s.has("i init")
        });
        quit_cleanly(pty);
        assert!(
            !folder.join(".git").exists(),
            "{decline:?}: nothing initialised"
        );
        assert!(
            !isolated.recent_file().exists(),
            "{decline:?}: nothing remembered"
        );
    }
}

/// Open fails (Lands row): `o` + a folder that is no repository — the screen
/// stays, the cursor row stays, the bar row reads the reason.
#[test]
fn opening_a_folder_that_is_not_a_repository_stays_and_says_why() {
    let tree = Tree::new();
    let (first, _) = tree.repo("payments-service");
    let (second, _) = tree.repo("web-dashboard");
    std::fs::create_dir(tree.plain().join("notes")).expect("plain subfolder");
    let isolated = Isolated::new();
    isolated.write_recent(&[(1_000_000_200, &first), (1_000_000_100, &second)]);
    let mut pty = Pty::spawn(&isolated, &tree.plain(), tree.root.path(), (120, 36));

    on_the_empty_state(&pty);
    pty.send(b"j");
    pty.wait_for("the cursor on the second row", |s| {
        s.has("\u{25b8} web-dashboard")
    });
    pty.send(b"o");
    pty.wait_for("the open prompt", |s| {
        s.has("open \u{203a} ") && s.has("esc cancel")
    });
    pty.send(b"notes\r");
    let screen = pty.wait_for("the reason in the bar row", |s| {
        s.lines()
            .last()
            .is_some_and(|bar| bar.contains("is not inside a git repository"))
    });
    assert!(
        screen.has("no repository here"),
        "the screen stayed:\n{}",
        screen.text()
    );
    assert!(
        screen.has("\u{25b8} web-dashboard"),
        "the cursor stayed:\n{}",
        screen.text()
    );
    // The next key clears it and is acted on.
    pty.send(b"k");
    pty.wait_for("the bar back, cursor moved", |s| {
        s.has("\u{25b8} payments-service")
            && s.lines().last().is_some_and(|bar| bar.contains("q quit"))
    });
    quit_cleanly(pty);
}

/// Starting inside a repository on a tty remembers it, and draws no empty
/// state at all (it lands on `HEAD`, as an open does:
/// `starting_inside_a_repository_lands_on_head`).
#[test]
fn starting_inside_a_repository_remembers_it() {
    let tree = Tree::new();
    let (repo, subject) = tree.repo("api");
    let isolated = Isolated::new();
    let pty = Pty::spawn(&isolated, &repo, tree.root.path(), (120, 36));
    let screen = pty.wait_for("the repository's graph", |s| {
        s.lines()[0].contains("api") && s.has(&subject)
    });
    assert!(!screen.has("no repository here"), "{}", screen.text());
    quit_cleanly(pty);
    assert_eq!(newest_recent(&isolated), repo);
}

/// m8-polish-and-ship/12d: launched inside a repository, the cursor lands on
/// `HEAD`'s row, as an open from the empty state does — not on the newest
/// row. `FeatureBranches` puts `feature/three` on top and `HEAD` (`main`)
/// below it.
#[test]
fn starting_inside_a_repository_lands_on_head() {
    let tree = Tree::new();
    let (repo, subject) = tree.repo("api");
    let top = git(&repo, &["log", "-1", "--format=%s", "feature/three"])
        .trim()
        .to_owned();
    assert_ne!(top, subject, "the fixture's newest row is not HEAD");
    let isolated = Isolated::new();
    let pty = Pty::spawn(&isolated, &repo, tree.root.path(), (120, 36));
    // Waits for the cursor itself: the rows can be drawn a frame before the
    // refs read answers and the landing is spent.
    let (cursor, _) = landed(&pty, &subject, &[]);
    assert!(cursor > 2, "not the top row");
    quit_cleanly(pty);
}

/// The cursor row's index and the graph's rows down to it, once the cursor
/// sits on `subject` and every line in `settled` is drawn (the working-tree
/// read answers on its own time, and its pinned row moves every row below).
fn landed(pty: &Pty, subject: &str, settled: &[&str]) -> (usize, Vec<String>) {
    let screen = pty.wait_for("the cursor on HEAD", |s| {
        s.lines()
            .iter()
            .any(|line| line.contains('\u{25b8}') && line.contains(subject))
            && settled.iter().all(|line| s.has(line))
    });
    let lines = screen.lines();
    let cursor = lines
        .iter()
        .position(|line| line.contains('\u{25b8}'))
        .expect("a cursor row");
    // The graph column only: the pane on the right fills in on its own time.
    let graph = lines[1..=cursor]
        .iter()
        .map(|line| line.chars().take(60).collect::<String>())
        .collect();
    (cursor, graph)
}

/// The pinned uncommitted row of [`launch_and_open_land_alike`]'s tree.
const DIRTY: &str = "Uncommitted changes (1 unstaged)";

/// m8-polish-and-ship/12d Decision 2: a launch lands exactly where an open
/// from the empty state lands on the same repository — dirty tree (the
/// uncommitted row pinned on top) and `HEAD` below the first window.
#[test]
fn launch_and_open_land_alike() {
    let tree = Tree::new();
    let (repo, subject) = tree.repo("api");
    // Forty commits on a side branch, newer than `HEAD`, put `HEAD` below
    // the first window of a 36-row terminal.
    git(&repo, &["switch", "--quiet", "-c", "side"]);
    for n in 0..40 {
        git(
            &repo,
            &[
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                &format!("side {n}"),
            ],
        );
    }
    git(&repo, &["switch", "--quiet", "main"]);
    let tracked = git(&repo, &["ls-files"]);
    let file = tracked.lines().next().expect("a tracked file");
    let mut dirty = std::fs::read(repo.join(file)).expect("readable");
    dirty.extend_from_slice(b"\nan edit\n");
    std::fs::write(repo.join(file), dirty).expect("dirty tree");

    let isolated = Isolated::new();
    let launched = Pty::spawn(&isolated, &repo, tree.root.path(), (120, 36));
    let at_launch = landed(&launched, &subject, &[DIRTY]);
    quit_cleanly(launched);

    let isolated = Isolated::new();
    isolated.write_recent(&[(1_000_000_200, &repo)]);
    let mut opener = Pty::spawn(&isolated, &tree.plain(), tree.root.path(), (120, 36));
    on_the_empty_state(&opener);
    opener.send(b"\r");
    let at_open = landed(&opener, &subject, &[DIRTY]);
    quit_cleanly(opener);

    assert!(at_launch.0 > 1, "HEAD is not the top row: {at_launch:?}");
    assert_eq!(at_launch, at_open, "launch and open land alike");
}

/// A home with the mock-up's three recent repositories under it (§1.15):
/// `~/code/payments-service` yesterday, `~/code/web-dashboard` three days
/// ago, `~/dotfiles` last week. Ages are counted back from now, mid-day, so
/// they read the same whenever the test runs.
fn mockup_home(isolated: &Isolated) -> PathBuf {
    let home = isolated.home();
    let code = home.join("code");
    std::fs::create_dir_all(&code).expect("~/code");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let day = 86_400;
    let mut entries = Vec::new();
    for (place, days) in [
        (code.join("payments-service"), 1),
        (code.join("web-dashboard"), 3),
        (home.join("dotfiles"), 9),
    ] {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
        let status = std::process::Command::new("cp")
            .arg("-R")
            .arg(scratch.path())
            .arg(&place)
            .status()
            .expect("cp runs");
        assert!(status.success(), "copied");
        entries.push((now - days * day - day / 2, place));
    }
    let borrowed: Vec<(u64, &Path)> = entries
        .iter()
        .map(|(secs, path)| (*secs, path.as_path()))
        .collect();
    isolated.write_recent(&borrowed);
    code
}

fn shot(
    shots: &mut graft_fixtures::Captures,
    name: &str,
    what_right_looks_like: &str,
    screen: &Screen,
) {
    shots.check(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(name),
        format!(
            "m8-polish-and-ship/08: what right looks like — {what_right_looks_like} \
             Colour unverified (rule 5).\n\n{}\n",
            screen.text()
        ),
    );
}

/// The captures (ticket 08, mock-ups §1.15), from the real binary on a pty.
#[test]
fn outside_repo_captures() {
    let mut shots = graft_fixtures::Captures::new();
    let isolated = Isolated::new();
    let code = mockup_home(&isolated);
    let pty = Pty::spawn(&isolated, &code, isolated.root.path(), (120, 36));
    let screen = pty.wait_for("the empty state", |s| s.has("RECENT"));
    shot(
        &mut shots,
        "m8-look-outside-repo-120x36.txt",
        "started in `~/code`, which is no repository: the header says so, the folder line \
         under it, `o Open… i Init c Clone… t Practice tour`, then RECENT newest first with \
         `▸` on the newest (selection fill under it), name · `~` path · age; a `─` rule over \
         the bar row `o open… · i init · c clone… · t practice tour · ⏎ open the selected · \
         q quit · ? keys` (m8/18c added clone, m8/19b the tour).",
        &screen,
    );
    quit_cleanly(pty);

    let mut pty = Pty::spawn(&isolated, &code, isolated.root.path(), (80, 24));
    pty.wait_for("the empty state", |s| s.has("RECENT"));
    pty.send(b"o");
    let screen = pty.wait_for("the open prompt", |s| s.has("open \u{203a} ~/code/"));
    shot(
        &mut shots,
        "m8-look-outside-repo-open-prompt-80x24.txt",
        "`o` opens the boxed `open ›` prompt under the list, prefilled `~/code/` with the cursor \
         `▏` after it; the bar row now holds only the prompt's keys \
         `⏎ open · ⇥ complete · esc cancel`.",
        &screen,
    );
    pty.send(b"\x1b");
    pty.wait_for("the prompt closed", |s| !s.has("esc cancel"));
    quit_cleanly(pty);

    let bare = Isolated::new();
    std::fs::create_dir_all(bare.home().join("code")).expect("~/code");
    let pty = Pty::spawn(&bare, &bare.home().join("code"), bare.root.path(), (80, 24));
    let screen = pty.wait_for("the empty state", |s| s.has("no repository here"));
    shot(
        &mut shots,
        "m8-look-outside-repo-no-recent-80x24.txt",
        "a first run, nothing remembered yet: no RECENT block and no `⏎` in the bar \
         (`o open… · i init · c clone… · t practice tour · q quit · ? keys`); the rest as \
         with recents.",
        &screen,
    );
    quit_cleanly(pty);
    shots.finish();
}

/// Every frame of the empty state is one synchronized update (ADR-0054 §3,
/// DESIGN.md §4.1), as every repository frame is: a terminal shows it whole
/// or not at all, and so can [`Pty::wait_for`]. Without it the frame went
/// out in 1 KiB writes and a capture could catch half of one (13h).
#[test]
fn the_empty_state_draws_each_frame_as_one_synchronized_update() {
    let isolated = Isolated::new();
    let code = mockup_home(&isolated);
    let mut pty = Pty::spawn(&isolated, &code, isolated.root.path(), (120, 36));
    pty.wait_for("the empty state", |s| s.has("RECENT"));
    pty.send(b"j");
    pty.wait_for("the cursor on the second", |s| {
        s.has("\u{25b8} web-dashboard")
    });
    let output = quit_cleanly(pty);
    let begins = count(&output, graft_term::SYNC_BEGIN);
    assert!(begins >= 2, "a frame before `j` and one after: {begins}");
    assert_eq!(
        begins,
        count(&output, graft_term::SYNC_END),
        "each one ended"
    );
    for needle in ["RECENT".as_bytes(), "quit".as_bytes()] {
        let at = output
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("drawn");
        let opened = output[..at]
            .windows(graft_term::SYNC_BEGIN.len())
            .rposition(|window| window == graft_term::SYNC_BEGIN);
        let closed = output[..at]
            .windows(graft_term::SYNC_END.len())
            .rposition(|window| window == graft_term::SYNC_END);
        assert!(
            opened.is_some_and(|open| closed.is_none_or(|close| close < open)),
            "{:?} drawn inside an update",
            String::from_utf8_lossy(needle)
        );
    }
}
