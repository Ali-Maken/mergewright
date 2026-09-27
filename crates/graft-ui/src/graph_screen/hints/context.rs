//! [`HintContext`]: where the cursor or focus is, which picks the list.

use crate::conflict_resolver::SideRoles;
use crate::finish_summary::SummaryFocus;

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
        /// The live operation is a merge, begun in Graft or in the shell:
        /// `c` opens the finish summary, so the bar says `c finish`
        /// rather than `c continue` (m6-conflicts/70, n2).
        merge: bool,
    },
    /// The Ghost Preview box has focus: its own border draws the footer,
    /// so the main footer's list is empty.
    Preview,
    /// The changes pane (working tree, not a commit) has focus.
    ChangesPane {
        /// The file under the cursor is unmerged (conflicted): the keys
        /// that write swap for the conflict-resolution keys.
        unmerged: bool,
        /// What `c` does while an operation is live; `None` when nothing
        /// is, and `c` commits (m6-conflicts/52).
        live: Option<LiveC>,
        /// Where one `⇥` lands from here (m6-conflicts/70): the diff from
        /// the file list, the banner or the graph from the diff. Read off
        /// the one reading of the cycle (`graft::update::banner::tab_target`).
        tab: TabWord,
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
        /// The open file is **done** and shown read-only (m6-conflicts/25):
        /// the list is exactly `d start over  e edit  esc close` when
        /// `restart`, else `e edit  esc close` (m6-conflicts/28); no other
        /// flag is read.
        done: bool,
        /// The operation's role words: `o`/`t` read as the sides' roles
        /// (`o yours  t theirs`; rebase `o upstream  t yours`), never as a
        /// branch name or sha (ADR-0043 §4).
        roles: SideRoles,
        /// `c` would act (m6-conflicts/31, P8): an operation is live and
        /// nothing is conflicted. Every table then offers `c finish` (a
        /// merge) or `c continue` just before `esc close`. Gated, not drawn
        /// always: the bar never names a key that would refuse. In practice
        /// only a done file is ever finishable.
        finishable: bool,
    },
    /// The finish summary has the keyboard (m6-conflicts/19, keymap §2.6b):
    /// modal, its own whole list, no `. more`. What `⏎` and `⇥` do depends
    /// on which part has focus, so the list follows `focus`.
    Finish {
        /// Files, subject or body.
        focus: SummaryFocus,
        /// The cursor row can show a diff (`space diff`): it is a file git
        /// combined on its own.
        expandable: bool,
    },
    /// A branch badge is being dragged (`docs/design/02-keymap.md` §2.15).
    /// The gesture ends with the button, not a key, so the bar says what
    /// the pointer must do and which key backs out.
    Dragging,
    /// **A confirm box is live** (rulings 07b, Q1): it claims the next key
    /// before any screen does (`graft::update::confirm::confirm_key`), so
    /// the bar under it offers its keys and no other. `←`/`→` (and `⇥`)
    /// move between the choices, `⏎` takes the one under the cursor, and
    /// every other key is swallowed — `esc` is the one the bar names.
    ///
    /// `yes` is the box's acting button when `y` takes it (ADR-0044 §3,
    /// m6-conflicts/50): the bar then leads with `y <that button>`. `None`
    /// for a box `y` never confirms (the lease push, undo-after-push and the
    /// revert drift guard), whose bar is the three keys alone.
    ConfirmBox {
        /// What `y` confirms, or `None` where it is swallowed.
        yes: Option<ConfirmYes>,
    },
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
    ///
    /// `space` (m7-power-tools/20h): with the files list focused
    /// (`opens`), `graft`'s `files_key` takes `space` on every row — it
    /// folds a directory and does nothing on a file — so the graph's merge
    /// fold (`TAIL`'s `space fold`) is dropped there and `folds` puts the
    /// pane's own fold back on a directory row. With the diff focused the
    /// pane does not take `space`; it reaches the graph and folds the merge,
    /// so the graph's hint stays.
    PaneFocused {
        /// The cursor row's own context, whose list this filters.
        row: Box<HintContext>,
        /// `⏎` still opens something: the files list has focus, so it opens
        /// the diff. `false` with the diff already focused.
        opens: bool,
        /// `B` blames the line under the cursor: a file row, a heading or
        /// a diff line, and the commit-graph offer not up (whose `B` it
        /// is while it can be acted on, keymap §2.1). Adds `B blame`
        /// after `⏎` (m7-power-tools/20).
        blamable: bool,
        /// The files list has focus and the cursor is on a directory row:
        /// `space fold`, after `⏎` and `B` (m7-power-tools/20h). Never true
        /// without `opens`, nor with `blamable` (a directory row has no file
        /// to blame); `every_context` produces neither shape.
        folds: bool,
    },
    /// **A blame place is on screen** (m7-power-tools/20; design note pass
    /// 2b B1): modal, its own bar, no doorway.
    Blame {
        /// A find is active: `n N match`.
        finding: bool,
        /// The repo has `.git-blame-ignore-revs`: `i ignore list`.
        ignore_file: bool,
    },
    /// **A blame trail waits under the graph** (`⏎` landed on a commit):
    /// the inner context's own bar, led by `esc back to blame` so the way
    /// back is the last thing the width drops (m7-power-tools/20).
    BackToBlame(Box<HintContext>),
    /// **An operation is live** over a graph row: the row's own list with
    /// its `c` said the way it acts now ([`LiveC`], m6-conflicts/52), and
    /// `⇥ banner` leading while the banner is one `⇥` away (m6-conflicts/12,
    /// P9).
    ///
    /// `graft` wraps every graph-row context in this while an operation is
    /// live — a bisect, and the detail pane open over a focused graph,
    /// included — so no row bar says `c commit` while `c` does otherwise.
    ///
    /// `⇥ banner`: `Esc` on a focused banner hands the keyboard back, and
    /// before m6-conflicts/12 the only way back in was a `Tab` nothing on
    /// screen named (`.scratch/m6-conflicts/scout-stuck-after-merge.md`).
    /// No key is bound for it: `⇥` already cycles into the banner
    /// (`graft::update::banner::take_focus`); the hint only says so, and
    /// only while one `Tab` really lands there
    /// (`graft::update::banner::one_tab_to_banner`).
    OperationLive {
        /// The cursor row's own context.
        row: Box<HintContext>,
        /// What `c` does here.
        c: LiveC,
        /// One `⇥` lands on the banner, so `⇥ banner` leads.
        tab_to_banner: bool,
    },
}

/// What `c` does in the graph or the changes pane while an operation is
/// live (m6-conflicts/52): the banner's `c` where that acts, else words
/// that refuse — so its hint says `finish`, `continue`, or nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveC {
    /// A merge: `c finish` opens the finish summary.
    Finish,
    /// A rebase, cherry-pick or revert Graft began: `c continue`.
    Continue,
    /// A bisect, or a rebase, cherry-pick or revert the shell began: `c`
    /// only refuses in words, so no `c` hint.
    Refuses,
}

/// Where one `⇥` lands from the changes pane, as its hint names it
/// (m6-conflicts/70): `⇥ diff`, `⇥ banner` or `⇥ graph`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabWord {
    /// The graph: from the diff with no banner drawn.
    Graph,
    /// The diff: from the file list.
    Diff,
    /// The operation banner: from the diff while an operation is live.
    Banner,
}

impl TabWord {
    /// Every form, for [`crate::graph_screen::hints::every_context`].
    pub const ALL: [TabWord; 3] = [TabWord::Graph, TabWord::Diff, TabWord::Banner];

    /// The hint's verb: `graph`, `diff` or `banner`.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            TabWord::Graph => "graph",
            TabWord::Diff => "diff",
            TabWord::Banner => "banner",
        }
    }
}

/// The acting button `y` confirms in a confirm box (ADR-0044 §3.1,
/// m6-conflicts/50): one per label a `y`-live box draws at index 1.
///
/// Built from the button's own label ([`ConfirmYes::from_label`]), never
/// chosen beside it, so the bar's `y <word>` and the button cannot name two
/// different acts: an untracked file's box reads `[ Delete ]`, its bar
/// `y delete` (conductor's decision (a), 2026-09-25).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmYes {
    /// `[ Push ]`, the plain push box.
    Push,
    /// `[ Drop ]`, the stash-drop box.
    Drop,
    /// `[ Discard ]`, a tracked file's or a hunk's discard box.
    Discard,
    /// `[ Delete ]`, an untracked file's discard box.
    Delete,
    /// `[ Skip ]`, the banner's skip box.
    Skip,
    /// `[ Abort ]`, the banner's abort box.
    Abort,
    /// `[ Start over ]`, the resolver's restart box.
    StartOver,
    /// `[ Undo ]`, the undo drift guard.
    Undo,
    /// `[ Redo ]`, the redo drift guard.
    Redo,
}

impl ConfirmYes {
    /// Every value, in declaration order: what the sweeps iterate.
    pub const ALL: [Self; 9] = [
        Self::Push,
        Self::Drop,
        Self::Discard,
        Self::Delete,
        Self::Skip,
        Self::Abort,
        Self::StartOver,
        Self::Undo,
        Self::Redo,
    ];

    /// The button's label exactly as the box draws it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Push => "Push",
            Self::Drop => "Drop",
            Self::Discard => "Discard",
            Self::Delete => "Delete",
            Self::Skip => "Skip",
            Self::Abort => "Abort",
            Self::StartOver => "Start over",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
        }
    }

    /// The word after `y` on the bar: the button's label, lowercased.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Drop => "drop",
            Self::Discard => "discard",
            Self::Delete => "delete",
            Self::Skip => "skip",
            Self::Abort => "abort",
            Self::StartOver => "start over",
            Self::Undo => "undo",
            Self::Redo => "redo",
        }
    }

    /// The value whose [`label`](Self::label) is `label`, or `None` for a
    /// label no `y`-live box draws — and then `y` does not act either, so
    /// the bar never names a key that would not.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|yes| yes.label() == label)
    }
}
