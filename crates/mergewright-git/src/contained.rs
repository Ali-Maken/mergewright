//! "Contained in": which branches, remote branches and tags reach a commit.
//!
//! One `git for-each-ref --contains` per question (design note m2, choice 3):
//! `gix` has no cheap reachability query, and building an index over the
//! whole history is a ticket of its own (ADR-0011 names this gap).

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// The most refs kept per class; the rest are only counted.
const PER_CLASS: usize = 200;

/// The refs a commit is reachable from, short names, in git's ref order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contained {
    /// Local branches, without `refs/heads/`: `main`.
    pub branches: Vec<Vec<u8>>,
    /// Remote-tracking branches, without `refs/remotes/`: `origin/main`.
    pub remotes: Vec<Vec<u8>>,
    /// Tags, without `refs/tags/`: `v1.0`.
    pub tags: Vec<Vec<u8>>,
    /// Refs past the per-class cap of 200, across all three classes.
    pub more: usize,
}

/// Lists the branches, remote branches and tags that contain `commit`.
///
/// Symbolic refs (`refs/remotes/origin/HEAD`) are left out: they are another
/// name for a ref already listed. Refs outside the three classes (notes, the
/// stash) are not asked for.
///
/// # Errors
/// [`RunError`] from the runner: git missing, stopped, or refusing — an id
/// that names no commit in this repository is git's `Exited` error.
pub fn contained_in(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Contained, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--contains"),
            GitArg::Oid(commit),
            GitArg::Flag("--format=%(refname)%00%(symref)%00"),
            GitArg::Ref(b"refs/heads".to_vec()),
            GitArg::Ref(b"refs/remotes".to_vec()),
            GitArg::Ref(b"refs/tags".to_vec()),
        ])
        .run(stop)?;
    Ok(classify(&output.stdout))
}

/// Which refs [`refs_where`] lists: those under `patterns` passing every
/// filter set. Each filter is one of git's `for-each-ref` options, with its
/// meaning.
#[derive(Clone, Copy, Debug, Default)]
pub struct RefFilter<'a> {
    /// Where to look: `refs/heads`, `refs/remotes`. Empty: every ref.
    pub patterns: &'a [&'a str],
    /// `--merged <oid>`: the tip is reachable from this commit.
    pub merged_into: Option<Oid>,
    /// `--no-merged <oid>`: the tip is not reachable from this commit.
    pub not_merged_into: Option<Oid>,
    /// `--contains <oid>`: the tip reaches this commit.
    pub contains: Option<Oid>,
}

/// One ref [`refs_where`] listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefTip {
    /// Full name, as git stores it: `refs/heads/main`.
    pub name: RefName,
    /// The commit it points at.
    pub tip: Oid,
}

/// The refs passing `filter`, with their tips, in git's ref order: one
/// `for-each-ref` spawn however many refs there are, uncapped. A symbolic
/// ref (`refs/remotes/origin/HEAD`) is listed as git lists it, under its own
/// name with its target's tip; callers keep their own set.
///
/// # Errors
/// [`RunError`] from the runner: git missing, stopped, or refusing (an id
/// naming no commit is git's `Exited`); [`RunError::Unparsable`] for a line
/// that is not `<oid> <name>`.
pub fn refs_where(
    repo: &RepoHandle,
    filter: &RefFilter<'_>,
    stop: &dyn Stop,
) -> Result<Vec<RefTip>, RunError> {
    let mut args = vec![
        GitArg::Flag("for-each-ref"),
        GitArg::Flag("--format=%(objectname) %(refname)"),
    ];
    let options = [
        ("--merged", filter.merged_into),
        ("--no-merged", filter.not_merged_into),
        ("--contains", filter.contains),
    ];
    for (flag, oid) in options {
        if let Some(oid) = oid {
            args.extend([GitArg::Flag(flag), GitArg::Oid(oid)]);
        }
    }
    args.extend(
        filter
            .patterns
            .iter()
            .map(|pattern| GitArg::Ref(pattern.as_bytes().to_vec())),
    );
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    parse_ref_tips(&output.stdout)
}

/// `<oid> <name> LF` records. A ref name holds no space or newline (git's
/// `check-ref-format`), so the first space ends the id.
fn parse_ref_tips(stdout: &[u8]) -> Result<Vec<RefTip>, RunError> {
    stdout
        .split(|&b| b == b'\n')
        .filter(|record| !record.is_empty())
        .map(|record| {
            let unparsable = || {
                RunError::Unparsable(format!(
                    "for-each-ref printed {:?}, not `<oid> <refname>`",
                    String::from_utf8_lossy(record)
                ))
            };
            let space = record
                .iter()
                .position(|&b| b == b' ')
                .ok_or_else(unparsable)?;
            let (hex, name) = (&record[..space], &record[space + 1..]);
            let tip = std::str::from_utf8(hex)
                .ok()
                .and_then(|hex| Oid::from_hex(hex).ok())
                .ok_or_else(unparsable)?;
            let name = RefName::new(name.to_vec()).map_err(|_| unparsable())?;
            Ok(RefTip { name, tip })
        })
        .collect()
}

/// Of `tips`, those no other of them reaches: one `merge-base
/// --independent` spawn. Duplicates come back once; order is git's. Empty
/// in, empty out, with no spawn.
///
/// # Errors
/// [`RunError`] from the runner (an id naming no commit is git's
/// `Exited`); [`RunError::Unparsable`] for a line that is not an id.
pub fn independent_tips(
    repo: &RepoHandle,
    tips: &[Oid],
    stop: &dyn Stop,
) -> Result<Vec<Oid>, RunError> {
    if tips.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec![GitArg::Flag("merge-base"), GitArg::Flag("--independent")];
    args.extend(tips.iter().map(|&tip| GitArg::Oid(tip)));
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            Oid::from_hex(line).map_err(|why| {
                RunError::Unparsable(format!("merge-base printed {line:?}, not an id: {why}"))
            })
        })
        .collect()
}

/// Sorts `for-each-ref` records (`name NUL symref NUL LF`) into classes.
fn classify(stdout: &[u8]) -> Contained {
    let mut contained = Contained::default();
    for record in stdout.split(|&b| b == b'\n') {
        let mut fields = record.split(|&b| b == 0);
        let (Some(name), Some(symref)) = (fields.next(), fields.next()) else {
            continue; // the empty tail after the last newline
        };
        if !symref.is_empty() {
            continue;
        }
        let (class, short) = if let Some(short) = name.strip_prefix(b"refs/heads/") {
            (&mut contained.branches, short)
        } else if let Some(short) = name.strip_prefix(b"refs/remotes/") {
            (&mut contained.remotes, short)
        } else if let Some(short) = name.strip_prefix(b"refs/tags/") {
            (&mut contained.tags, short)
        } else {
            continue; // only the three prefixes were asked for
        };
        if class.len() < PER_CLASS {
            class.push(short.to_vec());
        } else {
            contained.more += 1;
        }
    }
    contained
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    const BRANCH: &str = "refs/heads/feature/ünï";

    /// ```text
    /// m1 ─ m2           main
    ///  │└ s1            side
    ///  feature/ünï at m1
    /// ```
    fn forked() -> (Fixture, RepoHandle, [Oid; 3]) {
        let fixture = Fixture::empty_repo();
        fixture.commit("m1");
        fixture.commit("m2");
        fixture.git(&["branch", "side", "HEAD~1"]);
        fixture.git(&["branch", "feature/ünï", "HEAD~1"]);
        fixture.git(&["switch", "-q", "side"]);
        fixture.commit("s1");
        let oid = |rev: &str| Oid::from_hex(&fixture.rev_parse(rev)).expect("an id");
        let ids = [oid("main~1"), oid("main"), oid("side")];
        let repo = RepoHandle::open(fixture.root()).expect("it opens");
        (fixture, repo, ids)
    }

    fn names(filter: &RefFilter<'_>, repo: &RepoHandle) -> Vec<String> {
        refs_where(repo, filter, &())
            .expect("for-each-ref runs")
            .into_iter()
            .map(|r| String::from_utf8(r.name.as_bytes().to_vec()).expect("utf-8"))
            .collect()
    }

    const HEADS: &[&str] = &["refs/heads"];

    #[test]
    fn refs_where_merged_alone() {
        let (_f, repo, [_, m2, s1]) = forked();
        let filter = RefFilter {
            patterns: HEADS,
            merged_into: Some(m2),
            ..RefFilter::default()
        };
        assert_eq!(names(&filter, &repo), [BRANCH, "refs/heads/main"]);
        let filter = RefFilter {
            merged_into: Some(s1),
            ..filter
        };
        assert_eq!(names(&filter, &repo), [BRANCH, "refs/heads/side"]);
    }

    #[test]
    fn refs_where_not_merged_alone() {
        let (_f, repo, [_, m2, _]) = forked();
        let filter = RefFilter {
            patterns: HEADS,
            not_merged_into: Some(m2),
            ..RefFilter::default()
        };
        assert_eq!(names(&filter, &repo), ["refs/heads/side"]);
    }

    /// Both together: reachable from `side`, not from `main`. Git 2.29 and
    /// later combine them this way; the system git's version is in the
    /// ticket's NOTES (ease-pass 09p).
    #[test]
    fn refs_where_merged_and_not_merged_together() {
        let (_f, repo, [_, m2, s1]) = forked();
        let filter = RefFilter {
            patterns: HEADS,
            merged_into: Some(s1),
            not_merged_into: Some(m2),
            ..RefFilter::default()
        };
        assert_eq!(names(&filter, &repo), ["refs/heads/side"]);
    }

    #[test]
    fn refs_where_contains() {
        let (_f, repo, [m1, _, s1]) = forked();
        let filter = RefFilter {
            patterns: HEADS,
            contains: Some(m1),
            ..RefFilter::default()
        };
        assert_eq!(
            names(&filter, &repo),
            [BRANCH, "refs/heads/main", "refs/heads/side"]
        );
        let filter = RefFilter {
            contains: Some(s1),
            ..filter
        };
        assert_eq!(names(&filter, &repo), ["refs/heads/side"]);
    }

    /// A name with `/` and non-ASCII comes back byte for byte, with its tip.
    #[test]
    fn refs_where_reads_names_and_tips_back_exactly() {
        let (_f, repo, [m1, m2, s1]) = forked();
        let listed = refs_where(
            &repo,
            &RefFilter {
                patterns: HEADS,
                ..RefFilter::default()
            },
            &(),
        )
        .expect("for-each-ref runs");
        let expected = [
            (BRANCH, m1),
            ("refs/heads/main", m2),
            ("refs/heads/side", s1),
        ]
        .map(|(name, tip)| RefTip {
            name: RefName::new(name.as_bytes().to_vec()).expect("a ref name"),
            tip,
        });
        assert_eq!(listed, expected);
    }

    #[test]
    fn independent_tips_drop_what_another_reaches() {
        let (_f, repo, [m1, m2, s1]) = forked();
        let mut kept = independent_tips(&repo, &[m1, m2, s1, m2], &()).expect("merge-base runs");
        kept.sort_unstable();
        let mut expected = vec![m2, s1];
        expected.sort_unstable();
        assert_eq!(kept, expected, "m1 is under both; m2 named twice");
        assert_eq!(independent_tips(&repo, &[], &()).expect("no spawn"), []);
    }

    #[test]
    fn ref_tips_refuse_a_line_without_an_id() {
        assert!(matches!(
            parse_ref_tips(b"refs/heads/main\n"),
            Err(RunError::Unparsable(_))
        ));
        assert_eq!(parse_ref_tips(b"").expect("empty is fine"), []);
    }

    #[test]
    fn records_split_by_class_symrefs_dropped_and_capped() {
        let mut stdout = b"refs/heads/main\0\0\nrefs/remotes/origin/HEAD\0refs/remotes/origin/main\0\nrefs/remotes/origin/main\0\0\nrefs/tags/v1\0\0\n".to_vec();
        for n in 0..(PER_CLASS + 3) {
            stdout.extend_from_slice(format!("refs/tags/t{n}\0\0\n").as_bytes());
        }
        let contained = classify(&stdout);
        assert_eq!(contained.branches, vec![b"main".to_vec()]);
        assert_eq!(contained.remotes, vec![b"origin/main".to_vec()]);
        assert_eq!(contained.tags.len(), PER_CLASS);
        assert_eq!(contained.tags[0], b"v1".to_vec());
        assert_eq!(contained.more, 4);
    }
}
