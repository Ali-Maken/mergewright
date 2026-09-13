//! The graph gallery's repository cycle and coverage matrix (ticket
//! m1-the-graph/35): pure decisions about *which* repositories the gallery
//! walks, what each is expected to show, where a heavy one lives on disk,
//! and what the test suite and `docs/shots/` already cover. The terminal
//! driving lives in `examples/gallery.rs`, which includes this file.

use std::fs;
use std::path::{Path, PathBuf};

use graft::update::Banner;
use graft_fixtures::Shape;

/// One stop of the gallery.
pub struct Entry {
    /// What the banner calls it; also the stem of its `docs/shots/35-` files.
    pub name: &'static str,
    /// The run of stops it belongs to; neighbours share one, so the walk
    /// reads as a progression (one lane, merges, quirks, refs and badges,
    /// text, real repos).
    pub group: &'static str,
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
const CORPUS: [(Shape, &str, &str, &str); 20] = [
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
        "an ordinary merge of alpha into main: ◆ in lane 0, one curve in",
        "the control: evil is a tree fact (M2), so it must look like stop 3",
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
            source: Source::Shape(shape),
            shows,
            differs,
            stems: stems(shape),
        })
        .collect();
    all.push(Entry {
        name: "graft-self",
        group: REAL,
        source: Source::Workspace,
        shows: "this checkout: one lane, ● everywhere but ◆ at HEAD and ◇ at the bottom",
        differs: "a real repo with no merges: stop 2's shape, real subjects and dates",
        stems: &["gitter"],
    });
    all.push(Entry {
        name: "synthetic-100k",
        group: REAL,
        source: Source::Synthetic100k,
        shows: "100k commits: paints at once; lanes fold behind +N; j and G never stall",
        differs: "size is the test, the shape is synthetic; needs the heavy tier built",
        stems: &["100k", "synthetic-100000"],
    });
    all.push(Entry {
        name: "git-git",
        group: REAL,
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

/// The banner's title: where the user is and how to move on. The run
/// appends ` · <tier>` (`view::banner_lines`); [`TIER_SUFFIX`] reserves it.
pub fn title(entry: &Entry, index: usize, total: usize) -> String {
    format!(
        "gallery {}/{} {} · {} · {KEYS}",
        index + 1,
        total,
        entry.name,
        entry.group
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
const SIZES: [&str; 3] = ["80x24", "120x40", "200x60"];

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

/// The repo × tier × theme coverage matrix, with the sizes each cell has,
/// from the files that exist today. `S` a snapshot, `C` a pty capture, `—`
/// nothing; sizes follow, `?` when the file name does not say.
pub fn coverage_matrix(workspace: &Path) -> String {
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

        let matrix = coverage_matrix(temp.path());
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
