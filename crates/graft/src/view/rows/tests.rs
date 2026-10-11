use graft_term::sanitize_line;
use graft_ui::graph_screen::BannerView;

use super::*;

fn banner(focused: bool, conflict: Option<&str>) -> BannerView {
    BannerView {
        rewrite: None,
        kind: sanitize_line("MERGING"),
        step: None,
        conflict: conflict.map(sanitize_line),
        focused,
        skippable: false,
    }
}

/// A focused banner owns the keyboard, so the footer is the banner's —
/// whatever the cursor is sitting on. Untested until now: the tick for
/// it cited the hint-bar walk, which enumerates `HintContext` variants
/// and never calls this function, so a `footer_context` that ignored
/// `focused` passed every test in the crate (m5-09 review, Major 5).
#[test]
fn a_focused_banner_takes_the_footer() {
    let state = AppState::default();
    assert_eq!(
        footer_context(&state, &[], Some(&banner(true, Some("notes.md")))),
        HintContext::Banner {
            skippable: false,
            conflicted: true,
            merge: false,
        }
    );
}

/// An unfocused banner claims no key, so the cursor's own context
/// stands.
#[test]
fn an_unfocused_banner_leaves_the_footer_to_the_cursor() {
    let state = AppState::default();
    assert_eq!(
        footer_context(&state, &[], Some(&banner(false, Some("notes.md")))),
        hint_context(&state, &[]),
    );
}

/// m8 usability Q2: the picker's bar says `⏎ show it on the graph` on
/// HEAD's own row, `⏎ switch to it` on any other, and the latter while the
/// list loads.
#[test]
fn picker_bar_context_follows_the_row() {
    use graft_ops::switch_list::{Relation, SwitchKind, SwitchList, SwitchRow};
    let row = |full: &str, byte: u8, current: bool| SwitchRow {
        name: graft_core::ref_name::RefName::new(full.as_bytes().to_vec())
            .expect("a valid ref name"),
        kind: SwitchKind::Local,
        tip: graft_core::oid::Oid::from_bytes(&[byte; graft_core::oid::Oid::SHA1_BYTES])
            .expect("20 bytes is an oid"),
        subject: String::new(),
        author: String::new(),
        when: 0,
        relation: Relation::None,
        current,
    };
    let with = |list, selected| AppState {
        picker: Some(crate::update::picker::PickerState {
            list,
            selected,
            ..crate::update::picker::PickerState::default()
        }),
        ..AppState::default()
    };
    let ready = || {
        crate::update::detail::Loading::Ready(std::sync::Arc::new(SwitchList {
            previous: None,
            rows: vec![
                row("refs/heads/login-page", 3, false),
                row("refs/heads/main", 2, true),
            ],
        }))
    };
    assert_eq!(
        hint_context(&with(ready(), 1), &[]),
        HintContext::PickerOnCurrent
    );
    assert_eq!(hint_context(&with(ready(), 0), &[]), HintContext::Picker);
    assert_eq!(
        hint_context(
            &with(
                crate::update::detail::Loading::Pending(graft_core::job::JobId::new(1)),
                0
            ),
            &[]
        ),
        HintContext::Picker
    );
}

fn facts(path: &str, head: Head) -> RepositoryFacts {
    RepositoryFacts {
        path: PathBuf::from(path),
        bare: false,
        head,
        mailmap: graft_git::mailmap::Mailmap::default(),
        commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
    }
}

/// The header's location (ADR-0058 §4, J22): the repository's own
/// name, its path with the home directory written `~`, and the branch.
#[test]
fn the_location_names_the_repo_its_home_path_and_its_branch() {
    let home = Path::new("/work/ada");
    let at = location(
        &facts("/work/ada/code/api", Head::Branch(b"main".to_vec())),
        Some(home),
    );
    assert_eq!(at.repo.as_str(), "api");
    assert_eq!(at.path.as_str(), "~/code/api");
    assert_eq!(at.branch.as_str(), "main");
    assert!(!at.practice);

    // Outside the home, or with none, the path is shown whole.
    let away = location(
        &facts("/srv/api", Head::Unborn(b"trunk".to_vec())),
        Some(home),
    );
    assert_eq!(away.path.as_str(), "/srv/api");
    assert_eq!(away.branch.as_str(), "trunk");
    let homeless = location(&facts("/work/ada/api", Head::Unborn(b"x".to_vec())), None);
    assert_eq!(homeless.path.as_str(), "/work/ada/api");
    // A home of `/` would make every path `~/…`, which says nothing.
    let root = location(
        &facts("/srv/api", Head::Unborn(b"x".to_vec())),
        Some(Path::new("/")),
    );
    assert_eq!(root.path.as_str(), "/srv/api");
}

/// A detached `HEAD` says so, with seven characters of its id.
#[test]
fn a_detached_head_is_named_with_seven_characters_of_its_id() {
    let id = graft_core::oid::Oid::from_bytes(&[0xab; graft_core::oid::Oid::SHA1_BYTES])
        .expect("20 bytes");
    let at = location(&facts("/srv/api", Head::Detached(id)), None);
    assert_eq!(at.branch.as_str(), "detached abababa");
}

/// With nothing unmerged the banner's `M`/`e` refuse in words, so the
/// bar must not offer them: `conflicted` is what carries that.
#[test]
fn a_banner_with_no_conflict_says_so_in_its_context() {
    let state = AppState::default();
    assert_eq!(
        footer_context(&state, &[], Some(&banner(true, None))),
        HintContext::Banner {
            skippable: false,
            conflicted: false,
            merge: false,
        }
    );
}
