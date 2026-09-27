//! "Your commits" on a branch with no base of its own — `main` with no
//! upstream, or the only branch — are his unpushed commits, at most the
//! last 10, above any merge and the first commit (06c planner rulings,
//! Question 2 and `a`-into). Absorb and `a` share the one range.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeSet;

use graft_core::oid::Oid;
use graft_ops::absorb::{AbsorbMap, StaysWhy};
use graft_ops::facts::read_facts;
use graft_ops::operation::{FactsScope, Refusal};
use graft_ops::rewrite::{
    RewriteError, RewriteParams, RewriteRequest, TodoAction, absorb_plan_map, plan,
};

use crate::absorb::{branch, hunk, oid, stage};
use crate::support::{Never, TempRepo, git_version};

// ── Fixtures: one file per commit, so each line has one owner ───────────

/// A commit adding `<name>.txt` holding `<name>`.
pub(crate) fn commit(repo: &TempRepo, name: &str) {
    repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
    repo.commit_all(name);
}

/// `main`: `c1` … `c<n>`, no remote.
pub(crate) fn on_main(n: usize) -> TempRepo {
    let repo = TempRepo::init();
    for k in 1..=n {
        commit(&repo, &format!("c{k}"));
    }
    repo
}

/// [`on_main`] with `refs/remotes/origin/main` at `c<at>`: pushed that far,
/// no upstream set.
pub(crate) fn pushed_to(n: usize, at: usize) -> TempRepo {
    let repo = on_main(n);
    let pushed = oid(&repo, &format!("main~{}", n - at));
    repo.git(&[
        "update-ref",
        "refs/remotes/origin/main",
        &pushed.to_string(),
    ]);
    repo
}

/// `main`: `c1 c2`, `side` off `c2` with `s1`, `c3`, the merge `M` of
/// `side`, `c5 c6`. `M` is 3 back.
fn merge_three_back() -> TempRepo {
    let repo = on_main(2);
    repo.git(&["switch", "-q", "-c", "side"]);
    commit(&repo, "s1");
    repo.git(&["switch", "-q", "main"]);
    commit(&repo, "c3");
    repo.git(&["merge", "-q", "--no-ff", "-m", "M", "side"]);
    commit(&repo, "c5");
    commit(&repo, "c6");
    repo
}

/// `<name>.txt`'s one line, changed and staged: the commit `<name>` made
/// it, so it goes there if anywhere.
pub(crate) fn stage_line_of(repo: &TempRepo, name: &str) {
    stage(
        repo,
        &format!("{name}.txt"),
        format!("{name} changed\n").as_bytes(),
    );
}

pub(crate) fn absorb_map_of(repo: &TempRepo) -> Result<AbsorbMap, RewriteError> {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    absorb_plan_map(&handle, &facts, None, &Never)
}

pub(crate) fn planned(
    repo: &TempRepo,
    req: &RewriteRequest,
) -> Result<RewriteParams, RewriteError> {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    plan(&handle, &git_version(), req, &facts, None, &Never)
}

pub(crate) fn refused(result: Result<RewriteParams, RewriteError>) -> Refusal {
    match result {
        Err(RewriteError::Refused(refusal)) => refusal,
        other => panic!("refused, got {other:?}"),
    }
}

pub(crate) fn one_line(path: &str) -> graft_ops::absorb::StagedHunk {
    hunk(path, 1..2, 1..2)
}

// ── Absorb (Question 2) ─────────────────────────────────────────────────

#[test]
fn absorb_on_the_only_branch_places_into_its_commit() {
    // The review's probe: only `work`, no remote. Was NothingToAbsorb.
    let repo = TempRepo::init();
    repo.git(&["branch", "-m", "work"]);
    commit(&repo, "r0");
    repo.write("y.txt", b"y1\ny2\n");
    repo.commit_all("w1");
    commit(&repo, "w2");
    stage(&repo, "y.txt", b"y1\nY2\n");
    let w1 = oid(&repo, "work~1");

    let map = absorb_map_of(&repo).expect("it maps");
    let params = planned(&repo, &RewriteRequest::Absorb).expect("it plans");

    assert_eq!(map.placed, vec![(hunk("y.txt", 2..3, 2..3), w1)]);
    assert_eq!(params.onto, oid(&repo, "work~2"));
}

#[test]
fn absorb_on_main_stops_at_a_remote_ref() {
    let repo = pushed_to(6, 3);
    stage_line_of(&repo, "c5");
    stage_line_of(&repo, "c2");

    let map = absorb_map_of(&repo).expect("it maps");

    assert_eq!(map.placed, vec![(one_line("c5.txt"), oid(&repo, "main~1"))]);
    // 06e ruling: lines a remote ref holds stay as already pushed.
    assert_eq!(
        map.stays,
        vec![(one_line("c2.txt"), StaysWhy::AlreadyPushed)]
    );
}

#[test]
fn absorb_on_main_reaches_ten_back() {
    let repo = on_main(12);
    stage_line_of(&repo, "c3"); // 10 back, counting HEAD as 1
    stage_line_of(&repo, "c2"); // 11 back

    let map = absorb_map_of(&repo).expect("it maps");

    assert_eq!(map.placed, vec![(one_line("c3.txt"), oid(&repo, "main~9"))]);
    assert_eq!(
        map.stays,
        vec![(one_line("c2.txt"), StaysWhy::OutsideYourCommits)]
    );
}

#[test]
fn absorb_on_main_stops_above_a_merge() {
    let repo = merge_three_back();
    stage_line_of(&repo, "c5");
    stage_line_of(&repo, "c3");

    let map = absorb_map_of(&repo).expect("it maps");

    assert_eq!(map.placed, vec![(one_line("c5.txt"), oid(&repo, "main~1"))]);
    assert_eq!(
        map.stays,
        vec![(one_line("c3.txt"), StaysWhy::OutsideYourCommits)]
    );
}

#[test]
fn absorb_with_every_commit_pushed_is_refused_in_its_own_words() {
    let repo = pushed_to(3, 3);
    stage_line_of(&repo, "c2");

    let refusal = refused(planned(&repo, &RewriteRequest::Absorb));

    assert_eq!(
        refusal,
        Refusal::AllPushed {
            branch: "main".to_owned()
        }
    );
    assert_eq!(
        refusal.explain(),
        "every commit on main is already pushed · absorb only changes unpushed commits · \
         c commits them"
    );
}

#[test]
fn absorb_without_a_remote_keeps_before_your_branch() {
    // No remote at all: the base is where `feature` left `main`, and a line
    // older than it is "older than your branch" (06e ruling, stay rule 2).
    // The upstream case this test pinned before 06e is
    // `absorb_pushed::absorb_with_an_upstream_leaves_a_pushed_hunk_staged`.
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "old.txt", b"o1\nO2\no3\no4\no5\n");

    let map = absorb_map_of(&repo).expect("it maps");

    assert_eq!(
        map.placed,
        vec![(hunk("b.txt", 3..4, 3..4), oid(&repo, "feature~1"))]
    );
    assert_eq!(
        map.stays,
        vec![(hunk("old.txt", 2..3, 2..3), StaysWhy::BeforeYourBranch)]
    );
}

// ── `a`-into on a branch with no upstream ───────────────────────────────

#[test]
fn a_into_on_main_without_upstream_takes_an_unpushed_commit() {
    let repo = on_main(4);
    stage_line_of(&repo, "c3");
    let target = oid(&repo, "main~1");

    let params = planned(&repo, &RewriteRequest::AmendInto { target }).expect("it plans");

    let at = params
        .todo
        .iter()
        .position(|line| line.commit == target)
        .expect("the target is a row");
    assert_eq!(params.todo[at + 1].action, TodoAction::Fixup);
}

#[test]
fn a_into_below_a_remote_ref_is_already_pushed() {
    let repo = pushed_to(6, 3);
    stage_line_of(&repo, "c5");
    let c5 = oid(&repo, "main~1");
    let c2 = oid(&repo, "main~4");

    planned(&repo, &RewriteRequest::AmendInto { target: c5 }).expect("c5 plans");
    let refusal = refused(planned(&repo, &RewriteRequest::AmendInto { target: c2 }));

    assert_eq!(refusal, Refusal::AlreadyPushed { commit: c2 });
    assert_eq!(
        refusal.explain(),
        format!(
            "{} is already pushed · a only changes older commits you haven't pushed",
            &c2.to_string()[..7]
        )
    );
}

#[test]
fn beyond_reach_names_the_branch() {
    // 06e ruling test 8 (was `a_into_eleven_back_is_beyond_reach`): the
    // fallback runs only on the default or only branch, so it names it.
    let repo = on_main(12);
    stage_line_of(&repo, "c3");
    let ten_back = oid(&repo, "main~9");
    let eleven_back = oid(&repo, "main~10");

    planned(&repo, &RewriteRequest::AmendInto { target: ten_back }).expect("10 back plans");
    let refusal = refused(planned(
        &repo,
        &RewriteRequest::AmendInto {
            target: eleven_back,
        },
    ));

    assert_eq!(
        refusal,
        Refusal::BeyondReach {
            commit: eleven_back,
            branch: "main".to_owned()
        }
    );
    assert_eq!(
        refusal.explain(),
        format!(
            "{} is more than 10 commits back · on main a reaches the last 10 unpushed commits",
            &eleven_back.to_string()[..7]
        )
    );
}

#[test]
fn a_into_below_a_merge_is_merge_in_range() {
    let repo = merge_three_back();
    stage_line_of(&repo, "c3");
    let merge = oid(&repo, "main~2");

    for below in ["main~3", "side"] {
        let target = oid(&repo, below);
        let refusal = refused(planned(&repo, &RewriteRequest::AmendInto { target }));
        assert_eq!(refusal, Refusal::MergeInRange { merge }, "{below}");
    }
}

// `a_into_with_an_upstream_is_unchanged` pinned `a` planning into a commit
// the upstream holds; the 06e ruling removes that behaviour (a spec change,
// not a disabled test). Its replacement is
// `absorb_pushed::a_into_a_pushed_commit_with_an_upstream_is_already_pushed`.

// ── One range for both ──────────────────────────────────────────────────

/// The commits absorb places a staged line into, with every commit's own
/// line staged; and the commits `a` plans into. They must be one set, and
/// it is returned. `HEAD` is set aside: `a` there is amend, whatever the
/// range (06e ruling 5). A refused absorb places into nothing.
pub(crate) fn both_reach_the_same(repo: &TempRepo) -> BTreeSet<Oid> {
    let head = oid(repo, "HEAD");
    let every: Vec<Oid> = repo
        .git(&["rev-list", "HEAD"])
        .lines()
        .map(|hex| Oid::from_hex(hex).expect("an id"))
        .filter(|&commit| commit != head)
        .collect();
    for name in repo.git(&["ls-files"]).lines() {
        let name = name.strip_suffix(".txt").expect("one file per commit");
        stage_line_of(repo, name);
    }
    let placed = match absorb_map_of(repo) {
        Ok(map) => map.placed,
        Err(RewriteError::Refused(_)) => Vec::new(),
        Err(other) => panic!("{other:?}"),
    };
    let absorbed: BTreeSet<Oid> = placed
        .into_iter()
        .map(|(_, target)| target)
        .filter(|&target| target != head)
        .collect();
    let amended: BTreeSet<Oid> = every
        .into_iter()
        .filter(
            |&target| match planned(repo, &RewriteRequest::AmendInto { target }) {
                Ok(_) => true,
                Err(RewriteError::Refused(_)) => false,
                Err(other) => panic!("{other:?}"),
            },
        )
        .collect();
    assert_eq!(absorbed, amended);
    absorbed
}

// Minor 6: one agreement test per fixture, each with the count it reaches.

#[test]
fn absorb_and_a_agree_on_main_without_a_remote() {
    // c2 c3; c1 is the first commit, c4 is HEAD.
    assert_eq!(both_reach_the_same(&on_main(4)).len(), 2);
}

#[test]
fn absorb_and_a_agree_on_main_pushed_part_way() {
    // c4 c5; c6 is HEAD, c3 and below are pushed.
    assert_eq!(both_reach_the_same(&pushed_to(6, 3)).len(), 2);
}

#[test]
fn absorb_and_a_agree_on_main_past_ten_back() {
    // The 10 back, HEAD aside.
    assert_eq!(both_reach_the_same(&on_main(12)).len(), 9);
}

#[test]
fn absorb_and_a_agree_above_a_merge() {
    // c5; c6 is HEAD, M is the merge.
    assert_eq!(both_reach_the_same(&merge_three_back()).len(), 1);
}
