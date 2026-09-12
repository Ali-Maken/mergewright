//! The merge zoo. Every one of these is a shape a graph renderer gets wrong
//! in its own way: more than two parents, two branches that merged each other,
//! a first parent that is not the mainline, a merge that runs backwards, a
//! merge that changed something, and the pull that nobody meant to make.

use super::{BuildError, MAIN, REMOTE_URL, Site, file};
use crate::stream::Stream;

pub(super) fn octopus_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let topic_one = stream.commit(
        b"refs/heads/topic-1",
        b"topic 1\n",
        &[base],
        &[file(b"t1.txt", b"1\n")],
    );
    let topic_two = stream.commit(
        b"refs/heads/topic-2",
        b"topic 2\n",
        &[base],
        &[file(b"t2.txt", b"2\n")],
    );
    let topic_three = stream.commit(
        b"refs/heads/topic-3",
        b"topic 3\n",
        &[base],
        &[file(b"t3.txt", b"3\n")],
    );
    stream.commit(
        MAIN,
        b"Merge branches 'topic-1', 'topic-2' and 'topic-3'\n",
        &[base, topic_one, topic_two, topic_three],
        &[
            file(b"t1.txt", b"1\n"),
            file(b"t2.txt", b"2\n"),
            file(b"t3.txt", b"3\n"),
        ],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn criss_cross_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let alpha_start = stream.commit(
        b"refs/heads/alpha",
        b"alpha: work\n",
        &[base],
        &[file(b"alpha.txt", b"a\n")],
    );
    let beta_start = stream.commit(
        b"refs/heads/beta",
        b"beta: work\n",
        &[base],
        &[file(b"beta.txt", b"b\n")],
    );
    // Each branch merges the other, so the two tips have two merge bases and
    // no single "the" merge base.
    let alpha_merge = stream.commit(
        b"refs/heads/alpha",
        b"alpha: merge beta\n",
        &[alpha_start, beta_start],
        &[file(b"beta.txt", b"b\n")],
    );
    let beta_merge = stream.commit(
        b"refs/heads/beta",
        b"beta: merge alpha\n",
        &[beta_start, alpha_start],
        &[file(b"alpha.txt", b"a\n")],
    );
    stream.commit(
        MAIN,
        b"Merge branch 'alpha'\n",
        &[base, alpha_merge],
        &[file(b"alpha.txt", b"a\n"), file(b"beta.txt", b"b\n")],
    );
    stream.commit(
        b"refs/heads/beta",
        b"beta: carry on\n",
        &[beta_merge],
        &[file(b"beta.txt", b"b2\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn foxtrot_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let main_tip = stream.commit(
        MAIN,
        b"Advance main\n",
        &[base],
        &[file(b"main.txt", b"m\n")],
    );
    let feature_tip = stream.commit(
        b"refs/heads/feature",
        b"feature: work\n",
        &[base],
        &[file(b"feature.txt", b"f\n")],
    );
    // The foxtrot: the merge lands on `main`, but its *first* parent is the
    // feature branch, so `--first-parent main` walks off the default branch.
    stream.commit(
        MAIN,
        b"Merge branch 'main' into feature\n",
        &[feature_tip, main_tip],
        &[file(b"main.txt", b"m\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn backward_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let main_tip = stream.commit(
        MAIN,
        b"Advance main\n",
        &[base],
        &[file(b"main.txt", b"m\n")],
    );
    let feature_tip = stream.commit(
        b"refs/heads/feature",
        b"feature: work\n",
        &[base],
        &[file(b"feature.txt", b"f\n")],
    );
    // The default branch merged *into* the feature branch, and never back.
    stream.commit(
        b"refs/heads/feature",
        b"Merge branch 'main' into feature\n",
        &[feature_tip, main_tip],
        &[file(b"main.txt", b"m\n")],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn evil_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let side_tip = stream.commit(
        b"refs/heads/alpha",
        b"alpha: work\n",
        &[base],
        &[file(b"alpha.txt", b"a\n")],
    );
    let main_tip = stream.commit(
        MAIN,
        b"Advance main\n",
        &[base],
        &[file(b"main.txt", b"m\n")],
    );
    // `evil.txt` is in neither parent: the merge's tree is not the mechanical
    // merge of the two sides.
    stream.commit(
        MAIN,
        b"Merge branch 'alpha'\n",
        &[main_tip, side_tip],
        &[
            file(b"alpha.txt", b"a\n"),
            file(b"evil.txt", b"introduced by the merge alone\n"),
        ],
    );
    site.import(stream)?;
    site.checkout_main()
}

pub(super) fn accidental_pull_merge(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let base = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    let theirs = stream.commit(
        b"refs/remotes/origin/main",
        b"Their commit\n",
        &[base],
        &[file(b"theirs.txt", b"t\n")],
    );
    let mine = stream.commit(MAIN, b"My commit\n", &[base], &[file(b"mine.txt", b"m\n")]);
    // Exactly what `git pull` writes when a push was rejected.
    let message = format!("Merge branch 'main' of {REMOTE_URL} into main\n");
    stream.commit(
        MAIN,
        message.as_bytes(),
        &[mine, theirs],
        &[file(b"theirs.txt", b"t\n")],
    );
    site.import(stream)?;
    site.git(&["remote", "add", "origin", REMOTE_URL])?;
    site.checkout_main()
}
