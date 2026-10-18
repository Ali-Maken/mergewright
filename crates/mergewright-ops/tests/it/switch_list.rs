//! The branch picker's rows (m7-power-tools/10c): what `switch_list` lists,
//! in what order, and how `filter_switch_rows` ranks a query. Repos are real;
//! remote-tracking refs are written with `update-ref` under a remote added
//! with `git remote add` (no fetch needed: the rows read refs, not remotes).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::run::Stop;
use mergewright_ops::switch_list::{
    Relation, SwitchKind, SwitchList, SwitchRow, filter_switch_rows, switch_list,
};

use crate::support::{Never, TempRepo};

fn names(rows: &[SwitchRow]) -> Vec<String> {
    rows.iter()
        .map(|row| String::from_utf8_lossy(row.name.as_bytes()).into_owned())
        .collect()
}

/// `main` with one commit, branches `feature-a` and `login` at it.
fn repo() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("first");
    repo.git(&["branch", "feature-a"]);
    repo.git(&["branch", "login"]);
    repo
}

fn remote_ref(repo: &TempRepo, name: &str) {
    let tip = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["update-ref", name, tip.trim()]);
}

#[test]
fn switch_list_puts_the_previous_branch_first() {
    let repo = repo();
    repo.git(&["switch", "-q", "feature-a"]);
    repo.git(&["switch", "-q", "main"]);
    let list = switch_list(&repo.open(), &Never).unwrap();
    let previous = list.previous.as_ref().expect("feature-a was before");
    assert_eq!(previous.name.as_bytes(), b"refs/heads/feature-a");
    assert!(
        !names(&list.rows).contains(&"refs/heads/feature-a".to_owned()),
        "shown once"
    );
    let drawn = filter_switch_rows(&list, "");
    assert_eq!(drawn[0].name.as_bytes(), b"refs/heads/feature-a");
}

#[test]
fn a_remote_with_a_local_copy_shows_once() {
    let repo = repo();
    repo.git(&["remote", "add", "origin", "https://example.invalid/r.git"]);
    remote_ref(&repo, "refs/remotes/origin/shared");
    repo.git(&["branch", "mine"]);
    repo.git(&["branch", "--set-upstream-to=origin/shared", "mine"]);
    let list = switch_list(&repo.open(), &Never).unwrap();
    let names = names(&list.rows);
    assert!(
        !names.contains(&"refs/remotes/origin/shared".to_owned()),
        "{names:?}"
    );
    let mine = list
        .rows
        .iter()
        .find(|row| row.name.as_bytes() == b"refs/heads/mine")
        .unwrap();
    assert_eq!(
        mine.relation,
        Relation::Tracks(RefName::new("refs/remotes/origin/shared").unwrap())
    );
}

#[test]
fn a_remote_whose_name_a_local_branch_has_is_left_out() {
    let repo = repo();
    repo.git(&["remote", "add", "origin", "https://example.invalid/r.git"]);
    remote_ref(&repo, "refs/remotes/origin/login");
    let list = switch_list(&repo.open(), &Never).unwrap();
    assert!(!names(&list.rows).contains(&"refs/remotes/origin/login".to_owned()));
}

#[test]
fn remote_only_branches_are_listed_with_their_local_name() {
    let repo = repo();
    repo.git(&["remote", "add", "a", "https://example.invalid/a.git"]);
    // `remote add` refuses a name nested under another; config does not.
    repo.git(&["config", "remote.a/b.url", "https://example.invalid/ab.git"]);
    remote_ref(&repo, "refs/remotes/a/b/sara/login-page");
    remote_ref(&repo, "refs/remotes/nobody/x");
    let list = switch_list(&repo.open(), &Never).unwrap();
    let row = list
        .rows
        .iter()
        .find(|row| row.name.as_bytes() == b"refs/remotes/a/b/sara/login-page")
        .expect("listed");
    assert_eq!(
        row.kind,
        SwitchKind::RemoteOnly {
            remote: "a/b".to_owned(),
            local: "sara/login-page".to_owned()
        }
    );
    assert_eq!(row.relation, Relation::None);
    assert!(
        !names(&list.rows).contains(&"refs/remotes/nobody/x".to_owned()),
        "under no listed remote: left out"
    );
    // Remote-only rows come after every local one but before `here`.
    let kinds: Vec<bool> = list
        .rows
        .iter()
        .map(|r| matches!(r.kind, SwitchKind::Local))
        .collect();
    let first_remote = kinds.iter().position(|local| !local).unwrap();
    assert!(kinds[..first_remote].iter().all(|local| *local));
    assert!(list.rows.last().unwrap().current);
}

#[test]
fn an_upstream_under_heads_reads_built_on() {
    let repo = repo();
    repo.git(&["branch", "--set-upstream-to=feature-a", "login"]);
    let list = switch_list(&repo.open(), &Never).unwrap();
    let login = list
        .rows
        .iter()
        .find(|row| row.name.as_bytes() == b"refs/heads/login")
        .unwrap();
    assert_eq!(
        login.relation,
        Relation::BuiltOn(RefName::new("refs/heads/feature-a").unwrap())
    );
}

#[test]
fn here_is_last() {
    let repo = repo();
    let list = switch_list(&repo.open(), &Never).unwrap();
    let here = list.rows.last().unwrap();
    assert_eq!(here.name.as_bytes(), b"refs/heads/main");
    assert!(here.current);
    assert_eq!(list.rows.iter().filter(|row| row.current).count(), 1);
}

fn row(name: &str, when: i64) -> SwitchRow {
    SwitchRow {
        name: RefName::new(name).unwrap(),
        kind: SwitchKind::Local,
        tip: Oid::from_hex("1111111111111111111111111111111111111111").unwrap(),
        subject: String::new(),
        author: String::new(),
        when,
        relation: Relation::None,
        current: false,
    }
}

#[test]
fn filter_ranks_segment_prefix_above_substring() {
    let list = SwitchList {
        previous: Some(row("refs/heads/fix-blogin", 50)),
        rows: vec![
            row("refs/heads/xlogin", 300),
            row("refs/heads/sara/login-page", 100),
            row("refs/heads/LOGIN-old", 10),
            row("refs/heads/main", 999),
        ],
    };
    let ranked: Vec<_> = filter_switch_rows(&list, "Login")
        .into_iter()
        .map(|r| String::from_utf8_lossy(r.name.shorthand()).into_owned())
        .collect();
    assert_eq!(
        ranked,
        ["sara/login-page", "LOGIN-old", "xlogin", "fix-blogin"]
    );
    let all: Vec<_> = filter_switch_rows(&list, "")
        .into_iter()
        .map(|r| r.when)
        .collect();
    assert_eq!(all, [50, 300, 100, 10, 999], "empty query: the drawn order");
}

struct Always;

impl Stop for Always {
    fn should_stop(&self) -> bool {
        true
    }
}

#[test]
fn switch_list_honours_stop() {
    let repo = repo();
    let error = switch_list(&repo.open(), &Always).unwrap_err();
    assert!(
        matches!(error, mergewright_ops::facts::FactsError::Stopped),
        "{error:?}"
    );
}
