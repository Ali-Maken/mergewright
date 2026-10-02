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

/// How many feature branches `many_lanes` keeps open at once.
pub(super) const MANY_LANES_OPEN: u32 = 14;

/// More lanes open at once than any window draws: compaction's corpus shape
/// (m8-polish-and-ship/22). `main` gets one commit per branch; branch
/// `lane-N` forks off `main`'s Nth commit and gets two commits, all of them
/// after every `main` commit, so every branch runs down past the others to
/// its fork point and all `MANY_LANES_OPEN` lanes are live on the top rows.
/// The last two branches are then merged into `main`, so the top of the
/// graph also closes lanes into lane 0.
pub(super) fn many_lanes(site: &Site<'_>) -> Result<(), BuildError> {
    let mut stream = Stream::new();
    let mut spine = Vec::new();
    let mut tip = stream.commit(MAIN, b"Add the base\n", &[], &[file(b"base.txt", b"0\n")]);
    spine.push(tip);
    for n in 1..MANY_LANES_OPEN {
        let content = format!("{n}\n");
        tip = stream.commit(
            MAIN,
            format!("main {n}\n").as_bytes(),
            &[tip],
            &[file(b"main.txt", content.as_bytes())],
        );
        spine.push(tip);
    }
    let mut lane_tips = Vec::new();
    for (n, &fork) in spine.iter().enumerate() {
        let refname = format!("refs/heads/lane-{n:02}");
        let path = format!("lane-{n:02}.txt");
        let first = stream.commit(
            refname.as_bytes(),
            format!("lane {n}: start\n").as_bytes(),
            &[fork],
            &[file(path.as_bytes(), b"1\n")],
        );
        lane_tips.push(stream.commit(
            refname.as_bytes(),
            format!("lane {n}: finish\n").as_bytes(),
            &[first],
            &[file(path.as_bytes(), b"2\n")],
        ));
    }
    // A clean merge: the tree takes the branch's one file, as git would.
    for (n, &merged) in lane_tips.iter().enumerate().skip(lane_tips.len() - 2) {
        let path = format!("lane-{n:02}.txt");
        tip = stream.commit(
            MAIN,
            format!("Merge branch 'lane-{n:02}'\n").as_bytes(),
            &[tip, merged],
            &[file(path.as_bytes(), b"2\n")],
        );
    }
    site.import(stream)?;
    site.checkout_main()
}
