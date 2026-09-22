//! Repositories stopped at a conflict (m6-conflicts/01): one history, two
//! ways of stopping in it. Every shape of unmerged path git has appears
//! once, so `UnmergedHow`, the conflict parser and the resolver's whole-file
//! paths all have a real repository to run against.
//!
//! The history: a root commit on `main` with the files, then one commit on
//! `feature` and one on `main` that each change every file their own way.
//! `StoppedMerge` is `git merge feature` on `main`; `StoppedRebase` is `git
//! rebase main` on `feature`. In both, **ours** is `main` and **theirs** is
//! `feature`, so a file named for a side means the same thing in each.
//!
//! The files, and what each is for:
//!
//! | path | XY | why |
//! |---|---|---|
//! | `both.txt` | UU | one block, the plain case |
//! | `three.txt` | UU | three blocks, four unchanged lines apart (git's default style joins conflicts closer than that; measured 2026-09-22); every base differs from both sides, so `diff3` has something to say |
//! | `crlf.txt` | UU | CRLF everywhere — git writes CRLF-terminated markers |
//! | `nofinal.txt` | UU | no final newline in any version, so the file git writes ends in a partial line (a side lacking one *inside* a block gets it added by git, measured 2026-09-22, so that case cannot occur) |
//! | `md.md` | UU | a `=======` setext underline inside theirs' side, as content |
//! | `zdiff3.txt` | UU | re-made with `merge.conflictStyle=zdiff3`, so its block carries a base |
//! | `pic.bin` | UU | binary: NUL in the first bytes, no markers |
//! | `theirs-deleted.txt` | UD | ours changed it, theirs deleted it |
//! | `ours-deleted.txt` | DU | ours deleted it, theirs changed it |
//! | `both-added.txt` | AA | new on both sides, different content |
//! | `both-deleted.txt` | DD | renamed to a different name by each side (`rename/rename`), which is the one recipe that leaves a `DD`; it brings the next two |
//! | `added-by-us.txt` | AU | our name for `both-deleted.txt` |
//! | `added-by-them.txt` | UA | their name for `both-deleted.txt` |

use super::{BuildError, MAIN, Site, file};
use crate::git::GitError;
use crate::stream::Stream;

/// How the repository is stopped.
#[derive(Clone, Copy)]
enum Stop {
    Merge,
    Rebase,
}

pub(super) fn stopped_rebase(site: &Site<'_>) -> Result<(), BuildError> {
    stopped(site, Stop::Rebase)
}

pub(super) fn stopped_merge(site: &Site<'_>) -> Result<(), BuildError> {
    stopped(site, Stop::Merge)
}

fn stopped(site: &Site<'_>, how: Stop) -> Result<(), BuildError> {
    root_commit(site)?;
    theirs(site)?;
    ours(site)?;
    match how {
        Stop::Merge => stop_on_conflict(site, &["merge", "--no-edit", "feature"])?,
        Stop::Rebase => {
            site.git(&["checkout", "-q", "feature"])?;
            stop_on_conflict(site, &["rebase", "main"])?;
        }
    }
    // One file in `zdiff3` style, remade from its stages; the rest keep the
    // default `merge` style the stop wrote. `checkout --merge` labels the
    // sides `ours`/`theirs`/`base` rather than by branch — a genuine
    // git-written zdiff3 file all the same.
    site.git(&[
        "-c",
        "merge.conflictStyle=zdiff3",
        "checkout",
        "--merge",
        "--",
        "zdiff3.txt",
    ])
    .map(|_| ())
}

/// The files as both sides start from, on `main`.
fn root_commit(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    stream.commit(
        MAIN,
        b"Add the files both sides will change\n",
        &[],
        &[
            file(
                b"both.txt",
                b"same on both sides\nthe line both sides change\nand a tail\n",
            ),
            file(
                b"three.txt",
                b"alpha\none\nb1\nb2\nb3\nb4\ntwo\nc1\nc2\nc3\nc4\nthree\ndelta\n",
            ),
            file(b"crlf.txt", b"one\r\ntwo\r\nthree\r\n"),
            file(b"nofinal.txt", b"head\nlast"),
            file(b"md.md", b"Title\n=====\n\nIntro line\n"),
            file(b"zdiff3.txt", b"start\nmiddle\nend\n"),
            file(b"pic.bin", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR base\0"),
            file(b"theirs-deleted.txt", b"kept by us, deleted by them\n"),
            file(b"ours-deleted.txt", b"deleted by us, kept by them\n"),
            file(
                b"both-deleted.txt",
                b"renamed to a different name by each side\n",
            ),
        ],
    );
    site.import(stream)?;
    site.checkout_main()
}

/// Theirs: one commit on `feature` changing every file its way.
fn theirs(site: &Site<'_>) -> Result<(), BuildError> {
    site.git(&["checkout", "-q", "-b", "feature"])?;
    site.write(
        "both.txt",
        "same on both sides\nthe line, as they changed it\nand a tail\n",
    )?;
    site.write(
        "three.txt",
        "alpha\none-theirs\nb1\nb2\nb3\nb4\ntwo-theirs\nc1\nc2\nc3\nc4\nthree-theirs\ndelta\n",
    )?;
    site.write_bytes("crlf.txt", b"one\r\ntwo theirs\r\nthree\r\n")?;
    site.write("nofinal.txt", "head theirs\nlast")?;
    site.write(
        "md.md",
        "Title\n=====\n\nHeading\n=======\n\nIntro line theirs\n",
    )?;
    site.write("zdiff3.txt", "start\nmiddle theirs\nend\n")?;
    site.write_bytes("pic.bin", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR theirs\0")?;
    site.remove("theirs-deleted.txt")?;
    site.write("ours-deleted.txt", "deleted by us, changed by them\n")?;
    site.write("both-added.txt", "added by them\n")?;
    site.git(&["mv", "both-deleted.txt", "added-by-them.txt"])?;
    site.git(&["add", "-A", "--", "."])?;
    site.git(&["commit", "-q", "-m", "Change every file on feature"])
        .map(|_| ())
}

/// Ours: one commit on `main` changing every file its way.
fn ours(site: &Site<'_>) -> Result<(), BuildError> {
    site.git(&["checkout", "-q", "main"])?;
    site.write(
        "both.txt",
        "same on both sides\nthe line, as we changed it\nand a tail\n",
    )?;
    site.write(
        "three.txt",
        "alpha\none-ours\nb1\nb2\nb3\nb4\ntwo-ours\nc1\nc2\nc3\nc4\nthree-ours\ndelta\n",
    )?;
    site.write_bytes("crlf.txt", b"one\r\ntwo ours\r\nthree\r\n")?;
    site.write("nofinal.txt", "head ours\nlast")?;
    site.write("md.md", "Title\n=====\n\nIntro line ours\n")?;
    site.write("zdiff3.txt", "start\nmiddle ours\nend\n")?;
    site.write_bytes("pic.bin", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR ours\0")?;
    site.write("theirs-deleted.txt", "changed by us, deleted by them\n")?;
    site.remove("ours-deleted.txt")?;
    site.write("both-added.txt", "added by us\n")?;
    site.git(&["mv", "both-deleted.txt", "added-by-us.txt"])?;
    site.git(&["add", "-A", "--", "."])?;
    site.git(&["commit", "-q", "-m", "Change every file on main"])
        .map(|_| ())
}

/// Runs a command that is *meant* to stop on conflicts (exit 1); anything
/// else — success included — is a broken recipe.
fn stop_on_conflict(site: &Site<'_>, args: &[&str]) -> Result<(), BuildError> {
    match site.git(args) {
        Ok(_) => Err(BuildError::Unexpected {
            shape: site.shape,
            what: format!(
                "`git {}` succeeded; it was meant to stop on conflicts",
                args.join(" ")
            ),
        }),
        Err(BuildError::Git {
            source: GitError::Failed { status, .. },
            ..
        }) if status.code() == Some(1) => Ok(()),
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use crate::Shape;
    use crate::git;

    /// `(XY, path)` for every unmerged record, sorted by path.
    fn unmerged(dir: &Path) -> Vec<(String, String)> {
        let out = git::run(dir, &["status", "--porcelain=v2", "-z"])
            .unwrap_or_else(|why| panic!("{why}"));
        let mut found: Vec<(String, String)> = out
            .split(|&b| b == 0)
            .filter(|record| record.first() == Some(&b'u'))
            .map(|record| {
                let text = String::from_utf8_lossy(record);
                let fields: Vec<&str> = text.splitn(11, ' ').collect();
                assert_eq!(fields.len(), 11, "an unmerged record has 11 fields: {text}");
                (fields[1].to_owned(), fields[10].to_owned())
            })
            .collect();
        found.sort_by(|a, b| a.1.cmp(&b.1));
        found
    }

    fn expected() -> Vec<(String, String)> {
        let mut rows: Vec<(String, String)> = [
            ("UA", "added-by-them.txt"),
            ("AU", "added-by-us.txt"),
            ("AA", "both-added.txt"),
            ("DD", "both-deleted.txt"),
            ("UU", "both.txt"),
            ("UU", "crlf.txt"),
            ("UU", "md.md"),
            ("UU", "nofinal.txt"),
            ("DU", "ours-deleted.txt"),
            ("UU", "pic.bin"),
            ("UD", "theirs-deleted.txt"),
            ("UU", "three.txt"),
            ("UU", "zdiff3.txt"),
        ]
        .into_iter()
        .map(|(xy, path)| (xy.to_owned(), path.to_owned()))
        .collect();
        rows.sort_by(|a, b| a.1.cmp(&b.1));
        rows
    }

    #[test]
    fn stopped_shapes_are_as_documented() {
        let root = tempfile::tempdir().expect("a scratch directory");
        let streams = root.path().join("streams");
        std::fs::create_dir(&streams).expect("the streams directory");

        for (shape, marker) in [
            (Shape::StoppedRebase, "rebase-merge"),
            (Shape::StoppedMerge, "MERGE_HEAD"),
        ] {
            let dir = root.path().join(shape.name());
            let started = Instant::now();
            crate::shapes::build(shape, &dir, &streams).unwrap_or_else(|why| panic!("{why}"));
            let took = started.elapsed();
            assert!(
                took < Duration::from_secs(2),
                "{shape} took {took:?} to build; the budget is 2 s"
            );
            assert!(
                dir.join(".git").join(marker).exists(),
                "{shape} should have left .git/{marker} behind"
            );
            assert_eq!(unmerged(&dir), expected(), "{shape}'s unmerged paths");
            let zdiff3 = std::fs::read(dir.join("zdiff3.txt")).expect("zdiff3.txt");
            assert!(
                zdiff3.windows(8).any(|w| w == b"||||||| "),
                "zdiff3.txt should carry a base marker: {:?}",
                String::from_utf8_lossy(&zdiff3)
            );
        }
    }

    #[test]
    fn every_stopped_shape_has_a_name_and_is_found_by_it() {
        for shape in Shape::STOPPED {
            assert_eq!(Shape::from_name(shape.name()), Some(*shape));
            assert!(
                !Shape::ALL.contains(shape),
                "{shape} is listed apart from ALL"
            );
        }
    }
}
