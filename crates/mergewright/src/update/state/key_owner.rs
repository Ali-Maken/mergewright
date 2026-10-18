//! Who owns the keyboard and the bottom row right now: quitting, text
//! fields, modal boxes, and which status line is drawn.

use super::{AppState, Status};

impl AppState {
    /// Is the loop finished?
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit.is_some()
    }

    /// Seeds the process's keymap and his config's problems, and stands the
    /// bottom row's `config: N problems · ? shows them` notice up when there
    /// are any (ADR-0055). Startup's, once per repository screen.
    pub(crate) fn seed_keys(
        &mut self,
        keymap: std::sync::Arc<mergewright_ui::keymap::Keymap>,
        problems: Vec<String>,
    ) {
        self.keymap = keymap;
        self.config_notice = !problems.is_empty();
        self.config_problems = problems;
    }

    /// Is a text field taking the keyboard right now?
    ///
    /// Every global binding — `u`, `U`, `Ctrl+Z`, `Esc` on a failure — asks
    /// this before it fires. A letter typed into a field must reach the
    /// field, not a keybinding: spelling `update` as a stash message once
    /// launched an undo at the `u` (m3/06 review, Critical 1). Structural
    /// ordering was the old answer and it broke the moment a fourth text
    /// field was added below the undo check, so the rule is a predicate
    /// naming every field instead. **A new text field belongs in this list.**
    #[must_use]
    pub fn text_field_open(&self) -> bool {
        self.search_prompt.is_some()
            || self.palette.is_some()
            || crate::update::place::prompt_open(self)
            || crate::update::journal::find::typing(self)
            || self.commit_editor.is_some()
            || self.stash.prompt.is_some()
            || self.credential.is_some()
            || self.name_prompt.is_some()
            || crate::update::picker::shown(self)
            || crate::update::rewrite_editor::field_open(self)
    }

    /// Is a modal pane or box taking the keyboard right now
    /// (`docs/design/02-keymap.md` §2.1)?
    ///
    /// The graph's `/`, `n` and `N` ask this before the search answers
    /// them: the search used to run ahead of the whole modal chain, so a
    /// `/` typed into a commit subject opened an invisible prompt and took
    /// the rest of the line with it (review m6-07, Major 1). Every text
    /// field, both boxes, the resolver and the focused banner: the panes
    /// that claim every key but `Ctrl+C`. **A new one belongs in this
    /// list**; `update::search::tests::slash_belongs_to_every_modal_that_is_up`
    /// sweeps it.
    ///
    /// Not the Map, journal or stash overlay: each claims only the keys it
    /// shows (§2.1), and this also drives the banner tab and the bottom-row
    /// rungs. The graph's search stands down under them by its own rule —
    /// it answers only while the graph is what's shown
    /// (`update::search::graph_is_shown`, m8/09e). J's own find answers
    /// `/`, `n`, `N` first with graph focus
    /// (`update::journal::find::find_key`, keymap §2.11).
    #[must_use]
    pub fn modal_owns_keys(&self) -> bool {
        self.text_field_open()
            || self.confirm.is_some()
            || self.preview.is_some()
            || self.resolver.is_some()
            || self.finish.is_some()
            || self.rewrite.editor.is_some()
            || crate::update::place::owns_keys(self)
            || (self.banner_focus && crate::update::banner::banner_view(self).is_some())
    }

    /// True while a modal stands over the bottom row: the credential
    /// prompt, the name prompt, a confirm box (the stash drop box and the
    /// banner's abort box among them), or the Ghost Preview. Every rung of
    /// the bottom row asks it before drawing a status: the modal owns the
    /// row, so a status from before it — a push rejection, a stale `Busy`, a
    /// `Done` — never sits under keys it does not belong to (m6-conflicts/69,
    /// Decided 1). It **hides**; it never clears: `esc` on the modal draws
    /// the status again (Decided 2). On the state, not in `view`, because
    /// `update` asks it too: nothing is acknowledged, and no failure let go,
    /// while the row cannot show it (m6-conflicts/78).
    #[must_use]
    pub(crate) fn modal_owns_bottom_row(&self) -> bool {
        self.credential.is_some()
            || self.name_prompt.is_some()
            || self.confirm.is_some()
            || self.preview.is_some()
    }

    /// What the status rungs draw once no modal owns the row: a failure
    /// that arrived under a box and was replaced before the row drew it
    /// (`unseen`), while it stands; else the wait's own line while Mergewright
    /// waits to quit (`quit_when_done`, m8/13b: drawn over `status`, so no
    /// progress or busy line the write sends can replace it, and it is gone
    /// the moment the wait is); else `status` (m6-conflicts/78).
    #[must_use]
    pub(crate) fn shown_status(&self) -> &Status {
        self.unseen
            .as_ref()
            .or_else(|| crate::update::quit::waiting(self).map(|wait| &wait.line))
            .unwrap_or(&self.status)
    }
}
