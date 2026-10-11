//! The typed fields' fixed keys and the branch picker's two bars. Moved out
//! of `hints/tests.rs` (m8/47) to keep that file under its size row.

use super::*;

/// Ruling R4 (ticket m8/10), as the code stands: a `Fixed` key — spelled
/// from the shipped table whatever his config says — is drawn only where
/// the edge passes every press as typed, so the default *is* the key that
/// acts. Those are the fields `graft::update::state::AppState::text_field_open`
/// names (state.rs: the name prompt, the credential box, the picker, the
/// rewrite editor's `r` field, and the finish summary while its focus is
/// not `Files`); `graft`'s `x_typed_into_every_field_is_an_x_with_merge_on_x`
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

/// m8 usability Q2: on HEAD's own row the picker's `⏎` closes it, so the
/// bar says `close`; on any other row it still says `switch to it`.
#[test]
fn picker_on_current_offers_close() {
    let verbs = |context: &HintContext| -> Vec<&'static str> {
        hints_for(context, Divider::Hidden, 200)
            .iter()
            .map(|hint| hint.verb)
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
    assert_eq!(
        verbs(&HintContext::PickerOnCurrent),
        ["to find", "close", "close", "show it on the graph", "row"]
    );
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
    assert!(text.contains("⏎ close"), "{text}");
    assert!(!text.contains("switch to it"), "{text}");
}
