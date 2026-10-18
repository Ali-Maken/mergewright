//! The search's keys against every modal pane (m6-07 final fix, review
//! Major 1): `/`, `n` and `N` are the graph's, and a modal pane or box that
//! is up owns them — a text field most of all, where `/` is a character.

use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_term::sanitize_line;
use mergewright_ui::text_input::TextField;

use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::credential::{CredentialPrompt, PromptField};
use crate::update::name_prompt::{NameKind, NamePrompt};
use crate::update::stash::StashPrompt;
use crate::update::{AppEvent, AppState, update};

fn press(state: &AppState, code: KeyCode) -> AppState {
    update(state, AppEvent::Core(Event::Key(KeyPress::plain(code)))).0
}

/// Walk loaded, graph on screen: where a bare `/` opens the prompt.
fn on_graph() -> AppState {
    AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        ..AppState::default()
    }
}

/// A search from before the modal opened, still live: what `n`/`N` step.
fn with_a_live_search(mut state: AppState) -> AppState {
    let query = crate::search::Query::parse("a").expect("a non-empty query");
    let mut search = crate::search::Search::new(query, 0, (0, 0));
    let hit = |byte: u8| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes");
    search.batches = vec![std::sync::Arc::from(vec![hit(2), hit(4)])];
    search.state = crate::search::SearchState::Done { hits: 2 };
    state.search = Some(search);
    state
}

/// What a text field holds, read back after typing; `None` once it shut.
type Readback = fn(&AppState) -> Option<String>;

/// A modal by name, the state with it up, and — for a text field — how to
/// read back what it holds.
type Modal = (&'static str, AppState, Option<Readback>);

/// Every pane and box that claims every key but `Ctrl+C` (§2.1), each over
/// a loaded walk — the states in which `/` must not open a search behind
/// it. Not the Map, journal or stash overlay: each claims only the keys it
/// shows, and the bar behind each offers `/ search` (see
/// `AppState::modal_owns_keys`).
fn every_modal() -> Vec<Modal> {
    let mut modals = text_fields();
    modals.push(rewrite_field());
    modals.extend(boxes());
    modals
}

/// The modals that hold a text field: `/` there is a character.
fn text_fields() -> Vec<Modal> {
    let mut editor = on_graph();
    editor.commit_editor = Some(Box::new(crate::update::commit_editor::CommitEditor::new()));

    let mut stash = on_graph();
    stash.stash.shown = true;
    stash.stash.prompt = Some(StashPrompt {
        message: TextField::new(""),
        staged_only: false,
    });

    let mut name = on_graph();
    name.name_prompt = Some(NamePrompt {
        kind: NameKind::Tag {
            at: Oid::from_bytes(&[1; Oid::SHA1_BYTES]).expect("20 bytes"),
        },
        field: TextField::new(""),
        refusal: None,
    });

    let mut credential = on_graph();
    // A prompt is live only while its write runs (`close_if_orphaned`).
    credential.operating = Some(JobId::new(1));
    credential.credential = Some(CredentialPrompt {
        title: sanitize_line("Credentials — example.com"),
        prompt: sanitize_line("Username for 'https://example.com':"),
        field: PromptField::Open(TextField::new("")),
        job: JobId::new(1),
    });

    // The branch picker (m7-power-tools/10): the list need not have landed.
    let mut picker = on_graph();
    picker.picker = Some(crate::update::picker::PickerState::default());

    vec![
        (
            "the branch picker",
            picker,
            Some(|s| s.picker.as_ref().map(|p| p.query.text().to_owned())),
        ),
        (
            "the commit editor",
            editor,
            Some(|s| {
                s.commit_editor
                    .as_ref()
                    .map(|e| e.subject.text().to_owned())
            }),
        ),
        (
            "the stash message prompt",
            stash,
            Some(|s| s.stash.prompt.as_ref().map(|p| p.message.text().to_owned())),
        ),
        (
            "the name prompt",
            name,
            Some(|s| s.name_prompt.as_ref().map(|p| p.field.text().to_owned())),
        ),
        (
            "the credential prompt",
            credential,
            Some(|s| match s.credential.as_ref().map(|c| &c.field) {
                Some(PromptField::Open(field)) => Some(field.text().to_owned()),
                _ => None,
            }),
        ),
    ]
}

/// The rewrite editor's `r` field (m7-power-tools/08).
fn rewrite_field() -> Modal {
    let mut rewrite = on_graph();
    crate::update::rewrite_editor::tests::open_an_editor_field(&mut rewrite);
    (
        "the rewrite editor's message field",
        rewrite,
        Some(crate::update::rewrite_editor::tests::field_text),
    )
}

/// The modals with no text field: each shuts on a key it has no use for.
fn boxes() -> Vec<Modal> {
    let mut confirm = on_graph();
    confirm.confirm = Some(PendingConfirm {
        view: mergewright_ui::confirm::ConfirmView::new(
            sanitize_line("Discard changes?"),
            vec![sanitize_line("3 files lose their edits.")],
            sanitize_line("git restore ."),
        ),
        then: ConfirmThen::Undo { redo: false },
    });

    let (preview, _) =
        crate::update::preview::tests::opened_with(crate::update::preview::tests::a_plan());
    let resolver = crate::update::tests::resolver_on_screen(JobId::new(4_100));

    let mut banner = on_graph();
    banner.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind: mergewright_git::state::InProgress::Rebase,
        step: Some((1, 2)),
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    banner.banner_focus = true;

    // The rewrite editor (m7-power-tools/08): modal, no field until `r`.
    let mut rewrite = on_graph();
    crate::update::rewrite_editor::tests::open_an_editor(&mut rewrite);

    vec![
        ("the rewrite editor", rewrite, None),
        ("the confirm box", confirm, None),
        ("the Ghost Preview", preview, None),
        ("the conflict resolver", resolver, None),
        ("the focused operation banner", banner, None),
    ]
}

/// The control: on the bare graph `/` does open the prompt, so the sweep
/// below cannot pass on a state where `/` would have done nothing anyway.
#[test]
fn slash_opens_the_prompt_on_the_bare_graph() {
    assert_eq!(
        press(&on_graph(), KeyCode::Char('/'))
            .search_prompt
            .as_deref(),
        Some("")
    );
}

/// **`/` never opens a search behind a modal pane or box** — and in a text
/// field it is a character like any other: `a/b` typed into a commit
/// subject is the subject `a/b` (review m6-07, Major 1). One sweep over
/// every modal §2.1 names, not the one case the finding observed.
#[test]
fn slash_belongs_to_every_modal_that_is_up() {
    let mut wrong = Vec::new();
    for (name, state, text) in every_modal() {
        assert!(state.history.is_some(), "{name}: a walk is loaded");
        if press(&state, KeyCode::Char('/')).search_prompt.is_some() {
            wrong.push(format!("{name}: `/` opened the search prompt"));
        }
        // A box shuts on a key it has no use for, so only a field is
        // typed into past the first key.
        if let Some(text) = text {
            let after = [KeyCode::Char('a'), KeyCode::Char('/'), KeyCode::Char('b')]
                .into_iter()
                .fold(state, |state, code| press(&state, code));
            if text(&after).as_deref() != Some("a/b") || after.search_prompt.is_some() {
                wrong.push(format!(
                    "{name}: the field holds {:?}, not \"a/b\"",
                    text(&after)
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The control for the sweep below: on the bare graph `n` does step it.
#[test]
fn n_steps_a_live_search_on_the_bare_graph() {
    let state = with_a_live_search(on_graph());
    assert_eq!(
        press(&state, KeyCode::Char('n'))
            .search
            .and_then(|s| s.current),
        Some(0)
    );
}

/// With a search live from before, `n` and `N` step it only on the bare
/// graph: under a modal they are the modal's letters (a text field's
/// characters, the resolver's block walk), never a jump behind it.
#[test]
fn n_and_capital_n_never_step_a_search_behind_a_modal() {
    let mut wrong = Vec::new();
    for (name, state, _) in every_modal() {
        let state = with_a_live_search(state);
        for code in [KeyCode::Char('n'), KeyCode::Char('N')] {
            let after = press(&state, code);
            if after.search.is_some_and(|search| search.current.is_some()) {
                wrong.push(format!("{name}: {code:?} stepped the search behind it"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The three overlays the graph's search stands down under (09e): the frame
/// draws the overlay, not the graph screen, so a prompt opened there would
/// never be drawn (`view::frame` `draw_map`/`draw_stash`).
fn under_each_overlay(state: &AppState) -> Vec<(&'static str, AppState)> {
    let mut map = state.clone();
    map.map.shown = true;
    let mut journal = state.clone();
    journal.journal.shown = true;
    let mut stash = state.clone();
    stash.stash.shown = true;
    vec![("Map", map), ("journal", journal), ("stash list", stash)]
}

/// An unbound key's outcome: what `/` must equal where it stands down.
fn idle(state: &AppState) -> AppState {
    press(state, KeyCode::Char('\u{e000}'))
}

/// 09e Defects 1–2: under the Map `/` opens no search; the state is an
/// unbound key's.
#[test]
fn slash_under_the_map_opens_no_search() {
    let mut state = on_graph();
    state.map.shown = true;
    let after = press(&state, KeyCode::Char('/'));
    assert!(after.search_prompt.is_none());
    assert_eq!(after, idle(&state));
}

/// 09e Defects 1–2: the same under the stash list.
#[test]
fn slash_under_the_stash_list_opens_no_search() {
    let mut state = on_graph();
    state.stash.shown = true;
    let after = press(&state, KeyCode::Char('/'));
    assert!(after.search_prompt.is_none());
    assert_eq!(after, idle(&state));
}

/// 09e Defects 1–2: with the detail pane focused over any of the three
/// overlays, `/` opens no search either.
#[test]
fn slash_from_the_pane_over_an_overlay_opens_no_search() {
    let mut base = on_graph();
    base.detail.focus = mergewright_ui::detail::DetailFocus::Files;
    let mut wrong = Vec::new();
    for (name, state) in under_each_overlay(&base) {
        if press(&state, KeyCode::Char('/')).search_prompt.is_some() {
            wrong.push(name);
        }
    }
    assert!(wrong.is_empty(), "`/` opened the search under {wrong:?}");
}

/// 09e: a kept graph search does not step under an overlay (it would move
/// the cursor of a graph that isn't shown), and steps again once closed.
#[test]
fn n_under_an_overlay_leaves_the_graph_where_it_was() {
    let base = with_a_live_search(on_graph());
    for (name, state) in under_each_overlay(&base) {
        let after = press(&state, KeyCode::Char('n'));
        assert_eq!(
            after.search.as_ref().and_then(|s| s.current),
            None,
            "{name}: `n` stepped the hidden graph's search"
        );
        assert_eq!(
            after.history.as_ref().map(|h| h.cursor),
            state.history.as_ref().map(|h| h.cursor),
            "{name}: the hidden cursor moved"
        );
    }
    assert_eq!(
        press(&base, KeyCode::Char('n'))
            .search
            .and_then(|s| s.current),
        Some(0),
        "closed, `n` steps"
    );
}

mod folded;
mod reveal;
