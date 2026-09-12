//! Histories with no merge in them: the smallest possible repository, a long
//! straight line, a fork that never joins, and a commit that exists twice.

use super::{BuildError, MAIN, Site, file};
use crate::stream::{Mark, Stream};

pub(super) fn trivial(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let readme = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"A trivial repo.\n")],
    );
    let note = stream.commit(
        MAIN,
        b"Add the first note\n",
        &[readme],
        &[file(b"notes.txt", b"one\n")],
    );
    stream.commit(
        MAIN,
        b"Correct the note\n",
        &[note],
        &[file(b"notes.txt", b"one, corrected\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn linear(site: &Site<'_>) -> Result<(), BuildError> {
    const COMMITS: u32 = 200;
    let mut stream = Stream::new();
    let mut tip: Option<Mark> = None;
    for n in 1..=COMMITS {
        let message = format!("Commit {n:03}\n");
        let content = format!("line {n}\n");
        let parents: Vec<Mark> = tip.into_iter().collect();
        tip = Some(stream.commit(
            MAIN,
            message.as_bytes(),
            &parents,
            &[file(b"log.txt", content.as_bytes())],
        ));
    }
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn feature_branches(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let root = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Features.\n")],
    );
    let added_a = stream.commit(MAIN, b"Add a.txt\n", &[root], &[file(b"a.txt", b"1\n")]);
    let added_b = stream.commit(MAIN, b"Add b.txt\n", &[added_a], &[file(b"b.txt", b"1\n")]);

    let one_a = stream.commit(
        b"refs/heads/feature/one",
        b"one: start\n",
        &[added_a],
        &[file(b"one.txt", b"a\n")],
    );
    let one_b = stream.commit(
        b"refs/heads/feature/one",
        b"one: finish\n",
        &[one_a],
        &[file(b"one.txt", b"b\n")],
    );
    let merged_one = stream.commit(
        MAIN,
        b"Merge branch 'feature/one'\n",
        &[added_b, one_b],
        &[file(b"one.txt", b"b\n")],
    );

    let two_a = stream.commit(
        b"refs/heads/feature/two",
        b"two: start\n",
        &[added_b],
        &[file(b"two.txt", b"a\n")],
    );
    let two_b = stream.commit(
        b"refs/heads/feature/two",
        b"two: finish\n",
        &[two_a],
        &[file(b"two.txt", b"b\n")],
    );
    stream.commit(
        MAIN,
        b"Merge branch 'feature/two'\n",
        &[merged_one, two_b],
        &[file(b"two.txt", b"b\n")],
    );

    // Left open on purpose: a graph with no unmerged branch in it never
    // exercises the lane that has nowhere to land.
    stream.commit(
        b"refs/heads/feature/three",
        b"three: in progress\n",
        &[added_b],
        &[file(b"three.txt", b"a\n")],
    );

    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn orphan_branch(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let root = stream.commit(
        MAIN,
        b"Add a readme\n",
        &[],
        &[file(b"README.md", b"Code.\n")],
    );
    stream.commit(
        MAIN,
        b"Add the code\n",
        &[root],
        &[file(b"code.txt", b"1\n")],
    );
    // No parent at all: a second root commit.
    let docs_start = stream.commit(
        b"refs/heads/docs",
        b"docs: first page\n",
        &[],
        &[file(b"index.md", b"Docs.\n")],
    );
    stream.commit(
        b"refs/heads/docs",
        b"docs: second page\n",
        &[docs_start],
        &[file(b"guide.md", b"Guide.\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn cherry_pick_duplicates(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(
        MAIN,
        b"Add the greeting\n",
        &[],
        &[file(b"shared.txt", b"hello\nworld\n")],
    );
    // Same parent tree, same resulting tree, so the same patch id — and two
    // different commits, on branches with no ancestry between them.
    stream.commit(
        MAIN,
        b"Fix the greeting\n",
        &[base],
        &[file(b"shared.txt", b"hello\nthere\n")],
    );
    let picked = stream.commit(
        b"refs/heads/topic",
        b"Fix the greeting (cherry picked)\n",
        &[base],
        &[file(b"shared.txt", b"hello\nthere\n")],
    );
    stream.commit(
        b"refs/heads/topic",
        b"topic: unrelated work\n",
        &[picked],
        &[file(b"topic.txt", b"x\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}
