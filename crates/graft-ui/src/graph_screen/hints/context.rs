//! [`HintContext`]: where the cursor or focus is, which picks the list.

use crate::conflict_resolver::SideRoles;

/// Where the cursor or focus is, for the footer's hint list. `Preview` is
/// the one context allowed an empty list (the Ghost Preview box carries its
/// own footer); every other variant's list is non-empty by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HintContext {
    /// The cursor is on the collapsed Uncommitted-changes row.
    WorkingTreeRow,
    /// The cursor is on a row with a local branch badge; `current` is the
    /// branch `HEAD` points at.
    BranchRow {
        /// The badge is the current branch (`HEAD`).
        current: bool,
    },
    /// The cursor is on a row with no branch badge.
    PlainRow,
    /// The cursor row carries a Mistake Radar marker: `. explain` leads,
    /// then the inner context's own list, `. more` omitted (`.` is already
    /// shown).
    Marked(Box<HintContext>),
    /// The operation banner has focus (a conflict just landed, or the user
    /// tabbed to it); `skippable` is false for a plain merge, which has no
    /// `s` skip.
    Banner {
        /// `s` (skip) applies — a rebase/cherry-pick/revert sequence, not a
        /// plain merge.
        skippable: bool,
        /// A file is conflicted, so `M mergetool` and `e edit` have
        /// something to open. With none, both refuse in words
        /// (`graft::update::banner::banner_key`) and the bar leaves them
        /// out rather than teaching a key that cannot act (m5-09 review,
        /// Major 2).
        conflicted: bool,
    },
    /// The Ghost Preview box has focus: its own border draws the footer,
    /// so the main footer's list is empty.
    Preview,
    /// The changes pane (working tree, not a commit) has focus.
    ChangesPane {
        /// The file under the cursor is unmerged (conflicted): the keys
        /// that write swap for the conflict-resolution keys.
        unmerged: bool,
    },
    /// The undo journal browser has focus (overlay; draws its own footer
    /// today — reserved for the `?` help overlay, M8).
    Journal,
    /// The stash list has focus (overlay; draws its own footer today —
    /// reserved for the `?` help overlay, M8).
    Stash,
    /// The Map view has focus (overlay; draws its own footer today, which
    /// carries no keys — `docs/design/02-keymap.md` §2.3a is where this
    /// list comes from).
    Map,
    /// **The detail pane has the keyboard with an overlay, not the graph,
    /// standing behind it** (m5/42).
    ///
    /// [`HintContext::PaneFocused`] borrows the cursor row's list because
    /// the graph is still underneath and still answers those keys. Under an
    /// overlay it is not: the Map view, the journal browser and the stash
    /// list each swallow every key their own `shown_key` does not use, so
    /// `b`, `t`, `c`, `m`, `r`, `y`, `q`, `x`, `+`, `.` and the rest of the
    /// graph's vocabulary reach nothing at all. Borrowing the row's list
    /// here drew a bar on which every verb was dead (`graft`'s
    /// `update::tests::the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows`).
    ///
    /// What is left is this list: the pane's own `⏎` and `j`/`k`, and the
    /// two verbs dispatched **before** the panes and so still live under an
    /// overlay — `u undo` and `/ search` (`graft::update::keys::key_event`,
    /// which answers both above `pane_and_overlay_key`).
    ///
    /// The overlay's *own* verbs are not here either. `⏎` and `Esc` never
    /// reach it while the pane has focus — `detail::keys::detail_key` runs
    /// first and spends them opening the diff and closing the pane — so
    /// `⏎ apply` would name a key that does something else, which is the
    /// failure this context exists to end rather than repeat. The rest of
    /// the overlay's list acts on a list the Detail tab is not showing.
    PaneOverOverlay {
        /// `⏎` still opens something: the files list has focus, so it opens
        /// the diff. `false` with the diff already focused, exactly as
        /// [`HintContext::PaneFocused`] reads it.
        opens: bool,
    },
    /// The conflict resolver has the keyboard (M6, `docs/design-notes/m6-conflicts.md`
    /// "New UI"): modal, its own whole list, no `. more`.
    ///
    /// **Live since m6-conflicts/04**, which built `update::resolver`: the
    /// resolver is an `AppState::resolver` and a `GraphOverlay::Resolver`,
    /// and `resolver_key` answers every key [`resolver_list`] draws, `B`
    /// and `d` included since m6-conflicts/05.
    Resolver {
        /// `B` show/hide base (Proposed change 2b), live since m6-conflicts/05.
        base: bool,
        /// `d` start this file over (Proposed change 2c) — same.
        restart: bool,
        /// The open file is a whole-file card (`deleted by them`, a binary
        /// `both modified`, …): no blocks, so no `b`, `N`, `⏎`, `B` or `d`
        /// — `o`/`t` resolve the path whole, and `n` crosses to the next
        /// conflicted file (m6-05 review, Major 1). `base`/`restart` are
        /// not read while this is set.
        whole_file: bool,
        /// The operation's role words: `o`/`t` read as the sides' roles
        /// (`o yours  t theirs`; rebase `o upstream  t yours`), never as a
        /// branch name or sha (ADR-0043 §4).
        roles: SideRoles,
    },
    /// A branch badge is being dragged (`docs/design/02-keymap.md` §2.15).
    /// The gesture ends with the button, not a key, so the bar says what
    /// the pointer must do and which key backs out.
    Dragging,
    /// **A confirm box is live** (rulings 07b, Q1): it claims the next key
    /// before any screen does (`graft::update::confirm::confirm_key`), so
    /// the bar under it offers its keys and no other. `←`/`→` (and `⇥`)
    /// move between the choices, `⏎` takes the one under the cursor, and
    /// every other key cancels — `esc` is the one the bar names.
    ConfirmBox,
    /// **The name prompt is live** (same ruling): `⏎` creates, `esc`
    /// cancels, and `⇥` flips "check out after" for a branch only — for a
    /// tag or a stash branch the key is swallowed, so `toggle` is `false`
    /// and it is not drawn. The box's own bottom border says the same.
    NamePrompt {
        /// `⇥` toggles check-out: a plain branch, not a tag.
        toggle: bool,
    },
    /// **git's credential question is up** (ADR-0040): `⏎` sends, `esc`
    /// cancels the write. Usually hidden behind the notice the running
    /// write puts on the same row (`push main → origin/main · esc
    /// cancels`), which the status ladder draws first.
    Credential,
    /// The cursor row's own context, drawn while the **detail pane has the
    /// keyboard and nothing left to open** — the same list, minus `⏎`.
    ///
    /// A filter, not a list of its own: no key is invented here and none is
    /// bound (`docs/design/02-keymap.md` governs binding; hiding a key it
    /// already names needs no allocation). `. more` still leads to every
    /// verb the width dropped, so the key is one press away rather than
    /// gone.
    ///
    /// Five keys go. Four are the graph screen's own doors — `S` stash,
    /// `J` journal, `M` map, `O` overview — and every one of them is
    /// guarded on the graph having the keyboard, so every one of them does
    /// nothing while the pane has it (`graft::update::stash::stash_key`,
    /// `journal::journal_key`, `map::map_key`, `strip::strip_key`, all
    /// reading `AppState::graph_has_focus`). The fifth is `⏎`, which is
    /// what opened the pane: with the diff focused there is nothing left
    /// for it to open and it is consumed doing nothing
    /// (`graft::update::detail::keys::enter_opens`, the same function this
    /// wrapper's `opens` is set from) — the bar was teaching it at 80
    /// columns on the Detail tab, where the pane draws the only bar there
    /// is (m5/29), and beside the graph above 100 (m5 review Q2, m5/31).
    PaneFocused {
        /// The cursor row's own context, whose list this filters.
        row: Box<HintContext>,
        /// `⏎` still opens something: the files list has focus, so it opens
        /// the diff. `false` with the diff already focused.
        opens: bool,
    },
    /// **An operation is live and the banner is one `⇥` away, unfocused**
    /// (m6-conflicts/12, P9): the row's own list with `⇥ banner` leading.
    ///
    /// `Esc` on a focused banner hands the keyboard back, and before this
    /// the only way back in was a `Tab` nothing on screen named — the user
    /// pressed `Esc` on the one hint shown and was stranded
    /// (`.scratch/m6-conflicts/scout-stuck-after-merge.md`). No key is
    /// bound here: `⇥` already cycles into the banner
    /// (`graft::update::banner::take_focus`); this only says so. `graft`
    /// draws it only while one `Tab` really lands on the banner
    /// (`graft::update::banner::one_tab_to_banner`).
    OperationLive(Box<HintContext>),
}
