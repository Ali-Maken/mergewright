//! Absorb's reads: the staged diff, the commits in the range, a file at
//! `HEAD`, and which commit last changed each line asked about.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use graft_core::oid::Oid;
use graft_git::diff::{FileDiff, Hunk, parse_unified};
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, Stop};

use crate::facts::FactsError;

use super::{parse_oid, unparsable};

/// The staged diff, `-U0`, one entry per file. Plumbing `diff-index`
/// rather than porcelain `git diff --cached`: the porcelain refreshes
/// `.git/index` (a read must not write it) and follows the user's
/// `diff.*` settings. `-M`: a rename is read as one, so both its sides stay
/// ([`super::StaysWhy::Renamed`]); read as a deletion and a new file, the
/// deletion was placed and the new file stayed (review m7-06 M2).
pub(super) fn staged(
    repo: &RepoHandle,
    head: Oid,
    stop: &dyn Stop,
) -> Result<Vec<FileDiff>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff-index"),
            GitArg::Flag("--cached"),
            GitArg::Flag("-p"),
            GitArg::Count("-U", 0),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("-M"),
            GitArg::Oid(head),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    parse_unified(&output.stdout).map_err(|error| unparsable(format!("the staged diff: {error:?}")))
}

/// The staged paths that are submodules (a gitlink, mode `160000`, on either
/// side). A second, `--raw` read: the patch's `index <a>..<b> 160000` line,
/// the only place a bump's mode shows, is not one `parse_unified` keeps
/// (review m7-06 M4).
pub(super) fn gitlinks(
    repo: &RepoHandle,
    head: Oid,
    stop: &dyn Stop,
) -> Result<BTreeSet<Vec<u8>>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff-index"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--raw"),
            GitArg::Flag("-z"),
            GitArg::Flag("--no-renames"),
            GitArg::Oid(head),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    // `:<old mode> <new mode> <old id> <new id> <status>\0<path>\0`, each.
    let mut found = BTreeSet::new();
    let mut fields = output.stdout.split(|byte| *byte == 0);
    while let Some(header) = fields.next() {
        if header.is_empty() {
            continue;
        }
        let Some(path) = fields.next() else {
            return Err(unparsable(format!(
                "a raw diff record with no path: {:?}",
                String::from_utf8_lossy(header)
            )));
        };
        let mut modes = header
            .strip_prefix(b":")
            .unwrap_or(header)
            .split(|byte| *byte == b' ');
        if modes.by_ref().take(2).any(|mode| mode == b"160000") {
            found.insert(path.to_vec());
        }
    }
    Ok(found)
}

/// The commits in `base..head`.
pub(super) fn commits_in(
    repo: &RepoHandle,
    base: Oid,
    head: Oid,
    stop: &dyn Stop,
) -> Result<BTreeSet<Oid>, FactsError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Oid(head),
            GitArg::Word(format!("^{base}")),
        ])
        .run(stop)
        .map_err(FactsError::of_history)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(parse_oid)
        .collect()
}

/// A file's lines at `HEAD`, for the context lines and for knowing where
/// it ends.
pub(super) struct FileAtHead {
    pub(super) lines: Vec<Vec<u8>>,
    /// The last line has its `\n`.
    pub(super) ends_with_newline: bool,
}

impl FileAtHead {
    pub(super) fn read(
        repo: &RepoHandle,
        head: Oid,
        path: &[u8],
        stop: &dyn Stop,
    ) -> Result<FileAtHead, FactsError> {
        let mut spec = format!("{head}:").into_bytes();
        spec.extend_from_slice(path);
        let output = GitInvocation::new(repo)
            .args([
                GitArg::Flag("cat-file"),
                GitArg::Flag("blob"),
                GitArg::Ref(spec),
            ])
            .run(stop)
            .map_err(FactsError::of_history)?;
        let bytes = output.stdout;
        let ends_with_newline = bytes.last().is_none_or(|last| *last == b'\n');
        let body = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
        let lines = if bytes.is_empty() {
            Vec::new()
        } else {
            body.split(|byte| *byte == b'\n')
                .map(<[u8]>::to_vec)
                .collect()
        };
        Ok(FileAtHead {
            lines,
            ends_with_newline,
        })
    }
}

/// Which commit each line a hunk of `diff` asks about was last changed in,
/// by `HEAD`'s line number. One `blame` per file, every range at once.
pub(super) fn blame(
    repo: &RepoHandle,
    base: Oid,
    head: Oid,
    diff: &FileDiff,
    line_count: usize,
    stop: &dyn Stop,
) -> Result<BTreeMap<u32, Oid>, FactsError> {
    let count = u32::try_from(line_count).unwrap_or(u32::MAX);
    let wanted: BTreeSet<u32> = diff
        .hunks
        .iter()
        .flat_map(asks_about)
        .filter(|line| (1..=count).contains(line))
        .collect();
    if wanted.is_empty() {
        return Ok(BTreeMap::new());
    }
    // `--no-ignore-revs-file` drops his `blame.ignoreRevsFile`: a configured
    // file that is missing fails the blame, and one that is present blames
    // a listed commit's lines past it, to a commit the hunk does not
    // commute to (review m7-06 M1). Not `--ignore-revs-file=`: git keeps
    // the list sorted, and reads a missing configured file anyway (probed,
    // git 2.54).
    let mut args = vec![
        GitArg::Flag("blame"),
        GitArg::Flag("--porcelain"),
        GitArg::Flag("--no-ignore-revs-file"),
    ];
    for run in runs(&wanted) {
        args.push(GitArg::Flag("-L"));
        args.push(GitArg::Word(format!("{},{}", run.start, run.end - 1)));
    }
    args.push(GitArg::Word(format!("{base}..{head}")));
    args.push(GitArg::Path(diff.entry.path.clone()));
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(FactsError::of_history)?;
    parse_blame(&String::from_utf8_lossy(&output.stdout))
}

/// The lines of `HEAD`'s file a hunk is judged by: its old lines, or for a
/// pure insertion the line above and the line below.
pub(super) fn asks_about(hunk: &Hunk) -> Vec<u32> {
    if hunk.old_lines == 0 {
        vec![hunk.old_start, hunk.old_start + 1]
    } else {
        (hunk.old_start..hunk.old_start + hunk.old_lines).collect()
    }
}

/// Contiguous runs of line numbers, as `-L` ranges want them.
fn runs(lines: &BTreeSet<u32>) -> Vec<Range<u32>> {
    let mut runs: Vec<Range<u32>> = Vec::new();
    for &line in lines {
        match runs.last_mut() {
            Some(run) if run.end == line => run.end = line + 1,
            _ => runs.push(line..line + 1),
        }
    }
    runs
}

/// `blame --porcelain`: each line's header is `<id> <orig> <final> [<n>]`;
/// the commit's details and the `\t`-led content follow and are skipped.
fn parse_blame(text: &str) -> Result<BTreeMap<u32, Oid>, FactsError> {
    let mut blamed = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('\t') {
            continue;
        }
        let mut words = line.split(' ');
        let (Some(id), Some(_orig), Some(last)) = (words.next(), words.next(), words.next()) else {
            continue;
        };
        let Ok(commit) = Oid::from_hex(id) else {
            // A detail line (`author …`, `filename …`, `boundary`).
            continue;
        };
        let last = last
            .parse::<u32>()
            .map_err(|_| unparsable(format!("blame line {line:?}")))?;
        blamed.insert(last, commit);
    }
    Ok(blamed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_join_neighbouring_lines() {
        let lines: BTreeSet<u32> = [1, 2, 3, 5, 7, 8].into_iter().collect();
        assert_eq!(runs(&lines), vec![1..4, 5..6, 7..9]);
    }

    #[test]
    fn porcelain_blame_reads_each_final_line() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let text = format!(
            "{a} 1 3 2\nauthor x\nfilename f\n\tline three\n{a} 2 4\n\tline four\n\
             {b} 9 5 1\nauthor y\nboundary\nfilename f\n\tline five\n"
        );
        let blamed = parse_blame(&text).expect("it parses");
        let a = Oid::from_hex(&a).expect("an id");
        let b = Oid::from_hex(&b).expect("an id");
        assert_eq!(
            blamed.into_iter().collect::<Vec<_>>(),
            vec![(3, a), (4, a), (5, b)]
        );
    }
}
