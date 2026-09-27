//! Absorb's rebuilt stash: `S` on the rewrite's index base (06b), one
//! input to step 6's apply ([`super::put_back`]).

use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::objects::{commit_tree, printed_oid, resolve};

use super::Context;

/// `S` rebuilt on `base` for absorb's put-back (06b), or `None` when
/// `HEAD`'s tree is not `base`'s — the rewrite did not land where it was
/// planned (a resume skipped a row or resolved a stop differently), and the
/// base would measure the edits from a tree `HEAD` does not have; `S` itself
/// then goes back with `--index`, and falls back like any other.
///
/// The rebuilt stash has `S`'s working tree, first parent `base`, and as
/// its index commit `S`'s index tree on `base`. `stash apply --index` then:
/// - applies `base..index` to the index — exactly the hunks that stayed, the
///   ones only the index holds and mode changes included (review m7-06 C1);
/// - merges `S`'s working tree with `base` as the merge base. `HEAD`'s tree
///   is `base`'s, so the working tree comes back as it was, an edit made on
///   past a placed hunk included (review m7-06 M6: measured from the old
///   `HEAD`, both sides changed that line).
///
/// Objects only: no ref, index or file is written. `S` itself stays the
/// listed copy, dropped by id when the put-back is clean.
pub(super) fn from_index_base(
    ctx: &Context<'_>,
    stash: Oid,
    base: Oid,
    commands: &mut Vec<String>,
) -> Result<Option<Oid>, RunError> {
    let tree = |rev: &str| resolve(ctx.repo, rev, &());
    let base_tree = tree(&format!("{base}^{{tree}}"))?;
    if tree("HEAD^{tree}")? != base_tree {
        return Ok(None);
    }
    let index_tree = tree(&format!("{stash}^2^{{tree}}"))?;
    let work_tree = tree(&format!("{stash}^{{tree}}"))?;
    let index = commit(ctx, index_tree, &[base], "graft: index", commands)?;
    let rebuilt = commit(
        ctx,
        work_tree,
        &[base, index],
        "graft: your edits, put back with the index",
        commands,
    )?;
    Ok(Some(rebuilt))
}

/// A commit object, no ref; its command line recorded on the entry.
fn commit(
    ctx: &Context<'_>,
    tree: Oid,
    parents: &[Oid],
    message: &str,
    commands: &mut Vec<String>,
) -> Result<Oid, RunError> {
    let git = commit_tree(ctx.repo, tree, parents, message);
    commands.push(git.command_line());
    printed_oid(&git.run(&())?.stdout, "commit-tree")
}
