//! The footer's key hints, by context (`hint-bar-redesign`, ADR-0037 §4: the
//! bar is how anything is discoverable; a feature nobody can see doesn't
//! exist). Shows what the cursor or focus can do, filled **from the most
//! relevant end**: as many hints as the bar's own room holds, each on the
//! best of its three rungs — `c commit` → `c cmt` → `c` — that still leaves
//! room for the hints ahead of it ([`ladder`], ADR-0039's third amendment).
//! Never all hidden, never a hint cut in half, and never below the three
//! most relevant at their **floor rung** (ADR-0037 §4).
//!
//! Replaces the old fixed nineteen-entry list (`crates/graft-ui/MAP.md`'s
//! `graph_screen` entry: 144/155-column thresholds): that list answered
//! "what can this screen ever do", which grew too long to read at 80 or 120
//! columns and buried the keys that mattered right now. This one answers
//! "what does the *cursor* do right now" first, and only then what the whole
//! screen answers ([`TAIL`], the old list's leftovers in the old order, so a
//! wide terminal still teaches `q quit` or `x compare`).
//!
//! The per-context lists are the design note's priority table
//! (`docs/design-notes/m5-history-and-ghost-preview.md`, "Hint bar") — with
//! `Journal`, `Stash` and `Map`, which the note leaves out, taken from
//! `docs/design/02-keymap.md` §2.11, §2.9 and §2.3a.
//!
//! **A hint is only drawn once its key is bound.** Every entry carries a
//! [`HintState`]: the table stays as the design wrote it, but
//! [`HintState::Planned`] hints are dropped before the width rule
//! runs, so the bar never teaches a key that does nothing (m5-03 review,
//! Critical 2). `graft` proves both halves —
//! `update::tests::every_key_the_bar_draws_is_answered` (Live ⇒ bound, key
//! *and* verb, **and answered in the state the bar is drawn in**)
//! and `update::tests::every_planned_hint_is_still_unbound` (Planned ⇒ not
//! bound, so binding one fails the build until the table is flipped).

use unicode_width::UnicodeWidthStr;

use crate::fit::fit;

/// Whether the key a hint names actually does the verb it names, today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintState {
    /// Bound in the app: pressing the key does the verb.
    Live,
    /// Named by `docs/design/02-keymap.md` for this context, but no group
    /// has built it yet. Never drawn; kept here so the table stays the
    /// design's and the group that binds it has one word to change.
    Planned,
}

/// How much of a hint the bar had room to draw — ADR-0039's ladder
/// ("shrink before dropping"), corrected by its third amendment: a hint
/// steps down its own rungs, never the whole list together.
///
/// Ordered worst to best, so `<` means "less of the hint".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// The key on its own: `S`. What the doorway recovers.
    Key,
    /// The key and the user's short label: `c cmt`. A verb with no short
    /// form draws its whole verb here — its middle rung *is* its full form.
    Short,
    /// The key and the whole verb: `c commit`.
    Full,
}

/// One hint: a key, the verb it does, whether it does it yet, and how much
/// of it the bar drew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    /// The key as it reads on screen (`m`, `⏎`, `⌃A`, `.`).
    pub key: &'static str,
    /// The verb it does (`merge`, `open`, `more`).
    pub verb: &'static str,
    /// Bound today, or still to be built.
    pub state: HintState,
    /// The rung [`hints_for`] drew it at. Every table entry is [`Rung::Full`];
    /// only the width rule steps one down, so `verb` always names the whole
    /// verb whatever the rung (which is what `graft`'s
    /// `update::tests::every_key_the_bar_draws_is_answered` compares against the
    /// keymap).
    pub rung: Rung,
}

impl Hint {
    /// The same hint, drawn at `rung`.
    const fn at(mut self, rung: Rung) -> Self {
        self.rung = rung;
        self
    }

    /// What the bar draws for this hint at its rung, key included.
    #[must_use]
    pub fn text(&self) -> String {
        match self.rung {
            Rung::Key => self.key.to_owned(),
            Rung::Short => format!(
                "{} {}",
                self.key,
                short_form(self.verb).unwrap_or(self.verb)
            ),
            Rung::Full => format!("{} {}", self.key, self.verb),
        }
    }

    /// The rungs this hint really has, worst to best. A verb with no short
    /// form has two, not three: dropping the middle rung is what keeps the
    /// ladder from spending a step on nothing, and what keeps a rung from
    /// meaning two different amounts of text.
    fn rungs(&self) -> &'static [Rung] {
        if short_form(self.verb).is_some() {
            &[Rung::Key, Rung::Short, Rung::Full]
        } else {
            &[Rung::Key, Rung::Full]
        }
    }

    /// The lowest rung ADR-0037 §4 lets one of the three most relevant hints
    /// fall to: its short form where the user's table gives one, and its
    /// whole verb where it does not.
    fn floor_rung(&self) -> Rung {
        if short_form(self.verb).is_some() {
            Rung::Short
        } else {
            Rung::Full
        }
    }
}

/// The user's short-form table, restated 2026-09-21 (m5/27, correcting
/// m5/24): **the short form is a prefix of the full label, or there is no
/// short form.**
///
/// This table is also what the floor is measured against. **A hint's floor
/// rung is its short rung where this table names one, and its full verb
/// where it does not** ([`Hint::floor_rung`]) — so since m5/27 deleted
/// thirteen short forms, "the three most relevant at their short rung" is
/// no longer the floor for most verbs, and the prose says *floor rung*
/// where it means the floor.
///
/// Three kinds were on the table before this pass, and only one survives:
///
/// - a **prefix** keeps the label's beginning (`cont` for `continue`) — the
///   reader recovers the word from the fragment, the way `git`'s own
///   abbreviated flags do;
/// - a **squeeze** removes letters from the middle (`stsh` for `stash`,
///   `cmt` for `commit`) — the reader cannot recover the word from the
///   fragment, only recognise it once told;
/// - a **substitution** replaces the word outright (`find` for `search`,
///   `tool` for `mergetool`) — it gives one verb two names, which
///   `docs/design/02-keymap.md:12`'s "one meaning per verb" forbids
///   outright, and two of them (`a stop`, `M tool`) also broke
///   `docs/design/02-keymap.md:45`'s "match the buttons' initials exactly".
///
/// Squeezes and substitutions are gone. A verb this function does not name
/// simply has no short rung — adding a row is never required to make a
/// label fit, because the ladder's bare-key rung is already the mechanism
/// for "no room" (ADR-0039).
///
/// `stash`, `merge` and `rebase` are live and absent because a squeeze was
/// the only short form that ever fit them and the user ruled squeezes out;
/// `branch`, `tag` and `resolved` are [`HintState::Planned`] and never
/// drawn. All six keep their full form: the table is the user's, and a
/// label it does not name is reported, not replaced.
fn short_form(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "first-parent" => "first",
        "continue" => "cont",
        "discard" => "disc",
        "hide stash" => "hide",
        "overview" => "over",
        "renderer" => "rend",
        "status" => "stat",
        "revert" => "rev",
        "lanes" => "lane",
        _ => return None,
    })
}

/// A hint whose key is bound today.
const fn hint(key: &'static str, verb: &'static str) -> Hint {
    Hint {
        key,
        verb,
        state: HintState::Live,
        rung: Rung::Full,
    }
}

/// A hint the keymap names for this context that no group has built yet.
/// Never drawn (see the module comment).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "no key is planned-but-unbound today (m6-conflicts/05 bound the last two); kept for the next one a design names ahead of its group, and used by the tests' positive control"
    )
)]
const fn planned(key: &'static str, verb: &'static str) -> Hint {
    Hint {
        key,
        verb,
        state: HintState::Planned,
        rung: Rung::Full,
    }
}

/// Leads a truncated list: press `.` for the rest (the context-actions key,
/// `docs/design/02-keymap.md` §2.1 "Rare verbs").
const MORE: Hint = hint(".", "more");
/// Leads a marked row's list: press `.` to explain the Mistake Radar finding
/// (m2-details-and-diffs/17, design note "Marker column").
const EXPLAIN: Hint = hint(".", "explain");
/// The detail pane's divider (ADR-0028): shown by every context while a
/// pane is open, not only the changes pane (m5-03 review, Major 4).
const RESIZE: Hint = hint("<>", "resize");

/// Whether a detail pane is on screen, so its divider can be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Divider {
    /// A detail pane is open: `<> resize` applies.
    Shown,
    /// No pane, no divider, no hint.
    Hidden,
}

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
}

/// The hints for `context` that fit in `columns` cells: **as many whole
/// hints as fit the row the bar actually has**, each at the best rung
/// [`ladder`] can afford it, and the ones that will not fit even at their
/// bare key dropped from the right — never below three (or the context's own
/// count, if it has fewer) plus `. more`. Never empty except for
/// [`HintContext::Preview`].
///
/// `columns` is the bar's own room, not the screen's, and there is no
/// width tier on top of it. The 3/6/all tiers this used to carry were
/// never spec: ADR-0037 §4 sets a floor (three most relevant keys, never
/// all hidden) and says nothing about width. Laid over a `columns` that is
/// already the bar's own region, a tier is a *cap*: at 160 columns with the
/// detail pane beside the graph the bar has about 92 cells and fell into
/// the "narrow" tier, so it drew three hints — while a 120-column screen
/// with the pane below kept its 120 cells and drew six. The wider terminal
/// said less (ADR-0039's 2026-09-21 amendment, m5/20 item 7).
///
/// `divider` adds `<> resize` when a detail pane is open, whichever context
/// the cursor is in (ADR-0028: the pane being shown is what makes the
/// divider exist).
///
/// [`HintContext::Banner`] is the exception to both trailers. The banner is
/// modal — `banner_key` answers every key but `Ctrl+C` — so `.` opens
/// nothing and `<>` moves no divider while it has focus. ADR-0039's doorway
/// is a promise that what the width rule dropped is still reachable; under a
/// modal that reaches nothing, drawing it would be the dead key the ADR's
/// own first condition rules out. The banner's list is short enough to draw
/// whole at 80 columns, so nothing is dropped and nothing needs a doorway.
#[must_use]
pub fn hints_for(context: &HintContext, divider: Divider, columns: u16) -> Vec<Hint> {
    // The resolver below its own floor is the one context whose *keys*
    // depend on the width: under `conflict_resolver::MIN_COLUMNS` the
    // widget draws one line of text instead of the document, and the app
    // answers nothing but `Esc` — there is no document on screen to take a
    // side of. So the bar says the one thing that is true there, which is
    // also what that line itself says (m6-conflicts/04).
    if matches!(context, HintContext::Resolver { .. })
        && columns < crate::conflict_resolver::MIN_COLUMNS
    {
        return vec![hint("esc", "close")];
    }
    let (list, trailer) = whole_bar(context, divider);
    fit_list(&list, trailer, columns)
}

/// The bar `context` would draw on a row with no end to it, and the doorway
/// that leads whatever the width drops. [`hints_for`] is this plus the width
/// rule — the split exists so that a context which *filters* another
/// ([`HintContext::PaneFocused`]) can take the whole list away and hand back
/// less **before** the ladder runs. Filtering after it would leave a gap in
/// a row the ladder had already fitted.
fn whole_bar(context: &HintContext, divider: Divider) -> (Vec<Hint>, Option<Hint>) {
    match context {
        HintContext::Preview => (Vec::new(), None),
        // No width tier for the banner: every key it answers draws while
        // the width allows it, `⇥ graph` — the way out of the mode —
        // included. Three keys at 80 columns would have hidden the exit of
        // a modal that swallows every other key, which is the failure
        // `banner_key`'s own doc comment warns of.
        // A drag takes no doorway either, and for the banner's reason: `.`
        // does not open the menu while a badge is in the air, it cancels the
        // drag. Two hints fit at every width the app draws at.
        // The banner, a badge in the air and the two overlays are all
        // modal: each swallows every key but its own, so `.` opens no menu
        // and `<>` moves no divider while it has focus (`banner_key`,
        // `drag_key`, `journal::shown_key` and `stash::shown_key`, whose
        // catch-all arms return "claimed"). ADR-0039's doorway is a promise
        // that what the width dropped is still reachable; under a modal
        // that reaches nothing it would be the dead key the ADR's own first
        // condition rules out (its first amendment, extended to the
        // overlays in m5/31 — the sweep pressed `.` in the journal and
        // nothing happened).
        // The Map view is the third overlay and swallows keys the same way
        // (`graft::update::map::shown_key`'s catch-all). The detail pane
        // with an overlay behind it takes no doorway either: `.` reaches
        // the overlay, which swallows it — pressed in every one of these
        // states and nothing happened (m5/42).
        HintContext::Banner { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Resolver { .. } => (drawable(context, divider), None),
        HintContext::Marked(inner) => {
            // `.` is already on screen as `. explain`, so no `. more`; it
            // is prepended before fitting and not after, because a list
            // fitted first and prepended to second overflows by one hint.
            let mut list = vec![EXPLAIN];
            list.extend(drawable(inner, divider));
            (list, None)
        }
        // The inner context's own bar, minus the key the focused pane has
        // nothing left to do with. Nothing else changes: every other key on
        // a row context is still answered with the pane up — the graph's
        // own keys fall through to it, and the pane answers the motions
        // itself (m5/31).
        HintContext::PaneFocused { row, opens } => {
            let (list, trailer) = whole_bar(row, divider);
            let list = list
                .into_iter()
                .filter(|hint| !GRAPH_ONLY.contains(&hint.key) && (*opens || hint.key != ENTER))
                .collect();
            (list, trailer)
        }
        HintContext::WorkingTreeRow
        | HintContext::BranchRow { .. }
        | HintContext::PlainRow
        | HintContext::ChangesPane { .. } => (drawable(context, divider), Some(MORE)),
    }
}

/// `Enter`, as every table spells it. Named because
/// [`HintContext::PaneFocused`] filters on it.
const ENTER: &str = "⏎";

/// The keys only the graph screen answers: each opens something that stands
/// *where the graph stands*, and each is guarded on the graph having the
/// keyboard, so each does nothing while the detail pane has it
/// (`graft::update::state::AppState::graph_has_focus`, the one function all
/// four handlers read).
const GRAPH_ONLY: [&str; 4] = ["S", "J", "M", "O"];

/// The keys that need a commit under the cursor, and so do nothing on the
/// uncommitted-changes row: `y` copies a sha there is none of and `x` marks
/// a compare base there is no oid for. Both return at their first line on
/// that row (`graft::update::keys::copy_selected_sha`,
/// `toggle_compare_selected`), which the design note already said and the
/// bar drew anyway (m5/31). `space` folds the merge under the cursor, and
/// the uncommitted row is no merge (gap row 553).
const NEEDS_A_COMMIT: [&str; 3] = ["y", "x", "space"];

/// Every hint the footer could draw for `context`, in priority order, with
/// the ones nothing has built yet already dropped: the context's own keys,
/// the divider's when a pane is open, then — on the graph's row contexts
/// only — the keys the whole screen answers ([`TAIL`]).
///
/// A pane keeps to its own list: while the changes pane, an overlay or the
/// banner has focus it answers keys the graph does not, and the graph
/// answers keys it does not, so [`TAIL`] would be advertising the wrong
/// screen.
///
/// The focused banner goes further and takes no divider hint either: it is
/// modal, so `<>` is as dead as every other key it swallows. (Under 100
/// columns the angle keys are a no-op in any case —
/// `graft::update::detail`'s own rule for the tab layout.)
fn drawable(context: &HintContext, divider: Divider) -> Vec<Hint> {
    let mut list: Vec<Hint> = context_list(context)
        .into_iter()
        .filter(|hint| hint.state == HintState::Live)
        .collect();
    if divider == Divider::Shown
        && !matches!(
            context,
            HintContext::Banner { .. }
                | HintContext::Dragging
                | HintContext::Journal
                | HintContext::Stash
                | HintContext::Map
                | HintContext::PaneOverOverlay { .. }
                | HintContext::Resolver { .. }
        )
    {
        // Fourth, not last: appended, it sat behind ten context keys and the
        // six-hint tier never reached it, so the hint existed and was never
        // drawn (found by rendering a real frame —
        // `graft::it::hint_bar::the_changes_pane_hints_the_keys_that_write`).
        // Fourth keeps the three verbs that do work ahead of it and still
        // shows the divider from 100 columns, which is where a pane and a
        // graph first fit side by side.
        list.insert(3.min(list.len()), RESIZE);
    }
    let rows = matches!(
        context,
        HintContext::WorkingTreeRow | HintContext::BranchRow { .. } | HintContext::PlainRow
    );
    if rows {
        let commits = *context != HintContext::WorkingTreeRow;
        for extra in TAIL {
            let dead = !commits && NEEDS_A_COMMIT.contains(&extra.key);
            if !dead && !list.iter().any(|hint| hint.key == extra.key) {
                list.push(extra);
            }
        }
    }
    list
}

/// One rung's worth of bar: either one more hint at its bare key, or one
/// hint already drawn raised to a better rung. [`ladder`] puts them in a
/// fixed order and [`fit_list`] takes the longest prefix that fits, which is
/// what makes the bar monotone in its own width — every prefix costs at
/// least as much as the one before it, so more room can only add a hint or
/// improve one, never the reverse.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Draw the next hint, at its bare key.
    Add,
    /// Raise the hint at this index to this rung.
    Raise(usize, Rung),
}

/// ADR-0039's ladder, third amendment: the bar fills **from the most
/// relevant end**, and each hint takes the best rung that still leaves room
/// for the hints ahead of it.
///
/// Read as an order of steps that is the same at every width:
///
/// 1. the floor (ADR-0037 §4) — the three most relevant, at their **floor
///    rung**, plus the doorway. Not a step: drawn whatever the width,
///    because "never all hidden" outranks fitting;
/// 2. those three to their full verbs. The leading verbs are the ones worth
///    the most cells, and they go first so that 80 columns still reads
///    `⏎ open  c commit  y sha` and not three abbreviations;
/// 3. every remaining hint, in priority order, at its **bare key**. A hint
///    may only rise above its key once every hint ahead of it is at least
///    present — that is the "leaves room for the hints ahead of it" clause,
///    and it is what makes the tail of bare keys exist at all;
/// 4. those tail hints raised, one at a time from the most relevant end,
///    short rung then full (the two the ladder can raise to).
///
/// The list-wide version of this — every hint stepping down a rung together
/// — was built, measured and reverted: a row context is 22 hints, so below
/// 190 columns nothing but the bare-key rung fitted and 80, 120 and 160
/// drew the same keys-with-no-verbs bar this ADR had already rejected
/// (ADR-0039's third amendment, which corrects its second).
fn ladder(list: &[Hint], floor: usize) -> Vec<Step> {
    let mut steps = Vec::new();
    for (index, hint) in list.iter().enumerate().take(floor) {
        if hint.rungs().contains(&Rung::Short) {
            steps.push(Step::Raise(index, Rung::Full));
        }
    }
    for _ in floor..list.len() {
        steps.push(Step::Add);
    }
    for (index, hint) in list.iter().enumerate().skip(floor) {
        for rung in hint.rungs().iter().skip(1) {
            steps.push(Step::Raise(index, *rung));
        }
    }
    steps
}

/// `list` drawn into `columns` cells by [`ladder`], plus `trailer`
/// (`. more`, unless the caller has already shown `.`).
///
/// Never fewer than three hints (or `list.len()` if it is shorter) and never
/// below their floor rung — ADR-0037 §4's floor, the only number in the
/// rule. Below that room [`below_the_floor`] takes over: the bar collapses
/// to the doorway hint alone, shrunk rung by rung until it fits, or to
/// nothing — never a bar the footer has to cut, which is the one outcome
/// the ADR forbids.
fn fit_list(list: &[Hint], trailer: Option<Hint>, columns: u16) -> Vec<Hint> {
    let room = usize::from(columns);
    let floor = list.len().min(3);
    let mut drawn: Vec<Hint> = list[..floor]
        .iter()
        .map(|hint| hint.at(hint.floor_rung()))
        .collect();
    if !fits(&drawn, trailer, room) {
        return below_the_floor(list, trailer, room);
    }
    for step in ladder(list, floor) {
        let mut candidate = drawn.clone();
        match step {
            Step::Add => candidate.push(list[candidate.len()].at(Rung::Key)),
            Step::Raise(index, rung) => candidate[index] = candidate[index].at(rung),
        }
        if !fits(&candidate, trailer, room) {
            break;
        }
        drawn = candidate;
    }
    place_doorway(drawn, trailer)
}

/// Does `drawn` plus `trailer` fit in `room`?
///
/// The trailer is measured on the end wherever [`place_doorway`] will put
/// it: [`hints_text`] separates every pair of hints the same way, so moving
/// one hint within the row changes no cell count.
fn fits(drawn: &[Hint], trailer: Option<Hint>, room: usize) -> bool {
    let mut measured = drawn.to_vec();
    measured.extend(trailer);
    text_width(&hints_text(&measured)) <= room
}

/// **The doorway sits at the head of the bare keys — which is the end of
/// the row when there are none.**
///
/// One sentence, two layouts, no special case. On a graph row context the
/// bar ends in a run of bare keys, and `. more` leads it: everything after
/// the doorway then reads as *keys the menu will spell out*, which is what
/// the run is. In the changes pane and the journal browser nothing is drawn
/// at its bare key, so the same rule leaves `. more` last, where it always
/// was.
///
/// The doorway's promise is that it is **reliable** — never dropped, never
/// clipped — which is not the same as being last (ADR-0039).
fn place_doorway(mut drawn: Vec<Hint>, trailer: Option<Hint>) -> Vec<Hint> {
    if let Some(trailer) = trailer {
        let head_of_the_bare_keys = drawn
            .iter()
            .position(|hint| hint.rung == Rung::Key)
            .unwrap_or(drawn.len());
        drawn.insert(head_of_the_bare_keys, trailer);
    }
    drawn
}

/// Below the floor: what the bar draws when it cannot draw its floor.
///
/// ADR-0039 mandates a floor and a floor that cannot be met is not a floor,
/// so the sequence is stated to the bottom: floor, then the doorway whole,
/// then the doorway's bare key, then nothing. **The bar never draws a
/// fragment of itself at any width** — a clipped `. mo` on the one element
/// meant to be reliable is worse than an empty bar, and worse than a
/// squeeze, because nobody chose where it cut.
///
/// The doorway is the trailer where there is one, and otherwise a leading
/// `.` the caller put into the list itself ([`HintContext::Marked`]'s
/// `. explain`). A context with neither is modal and swallows `.` — the
/// focused banner — so it has no doorway and goes straight to nothing.
fn below_the_floor(list: &[Hint], trailer: Option<Hint>, room: usize) -> Vec<Hint> {
    let doorway = trailer.or_else(|| list.first().copied().filter(|hint| hint.key == "."));
    let Some(doorway) = doorway else {
        return Vec::new();
    };
    for rung in doorway.rungs().iter().rev() {
        let row = vec![doorway.at(*rung)];
        if text_width(&hints_text(&row)) <= room {
            return row;
        }
    }
    Vec::new()
}

fn text_width(text: &str) -> usize {
    text.width()
}

/// The design note's priority table, one context at a time, including the
/// entries no group has built yet ([`drawable`] filters those out).
/// `Marked` and `Preview` are handled in [`hints_for`], not here.
fn context_list(context: &HintContext) -> Vec<Hint> {
    match context {
        HintContext::WorkingTreeRow => working_tree_row_list(),
        HintContext::BranchRow { current: false } => branch_row_list(),
        HintContext::BranchRow { current: true } => current_branch_row_list(),
        HintContext::PlainRow => plain_row_list(),
        HintContext::Banner {
            skippable,
            conflicted,
        } => banner_list(*skippable, *conflicted),
        HintContext::ChangesPane { unmerged } => changes_pane_list(*unmerged),
        HintContext::Resolver {
            base,
            restart,
            whole_file,
        } => {
            if *whole_file {
                resolver_card_list()
            } else {
                resolver_list(*base, *restart)
            }
        }
        // Keymap §2.11: the journal browser answers `⏎` and `Esc` only.
        HintContext::Journal => vec![hint("⏎", "revert"), hint("esc", "close")],
        // Keymap §2.9: `b` (branch from this stash) is named there as
        // planned, so it is listed and not drawn.
        HintContext::Stash => vec![
            hint("⏎", "apply"),
            hint("p", "pop"),
            hint("d", "drop"),
            hint("b", "branch"),
            hint("esc", "close"),
        ],
        // Keymap §2.3a: the Map view answers `Enter`, the movement keys and
        // `M`/`Esc`. `M` is left out for the reason the journal leaves `J`
        // out — one way of saying "close" is enough for a bar this short.
        HintContext::Map => vec![hint("⏎", "seek"), hint("j/k", "move"), hint("esc", "close")],
        // Only ever drawn in the sub-100-column `Tab` layout, which has no
        // divider to drag: everywhere else the overlay draws its own footer
        // beside the pane (`graft::view::frame::split_for_detail`, which
        // draws a hint row only for `DetailLayout::Tab`).
        HintContext::PaneOverOverlay { opens } => {
            let mut list = Vec::new();
            if *opens {
                list.push(hint("⏎", "open"));
            }
            list.extend([hint("j/k", "move"), hint("u", "undo"), hint("/", "search")]);
            list
        }
        // The drag is a mouse gesture: `release` is not a key, it is what
        // the user's hand has to do, and `esc` is the one key that answers
        // while a badge is in the air (`graft::update::drag::drag_cancel`).
        // Nothing else is offered because nothing else acts — every other
        // key cancels the drag and then means what it always means.
        HintContext::Dragging => vec![hint("release", "drop on a row"), hint("esc", "cancel")],
        HintContext::ConfirmBox => vec![
            hint("←/→", "move"),
            hint("⏎", "choose"),
            hint("esc", "cancel"),
        ],
        HintContext::NamePrompt { toggle } => {
            let mut list = vec![hint("⏎", "create")];
            if *toggle {
                list.push(hint("⇥", "toggle"));
            }
            list.push(hint("esc", "cancel"));
            list
        }
        HintContext::Credential => vec![hint("⏎", "send"), hint("esc", "cancel")],
        // All three are answered in `whole_bar` before this is reached; a
        // `RowContext` sub-enum would say so in the type, at the cost of
        // wrapping every row context at every call site.
        HintContext::Marked(_) | HintContext::PaneFocused { .. } | HintContext::Preview => {
            Vec::new()
        }
    }
}

/// The keys the graph screen answers wherever the cursor is, in the order
/// the old fixed list dropped them (PROMPT.md principle 4: the hint bar
/// teaches the tool). Appended after a row context's own keys, so a wide
/// terminal still shows `q quit`, `x compare` and `+ lanes` — which the
/// per-context table leaves out entirely (m5-03 review, Major 3). Whatever
/// the context already hints is not repeated ([`drawable`] dedupes by key).
const TAIL: [Hint; 20] = [
    hint("q", "quit"),
    hint("F2", "status"),
    hint("F3", "renderer"),
    hint("u", "undo"),
    hint("x", "compare"),
    hint("+", "lanes"),
    hint("/", "search"),
    hint("j/k", "move"),
    hint("space", "fold"),
    hint("1", "first-parent"),
    hint("F", "focus"),
    hint("S", "stash"),
    hint("H", "hide stash"),
    hint("f", "fetch"),
    hint("p", "pull"),
    hint("P", "push"),
    hint("J", "journal"),
    hint("y", "sha"),
    hint("M", "map"),
    hint("O", "overview"),
];

fn working_tree_row_list() -> Vec<Hint> {
    vec![
        hint("⏎", "changes"),
        hint("c", "commit"),
        hint("S", "stash"),
        hint("p", "pull"),
        hint("P", "push"),
        hint("f", "fetch"),
        hint("u", "undo"),
        hint("J", "journal"),
        hint("/", "search"),
        hint("j/k", "move"),
    ]
}

fn branch_row_list() -> Vec<Hint> {
    vec![
        hint("m", "merge"),
        hint("r", "rebase"),
        hint("⏎", "open"),
        hint("b", "branch"),
        hint("t", "tag"),
        hint("P", "push"),
        hint("c", "commit"),
        hint("p", "pull"),
        hint("f", "fetch"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("/", "search"),
    ]
}

fn current_branch_row_list() -> Vec<Hint> {
    vec![
        hint("⏎", "open"),
        hint("c", "commit"),
        hint("P", "push"),
        hint("p", "pull"),
        hint("f", "fetch"),
        hint("b", "branch"),
        hint("t", "tag"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("J", "journal"),
        hint("/", "search"),
    ]
}

fn plain_row_list() -> Vec<Hint> {
    vec![
        hint("⏎", "open"),
        hint("b", "branch"),
        hint("r", "rebase"),
        hint("t", "tag"),
        hint("c", "commit"),
        hint("y", "sha"),
        hint("m", "merge"),
        hint("u", "undo"),
        hint("S", "stash"),
        hint("J", "journal"),
        hint("/", "search"),
    ]
}

/// The banner's own keys (design note, "Banner focused"), every one of them
/// answered by `graft::update::banner::banner_key` while the banner has
/// focus.
///
/// The list is the *whole* bar in that context: the banner is modal, so
/// `<> resize` and `. more` are dropped by [`hints_for`] and [`drawable`]
/// rather than advertised at keys the banner swallows. `M`/`e` appear only
/// while something is conflicted — with nothing unmerged they refuse in
/// words, and a hint whose key can only say "no" is a hint that lies
/// (m5-09 review, Major 2).
fn banner_list(skippable: bool, conflicted: bool) -> Vec<Hint> {
    let mut list = vec![hint("c", "continue")];
    if skippable {
        list.push(hint("s", "skip"));
    }
    list.push(hint("a", "abort"));
    if conflicted {
        // `⏎ resolve` leads (Proposed change 2a): live since
        // m6-conflicts/04, which binds it in `update::banner::banner_key`.
        list.insert(0, hint("⏎", "resolve"));
        list.push(hint("M", "mergetool"));
        list.push(hint("e", "edit"));
    }
    list.push(hint("⇥", "graph"));
    list
}

/// The changes pane (design note, "Changes pane"). On an unmerged file the
/// write keys swap for the conflict keys: `⏎` opens the resolver
/// (`update::detail::keys`), `M` launches the mergetool and `space` marks
/// the file resolved — which is what staging an unmerged path *is*
/// (`update::operate::stage_selected`). All three bound by m6-conflicts/04.
fn changes_pane_list(unmerged: bool) -> Vec<Hint> {
    if unmerged {
        vec![
            // `⏎ resolve` leads (Proposed change 2a).
            hint("⏎", "resolve"),
            hint("M", "mergetool"),
            hint("e", "edit"),
            hint("space", "resolved"),
            hint("c", "commit"),
            hint("⇥", "graph"),
        ]
    } else {
        vec![
            hint("space", "stage"),
            hint("d", "discard"),
            hint("c", "commit"),
            hint("⌃A", "all"),
            hint("e", "edit"),
            hint("⇥", "graph"),
        ]
    }
}

/// The resolver's own key list (`docs/design-notes/m6-conflicts.md`, "New
/// UI" footer mockups): `o ours  t theirs  b both  [B base]  e edit
/// n next  N prev  ⏎ resolved  M mergetool  [d start over]  esc close`,
/// `B base` after `b both` only while `base` (Proposed change 2b), `d start
/// over` before `esc close` only while `restart` (Proposed change 2c).
///
/// Live since m6-conflicts/04, which built `update::resolver` — every one
/// of these is answered by `resolver_key`, the two in brackets included
/// since m6-conflicts/05 bound them (`toggle_base`, `restart_box`).
fn resolver_list(base: bool, restart: bool) -> Vec<Hint> {
    let mut list = vec![hint("o", "ours"), hint("t", "theirs"), hint("b", "both")];
    if base {
        list.push(hint("B", "base"));
    }
    list.extend([
        hint("e", "edit"),
        hint("n", "next"),
        hint("N", "prev"),
        hint("⏎", "resolved"),
        hint("M", "mergetool"),
    ]);
    if restart {
        list.push(hint("d", "start over"));
    }
    list.push(hint("esc", "close"));
    list
}

/// A whole-file card's own list (m6-conflicts/05, m6-05 review Major 1):
/// `o ours  t theirs  e edit  n next  M mergetool  esc close`.
///
/// Only the keys a card answers. `b` is refused in words (there is no
/// "both" of a path), `⏎` has nothing to mark — `o`/`t` already stage —
/// `B` has no blocks to show a base in, `d` has no markers to bring back,
/// and `N` has no previous file to walk to. `n` crosses to the next
/// conflicted file, as it does from a document's last block.
fn resolver_card_list() -> Vec<Hint> {
    vec![
        hint("o", "ours"),
        hint("t", "theirs"),
        hint("e", "edit"),
        hint("n", "next"),
        hint("M", "mergetool"),
        hint("esc", "close"),
    ]
}

/// **Every context the footer can be put in**, every field combination, each
/// row context also wrapped in [`HintContext::Marked`]. The one enumeration
/// every sweep reads — `graft-ui`'s own hint tests, [`every_hint`] and
/// `graft`'s key sweeps.
///
/// It exists because three hand-written copies of this list drifted:
/// [`HintContext::Dragging`] was added in m5/10 and only *one* of them
/// gained it, so the drag's bar went two tickets without a sweep over it.
///
/// **It cannot come from the type.** Rust has no way to enumerate a type's
/// values without a derive macro, and `Marked`/`PaneFocused` wrap a context
/// of their own, so the set is not even finite without choosing a depth by
/// hand. What the compiler can do is refuse, and three exhaustive matches
/// make it: [`marked_form`] and [`pane_focused_form`] here, and
/// `tests::coverage_slot`, which additionally spells out every **boolean
/// field combination**. A shape added to the enum stops this crate
/// compiling until it is named in all three, and
/// `tests::every_context_reaches_every_shape` then fails until the list
/// below actually produces one.
///
/// So adding a variant is still a human's edit; it is no longer a human's
/// *omission* (m5-history-and-ghost-preview/33, review M3, which corrected
/// an earlier claim that the compiler forced the list itself — it does
/// not).
#[must_use]
pub fn every_context() -> Vec<HintContext> {
    let plain = [
        HintContext::WorkingTreeRow,
        HintContext::BranchRow { current: false },
        HintContext::BranchRow { current: true },
        HintContext::PlainRow,
        HintContext::Banner {
            skippable: true,
            conflicted: true,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: true,
        },
        HintContext::Banner {
            skippable: true,
            conflicted: false,
        },
        HintContext::Banner {
            skippable: false,
            conflicted: false,
        },
        HintContext::Preview,
        HintContext::ChangesPane { unmerged: false },
        HintContext::ChangesPane { unmerged: true },
        HintContext::Journal,
        HintContext::Stash,
        HintContext::Map,
        HintContext::PaneOverOverlay { opens: false },
        HintContext::PaneOverOverlay { opens: true },
        HintContext::Dragging,
        HintContext::ConfirmBox,
        HintContext::NamePrompt { toggle: true },
        HintContext::NamePrompt { toggle: false },
        HintContext::Credential,
    ];
    let mut all: Vec<HintContext> = plain.to_vec();
    all.extend(resolver_forms());
    all.extend(plain.iter().filter_map(marked_form));
    let rows: Vec<HintContext> = all.iter().flat_map(pane_focused_form).collect();
    all.extend(rows);
    all
}

/// Every [`HintContext::Resolver`] form [`every_context`] names: the four
/// `base`/`restart` combinations of a document, and a whole-file card
/// (whose list reads neither flag, so one form covers it).
fn resolver_forms() -> [HintContext; 5] {
    let document = |base, restart| HintContext::Resolver {
        base,
        restart,
        whole_file: false,
    };
    [
        document(false, false),
        document(true, false),
        document(false, true),
        document(true, true),
        HintContext::Resolver {
            base: false,
            restart: false,
            whole_file: true,
        },
    ]
}

/// `context` wrapped in [`HintContext::Marked`], for the contexts a Mistake
/// Radar marker can actually reach — the cursor's own row, once
/// `graft::view::rows::hint_context` has ruled out the working tree, the
/// changes pane and a drag in the air. Everything else has no cursor row to
/// carry a marker.
///
/// Exhaustive on purpose: see [`every_context`].
fn marked_form(context: &HintContext) -> Option<HintContext> {
    match context {
        HintContext::BranchRow { .. } | HintContext::PlainRow => {
            Some(HintContext::Marked(Box::new(context.clone())))
        }
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::Marked(_)
        | HintContext::PaneFocused { .. } => None,
    }
}

/// `context` wrapped in [`HintContext::PaneFocused`], for the contexts
/// `graft::view::rows::hint_context` can reach with the pane focused and
/// `⏎` spent — a cursor row, marked or not.
///
/// Not the working-tree row: a pane open on *it* is the changes pane, so
/// the classifier answers `ChangesPane` long before it looks at the cursor
/// row. Not the changes pane or the banner either — neither list has an
/// `⏎` this could take, and wrapping a list to change nothing would be a
/// context nobody can tell from the one inside it.
///
/// Not the overlays: a pane focused with one of them behind it draws
/// neither the overlay's list nor a row's, because the graph is not there
/// to answer the row's keys. That is [`HintContext::PaneOverOverlay`],
/// which [`every_context`] names in its own right.
///
/// Exhaustive on purpose: see [`every_context`].
fn pane_focused_form(context: &HintContext) -> Vec<HintContext> {
    match context {
        HintContext::BranchRow { .. } | HintContext::PlainRow | HintContext::Marked(_) => {
            [true, false]
                .into_iter()
                .map(|opens| HintContext::PaneFocused {
                    row: Box::new(context.clone()),
                    opens,
                })
                .collect()
        }
        HintContext::WorkingTreeRow
        | HintContext::Banner { .. }
        | HintContext::Preview
        | HintContext::ChangesPane { .. }
        | HintContext::Journal
        | HintContext::Stash
        | HintContext::Map
        | HintContext::PaneOverOverlay { .. }
        | HintContext::Dragging
        | HintContext::ConfirmBox
        | HintContext::NamePrompt { .. }
        | HintContext::Credential
        | HintContext::Resolver { .. }
        | HintContext::PaneFocused { .. } => Vec::new(),
    }
}

/// Every hint in every context's table, planned ones included, deduped by
/// `(key, verb, state)` — what `graft`'s guard tests walk so that neither
/// half of the table can drift from what the app binds.
#[must_use]
pub fn every_hint() -> Vec<Hint> {
    let mut all = Vec::new();
    for context in every_context() {
        all.extend(context_list(&context));
    }
    all.extend(TAIL);
    all.push(MORE);
    all.push(EXPLAIN);
    all.push(RESIZE);
    let mut seen = Vec::new();
    all.retain(|hint| {
        let fresh = !seen.contains(hint);
        if fresh {
            seen.push(*hint);
        }
        fresh
    });
    all
}

/// `hints` as the footer draws them: `" ⏎ open  c commit  P push  u  S"` — a
/// leading space (matching the footer's other slots, all built the same
/// way), two spaces between hints, and each hint at its own
/// [`Rung`](Hint::rung). A hint built straight from the table is `Rung::Full`,
/// so a list that never went through [`hints_for`] reads exactly as it
/// always did.
#[must_use]
pub fn hints_text(hints: &[Hint]) -> String {
    let mut line = String::new();
    for hint in hints {
        line.push_str(if line.is_empty() { " " } else { "  " });
        line.push_str(&hint.text());
    }
    line
}

/// A status line, worded `reason · keys` (the design note's "Failure line",
/// `remote_failed`'s callers): keeps ` · keys` whole and cuts the reason
/// with `…` first, since the key the message names is the point (ADR-0037
/// §4, acceptance M2 — at 80 columns the old blind cut lost `P force`).
/// When even ` · keys` cannot fit, the keys are cut on their own, without
/// the separator — three cells spent on a `·` with nothing in front of it
/// buy nothing. No `keys`: the whole line is cut like any other footer text.
#[must_use]
pub fn fit_status_line(reason: &str, keys: Option<&str>, columns: u16) -> String {
    let columns = usize::from(columns);
    let Some(keys) = keys else {
        return fit(reason, columns);
    };
    let suffix = format!(" · {keys}");
    let suffix_width = text_width(&suffix);
    if suffix_width >= columns {
        return fit(keys, columns);
    }
    let room = columns - suffix_width;
    if text_width(reason) <= room {
        return format!("{reason}{suffix}");
    }
    format!("{}{suffix}", fit(reason, room))
}

#[cfg(test)]
mod tests;
