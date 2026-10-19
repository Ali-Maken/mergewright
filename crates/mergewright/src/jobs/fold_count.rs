//! The read behind a folded merge's count, the `N` of its tail `▸ N commits
//! from …` (m8-polish-and-ship/17b; ease-pass 06): how many
//! commits the merge brought in. A fact of the merge commit, so the answer
//! never changes and `update` keeps it for the session
//! (`AppState::fold_counts`).

use mergewright_core::oid::Oid;
use mergewright_git::history::read_commit;
use mergewright_git::history_reads::count_between;

use super::Cancel;
use crate::repos::RepoStore;

/// The work behind
/// [`RestRead::CountFolded`](crate::update::RestRead::CountFolded): the commits
/// `merge` reaches that its first parent does not, less `merge` itself —
/// `rev-list --count M ^M^1`, minus one. One count covers every later parent
/// of an octopus, so a commit two sides share is counted once.
///
/// # Errors
/// The repository not opening, the merge not reading, a merge with no first
/// parent (a shallow boundary), or `git` failing (stopped included), as one
/// line.
pub(crate) fn count_folded(
    repos: &RepoStore,
    merge: Oid,
    cancel: &Cancel,
) -> Result<usize, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let commit = read_commit(&handle, merge).map_err(|error| error.to_string())?;
    let Some(&first) = commit.parents.first() else {
        return Err("the merge has no parents here (a shallow boundary)".to_owned());
    };
    let reached =
        count_between(&handle, first, merge, cancel).map_err(|error| error.to_string())?;
    // The range holds the merge itself; the count is what it brought in.
    Ok(reached.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use mergewright_fixtures::Shape;

    use super::*;

    fn oid(repo: &std::path::Path, rev: &str) -> Oid {
        let out = mergewright_fixtures::git::run(repo, &["rev-parse", rev]).expect("rev-parse");
        Oid::from_hex(String::from_utf8(out).expect("hex").trim()).expect("an object id")
    }

    /// The corpus octopus brings in three one-commit topics: `+3`.
    #[test]
    fn count_folded_counts_an_octopus() {
        let repo = mergewright_fixtures::borrow(Shape::OctopusMerge);
        let store = RepoStore::new(repo.clone(), None);
        let merge = oid(&repo, "main");
        assert_eq!(count_folded(&store, merge, &Cancel::new()), Ok(3));
    }

    /// An octopus whose sides overlap: `base` merged with `topic-1`, `t4`
    /// (on top of `topic-1`) and `topic-2`. Side by side that is 1 + 2 + 1
    /// = 4; the commits it brings in are `topic-1`, `t4`, `topic-2` = 3.
    #[test]
    fn count_folded_counts_an_octopus_union() {
        let scratch = mergewright_fixtures::copy_into_temp(Shape::OctopusMerge);
        let repo = scratch.path();
        let tree = |rev: &str| oid(repo, &format!("{rev}^{{tree}}")).to_string();
        let commit_tree = |args: &[&str]| {
            let out = mergewright_fixtures::git::run(repo, args).expect("commit-tree");
            String::from_utf8(out).expect("hex").trim().to_owned()
        };
        let t4 = commit_tree(&["commit-tree", &tree("topic-1"), "-p", "topic-1", "-m", "t4"]);
        let merge = commit_tree(&[
            "commit-tree",
            &tree("main"),
            "-p",
            "main^1",
            "-p",
            "topic-1",
            "-p",
            &t4,
            "-p",
            "topic-2",
            "-m",
            "overlapping octopus",
        ]);
        let store = RepoStore::new(repo.to_path_buf(), None);
        let merge = oid(repo, &merge);
        assert_eq!(count_folded(&store, merge, &Cancel::new()), Ok(3));
    }

    /// A commit that is not there: an error naming why, not a zero.
    #[test]
    fn count_folded_says_why_it_could_not_count() {
        let repo = mergewright_fixtures::borrow(Shape::OctopusMerge);
        let store = RepoStore::new(repo, None);
        let missing =
            Oid::from_hex("0123456789abcdef0123456789abcdef01234567").expect("an object id");
        let answer = count_folded(&store, missing, &Cancel::new());
        assert!(answer.is_err(), "{answer:?}");
    }
}
