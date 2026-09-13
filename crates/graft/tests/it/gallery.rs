//! The graph gallery's repository cycle and coverage matrix (ticket
//! m1-the-graph/35): pure decisions about *which* repositories the gallery
//! walks, what each is expected to show, where a heavy one lives on disk,
//! and what the test suite and `docs/shots/` already cover. The terminal
//! driving lives in `examples/gallery.rs`, which includes this file.

use std::fs;
use std::path::{Path, PathBuf};

use graft_fixtures::Shape;

/// One stop of the gallery.
pub struct Entry {
    /// What the footer calls it; also the stem of its `docs/shots/35-` files.
    pub name: &'static str,
    /// Where the repository comes from.
    pub source: Source,
    /// What right looks like, in one line (TESTING.md "Graph gallery" has
    /// the long form). Never "looks correct".
    pub expect: &'static str,
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

/// The gallery's stops, in walking order: every `Shape::ALL` fixture, then
/// this checkout, then the two heavy repositories.
pub fn entries() -> Vec<Entry> {
    let mut all: Vec<Entry> = Shape::ALL
        .iter()
        .map(|&shape| Entry {
            name: shape.name(),
            source: Source::Shape(shape),
            expect: expectation(shape),
            stems: stems(shape),
        })
        .collect();
    all.push(Entry {
        name: "graft-self",
        source: Source::Workspace,
        expect: "one straight lane, every node the plain shape, no curve anywhere",
        stems: &["gitter"],
    });
    all.push(Entry {
        name: "synthetic-100k",
        source: Source::Synthetic100k,
        expect: "busy but legible: lanes fold behind +N, first paint at once, j/G never stall",
        stems: &["100k", "synthetic-100000"],
    });
    all.push(Entry {
        name: "git-git",
        source: Source::GitGit,
        expect: "real merge-heavy history: master straight in lane 0, merges curve in, tags on releases",
        stems: &["git-git"],
    });
    all
}

/// The one-line finding per corpus shape. The subjects and branch names are
/// the fixture's (ticket 01), so the line can be checked, not felt.
fn expectation(shape: Shape) -> &'static str {
    match shape {
        Shape::Trivial => "3 commits in one straight lane; plain nodes; no tag, no curve",
        Shape::Linear => "200 commits, one straight lane the whole way; scrolling never shifts it",
        Shape::FeatureBranches => {
            "two merge nodes on main; feature/one and /two curve in, feature/three stays open"
        }
        Shape::OctopusMerge => {
            "one merge node with three curves in (topic-1..3), each in its own colour"
        }
        Shape::CrissCrossMerge => {
            "alpha and beta cross-merge each other (two curves crossing), then main merges alpha"
        }
        Shape::FoxtrotMerge => {
            "main's first-parent line jumps lanes at the merge: the merge node sits on feature's line"
        }
        Shape::BackwardMerge => {
            "the merge node sits on feature, main curves into it; main's own lane stays straight"
        }
        Shape::EvilMerge => {
            "an ordinary-looking merge of alpha into main; nothing in the shape marks it"
        }
        Shape::AccidentalPullMerge => {
            "a merge of origin/main into main: the 'Merge branch main of …' subject on a merge node"
        }
        Shape::OrphanBranch => {
            "docs starts from nothing: its lane has a root node with no line below it"
        }
        Shape::CherryPickDuplicates => {
            "'Fix the greeting' appears twice, on main and on topic, with no line joining them"
        }
        Shape::SkewedDates => {
            "topological order: 'Commit from a fast clock' sits below Main 01, not above it"
        }
        Shape::Tags => {
            "four commits, tag badges v0.1 v0.2 v1.0 v2.0 one per row, annotated ones distinct"
        }
        Shape::UnicodeNames => {
            "branches fonctionnalité 機能 ветка and the ☕ subject align; no cell out of place"
        }
        Shape::RefStates => {
            "main ⇄ origin merged badge; ahead ↑1; behind ↓1; 'archived' tag alone on its commit"
        }
        Shape::DetachedHead => "HEAD badge on Commit 2, two rows below main's tip",
        Shape::UnbornBranch => {
            "no HEAD row at the top (branch fresh has no commit); main and other still draw"
        }
        Shape::Stash => "stash badge above main's tip, and H hides that row",
        Shape::NonUtf8Path => {
            "two lanes, main and latin1; the Latin-1 path never reaches the screen"
        }
        Shape::NonUtf8RefName => {
            "a branch badge for caf\u{fffd}: the bad byte replaced, nothing else disturbed"
        }
        // `Shape` is non-exhaustive. A new shape lands here, and the test
        // that every finding is concrete fails until one is written.
        _ => "no finding written yet: add one to tests/it/gallery.rs",
    }
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

/// The footer line while an entry is on screen.
pub fn footer(entry: &Entry, index: usize, total: usize) -> String {
    format!(
        "{} · {}/{} · expect: {}",
        entry.name,
        index + 1,
        total,
        entry.expect
    )
}

/// The page shown instead of a repository that is not on disk.
pub fn missing_line(entry: &Entry, index: usize, total: usize, build_with: &str) -> String {
    format!(
        "{} · {}/{} · not available — build with: {}",
        entry.name,
        index + 1,
        total,
        build_with
    )
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

    fn workspace() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn the_footer_names_the_repo_its_position_and_the_finding() {
        let all = entries();
        let foxtrot = all
            .iter()
            .position(|entry| entry.name == "foxtrot-merge")
            .unwrap();
        assert_eq!(
            footer(&all[foxtrot], foxtrot, all.len()),
            format!(
                "foxtrot-merge · {}/{} · expect: main's first-parent line jumps lanes at the merge: the merge node sits on feature's line",
                foxtrot + 1,
                all.len()
            )
        );
    }

    #[test]
    fn every_shape_is_a_stop_and_every_finding_is_concrete() {
        let all = entries();
        let names: Vec<&str> = all.iter().map(|entry| entry.name).collect();
        for shape in Shape::ALL {
            assert!(names.contains(&shape.name()), "{} missing", shape.name());
        }
        assert_eq!(
            &names[names.len() - 3..],
            &["graft-self", "synthetic-100k", "git-git"]
        );
        for entry in &all {
            let lower = entry.expect.to_ascii_lowercase();
            assert!(
                !lower.contains("looks correct") && !lower.contains("renders fine"),
                "{}",
                entry.name
            );
            assert!(!lower.contains("no finding written"), "{}", entry.name);
            assert!(entry.expect.len() > 20, "{}: too vague", entry.name);
        }
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
        assert_eq!(
            missing_line(git_git, 22, 23, "scripts/clone-git-git.sh"),
            "git-git · 23/23 · not available — build with: scripts/clone-git-git.sh"
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
