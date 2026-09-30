//! The before-snapshot's `raw/` (ADR-0052): his bytes where git would not
//! give them back.
//!
//! `worktree/` is git's **cleaned** tree (`add -A`), and a restore writes it
//! through git's **smudge**. For most files the two are inverses. They are
//! not for an LF file under `core.autocrlf=true` (it comes back CRLF), a CRLF
//! file under `input` (it comes back LF), or a one-way clean filter such as
//! nbstripout (the outputs never come back). His bytes are his work, so those
//! paths are kept as they are on disk.
//!
//! Three questions, each asked only of the paths the one before let through,
//! so a clean worktree costs one `diff-tree` and a repo that converts nothing
//! three spawns:
//!
//! 1. **Uncommitted?** `HEAD`'s tree against the worktree tree: added,
//!    modified or type-changed regular files. A path that matches `HEAD` is
//!    in history and comes back as git's own checkout writes it.
//! 2. **Would git convert it?** `core.autocrlf` and `check-attr` on the
//!    conversion attributes. Saying yes wrongly costs one comparison; saying
//!    no wrongly loses his bytes, so every doubt is a yes.
//! 3. **Would git's checkout give it back?** The cleaned blobs checked out by
//!    git's own `checkout-index` into scratch, then compared with the disk.
//!    Not `cat-file --batch --filters`: its header prints the size *before*
//!    the filter (git 2.54), so its output cannot be split.
//!
//! What fails the third question is hashed with `--no-filters` and written as
//! a tree at its own path and mode.

use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use super::SnapshotError;
use super::plumbing::{TempIndex, oid_from, path_of, split_nul, stopped, write_tree};

/// The conversion attributes `check-attr` is asked about: any of these set
/// means git may rewrite the file's bytes on the way in or out.
const CONVERTING: [&str; 6] = [
    "text",
    "eol",
    "crlf",
    "filter",
    "ident",
    "working-tree-encoding",
];

/// One regular file of the worktree tree.
#[derive(Clone, Debug, PartialEq, Eq)]
struct File {
    /// `100644` or `100755`, as the worktree tree records it.
    mode: Vec<u8>,
    /// The cleaned blob (`worktree/`'s).
    oid: Oid,
    /// Relative to the worktree root, raw bytes.
    path: Vec<u8>,
}

/// R2: the paths of `worktree_tree` that are uncommitted, that git would
/// convert, and whose bytes on disk git's smudge would not give back, as a
/// tree of their raw bytes (`hash-object -w --no-filters`). `None` when there
/// are none.
///
/// `head` is `HEAD`'s commit, `None` when unborn: then every path is
/// uncommitted. `worktree_tree` is the tree `worktree_tree` wrote a moment
/// ago, over the same disk.
///
/// # Errors
/// [`SnapshotError`] when a `git` step fails or prints what it should not, or
/// a file cannot be read; `Stopped` when `stop` fires.
pub fn raw_record(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
    head: Option<Oid>,
    worktree_tree: Oid,
) -> Result<Option<Oid>, SnapshotError> {
    // A bare repository has no working tree, so no bytes of his to keep.
    let Some(root) = repo.workdir() else {
        return Ok(None);
    };
    let changed = uncommitted(repo, head, worktree_tree, stop)?;
    if changed.is_empty() {
        return Ok(None);
    }
    let autocrlf = autocrlf_on(repo, stop)?;
    let converted = converted(repo, changed, autocrlf, stop)?;
    if converted.is_empty() {
        return Ok(None);
    }
    let differ = not_given_back(repo, scratch, root, converted, stop)?;
    if differ.is_empty() {
        return Ok(None);
    }
    raw_tree(repo, scratch, &differ, stop).map(Some)
}

/// Question 1: the regular files `worktree_tree` holds apart from `HEAD`.
///
/// A deletion has mode `000000` on the new side, a symlink `120000` and a
/// submodule `160000`: none is a regular file, so the mode alone leaves them
/// out. git filters neither a symlink nor a gitlink.
fn uncommitted(
    repo: &RepoHandle,
    head: Option<Oid>,
    worktree_tree: Oid,
    stop: &dyn Stop,
) -> Result<Vec<File>, SnapshotError> {
    let files = if let Some(head) = head {
        let output = GitInvocation::new(repo)
            .arg(GitArg::Flag("diff-tree"))
            .arg(GitArg::Flag("-r"))
            .arg(GitArg::Flag("-z"))
            .arg(GitArg::Flag("--no-renames"))
            .arg(GitArg::Oid(head))
            .arg(GitArg::Oid(worktree_tree))
            .run(stop)
            .map_err(stopped)?;
        diff_tree_files(&output.stdout)?
    } else {
        let output = GitInvocation::new(repo)
            .arg(GitArg::Flag("ls-tree"))
            .arg(GitArg::Flag("-r"))
            .arg(GitArg::Flag("-z"))
            .arg(GitArg::Flag("--full-tree"))
            .arg(GitArg::Oid(worktree_tree))
            .run(stop)
            .map_err(stopped)?;
        ls_tree_files(&output.stdout)?
    };
    Ok(files
        .into_iter()
        .filter(|file| file.mode == b"100644" || file.mode == b"100755")
        .collect())
}

/// `diff-tree -r -z` raw records: `:<old> <new> <old-oid> <new-oid> <S>\0`
/// then `<path>\0`. `--no-renames` keeps it to one path per record.
fn diff_tree_files(stdout: &[u8]) -> Result<Vec<File>, SnapshotError> {
    let fields = split_nul(stdout);
    let mut files = Vec::with_capacity(fields.len() / 2);
    for pair in fields.chunks(2) {
        let [meta, path] = pair else {
            return Err(unreadable("diff-tree", &pair.concat()));
        };
        let words: Vec<&[u8]> = meta.split(|byte| *byte == b' ').collect();
        let [_, new_mode, _, new_oid, _] = words.as_slice() else {
            return Err(unreadable("diff-tree", meta));
        };
        files.push(File {
            mode: new_mode.to_vec(),
            oid: oid_from(new_oid, "diff-tree")?,
            path: path.clone(),
        });
    }
    Ok(files)
}

/// `ls-tree -r -z` records: `<mode> <type> <oid>\t<path>\0`.
fn ls_tree_files(stdout: &[u8]) -> Result<Vec<File>, SnapshotError> {
    let mut files = Vec::new();
    for record in split_nul(stdout) {
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            return Err(unreadable("ls-tree", &record));
        };
        let (head, path) = (&record[..tab], &record[tab + 1..]);
        let words: Vec<&[u8]> = head.split(|byte| *byte == b' ').collect();
        let [mode, _, oid] = words.as_slice() else {
            return Err(unreadable("ls-tree", &record));
        };
        files.push(File {
            mode: mode.to_vec(),
            oid: oid_from(oid, "ls-tree")?,
            path: path.to_vec(),
        });
    }
    Ok(files)
}

/// `core.autocrlf`, as git's own reader takes it: `input` and every true
/// value convert. Unset (exit 1) does not.
///
/// Only a value git reads as false is off. Anything else — `true`, `input`,
/// a bare key, a spelling this reader does not know — is on: a wrong "on"
/// costs question 3 a comparison, a wrong "off" loses his bytes.
fn autocrlf_on(repo: &RepoHandle, stop: &dyn Stop) -> Result<bool, SnapshotError> {
    let ran = GitInvocation::new(repo)
        .arg(GitArg::Flag("config"))
        .arg(GitArg::Flag("--get"))
        .arg(GitArg::Flag("core.autocrlf"))
        .run(stop);
    match ran {
        Ok(output) => {
            let value = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_ascii_lowercase();
            let off = matches!(value.as_str(), "false" | "no" | "off")
                || value.parse::<i64>().is_ok_and(|number| number == 0);
            Ok(!off)
        }
        // `git config --get`: exit 1 is "the key was not found", its one
        // documented non-error failure. Unset means git converts nothing.
        Err(RunError::Exited { status: 1, .. }) => Ok(false),
        Err(why) => Err(stopped(why)),
    }
}

/// Question 2: the files git would convert, by `check-attr` on
/// [`CONVERTING`] and `core.autocrlf`.
///
/// A file converts when any of the attributes has a value other than
/// `unspecified` or `unset`, or `core.autocrlf` is on and `text` is not
/// `unset` (`-text`: git's "binary").
fn converted(
    repo: &RepoHandle,
    files: Vec<File>,
    autocrlf: bool,
    stop: &dyn Stop,
) -> Result<Vec<File>, SnapshotError> {
    let mut stdin = Vec::new();
    for file in &files {
        stdin.extend_from_slice(&file.path);
        stdin.push(0);
    }
    let mut invocation = GitInvocation::new(repo)
        .arg(GitArg::Flag("check-attr"))
        .arg(GitArg::Flag("-z"))
        .arg(GitArg::Flag("--stdin"));
    for attribute in CONVERTING {
        invocation = invocation.arg(GitArg::Flag(attribute));
    }
    let output = invocation.stdin(stdin).run(stop).map_err(stopped)?;
    let converts = converting_paths(&output.stdout, autocrlf)?;
    Ok(files
        .into_iter()
        .filter(|file| converts.contains(&file.path))
        .collect())
}

/// `check-attr -z` triples, `<path>\0<attribute>\0<value>\0`, folded into the
/// paths that convert. Pure, so the rule is testable without a repository.
fn converting_paths(
    stdout: &[u8],
    autocrlf: bool,
) -> Result<std::collections::BTreeSet<Vec<u8>>, SnapshotError> {
    use std::collections::BTreeMap;

    // Per path: (text is unset, some attribute is set).
    let mut seen: BTreeMap<Vec<u8>, (bool, bool)> = BTreeMap::new();
    for triple in split_nul_keeping_empty(stdout).chunks(3) {
        let [path, attribute, value] = triple else {
            return Err(unreadable("check-attr", &triple.concat()));
        };
        let at = seen.entry(path.clone()).or_insert((false, false));
        let quiet = value.as_slice() == b"unspecified" || value.as_slice() == b"unset";
        if !quiet {
            at.1 = true;
        }
        if attribute.as_slice() == b"text" && value.as_slice() == b"unset" {
            at.0 = true;
        }
    }
    Ok(seen
        .into_iter()
        .filter(|(_, (text_unset, set))| *set || (autocrlf && !text_unset))
        .map(|(path, _)| path)
        .collect())
}

/// NUL-terminated fields, an empty one kept (a value can be empty); only the
/// terminator's own trailing split is dropped.
fn split_nul_keeping_empty(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut fields: Vec<Vec<u8>> = bytes.split(|byte| *byte == 0).map(<[u8]>::to_vec).collect();
    if fields.last().is_some_and(Vec::is_empty) {
        fields.pop();
    }
    fields
}

/// Question 3: of `files`, the ones whose bytes on disk differ from what
/// git's own checkout of the cleaned blob writes.
///
/// The blobs go on a fresh temporary index and `checkout-index` writes them
/// under a scratch directory, through every smudge, `eol` and filter the
/// path's attributes name — the same machinery a restore uses. The directory
/// is removed on every path out.
fn not_given_back(
    repo: &RepoHandle,
    scratch: &Path,
    root: &Path,
    files: Vec<File>,
    stop: &dyn Stop,
) -> Result<Vec<File>, SnapshotError> {
    let index = TempIndex::fresh(scratch)?;
    fill_index(
        repo,
        &index,
        files.iter().map(|file| (file, file.oid)),
        stop,
    )?;
    let out = ScratchDir::new(scratch)?;
    let mut prefix = out.path().as_os_str().as_bytes().to_vec();
    prefix.push(b'/');
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index.path())
        .arg(GitArg::Flag("checkout-index"))
        .arg(GitArg::Flag("-a"))
        .arg(GitArg::Flag("-f"))
        .arg(GitArg::Glued("--prefix=", prefix))
        .run(stop)
        .map_err(stopped)?;
    let mut differ = Vec::new();
    for file in files {
        let relative = path_of(file.path.clone());
        if !same_bytes(&root.join(&relative), &out.path().join(&relative))? {
            differ.push(file);
        }
    }
    Ok(differ)
}

/// The raw blobs (`hash-object -w --no-filters`, read from disk as they are)
/// on a fresh temporary index at each file's own path and mode, written as
/// a tree.
fn raw_tree(
    repo: &RepoHandle,
    scratch: &Path,
    files: &[File],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let mut invocation = GitInvocation::new(repo)
        .arg(GitArg::Flag("hash-object"))
        .arg(GitArg::Flag("-w"))
        .arg(GitArg::Flag("--no-filters"))
        .arg(GitArg::Separator);
    for file in files {
        invocation = invocation.arg(GitArg::Path(file.path.clone()));
    }
    let output = invocation.run(stop).map_err(stopped)?;
    let printed: Vec<&[u8]> = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    if printed.len() != files.len() {
        return Err(unreadable("hash-object", &output.stdout));
    }
    let raw: Vec<Oid> = printed
        .into_iter()
        .map(|line| oid_from(line, "hash-object"))
        .collect::<Result<_, _>>()?;
    let index = TempIndex::fresh(scratch)?;
    fill_index(repo, &index, files.iter().zip(raw), stop)?;
    write_tree(repo, index.path(), stop)
}

/// `update-index -z --index-info` onto `index`: each file at its own path and
/// mode, with the blob paired with it.
fn fill_index<'a>(
    repo: &RepoHandle,
    index: &TempIndex,
    entries: impl IntoIterator<Item = (&'a File, Oid)>,
    stop: &dyn Stop,
) -> Result<(), SnapshotError> {
    let mut stdin = Vec::new();
    for (file, blob) in entries {
        stdin.extend_from_slice(&file.mode);
        stdin.push(b' ');
        stdin.extend_from_slice(blob.to_string().as_bytes());
        stdin.push(b'\t');
        stdin.extend_from_slice(&file.path);
        stdin.push(0);
    }
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index.path())
        .arg(GitArg::Flag("update-index"))
        .arg(GitArg::Flag("-z"))
        .arg(GitArg::Flag("--index-info"))
        .stdin(stdin)
        .run(stop)
        .map_err(stopped)?;
    Ok(())
}

/// Whether two files hold the same bytes: lengths first, then 64 KiB at a
/// time, so a large file is never read whole.
fn same_bytes(one: &Path, other: &Path) -> Result<bool, SnapshotError> {
    if std::fs::metadata(one)?.len() != std::fs::metadata(other)?.len() {
        return Ok(false);
    }
    let mut one = std::fs::File::open(one)?;
    let mut other = std::fs::File::open(other)?;
    let mut left = vec![0_u8; 64 * 1024];
    let mut right = vec![0_u8; 64 * 1024];
    loop {
        let read = one.read(&mut left)?;
        if read == 0 {
            // Equal lengths and every chunk equal: the other is at its end too.
            return Ok(true);
        }
        other.read_exact(&mut right[..read])?;
        if left[..read] != right[..read] {
            return Ok(false);
        }
    }
}

fn unreadable(what: &'static str, printed: &[u8]) -> SnapshotError {
    SnapshotError::NotAnOid {
        what,
        printed: String::from_utf8_lossy(printed)
            .lines()
            .next()
            .unwrap_or("")
            .to_owned(),
    }
}

/// A directory under the scratch that `checkout-index` writes into, removed
/// with everything in it when it goes out of scope, on every path.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(scratch: &Path) -> Result<ScratchDir, SnapshotError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = scratch.join(format!("tmp-raw-{}-{unique}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        std::fs::create_dir_all(&path)?;
        Ok(ScratchDir { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // As `TempIndex`'s drop: nothing can be done about a failed removal
        // here, and it must not mask the snapshot's own result. A survivor
        // is inert (the next one takes a fresh name), and it is said out
        // loud rather than asserted.
        if let Err(why) = std::fs::remove_dir_all(&self.path)
            && why.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!("graft: scratch {} survived: {why}", self.path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triple(path: &str, attribute: &str, value: &str) -> Vec<u8> {
        format!("{path}\0{attribute}\0{value}\0").into_bytes()
    }

    fn converting(records: &[Vec<u8>], autocrlf: bool) -> Vec<String> {
        converting_paths(&records.concat(), autocrlf)
            .expect("the records parse")
            .into_iter()
            .map(|path| String::from_utf8(path).expect("utf-8"))
            .collect()
    }

    #[test]
    fn nothing_set_and_autocrlf_off_converts_nothing() {
        let records = [
            triple("a", "text", "unspecified"),
            triple("a", "filter", "unspecified"),
        ];
        assert!(converting(&records, false).is_empty());
    }

    #[test]
    fn autocrlf_converts_unless_text_is_unset() {
        let records = [
            triple("a", "text", "unspecified"),
            triple("bin", "text", "unset"),
        ];
        assert_eq!(converting(&records, true), vec!["a"]);
    }

    #[test]
    fn any_converting_attribute_set_converts_even_without_autocrlf() {
        let records = [
            triple("f", "filter", "lfs"),
            triple("e", "eol", "crlf"),
            triple("t", "text", "auto"),
            triple("i", "ident", "set"),
            triple("w", "working-tree-encoding", "UTF-16"),
            triple("n", "text", "unset"),
        ];
        assert_eq!(converting(&records, false), vec!["e", "f", "i", "t", "w"]);
    }

    #[test]
    fn a_filter_on_a_binary_file_still_converts() {
        let records = [triple("b", "text", "unset"), triple("b", "filter", "lfs")];
        assert_eq!(converting(&records, true), vec!["b"]);
    }

    #[test]
    fn a_torn_triple_is_an_error() {
        assert!(converting_paths(b"a\0text\0", false).is_err());
    }

    #[test]
    fn diff_tree_records_keep_the_new_side() {
        let new = "1".repeat(40);
        let stdout = format!(
            ":100644 100755 {old} {new} M\0run.sh\0:100644 000000 {old} {zero} D\0gone\0",
            old = "2".repeat(40),
            zero = "0".repeat(40),
        );
        let files = diff_tree_files(stdout.as_bytes()).expect("the records parse");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].mode, b"100755");
        assert_eq!(files[0].oid.to_string(), new);
        assert_eq!(files[0].path, b"run.sh");
        assert_eq!(files[1].mode, b"000000");
    }
}
