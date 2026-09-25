//! The graph gallery's repository cycle and coverage matrix (ticket
//! m1-the-graph/35): pure decisions about *which* repositories the gallery
//! walks, what each is expected to show, where a heavy one lives on disk,
//! and what the test suite and `docs/shots/` already cover. The terminal
//! driving lives in `examples/gallery.rs`, which includes this file.

use std::fs;
use std::path::{Path, PathBuf};

use graft::session::Clock;
use graft::update::Banner;
use graft_core::event::{KeyCode, KeyPress};
use graft_fixtures::Shape;

/// One stop of the gallery.
pub struct Entry {
    /// What the banner calls it; also the stem of its `docs/shots/35-` files.
    pub name: &'static str,
    /// The run of stops it belongs to; neighbours share one, so the walk
    /// reads as a progression (one lane, merges, quirks, refs and badges,
    /// text, real repos).
    pub group: &'static str,
    /// Keys the harness presses before the first frame, in order
    /// (m2-details-and-diffs/23): `Enter` opens the pane on the top row,
    /// `x j j` marks a compare. Empty for every M1 stop. Fed by
    /// `update::prelude` only once the window and the refs are on hand.
    pub prelude: Vec<KeyPress>,
    /// Where the repository comes from.
    pub source: Source,
    /// What right looks like at this stop, in one line under 70 cells so an
    /// 80-column terminal reads it in full (TESTING.md "Graph gallery" has
    /// the long form). Never "looks correct".
    pub shows: &'static str,
    /// What sets this stop apart from its neighbours, one line under 70
    /// cells: the shape it exercises that they don't.
    pub differs: &'static str,
    /// Words a snapshot or capture file name uses for this repository, for
    /// the coverage matrix; the entry's own name always counts.
    pub stems: &'static [&'static str],
}

impl Entry {
    /// The clock to view this stop's dates at (m1-the-graph/53): a corpus
    /// shape is fixed at [`graft_fixtures::CORPUS_NOW`] — the instant every
    /// one of its dates, `relative-dates` included, was built to read
    /// against — so a capture never drifts; a real repository has no such
    /// instant, so it draws against the wall clock like the shipped binary
    /// does.
    #[must_use]
    pub fn clock(&self) -> Clock {
        match self.source {
            Source::Shape(_) => Clock::Fixed(graft_fixtures::CORPUS_NOW),
            Source::Workspace | Source::Synthetic100k | Source::GitGit => Clock::Wall,
        }
    }
}

/// Where an entry's repository comes from.
pub enum Source {
    /// A corpus fixture; built on demand, cheap.
    Shape(Shape),
    /// This checkout: the small real linear repository with no merges.
    Workspace,
    /// The heavy tier's 100k-commit fixture, used only if already built.
    Synthetic100k,
    /// `git/git`, used only if already cloned.
    GitGit,
}

/// Whether an entry can be opened right now.
#[derive(Debug, PartialEq, Eq)]
pub enum Availability {
    Ready(PathBuf),
    /// Not on disk; carries the exact command that puts it there.
    Missing {
        build_with: String,
    },
}

/// Where heavy repositories are looked for.
pub struct Roots {
    /// The fixture cache (`target/fixtures` or `GRAFT_FIXTURES_DIR`).
    pub fixtures: PathBuf,
    /// The bare `git/git` clone (`scripts/clone-git-git.sh --where`).
    pub git_git: PathBuf,
}

/// The keys that move between stops, as the banner spells them.
pub const KEYS: &str = "] next [ back q quit";

/// The corpus stops in walking order, each with its group, what right looks
/// like, and what sets it apart from its neighbours. The subjects and branch
/// names are the fixture's (ticket 01), so each line can be checked, not
/// felt; "stop n" refers to this table's order. A progression, not
/// `Shape::ALL`'s order: similar shapes sit together so `]` shows what
/// changes between neighbours. The test below keeps this a permutation of
/// `Shape::ALL`, so a new shape fails it until a row is written.
const CORPUS: [(Shape, &str, &str, &str); 21] = [
    (
        Shape::Trivial,
        ONE_LANE,
        "3 commits in one straight lane: ◆ HEAD on top, ◇ root at the bottom",
        "the smallest stop, so only 3 rows show: that is all of it, a baseline",
    ),
    (
        Shape::Linear,
        ONE_LANE,
        "200 commits in one lane; G, g and PageDown never shift the node column",
        "the same lane as stop 1, 200 rows deep: scrolling is what it tests",
    ),
    (
        Shape::FeatureBranches,
        MERGES,
        "two merges on main; feature/one and /two curve in, /three stays open",
        "the first curves; Space folds the merged two, 1 leaves main's column",
    ),
    (
        Shape::OctopusMerge,
        MERGES,
        "one merge node with three curves in, topic-1..3 each in its own colour",
        "three parents into one node, where stop 3 has one curve per merge",
    ),
    (
        Shape::CrissCrossMerge,
        MERGES,
        "alpha and beta merge each other, so two curves cross; main merges alpha",
        "the only stop where curves cross; F on 'alpha: merge beta' fades beta",
    ),
    (
        Shape::FoxtrotMerge,
        MERGES,
        "the ◆ merge sits in feature's column; main's own line curves into it",
        "1 drops 'Advance main': main's first-parent walk leaves lane 0",
    ),
    (
        Shape::BackwardMerge,
        MERGES,
        "stop 6's subjects, but the merge is feature's tip; lane 0 never bends",
        "6's mirror: [◆ main] stays on 'Advance main'; 1 keeps both main rows",
    ),
    (
        Shape::EvilMerge,
        MERGES,
        "◆ in lane 0, one curve in, and a ⚠ on the merge once the probe lands",
        "the only graph stop with a ⚠; the radar probes the selected row",
    ),
    (
        Shape::AccidentalPullMerge,
        MERGES,
        "a merge whose subject starts 'Merge branch main of https://…'",
        "the curve comes from [origin/main], with no local branch on that row",
    ),
    (
        Shape::OrphanBranch,
        ODD,
        "two roots: docs' ◇ has no line below it and never joins main's lane",
        "the only stop with two ◇ roots; nothing curves between the columns",
    ),
    (
        Shape::CherryPickDuplicates,
        ODD,
        "'Fix the greeting' twice, on main and on topic, no line joining them",
        "two lanes that share a root but never merge; stop 3's do",
    ),
    (
        Shape::SkewedDates,
        ODD,
        "'Commit from a fast clock' is dated a day ahead yet sits below Main 01",
        "the only stop where date order and topological order disagree",
    ),
    (
        Shape::RelativeDates,
        ODD,
        "one lane, top to bottom: now, 2m, 2h, 2d, 2w, 2mo, 1y — every unit",
        "the date column varies here; every other stop sits in one tight window",
    ),
    (
        Shape::Tags,
        REFS,
        "one [tag: vX] per row, v0.1 to v2.0; annotated v1.0 and v2.0 sit peeled",
        "badges only: one lane, no merge; up to stop 17 the refs are the point",
    ),
    (
        Shape::RefStates,
        REFS,
        "[◆ main ⇄ origin], [ahead ↑1], [behind ↓1]; 'archived' is a tag alone",
        "every upstream state at once; stop 13 had tags only, no remote",
    ),
    (
        Shape::DetachedHead,
        REFS,
        "[HEAD] on ◆ Commit 2, two rows below [main] (no ◆: not current)",
        "◆ and [main] on different rows; at every other stop they share one",
    ),
    (
        Shape::UnbornBranch,
        REFS,
        "no ◆ row at all: HEAD is the unborn branch fresh; main and other draw",
        "the one stop with no HEAD commit and no Uncommitted row at the top",
    ),
    (
        Shape::Stash,
        REFS,
        "■ [stash] 'On main: wip…' one row above ◆ main's tip; H hides that row",
        "the only ■ node; H removes one row and nothing else moves",
    ),
    (
        Shape::UnicodeNames,
        TEXT,
        "wide glyphs (機能, ☕, 🚀) in badge and subject; the sha column holds",
        "a plain shape (three branches): the cell widths are the test",
    ),
    (
        Shape::NonUtf8Path,
        TEXT,
        "two lanes, main and latin1; no \u{fffd} or stray byte anywhere on screen",
        "the bad byte is in a path, so nothing shows; 20 has one in a badge",
    ),
    (
        Shape::NonUtf8RefName,
        TEXT,
        "a branch badge reading caf\u{fffd}: one replacement char, nothing else moved",
        "the one stop that shows \u{fffd}; not built on APFS ('not available')",
    ),
];

const ONE_LANE: &str = "one lane";
const MERGES: &str = "merges";
const ODD: &str = "quirks";
const REFS: &str = "refs and badges";
const TEXT: &str = "text";
const REAL: &str = "real repos";

/// The gallery's stops, in walking order: the corpus as a progression, then
/// this checkout, then the two heavy repositories.
pub fn entries() -> Vec<Entry> {
    let mut all: Vec<Entry> = CORPUS
        .iter()
        .map(|&(shape, group, shows, differs)| Entry {
            name: shape.name(),
            group,
            prelude: Vec::new(),
            source: Source::Shape(shape),
            shows,
            differs,
            stems: stems(shape),
        })
        .collect();
    all.push(Entry {
        name: "graft-self",
        group: REAL,
        prelude: Vec::new(),
        source: Source::Workspace,
        shows: "this checkout: one lane, ● everywhere but ◆ at HEAD and ◇ at the bottom",
        differs: "a real repo with no merges: stop 2's shape, real subjects and dates",
        stems: &["gitter"],
    });
    all.push(Entry {
        name: "synthetic-100k",
        group: REAL,
        prelude: Vec::new(),
        source: Source::Synthetic100k,
        shows: "100k commits: paints at once; lanes fold behind +N; j and G never stall",
        differs: "size is the test, the shape is synthetic; needs the heavy tier built",
        stems: &["100k", "synthetic-100000"],
    });
    all.push(Entry {
        name: "git-git",
        group: REAL,
        prelude: Vec::new(),
        source: Source::GitGit,
        shows: "master straight in lane 0 throughout; ◉ merges curve in; [tag: v2.x.y]",
        differs: "stop 3's shapes at real scale; needs scripts/clone-git-git.sh first",
        stems: &["git-git"],
    });
    all
}

/// File-name words older tickets used for a shape, where they differ from
/// its name.
fn stems(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::CrissCrossMerge => &["criss-cross", "crisscross"],
        Shape::FoxtrotMerge => &["foxtrot"],
        Shape::OctopusMerge => &["octopus"],
        Shape::SkewedDates => &["skewed-dates"],
        _ => &[],
    }
}

/// Which progression the gallery walks (m2-details-and-diffs/23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// M1's: the shape of history, one repository per stop, no keys pressed.
    Graph,
    /// M2's: the detail pane and the Mistake Radar, each stop a repository
    /// plus the keys that open what it shows.
    Details,
}

impl Group {
    /// The word `--group` takes and `--list` heads each progression with.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Group::Graph => "graph",
            Group::Details => "details",
        }
    }

    /// The group `--group <word>` names, if any.
    #[must_use]
    pub fn parse(word: &str) -> Option<Group> {
        [Group::Graph, Group::Details]
            .into_iter()
            .find(|group| group.name() == word)
    }
}

/// One progression's stops in walking order.
pub fn stops(group: Group) -> Vec<Entry> {
    match group {
        Group::Graph => entries(),
        Group::Details => details(),
    }
}

const ENTER: KeyPress = KeyPress::plain(KeyCode::Enter);
const TAB: KeyPress = KeyPress::plain(KeyCode::Tab);

/// A printable key.
const fn ch(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

/// `/`, the query, `Enter`: a search the next key waits for.
fn search(query: &str) -> Vec<KeyPress> {
    let mut keys = vec![ch('/')];
    keys.extend(query.chars().map(ch));
    keys.push(ENTER);
    keys
}

/// The subject stop 18 searches for: a release merge on `git/git`'s master
/// whose tree runs to 805 files (`git diff-tree --name-only 03d05937a^
/// 03d05937a | wc -l`). The tagged commit itself ("Git 2.41-rc0") sits
/// below the merge, so the search's first hit going down is the merge.
pub const GIT_GIT_RELEASE_MERGE: &str = "v2.41.0-rc0";

const PANE: &str = "the pane";
const RADAR: &str = "radar";
const MAP_AND_STRIP: &str = "map and strip";

/// The "details and radar" progression (m2-details-and-diffs/23): the pane
/// on one merge, five ways; every radar kind the corpus can raise, and the
/// one it cannot; text edge cases in the files tree; the Map view and the
/// strip; then the same on a real repository and on `git/git`. Each stop's
/// `shows` names what the fixture (ticket 01) and the design note's New UI
/// say must be on screen; TESTING.md "Details and diffs" has the long form.
#[allow(clippy::too_many_lines, reason = "a table of 19 stops, like CORPUS")]
fn details() -> Vec<Entry> {
    let shape = |name: &str| {
        Shape::ALL
            .iter()
            .copied()
            .find(|shape| shape.name() == name)
            .map_or_else(|| panic!("no corpus shape named {name}"), Source::Shape)
    };
    let stop = |name: &'static str,
                group: &'static str,
                prelude: Vec<KeyPress>,
                source: Source,
                shows: &'static str,
                differs: &'static str| Entry {
        name,
        group,
        prelude,
        source,
        shows,
        differs,
        stems: &[],
    };
    vec![
        stop(
            "feature-branches",
            PANE,
            vec![ENTER],
            shape("feature-branches"),
            "pane on three: in progress (row 0): header, A three.txt +1 −0, 1 + a",
            "the baseline pane; the merge is row 1: j then Enter opens it instead",
        ),
        stop(
            "feature-branches",
            PANE,
            vec![ENTER, ch('j')],
            shape("feature-branches"),
            "j inside Files on a one-file commit: nothing moves; header and diff stay",
            "no corpus commit has two files (KNOWN_GAPS): the stop shows j is inert",
        ),
        stop(
            "feature-branches",
            PANE,
            vec![ENTER, TAB, ch('v')],
            shape("feature-branches"),
            "side-by-side: two halves with own gutters, │ between, unpaired blank",
            "stop 1's diff split in two; Tab moved focus to the body first",
        ),
        stop(
            "feature-branches",
            PANE,
            vec![ENTER, ch('W')],
            shape("feature-branches"),
            "word tint off: W drops the changed words' stronger bg, line colour stays",
            "stop 1 with W: word tint (default on) toggled off; judge in a terminal",
        ),
        stop(
            "feature-branches",
            PANE,
            vec![ch('x'), ch('j'), ch('j'), ch('x')],
            shape("feature-branches"),
            "pane header COMPARE 0683985 → e30e870 (no counts yet); ▏ on the base row",
            "no COMMIT header: two rows compared; the base stays marked as j moves",
        ),
        stop(
            "foxtrot-merge",
            RADAR,
            vec![],
            shape("foxtrot-merge"),
            "⚠ in the marker column on the foxtrot merge; footer gains . explain",
            "the first ⚠: ⓘ (no upstream) sat on branch tips since stop 1",
        ),
        stop(
            "foxtrot-merge",
            RADAR,
            vec![ch('.')],
            shape("foxtrot-merge"),
            "the . menu on the ⚠ merge: dim Checkout, Cherry-pick, then Explain",
            "Explain third: slots 1-2 are fixed (m6/71); stop 6's ⚠ adds Explain",
        ),
        stop(
            "backward-merge",
            RADAR,
            vec![],
            shape("backward-merge"),
            "⚠ backward merge on feature's tip merge; main's two rows unmarked",
            "stop 6's mirror: same subjects, the marker sits on feature's row",
        ),
        stop(
            "accidental-pull-merge",
            RADAR,
            vec![],
            shape("accidental-pull-merge"),
            "ⓘ on 'Merge branch main of https://…': info colour, not warning",
            "the only ⓘ the graph alone raises: no probe, no tree read",
        ),
        stop(
            "evil-merge",
            RADAR,
            vec![ENTER],
            shape("evil-merge"),
            "pane subject row gains · evil merge when the tree probe lands (live git)",
            "no marker until the probe answers: evil is a tree fact, not a shape",
        ),
        stop(
            "cherry-pick-duplicates",
            RADAR,
            vec![],
            shape("cherry-pick-duplicates"),
            "ⓘ duplicate commit on both 'Fix the greeting' rows when the probe lands",
            "topic's ⓘ is no upstream; the two fix rows share one finding",
        ),
        stop(
            "ref-states",
            RADAR,
            vec![],
            shape("ref-states"),
            "no marker column: ahead tracks origin/ahead and is not the default",
            "the negative control: classify_refs flags only unpushed-on-default",
        ),
        stop(
            "unicode-names",
            TEXT,
            vec![ENTER],
            shape("unicode-names"),
            "the CJK path in the files tree; the letter and +N −N columns aligned",
            "wide glyphs in the tree: the right-aligned counts must not drift",
        ),
        stop(
            "non-utf8-path",
            TEXT,
            vec![ENTER],
            shape("non-utf8-path"),
            "the Latin-1 path shows one � in the tree; its diff still loads",
            "one bad byte, one �: the tree's other entries are untouched",
        ),
        stop(
            "feature-branches",
            MAP_AND_STRIP,
            vec![ch('M')],
            shape("feature-branches"),
            "Map view: 4 branch rows, elbows on the two merged, ▸ main … ●",
            "no graph: one row per local branch, bars per bucket; M or Esc back",
        ),
        stop(
            "feature-branches",
            MAP_AND_STRIP,
            vec![ch('O')],
            shape("feature-branches"),
            "strip at the right edge: ◉ twice, ● at the top, █ over the viewport",
            "one column added to stop 3's graph; O again removes it",
        ),
        stop(
            "graft-self",
            REAL,
            vec![ENTER],
            Source::Workspace,
            "real commit: Contained in: main; ✉ row only if the message has a body",
            "real repo: syntax roles tint the diff; the corpus stops are plain",
        ),
        stop(
            "git-git",
            REAL,
            {
                let mut keys = search(GIT_GIT_RELEASE_MERGE);
                keys.push(ENTER);
                keys
            },
            Source::GitGit,
            "Merge tag 'v2.41.0-rc0': 805 files, ✉ message (24 lines), first diff",
            "scale: search lands the merge, Enter opens it; j to a long file, L",
        ),
        stop(
            "git-git",
            REAL,
            vec![ch('O')],
            Source::GitGit,
            "strip honestly incomplete: buckets below the walked range are blank",
            "stop 16's strip at 85k rows: it grows as the walk lands, never at once",
        ),
    ]
}

/// The prelude as the banner spells it: `Enter Tab v`; a search folds into
/// one word, `/v2.41.0-rc0⏎`, so the title stays under the width.
pub fn keys_label(prelude: &[KeyPress]) -> String {
    let mut words = Vec::new();
    let mut keys = prelude.iter().peekable();
    while let Some(key) = keys.next() {
        let word = match key.code {
            KeyCode::Char('/') => {
                let mut typed = String::from("/");
                for next in keys.by_ref() {
                    match next.code {
                        KeyCode::Char(c) => typed.push(c),
                        _ => break,
                    }
                }
                typed.push('⏎');
                typed
            }
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Escape => "Esc".to_string(),
            other => format!("{other:?}"),
        };
        words.push(word);
    }
    words.join(" ")
}

/// The stem of a details stop's capture files: `m2-<nn>-<name>[-<keys>]`,
/// keys lowercased and punctuation named, so part (b)'s files and the
/// matrix agree (`m2-07-foxtrot-merge-dot`).
// Read by the tests now; part (b) gives the example a `--stems` flag for
// its capture script, until which the example build has no caller.
pub fn capture_stem(entry: &Entry, index: usize) -> String {
    let mut stem = format!("m2-{:02}-{}", index + 1, entry.name);
    let mut keys = entry.prelude.iter();
    while let Some(key) = keys.next() {
        let word = match key.code {
            KeyCode::Char('/') => {
                // The typed query and its Enter are one word.
                for next in keys.by_ref() {
                    if next.code == KeyCode::Enter {
                        break;
                    }
                }
                "search".to_string()
            }
            KeyCode::Char('.') => "dot".to_string(),
            KeyCode::Char(c) => c.to_ascii_lowercase().to_string(),
            KeyCode::Enter => "enter".to_string(),
            KeyCode::Tab => "tab".to_string(),
            other => format!("{other:?}").to_ascii_lowercase(),
        };
        stem.push('-');
        stem.push_str(&word);
    }
    stem
}

/// The banner's title: where the user is and how to move on. The run
/// appends ` · <tier>` (`view::banner_lines`); [`TIER_SUFFIX`] reserves it.
/// A stop with a prelude names its keys where a plain stop names its group.
pub fn title(entry: &Entry, index: usize, total: usize) -> String {
    let label = if entry.prelude.is_empty() {
        entry.group.to_string()
    } else {
        keys_label(&entry.prelude)
    };
    format!(
        "gallery {}/{} {} · {label} · {KEYS}",
        index + 1,
        total,
        entry.name
    )
}

/// The banner while an entry is on screen: the title, then what to look
/// for and what sets the stop apart.
pub fn banner(entry: &Entry, index: usize, total: usize) -> Banner {
    Banner {
        title: title(entry, index, total),
        lines: vec![
            format!("shows: {}", entry.shows),
            format!("differs: {}", entry.differs),
        ],
    }
}

/// The page shown instead of a repository that is not on disk: the same
/// banner, with the build command where the graph would be.
pub fn missing_page(entry: &Entry, index: usize, total: usize, build_with: &str) -> Vec<String> {
    let banner = banner(entry, index, total);
    let mut lines = vec![banner.title];
    lines.extend(banner.lines);
    lines.push(String::new());
    lines.push(format!("not available — build with: {build_with}"));
    lines
}

/// What `--list` prints: every stop, its group and both finding lines, the
/// same words the banner shows.
pub fn listing(all: &[Entry]) -> String {
    let mut out = String::new();
    let mut group = "";
    for (index, entry) in all.iter().enumerate() {
        if entry.group != group {
            group = entry.group;
            out.push_str(&format!("\n{group}\n"));
        }
        out.push_str(&format!("{:2} {}\n", index + 1, entry.name));
        out.push_str(&format!("   shows: {}\n", entry.shows));
        out.push_str(&format!("   differs: {}\n", entry.differs));
    }
    out
}

/// Where the heavy repositories are looked for on this machine, following
/// the fixture cache's and `scripts/clone-git-git.sh`'s own rules.
pub fn roots(workspace: &Path) -> Roots {
    let fixtures = std::env::var_os("GRAFT_FIXTURES_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("CARGO_TARGET_DIR").map(|dir| PathBuf::from(dir).join("fixtures"))
        })
        .unwrap_or_else(|| workspace.join("target/fixtures"));
    let git_git = std::env::var_os("GRAFT_GIT_GIT_DIR").map_or_else(
        || {
            std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
                .unwrap_or_else(|| workspace.join(".cache"))
                .join("graft/git-git.git")
        },
        PathBuf::from,
    );
    Roots { fixtures, git_git }
}

/// Open an entry, or say what would put it on disk. Corpus shapes are
/// built here when missing (a second's work); heavy ones never are.
pub fn resolve(entry: &Entry, workspace: &Path, roots: &Roots) -> Availability {
    match entry.source {
        Source::Shape(shape) => match graft_fixtures::try_borrow(shape) {
            Ok(dir) => Availability::Ready(dir),
            Err(error) => Availability::Missing {
                build_with: format!("cargo run -p graft-fixtures  ({error})"),
            },
        },
        Source::Workspace => Availability::Ready(workspace.to_path_buf()),
        Source::Synthetic100k => match synthetic_100k(&roots.fixtures) {
            Some(dir) => Availability::Ready(dir),
            None => Availability::Missing {
                build_with: String::from("cargo test --features heavy-tests -- --test-threads=2"),
            },
        },
        Source::GitGit => {
            if roots.git_git.join("HEAD").is_file() {
                Availability::Ready(roots.git_git.clone())
            } else {
                Availability::Missing {
                    build_with: String::from("scripts/clone-git-git.sh"),
                }
            }
        }
    }
}

/// The 100k fixture's directory, if the heavy tier has built one. Its cache
/// key is private to the fixtures crate and heavy-only, so the directory is
/// found by name and by its `corpus.ok` marker rather than computed.
fn synthetic_100k(fixtures: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(fixtures).ok()?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|dir| {
            dir.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("synthetic-100000-"))
                && dir.join("corpus.ok").is_file()
        })
        .collect();
    found.sort();
    found.pop()
}

/// A tier column of the coverage matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    Pixel,
    Unicode,
    Ascii,
    /// A layout diagram with no tier at all (graft-graph's corpus snapshots).
    Layout,
}

const TIERS: [Tier; 4] = [Tier::Pixel, Tier::Unicode, Tier::Ascii, Tier::Layout];
const THEMES: [&str; 2] = ["dark", "light"];
const SIZES: [&str; 5] = ["80x24", "100x30", "120x40", "160x40", "200x60"];

/// One snapshot or capture file, read from its name and where it sits.
struct Evidence {
    snapshot: bool,
    tier: Tier,
    theme: &'static str,
    size: &'static str,
    name: String,
}

/// Every `.snap` under `crates/` and every file under `docs/shots/`,
/// classified by name: `raster`/`kitty`/`iterm2`/`.png` → pixel, `ascii` →
/// ASCII, a graft-graph corpus snapshot → layout, else Unicode; the word
/// `light` in the name → light, else dark; a `WxH` size token, else `?`.
fn evidence(workspace: &Path) -> Vec<Evidence> {
    let mut files = Vec::new();
    collect(&workspace.join("crates"), &mut files);
    collect(&workspace.join("docs/shots"), &mut files);
    files
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_ascii_lowercase();
            let snapshot = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("snap"));
            let in_shots = path.parent()?.ends_with("docs/shots");
            if !snapshot && !in_shots {
                return None;
            }
            let in_graph_crate = path.starts_with(workspace.join("crates/graft-graph"));
            let tier = if ["raster", "kitty", "iterm2", ".png"]
                .iter()
                .any(|word| name.contains(word))
            {
                Tier::Pixel
            } else if name.contains("ascii") {
                Tier::Ascii
            } else if snapshot && in_graph_crate {
                Tier::Layout
            } else {
                Tier::Unicode
            };
            // Whole words: `highlight` is not a light theme.
            let words: Vec<&str> = name.split(|c: char| !c.is_ascii_alphanumeric()).collect();
            let theme = if words.contains(&"light") {
                "light"
            } else {
                "dark"
            };
            let size = SIZES
                .iter()
                .find(|size| name.contains(*size))
                .copied()
                .unwrap_or("?");
            Some(Evidence {
                snapshot,
                tier,
                theme,
                size,
                name,
            })
        })
        .collect()
}

fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            collect(&path, into);
        } else {
            into.push(path);
        }
    }
}

/// The gallery's tables: repo × tier × theme with the sizes each cell has,
/// then the details group's, from the files that exist today. `S` a
/// snapshot, `C` a pty capture, `—` nothing; sizes follow, `?` when the file
/// name does not say. The whole matrix, walks included, is
/// `coverage::coverage_matrix`.
pub fn gallery_matrix(workspace: &Path) -> String {
    let evidence = evidence(workspace);
    let entries = entries();
    let width = entries
        .iter()
        .map(|entry| entry.name.len())
        .max()
        .unwrap_or(0)
        .max(4);
    let mut out = String::new();
    out.push_str("Graph coverage: repo × tier × theme, with sizes (m1-the-graph/35)\n");
    out.push_str("S snapshot (.snap under crates/)   C capture (docs/shots/)   — nothing\n");
    out.push_str("Sizes as the file name says; ? = not in the name. Read from names only:\n");
    out.push_str("a file covering more than its name says counts once, as its name says.\n");
    out.push_str("layout = graft-graph's tier-less layout diagrams.\n\n");
    out.push_str(&format!("{:width$}", "repo"));
    for tier in TIERS {
        for theme in THEMES {
            if tier == Tier::Layout && theme == "light" {
                continue;
            }
            out.push_str(&format!("  {:<22}", format!("{}/{theme}", tier_name(tier))));
        }
    }
    out.push('\n');
    for entry in &entries {
        out.push_str(&format!("{:width$}", entry.name));
        let mine: Vec<&Evidence> = evidence
            .iter()
            .filter(|file| {
                std::iter::once(&entry.name)
                    .chain(entry.stems.iter())
                    .any(|stem| file.name.contains(stem))
            })
            .collect();
        for tier in TIERS {
            for theme in THEMES {
                if tier == Tier::Layout && theme == "light" {
                    continue;
                }
                out.push_str(&format!("  {:<22}", cell(&mine, tier, theme)));
            }
        }
        out.push('\n');
    }
    out.push_str(&details_matrix(&evidence));
    out
}

/// The details group's table (m2-details-and-diffs/23): one row per stop,
/// keyed by its capture stem, so `m2-<nn>-<stop>-<keys>-<WxH>-<tier>.txt`
/// files land on their own stop and nowhere else. Same cells as above.
fn details_matrix(evidence: &[Evidence]) -> String {
    let all = stops(Group::Details);
    let stems: Vec<String> = all
        .iter()
        .enumerate()
        .map(|(index, entry)| capture_stem(entry, index))
        .collect();
    let width = stems.iter().map(String::len).max().unwrap_or(0).max(4);
    let mut out = String::new();
    out.push_str("\nDetails coverage: stop × tier × theme, with sizes (m2-details-and-diffs/23)\n");
    out.push_str("Files docs/shots/m2-<nn>-<stop>-<keys>-<WxH>-<tier>.txt, matched by stem;\n");
    out.push_str("the pane lives at 160x40, so a stop without a 160x40 capture is a gap.\n\n");
    out.push_str(&format!("{:width$}", "stop"));
    for tier in TIERS {
        for theme in THEMES {
            if tier == Tier::Layout && theme == "light" {
                continue;
            }
            out.push_str(&format!("  {:<22}", format!("{}/{theme}", tier_name(tier))));
        }
    }
    out.push('\n');
    for stem in &stems {
        out.push_str(&format!("{stem:width$}"));
        let mine: Vec<&Evidence> = evidence
            .iter()
            .filter(|file| file.name.starts_with(stem.as_str()))
            .collect();
        for tier in TIERS {
            for theme in THEMES {
                if tier == Tier::Layout && theme == "light" {
                    continue;
                }
                out.push_str(&format!("  {:<22}", cell(&mine, tier, theme)));
            }
        }
        out.push('\n');
    }
    out
}

fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Pixel => "pixel",
        Tier::Unicode => "unicode",
        Tier::Ascii => "ascii",
        Tier::Layout => "layout",
    }
}

fn cell(mine: &[&Evidence], tier: Tier, theme: &str) -> String {
    let mut marks = String::new();
    for (snapshot, mark) in [(true, 'S'), (false, 'C')] {
        let mut sizes: Vec<&str> = mine
            .iter()
            .filter(|file| file.snapshot == snapshot && file.tier == tier && file.theme == theme)
            .map(|file| file.size)
            .collect();
        if sizes.is_empty() {
            continue;
        }
        // In DESIGN.md's order, `?` last.
        sizes.sort_unstable_by_key(|size| SIZES.iter().position(|known| known == size));
        sizes.dedup();
        if !marks.is_empty() {
            marks.push(' ');
        }
        marks.push(mark);
        marks.push(':');
        marks.push_str(&sizes.join(","));
    }
    if marks.is_empty() {
        marks.push('—');
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    /// The widest a banner line may be, so an 80-column terminal shows every
    /// stop's line in full: the screen puts one cell of margin before it.
    const BANNER_WIDTH: usize = 79;

    /// Cells the run adds to the title for the tier in force: ` · unicode`,
    /// the longest tier name.
    const TIER_SUFFIX: usize = 10;

    fn workspace() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn the_banner_names_the_stop_its_position_the_keys_and_both_findings() {
        let all = entries();
        let foxtrot = all
            .iter()
            .position(|entry| entry.name == "foxtrot-merge")
            .unwrap();
        let banner = banner(&all[foxtrot], foxtrot, all.len());
        assert_eq!(
            banner.title,
            format!(
                "gallery {}/{} foxtrot-merge · merges · ] next [ back q quit",
                foxtrot + 1,
                all.len()
            )
        );
        assert_eq!(
            banner.lines,
            vec![
                "shows: the ◆ merge sits in feature's column; main's own line curves into it",
                "differs: 1 drops 'Advance main': main's first-parent walk leaves lane 0",
            ]
        );
        assert_eq!(banner.rows(), 3);
    }

    #[test]
    fn corpus_stops_view_at_corpus_now_real_repos_view_the_wall_clock() {
        for entry in entries() {
            let expected = match entry.source {
                Source::Shape(_) => Clock::Fixed(graft_fixtures::CORPUS_NOW),
                Source::Workspace | Source::Synthetic100k | Source::GitGit => Clock::Wall,
            };
            assert_eq!(entry.clock(), expected, "{}", entry.name);
        }
    }

    #[test]
    fn the_first_stop_is_the_smallest_and_says_so() {
        let all = entries();
        assert_eq!(all[0].name, "trivial");
        assert!(all[0].differs.contains("only 3 rows"), "{}", all[0].differs);
    }

    #[test]
    fn every_shape_is_a_stop_once_and_every_finding_is_concrete_and_fits() {
        let all = entries();
        let names: Vec<&str> = all.iter().map(|entry| entry.name).collect();
        for shape in Shape::ALL {
            assert_eq!(
                names.iter().filter(|name| **name == shape.name()).count(),
                1,
                "{} must be exactly one stop",
                shape.name()
            );
        }
        assert_eq!(all.len(), Shape::ALL.len() + 3);
        assert_eq!(
            &names[names.len() - 3..],
            &["graft-self", "synthetic-100k", "git-git"]
        );
        concrete_and_fits(&all);
    }

    /// Every finding is concrete, and every banner line fits 80 columns
    /// with the tier suffix; both groups hold to it.
    fn concrete_and_fits(all: &[Entry]) {
        for (index, entry) in all.iter().enumerate() {
            for text in [entry.shows, entry.differs] {
                let lower = text.to_ascii_lowercase();
                assert!(
                    !lower.contains("looks correct") && !lower.contains("renders fine"),
                    "{}",
                    entry.name
                );
                assert!(text.len() > 20, "{}: too vague", entry.name);
            }
            let banner = banner(entry, index, all.len());
            let title = UnicodeWidthStr::width(banner.title.as_str());
            assert!(
                title + TIER_SUFFIX <= BANNER_WIDTH,
                "{}: {:?} is {title} cells, over {} with the tier",
                entry.name,
                banner.title,
                BANNER_WIDTH - TIER_SUFFIX
            );
            for line in &banner.lines {
                assert!(
                    UnicodeWidthStr::width(line.as_str()) <= BANNER_WIDTH,
                    "{}: {line:?} is {} cells, over {BANNER_WIDTH}",
                    entry.name,
                    UnicodeWidthStr::width(line.as_str())
                );
            }
        }
    }

    /// m2-details-and-diffs/23: the details group is the ticket's 19 stops
    /// in its order, each concrete and fitting, its keys spelled in the
    /// title and folded into a capture stem; the graph group is untouched.
    #[test]
    fn the_details_group_walks_the_19_stops_with_their_keys_and_fits() {
        let all = stops(Group::Details);
        assert_eq!(all.len(), 19);
        concrete_and_fits(&all);
        let names: Vec<&str> = all.iter().map(|entry| entry.name).collect();
        assert_eq!(
            names,
            [
                "feature-branches",
                "feature-branches",
                "feature-branches",
                "feature-branches",
                "feature-branches",
                "foxtrot-merge",
                "foxtrot-merge",
                "backward-merge",
                "accidental-pull-merge",
                "evil-merge",
                "cherry-pick-duplicates",
                "ref-states",
                "unicode-names",
                "non-utf8-path",
                "feature-branches",
                "feature-branches",
                "graft-self",
                "git-git",
                "git-git",
            ]
        );
        assert_eq!(
            title(&all[2], 2, 19),
            "gallery 3/19 feature-branches · Enter Tab v · ] next [ back q quit"
        );
        assert_eq!(
            title(&all[5], 5, 19),
            "gallery 6/19 foxtrot-merge · radar · ] next [ back q quit",
            "no prelude: the group stands where the keys would"
        );
        assert_eq!(keys_label(&all[17].prelude), "/v2.41.0-rc0⏎ Enter");
        assert_eq!(capture_stem(&all[6], 6), "m2-07-foxtrot-merge-dot");
        assert_eq!(capture_stem(&all[17], 17), "m2-18-git-git-search-enter");
        assert_eq!(capture_stem(&all[15], 15), "m2-16-feature-branches-o");
        assert_eq!(
            listing(&stops(Group::Graph)),
            listing(&entries()),
            "the graph group is M1's, byte for byte"
        );
        assert!(listing(&all).starts_with("\nthe pane\n 1 feature-branches\n"));
        assert_eq!(Group::parse("details"), Some(Group::Details));
        assert_eq!(Group::parse("pane"), None);
    }

    #[test]
    fn neighbouring_stops_share_a_group_and_the_groups_read_as_a_progression() {
        let all = entries();
        let mut groups: Vec<&str> = Vec::new();
        for entry in &all {
            if groups.last() != Some(&entry.group) {
                groups.push(entry.group);
            }
        }
        assert_eq!(
            groups,
            [
                "one lane",
                "merges",
                "quirks",
                "refs and badges",
                "text",
                "real repos"
            ],
            "a group must not be split by another"
        );
        let listing = listing(&all);
        assert!(
            listing.contains("\nmerges\n 3 feature-branches\n"),
            "{listing}"
        );
        assert!(listing.contains("   differs: "), "{listing}");
    }

    #[test]
    fn a_missing_repo_keeps_the_banner_and_says_how_to_build() {
        let all = entries();
        let git_git = all.iter().find(|entry| entry.name == "git-git").unwrap();
        let page = missing_page(git_git, 22, 23, "scripts/clone-git-git.sh");
        assert!(page[0].starts_with("gallery 23/23 git-git · real repos"));
        assert_eq!(
            page.last().map(String::as_str),
            Some("not available — build with: scripts/clone-git-git.sh")
        );
    }

    #[test]
    fn heavy_repos_are_used_only_if_present_and_say_how_to_build_otherwise() {
        let temp = tempfile::tempdir().unwrap();
        let roots = Roots {
            fixtures: temp.path().join("fixtures"),
            git_git: temp.path().join("git-git.git"),
        };
        let all = entries();
        let synthetic = all
            .iter()
            .find(|entry| entry.name == "synthetic-100k")
            .unwrap();
        let git_git = all.iter().find(|entry| entry.name == "git-git").unwrap();

        assert_eq!(
            resolve(synthetic, temp.path(), &roots),
            Availability::Missing {
                build_with: String::from("cargo test --features heavy-tests -- --test-threads=2")
            }
        );
        assert_eq!(
            resolve(git_git, temp.path(), &roots),
            Availability::Missing {
                build_with: String::from("scripts/clone-git-git.sh")
            }
        );
        let half_built = roots.fixtures.join("synthetic-100000-no-commit-graph-aaaa");
        fs::create_dir_all(&half_built).unwrap();
        assert!(
            matches!(
                resolve(synthetic, temp.path(), &roots),
                Availability::Missing { .. }
            ),
            "no corpus.ok marker yet"
        );
        fs::write(half_built.join("corpus.ok"), "").unwrap();
        assert_eq!(
            resolve(synthetic, temp.path(), &roots),
            Availability::Ready(half_built)
        );

        fs::create_dir_all(&roots.git_git).unwrap();
        fs::write(roots.git_git.join("HEAD"), "ref: refs/heads/master\n").unwrap();
        assert_eq!(
            resolve(git_git, temp.path(), &roots),
            Availability::Ready(roots.git_git.clone())
        );
    }

    #[test]
    fn a_corpus_shape_and_this_checkout_are_always_ready() {
        let workspace = workspace();
        let roots = roots(&workspace);
        let all = entries();
        let trivial = all.iter().find(|entry| entry.name == "trivial").unwrap();
        assert!(matches!(
            resolve(trivial, &workspace, &roots),
            Availability::Ready(_)
        ));
        let this = all.iter().find(|entry| entry.name == "graft-self").unwrap();
        assert_eq!(
            resolve(this, &workspace, &roots),
            Availability::Ready(workspace.clone())
        );
    }

    #[test]
    fn the_matrix_marks_only_what_exists_by_name() {
        let temp = tempfile::tempdir().unwrap();
        let snaps = temp.path().join("crates/graft-ui/tests/snapshots");
        fs::create_dir_all(&snaps).unwrap();
        fs::write(snaps.join("graph_screen__feature-branches-80x24.snap"), "").unwrap();
        fs::write(
            snaps.join("ascii_tier__ascii-foxtrot-merge-no-color.snap"),
            "",
        )
        .unwrap();
        let layout = temp.path().join("crates/graft-graph/tests/it/snapshots");
        fs::create_dir_all(&layout).unwrap();
        fs::write(layout.join("it__corpus__linear.snap"), "").unwrap();
        let shots = temp.path().join("docs/shots");
        fs::create_dir_all(&shots).unwrap();
        fs::write(shots.join("19-ascii-tier-crisscross.txt"), "").unwrap();
        fs::write(shots.join("14-git-git-120x40.txt"), "").unwrap();
        fs::write(shots.join("22-raster-octopus-merge-on-dark.png"), "").unwrap();

        let matrix = gallery_matrix(temp.path());
        let row = |name: &str| {
            matrix
                .lines()
                .find(|line| line.starts_with(name))
                .unwrap()
                .to_string()
        };
        let cells = |name: &str| -> Vec<String> {
            row(name)
                .split("  ")
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .skip(1)
                .map(String::from)
                .collect()
        };
        // Columns: pixel/dark pixel/light unicode/dark unicode/light ascii/dark ascii/light layout/dark.
        assert_eq!(
            cells("feature-branches"),
            ["—", "—", "S:80x24", "—", "—", "—", "—"]
        );
        assert_eq!(
            cells("foxtrot-merge"),
            ["—", "—", "—", "—", "S:?", "—", "—"]
        );
        assert_eq!(cells("linear"), ["—", "—", "—", "—", "—", "—", "S:?"]);
        assert_eq!(
            cells("criss-cross-merge"),
            ["—", "—", "—", "—", "C:?", "—", "—"]
        );
        assert_eq!(cells("git-git"), ["—", "—", "C:120x40", "—", "—", "—", "—"]);
        assert_eq!(
            cells("octopus-merge"),
            ["C:?", "—", "—", "—", "—", "—", "—"]
        );
        assert_eq!(cells("trivial"), ["—", "—", "—", "—", "—", "—", "—"]);
    }
}
