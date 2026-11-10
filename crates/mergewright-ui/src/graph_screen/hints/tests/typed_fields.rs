//! The typed fields' fixed keys and the branch picker's two bars. Moved out
//! of `hints/tests.rs` (m8/47) to keep that file under its size row, with ease-pass/27's resolver-bar test.

use super::*;

/// Ruling R4 (ticket m8/10), as the code stands: a `Fixed` key — spelled
/// from the shipped table whatever his config says — is drawn only where
/// the edge passes every press as typed, so the default *is* the key that
/// acts. Those are the fields `mergewright::update::state::AppState::text_field_open`
/// names (state.rs: the name prompt, the credential box, the picker, the
/// rewrite editor's `r` field, and the finish summary while its focus is
/// not `Files`); `mergewright`'s `x_typed_into_every_field_is_an_x_with_merge_on_x`
/// holds each of them to it. Everywhere else a key follows his remap.
///
/// Not R4's letter: R4 has the finish Subject/Body follow a `Finish` row,
/// on the premise that his remap acts there. It does not — `edge::key`
/// returns before `context_of` while `text_field_open` holds, and that
/// includes the summary's Subject and Body — so a remapped spelling there
/// would draw a key that does nothing (ticket 10c NOTES).
#[test]
fn a_fixed_key_is_drawn_only_where_keys_go_on_as_typed() {
    let typed = |context: &HintContext| match context {
        HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Picker
        | HintContext::PickerOnCurrent
        | HintContext::RewriteMessage => true,
        HintContext::Finish { focus, .. } => *focus != SummaryFocus::Files,
        _ => false,
    };
    let mut fixed_drawn = 0;
    for context in every_context() {
        for divider in [Divider::Shown, Divider::Hidden] {
            let (list, door) = spelled_bar(&context, divider, Keymap::shipped());
            for hint in list.iter().chain(door.iter()) {
                if !hint.keys.iter().any(|key| matches!(key, HintKey::Fixed(_))) {
                    continue;
                }
                fixed_drawn += 1;
                assert!(
                    typed(&context),
                    "{context:?} draws `{} {}` from the shipped table, but his remap acts there",
                    hint.key(),
                    hint.verb
                );
            }
        }
    }
    assert!(fixed_drawn > 0, "no list draws a fixed key");
}

/// m8 usability Q2: on HEAD's own row the picker's `⏎` does what `⇥` does
/// (`update::picker` calls `show_on_graph`), so its clause is `⇥`'s words —
/// not `close`, `esc`'s word for a different act (review m8 g1, minor 1).
/// On any other row it still says `switch to it`.
#[test]
fn picker_on_current_enter_says_what_tab_says() {
    let verbs = |context: &HintContext| -> Vec<String> {
        hints_for(context, Divider::Hidden, 200)
            .iter()
            .map(|hint| hint.verb.to_string())
            .collect()
    };
    assert_eq!(
        verbs(&HintContext::Picker),
        [
            "to find",
            "switch to it",
            "close",
            "show it on the graph",
            "row"
        ]
    );
    let on_current = verbs(&HintContext::PickerOnCurrent);
    assert_eq!(
        on_current,
        [
            "to find",
            "show it on the graph",
            "close",
            "show it on the graph",
            "row"
        ]
    );
    // `⏎` and `⇥` are one act; every other pair of clauses shares no word.
    let distinct: Vec<&str> = [0, 2, 3, 4]
        .iter()
        .map(|at| on_current[*at].as_str())
        .collect();
    for (at, left) in distinct.iter().enumerate() {
        for right in &distinct[at + 1..] {
            let shared = left
                .split(' ')
                .find(|word| right.split(' ').any(|other| other == *word));
            assert_eq!(shared, None, "{left:?} and {right:?}");
        }
    }
    let enter = |context: &HintContext| hints_for(context, Divider::Hidden, 200)[1].keys;
    assert_eq!(
        enter(&HintContext::PickerOnCurrent),
        enter(&HintContext::Picker),
        "same key, same action id"
    );
    let text = hints_text(&hints_for(
        &HintContext::PickerOnCurrent,
        Divider::Hidden,
        200,
    ));
    assert!(text.contains("⏎ show it on the graph"), "{text}");
    assert!(!text.contains("⏎ close"), "{text}");
    assert!(!text.contains("switch to it"), "{text}");
}

/// ease-pass/27 item 3: the resolver bar at 120 columns with `B base` and
/// `d start over` both offered keeps the words of `d` and `esc` (the keys
/// the user reaches for last), whatever it shortens earlier in the row.
#[test]
fn resolver_bar_at_120_with_base_keeps_the_words_of_d_and_esc() {
    let hints = hints_for(&document(true, true), Divider::Hidden, 120);
    let text = hints_text(&hints);
    assert!(text.contains("B base"), "`B base` is drawn: {text}");
    assert!(text.ends_with("d start over  esc close"), "{text}");
}
