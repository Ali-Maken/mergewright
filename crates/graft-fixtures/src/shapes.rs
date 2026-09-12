//! How each named shape is built.
//!
//! One function per [`Shape`]. Each one writes a `fast-import` stream and, if
//! the shape needs something `fast-import` cannot express — a detached
//! `HEAD`, a stash, an upstream — follows it with a few plain `git` commands.
//! The split is deliberate: topology comes from the stream, because that is
//! what makes object ids reproducible; everything else is state *around* the
//! objects, where `git` itself is the only honest way to produce what git
//! would produce.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::git::{self, GitError};
use crate::shape::Shape;
use crate::stream::{FileContent, Mark, Stream};

/// The default branch of every fixture.
const MAIN: &[u8] = b"refs/heads/main";

/// The remote every fixture with remote-tracking refs pretends to have.
/// Unreachable by design: `.invalid` can never resolve (RFC 2606), so a bug
/// that makes Graft touch the network fails loudly instead of quietly
/// working.
const REMOTE_URL: &str = "https://example.invalid/graft.git";

/// Why one fixture could not be built.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// A `git` command failed.
    #[error("building the {shape} fixture: {source}")]
    Git {
        /// The shape being built.
        shape: Shape,
        /// What `git` did.
        source: GitError,
    },
    /// A file could not be written or a directory created.
    #[error("building the {shape} fixture: cannot write {}: {source}", path.display())]
    Io {
        /// The shape being built.
        shape: Shape,
        /// The path that could not be written.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
}

/// One fixture's repository, mid-build.
pub struct Site<'a> {
    shape: Shape,
    /// The repository's working-tree root.
    dir: &'a Path,
    /// Where `fast-import` streams are parked. Outside `dir`, so no stream
    /// ends up inside the fixture it built.
    streams: &'a Path,
}

impl<'a> Site<'a> {
    /// Creates the directory and runs `git init` in it.
    pub fn init(shape: Shape, dir: &'a Path, streams: &'a Path) -> Result<Self, BuildError> {
        let site = Site {
            shape,
            dir,
            streams,
        };
        site.mkdir(dir)?;
        // `--object-format=sha1` is stated rather than inherited: the corpus
        // records object ids, and a machine configured for sha256 by default
        // would silently produce a different corpus.
        site.git(&["init", "-q", "-b", "main", "--object-format=sha1", "."])?;
        Ok(site)
    }

    /// Runs one `git` in the fixture and returns its stdout.
    pub fn git(&self, args: &[&str]) -> Result<Vec<u8>, BuildError> {
        git::run(self.dir, args).map_err(|source| BuildError::Git {
            shape: self.shape,
            source,
        })
    }

    /// Feeds a finished stream to `git fast-import`.
    pub fn import(&self, stream: Stream) -> Result<(), BuildError> {
        let path = self.streams.join(format!("{}.fi", self.shape.name()));
        self.write_file(&path, &stream.finish())?;
        // `--done` makes a truncated stream an error instead of a partial
        // corpus that looks fine until a test disagrees with it.
        git::run_with_stdin_file(self.dir, &["fast-import", "--quiet", "--done"], &path).map_err(
            |source| BuildError::Git {
                shape: self.shape,
                source,
            },
        )?;
        Ok(())
    }

    /// Puts the working tree and index on the default branch.
    pub fn checkout_main(&self) -> Result<(), BuildError> {
        self.git(&["reset", "-q", "--hard", "refs/heads/main"])?;
        Ok(())
    }

    /// Writes a file in the working tree.
    pub fn write(&self, relative: &str, content: &str) -> Result<(), BuildError> {
        let path = self.dir.join(relative);
        self.write_file(&path, content.as_bytes())
    }

    fn mkdir(&self, path: &Path) -> Result<(), BuildError> {
        fs::create_dir_all(path).map_err(|source| BuildError::Io {
            shape: self.shape,
            path: path.to_path_buf(),
            source,
        })
    }

    fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), BuildError> {
        fs::write(path, content).map_err(|source| BuildError::Io {
            shape: self.shape,
            path: path.to_path_buf(),
            source,
        })
    }
}

/// One file in a commit.
///
/// Spelled out as a call rather than a bare tuple so that each byte-string
/// literal coerces to a slice on its own; written as a tuple, a list of files
/// with different name lengths would try to unify as fixed-size arrays.
fn file<'a>(path: &'a [u8], content: &'a [u8]) -> FileContent<'a> {
    (path, content)
}

/// Builds one fixture into `dir`, which must not exist yet.
pub fn build(shape: Shape, dir: &Path, streams: &Path) -> Result<(), BuildError> {
    let site = Site::init(shape, dir, streams)?;
    match shape {
        Shape::Trivial => trivial(&site),
        Shape::Linear => linear(&site),
        Shape::FeatureBranches => feature_branches(&site),
        Shape::OctopusMerge => octopus_merge(&site),
        Shape::CrissCrossMerge => criss_cross_merge(&site),
        Shape::FoxtrotMerge => foxtrot_merge(&site),
        Shape::BackwardMerge => backward_merge(&site),
        Shape::EvilMerge => evil_merge(&site),
        Shape::AccidentalPullMerge => accidental_pull_merge(&site),
        Shape::OrphanBranch => orphan_branch(&site),
        Shape::CherryPickDuplicates => cherry_pick_duplicates(&site),
        Shape::Tags => tags(&site),
        Shape::UnicodeNames => unicode_names(&site),
        Shape::RefStates => ref_states(&site),
        Shape::DetachedHead => detached_head(&site),
        Shape::UnbornBranch => unborn_branch(&site),
        Shape::Stash => stash(&site),
        Shape::NonUtf8Names => non_utf8_names(&site),
    }
}

fn trivial(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let c1 = s.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"A trivial repo.\n")],
    );
    let c2 = s.commit(
        MAIN,
        b"Add the first note\n",
        &[c1],
        &[file(b"notes.txt", b"one\n")],
    );
    s.commit(
        MAIN,
        b"Correct the note\n",
        &[c2],
        &[file(b"notes.txt", b"one, corrected\n")],
    );
    site.import(s)?;
    site.checkout_main()
}

fn linear(site: &Site<'_>) -> Result<(), BuildError> {
    const COMMITS: u32 = 200;
    let mut s = Stream::new();
    let mut tip: Option<Mark> = None;
    for n in 1..=COMMITS {
        let message = format!("Commit {n:03}\n");
        let content = format!("line {n}\n");
        let parents: Vec<Mark> = tip.into_iter().collect();
        tip = Some(s.commit(
            MAIN,
            message.as_bytes(),
            &parents,
            &[file(b"log.txt", content.as_bytes())],
        ));
    }
    site.import(s)?;
    site.checkout_main()
}

fn feature_branches(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Features.\n")],
    );
    let m2 = s.commit(MAIN, b"Add a.txt\n", &[m1], &[file(b"a.txt", b"1\n")]);
    let m3 = s.commit(MAIN, b"Add b.txt\n", &[m2], &[file(b"b.txt", b"1\n")]);

    let one_a = s.commit(
        b"refs/heads/feature/one",
        b"one: start\n",
        &[m2],
        &[file(b"one.txt", b"a\n")],
    );
    let one_b = s.commit(
        b"refs/heads/feature/one",
        b"one: finish\n",
        &[one_a],
        &[file(b"one.txt", b"b\n")],
    );
    let merged_one = s.commit(
        MAIN,
        b"Merge branch 'feature/one'\n",
        &[m3, one_b],
        &[file(b"one.txt", b"b\n")],
    );

    let two_a = s.commit(
        b"refs/heads/feature/two",
        b"two: start\n",
        &[m3],
        &[file(b"two.txt", b"a\n")],
    );
    let two_b = s.commit(
        b"refs/heads/feature/two",
        b"two: finish\n",
        &[two_a],
        &[file(b"two.txt", b"b\n")],
    );
    s.commit(
        MAIN,
        b"Merge branch 'feature/two'\n",
        &[merged_one, two_b],
        &[file(b"two.txt", b"b\n")],
    );

    // Left open on purpose: a graph with no unmerged branch in it never
    // exercises the lane that has nowhere to land.
    s.commit(
        b"refs/heads/feature/three",
        b"three: in progress\n",
        &[m3],
        &[file(b"three.txt", b"a\n")],
    );

    site.import(s)?;
    site.checkout_main()
}

fn octopus_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let base = s.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let t1 = s.commit(
        b"refs/heads/topic-1",
        b"topic 1\n",
        &[base],
        &[file(b"t1.txt", b"1\n")],
    );
    let t2 = s.commit(
        b"refs/heads/topic-2",
        b"topic 2\n",
        &[base],
        &[file(b"t2.txt", b"2\n")],
    );
    let t3 = s.commit(
        b"refs/heads/topic-3",
        b"topic 3\n",
        &[base],
        &[file(b"t3.txt", b"3\n")],
    );
    s.commit(
        MAIN,
        b"Merge branches 'topic-1', 'topic-2' and 'topic-3'\n",
        &[base, t1, t2, t3],
        &[
            file(b"t1.txt", b"1\n"),
            file(b"t2.txt", b"2\n"),
            file(b"t3.txt", b"3\n"),
        ],
    );
    site.import(s)?;
    site.checkout_main()
}

fn criss_cross_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let base = s.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let a1 = s.commit(
        b"refs/heads/alpha",
        b"alpha: work\n",
        &[base],
        &[file(b"alpha.txt", b"a\n")],
    );
    let b1 = s.commit(
        b"refs/heads/beta",
        b"beta: work\n",
        &[base],
        &[file(b"beta.txt", b"b\n")],
    );
    // Each branch merges the other, so the two tips have two merge bases and
    // no single "the" merge base.
    let a2 = s.commit(
        b"refs/heads/alpha",
        b"alpha: merge beta\n",
        &[a1, b1],
        &[file(b"beta.txt", b"b\n")],
    );
    let b2 = s.commit(
        b"refs/heads/beta",
        b"beta: merge alpha\n",
        &[b1, a1],
        &[file(b"alpha.txt", b"a\n")],
    );
    s.commit(
        MAIN,
        b"Merge branch 'alpha'\n",
        &[base, a2],
        &[file(b"alpha.txt", b"a\n"), file(b"beta.txt", b"b\n")],
    );
    s.commit(
        b"refs/heads/beta",
        b"beta: carry on\n",
        &[b2],
        &[file(b"beta.txt", b"b2\n")],
    );
    site.import(s)?;
    site.checkout_main()
}

fn foxtrot_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let m2 = s.commit(MAIN, b"Advance main\n", &[m1], &[file(b"main.txt", b"m\n")]);
    let f1 = s.commit(
        b"refs/heads/feature",
        b"feature: work\n",
        &[m1],
        &[file(b"feature.txt", b"f\n")],
    );
    // The foxtrot: the merge lands on `main`, but its *first* parent is the
    // feature branch, so `--first-parent main` walks off the default branch.
    s.commit(
        MAIN,
        b"Merge branch 'main' into feature\n",
        &[f1, m2],
        &[file(b"main.txt", b"m\n")],
    );
    site.import(s)?;
    site.checkout_main()
}

fn backward_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let m2 = s.commit(MAIN, b"Advance main\n", &[m1], &[file(b"main.txt", b"m\n")]);
    let f1 = s.commit(
        b"refs/heads/feature",
        b"feature: work\n",
        &[m1],
        &[file(b"feature.txt", b"f\n")],
    );
    // The default branch merged *into* the feature branch, and never back.
    s.commit(
        b"refs/heads/feature",
        b"Merge branch 'main' into feature\n",
        &[f1, m2],
        &[file(b"main.txt", b"m\n")],
    );
    site.import(s)?;
    site.checkout_main()
}

fn evil_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let base = s.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let a1 = s.commit(
        b"refs/heads/alpha",
        b"alpha: work\n",
        &[base],
        &[file(b"alpha.txt", b"a\n")],
    );
    let m2 = s.commit(
        MAIN,
        b"Advance main\n",
        &[base],
        &[file(b"main.txt", b"m\n")],
    );
    // `evil.txt` is in neither parent: the merge's tree is not the mechanical
    // merge of the two sides.
    s.commit(
        MAIN,
        b"Merge branch 'alpha'\n",
        &[m2, a1],
        &[
            file(b"alpha.txt", b"a\n"),
            file(b"evil.txt", b"introduced by the merge alone\n"),
        ],
    );
    site.import(s)?;
    site.checkout_main()
}

fn accidental_pull_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let base = s.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let theirs = s.commit(
        b"refs/remotes/origin/main",
        b"Their commit\n",
        &[base],
        &[file(b"theirs.txt", b"t\n")],
    );
    let mine = s.commit(MAIN, b"My commit\n", &[base], &[file(b"mine.txt", b"m\n")]);
    // Exactly what `git pull` writes when a push was rejected.
    let message = format!("Merge branch 'main' of {REMOTE_URL} into main\n");
    s.commit(
        MAIN,
        message.as_bytes(),
        &[mine, theirs],
        &[file(b"theirs.txt", b"t\n")],
    );
    site.import(s)?;
    site.git(&["remote", "add", "origin", REMOTE_URL])?;
    site.checkout_main()
}

fn orphan_branch(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Code.\n")],
    );
    s.commit(MAIN, b"Add the code\n", &[m1], &[file(b"code.txt", b"1\n")]);
    // No parent at all: a second root commit.
    let d1 = s.commit(
        b"refs/heads/docs",
        b"docs: first page\n",
        &[],
        &[file(b"index.md", b"Docs.\n")],
    );
    s.commit(
        b"refs/heads/docs",
        b"docs: second page\n",
        &[d1],
        &[file(b"guide.md", b"Guide.\n")],
    );
    site.import(s)?;
    site.checkout_main()
}

fn cherry_pick_duplicates(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let base = s.commit(
        MAIN,
        b"Add the greeting\n",
        &[],
        &[file(b"shared.txt", b"hello\nworld\n")],
    );
    // Same parent tree, same resulting tree, so the same patch id — and two
    // different commits, on branches with no ancestry between them.
    s.commit(
        MAIN,
        b"Fix the greeting\n",
        &[base],
        &[file(b"shared.txt", b"hello\nthere\n")],
    );
    let picked = s.commit(
        b"refs/heads/topic",
        b"Fix the greeting (cherry picked)\n",
        &[base],
        &[file(b"shared.txt", b"hello\nthere\n")],
    );
    s.commit(
        b"refs/heads/topic",
        b"topic: unrelated work\n",
        &[picked],
        &[file(b"topic.txt", b"x\n")],
    );
    site.import(s)?;
    site.checkout_main()
}

fn tags(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Tagged.\n")],
    );
    let m2 = s.commit(MAIN, b"Add a.txt\n", &[m1], &[file(b"a.txt", b"1\n")]);
    let m3 = s.commit(MAIN, b"Add b.txt\n", &[m2], &[file(b"b.txt", b"1\n")]);
    let m4 = s.commit(MAIN, b"Add c.txt\n", &[m3], &[file(b"c.txt", b"1\n")]);
    s.point(b"refs/tags/v0.1", m1);
    s.point(b"refs/tags/v0.2", m2);
    s.annotated_tag(b"v1.0", m3, b"Release 1.0\n\nThe first tagged release.\n");
    s.annotated_tag(b"v2.0", m4, b"Release 2.0\n");
    site.import(s)?;
    site.checkout_main()
}

fn unicode_names(site: &Site<'_>) -> Result<(), BuildError> {
    const AUTHOR: &str = "Ünïcödé Fixture <unicode@example.invalid>";
    let mut s = Stream::new();
    let m1 = s.commit_by(
        AUTHOR,
        MAIN,
        "Ajoute le fichier café ☕\n".as_bytes(),
        &[],
        &[file("café/naïve.txt".as_bytes(), "contenu\n".as_bytes())],
    );
    let m2 = s.commit_by(
        AUTHOR,
        MAIN,
        "日本語のコミットメッセージ\n".as_bytes(),
        &[m1],
        &[file("日本語.txt".as_bytes(), "内容\n".as_bytes())],
    );
    s.commit_by(
        AUTHOR,
        "refs/heads/fonctionnalité".as_bytes(),
        "fonctionnalité : première étape 🚀\n".as_bytes(),
        &[m2],
        &[file("café/étape.txt".as_bytes(), "étape\n".as_bytes())],
    );
    s.commit_by(
        AUTHOR,
        "refs/heads/機能".as_bytes(),
        "機能: 最初のコミット\n".as_bytes(),
        &[m2],
        &[file("機能.txt".as_bytes(), "内容\n".as_bytes())],
    );
    s.commit_by(
        AUTHOR,
        "refs/heads/ветка".as_bytes(),
        "ветка: первый коммит\n".as_bytes(),
        &[m2],
        &[file("документ.txt".as_bytes(), "содержимое\n".as_bytes())],
    );
    s.annotated_tag("версия-1.0".as_bytes(), m2, "Релиз 1.0 🎉\n".as_bytes());
    site.import(s)?;
    site.checkout_main()
}

fn ref_states(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let c1 = s.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let c2 = s.commit(MAIN, b"Advance main\n", &[c1], &[file(b"main.txt", b"1\n")]);
    let c3 = s.commit(
        MAIN,
        b"Advance main again\n",
        &[c2],
        &[file(b"main.txt", b"2\n")],
    );
    // Level with its upstream.
    s.point(b"refs/remotes/origin/main", c3);

    // One commit ahead of its upstream.
    let ahead_1 = s.commit(
        b"refs/heads/ahead",
        b"ahead: pushed\n",
        &[c2],
        &[file(b"ahead.txt", b"1\n")],
    );
    s.commit(
        b"refs/heads/ahead",
        b"ahead: not pushed yet\n",
        &[ahead_1],
        &[file(b"ahead.txt", b"2\n")],
    );
    s.point(b"refs/remotes/origin/ahead", ahead_1);

    // One commit behind its upstream.
    let behind_1 = s.commit(
        b"refs/heads/behind",
        b"behind: shared\n",
        &[c2],
        &[file(b"behind.txt", b"1\n")],
    );
    s.commit(
        b"refs/remotes/origin/behind",
        b"behind: theirs\n",
        &[behind_1],
        &[file(b"behind.txt", b"2\n")],
    );

    // A commit no branch reaches: the temporary branch below is deleted, and
    // only the tag is left pointing at it.
    let archived = s.commit(
        b"refs/heads/to-archive",
        b"Archived work\n",
        &[c1],
        &[file(b"archived.txt", b"a\n")],
    );
    s.point(b"refs/tags/archived", archived);

    site.import(s)?;
    site.git(&["update-ref", "-d", "refs/heads/to-archive"])?;
    site.git(&["remote", "add", "origin", REMOTE_URL])?;
    for branch in ["main", "ahead", "behind"] {
        site.git(&["config", &format!("branch.{branch}.remote"), "origin"])?;
        site.git(&[
            "config",
            &format!("branch.{branch}.merge"),
            &format!("refs/heads/{branch}"),
        ])?;
    }
    site.checkout_main()
}

fn detached_head(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let mut tip = s.commit(MAIN, b"Commit 1\n", &[], &[file(b"log.txt", b"1\n")]);
    for n in 2..=4u32 {
        let message = format!("Commit {n}\n");
        let content = format!("{n}\n");
        tip = s.commit(
            MAIN,
            message.as_bytes(),
            &[tip],
            &[file(b"log.txt", content.as_bytes())],
        );
    }
    site.import(s)?;
    site.checkout_main()?;
    site.git(&["checkout", "-q", "--detach", "refs/heads/main~2"])?;
    Ok(())
}

fn unborn_branch(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Unborn.\n")],
    );
    s.commit(MAIN, b"Add a.txt\n", &[m1], &[file(b"a.txt", b"1\n")]);
    s.commit(
        b"refs/heads/other",
        b"other: work\n",
        &[m1],
        &[file(b"other.txt", b"o\n")],
    );
    site.import(s)?;
    site.checkout_main()?;
    // `HEAD` now names a branch that has no commit — the state a fresh
    // `git init` is in, but with other branches already present.
    site.git(&["checkout", "-q", "--orphan", "fresh"])?;
    site.git(&["rm", "-r", "-f", "-q", "."])?;
    Ok(())
}

fn stash(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Stashes.\n")],
    );
    s.commit(
        MAIN,
        b"Add a.txt\n",
        &[m1],
        &[file(b"a.txt", b"committed\n")],
    );
    site.import(s)?;
    site.checkout_main()?;

    site.write("a.txt", "work in progress\n")?;
    site.git(&["stash", "push", "-q", "-m", "wip: adjust a.txt"])?;

    site.write("a.txt", "second attempt\n")?;
    site.write("untracked.txt", "not added yet\n")?;
    site.git(&[
        "stash",
        "push",
        "-q",
        "-u",
        "-m",
        "wip: with an untracked file",
    ])?;
    Ok(())
}

fn non_utf8_names(site: &Site<'_>) -> Result<(), BuildError> {
    let mut s = Stream::new();
    let m1 = s.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Latin-1.\n")],
    );
    // `\xe9` is `é` in Latin-1 and not valid UTF-8 in any encoding git cares
    // about. The branch name reaches the filesystem as a loose ref; the path
    // stays inside a tree object, which is why `main` is what gets checked
    // out below and this branch is not.
    s.commit(
        b"refs/heads/caf\xe9",
        b"Work under a Latin-1 name\n",
        &[m1],
        &[file(b"caf\xe9.txt", b"latin-1 path\n")],
    );
    site.import(s)?;
    site.checkout_main()
}
