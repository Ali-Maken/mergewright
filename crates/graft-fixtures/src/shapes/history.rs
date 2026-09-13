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

/// A commit from a clock a day fast, with a twelve-commit line on `main` and
/// a `feature` branch both growing from it.
///
/// A date-order walk reaches it through `feature` — the newest tip — and,
/// being the newest date in the queue, emits it second: above its twelve
/// `main` children. A topological walk puts it below all of them.
pub(super) fn skewed_dates(site: &Site<'_>) -> Result<(), BuildError> {
    const DAY: u64 = 24 * 60 * 60;
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let fast = stream.commit_ahead(
        DAY,
        MAIN,
        b"Commit from a fast clock\n",
        &[base],
        &[file(b"fast.txt", b"ahead\n")],
    );
    let mut tip = fast;
    for n in 1..=12 {
        let message = format!("Main {n:02}\n");
        let content = format!("line {n}\n");
        tip = stream.commit(
            MAIN,
            message.as_bytes(),
            &[tip],
            &[file(b"main.txt", content.as_bytes())],
        );
    }
    stream.commit(
        b"refs/heads/feature",
        b"feature: build on the fast commit\n",
        &[fast],
        &[file(b"feature.txt", b"f\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}

/// One straight lane, seven commits, each one age older than the last,
/// offset from [`crate::CORPUS_NOW`]: 5 seconds, 90 seconds, 90 minutes,
/// 36 hours, 14 days, 70 days, 400 days before it (m1-the-graph/48, dated
/// off a fixed clock in 53). Those are the exact points `relative_date`
/// steps to its next unit — 89s is still `now`, 90s is the smallest `m` git
/// ever shows, and so on through `h`/`d`/`w`/`mo`/`y` — so the gallery stop
/// shows every unit the column has, and each commit sits right where git's
/// own rounding puts it, not a round number a person picked.
///
/// Dated off `CORPUS_NOW`, not real time: unlike ticket 48's version, this
/// shape's object ids are byte-identical build to build, exactly like every
/// other shape's, because `CORPUS_NOW` never changes. Viewing it (the
/// gallery, `graph_view`) at `CORPUS_NOW` — not the wall clock — is what
/// keeps the seven ages reading `now`/`2m`/`2h`/`2d`/`2w`/`2mo`/`1y`.
pub(super) fn relative_dates(site: &Site<'_>) -> Result<(), BuildError> {
    // Oldest (the root, bottom of the graph) to newest (the tip, top).
    const AGO: [i64; 7] = [
        400 * 24 * 60 * 60, // 400d -> "1y"
        70 * 24 * 60 * 60,  // 70d  -> "2mo"
        14 * 24 * 60 * 60,  // 14d  -> "2w"
        36 * 60 * 60,       // 36h  -> "2d"
        90 * 60,            // 90m  -> "2h"
        90,                 // 90s  -> "2m"
        5,                  // 5s   -> "now"
    ];
    let mut stream = Stream::new();
    let mut tip: Option<Mark> = None;
    for (n, ago) in AGO.iter().enumerate() {
        let message = format!("Commit {}\n", n + 1);
        let content = format!("line {}\n", n + 1);
        let parents: Vec<Mark> = tip.into_iter().collect();
        #[allow(
            clippy::cast_sign_loss,
            reason = "CORPUS_NOW - ago is always positive: every AGO entry is well under CORPUS_NOW"
        )]
        let when = (crate::CORPUS_NOW - ago) as u64;
        tip = Some(stream.commit_at_time(
            when,
            MAIN,
            message.as_bytes(),
            &parents,
            &[file(b"log.txt", content.as_bytes())],
        ));
    }
    site.import(stream)?;
    site.checkout_main()
}
