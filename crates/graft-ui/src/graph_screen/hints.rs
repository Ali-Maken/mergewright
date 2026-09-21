//! The footer's key hints, by context (`hint-bar-redesign`, ADR-0037 §4: the
//! bar is how anything is discoverable; a feature nobody can see doesn't
//! exist). Shows what the cursor or focus can do, filled **from the most
//! relevant end**: as many hints as the bar's own room holds, each on the
//! best of its three rungs — `c commit` → `c cmt` → `c` — that still leaves
//! room for the hints ahead of it ([`ladder`], ADR-0039's third amendment).
//! Never all hidden, never a hint cut in half, and never below the three
//! most relevant at their short rung (ADR-0037 §4).
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
//! `Journal` and `Stash`, which the note leaves out, taken from
//! `docs/design/02-keymap.md` §2.11 and §2.9.
//!
//! **A hint is only drawn once its key is bound.** Every entry carries a
//! [`HintState`]: the table stays as the design wrote it, but
//! [`HintState::Planned`] hints are dropped before the width rule
//! runs, so the bar never teaches a key that does nothing (m5-03 review,
//! Critical 2). `graft` proves both halves —
//! `update::tests::every_hinted_key_is_bound` (Live ⇒ bound, key *and* verb)
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
    /// `update::tests::every_hinted_key_is_bound` compares against the
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
    /// A branch badge is being dragged (`docs/design/02-keymap.md` §2.15).
    /// The gesture ends with the button, not a key, so the bar says what
    /// the pointer must do and which key backs out.
    Dragging,
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
    match context {
        HintContext::Preview => Vec::new(),
        // No width tier for the banner: every key it answers draws while
        // the width allows it, `⇥ graph` — the way out of the mode —
        // included. Three keys at 80 columns would have hidden the exit of
        // a modal that swallows every other key, which is the failure
        // `banner_key`'s own doc comment warns of.
        // A drag takes no doorway either, and for the banner's reason: `.`
        // does not open the menu while a badge is in the air, it cancels the
        // drag. Two hints fit at every width the app draws at.
        HintContext::Banner { .. } | HintContext::Dragging => {
            fit_list(&drawable(context, divider), None, columns)
        }
        HintContext::Marked(inner) => {
            // `.` is already on screen as `. explain`, so no `. more`; it
            // is prepended before fitting and not after, because a list
            // fitted first and prepended to second overflows by one hint.
            let mut list = vec![EXPLAIN];
            list.extend(drawable(inner, divider));
            fit_list(&list, None, columns)
        }
        _ => fit_list(&drawable(context, divider), Some(MORE), columns),
    }
}

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
        && !matches!(context, HintContext::Banner { .. } | HintContext::Dragging)
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
        for extra in TAIL {
            if !list.iter().any(|hint| hint.key == extra.key) {
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
/// 1. the floor (ADR-0037 §4) — the three most relevant, at their **short**
///    rung, plus the doorway. Not a step: drawn whatever the width, because
///    "never all hidden" outranks fitting;
/// 2. those three to their full verbs. The leading verbs are the ones worth
///    the most cells, and they go first so that 80 columns still reads
///    `⏎ open  c commit  y sha` and not three abbreviations;
/// 3. every remaining hint, in priority order, at its **bare key**. A hint
///    may only rise above its key once every hint ahead of it is at least
///    present — that is the "leaves room for the hints ahead of it" clause,
///    and it is what makes the tail of bare keys exist at all;
/// 4. those tail hints raised, one at a time from the most relevant end,
///    short rung then full.
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
/// below their short rung — ADR-0037 §4's floor, the only number in the
/// rule. Below that the bar overflows and the footer cuts it, which is
/// still better than the one outcome the ADR forbids.
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
        // The drag is a mouse gesture: `release` is not a key, it is what
        // the user's hand has to do, and `esc` is the one key that answers
        // while a badge is in the air (`graft::update::drag::drag_cancel`).
        // Nothing else is offered because nothing else acts — every other
        // key cancels the drag and then means what it always means.
        HintContext::Dragging => vec![hint("release", "drop on a row"), hint("esc", "cancel")],
        // Both are answered in `hints_for` before this is reached; a
        // `RowContext` sub-enum would say so in the type, at the cost of
        // wrapping every row context at every call site.
        HintContext::Marked(_) | HintContext::Preview => Vec::new(),
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
        list.push(hint("M", "mergetool"));
        list.push(hint("e", "edit"));
    }
    list.push(hint("⇥", "graph"));
    list
}

/// The changes pane (design note, "Changes pane"). On an unmerged file the
/// write keys swap for the conflict keys — `M mergetool` and `space
/// resolved` are M6's to build (`space` is *refused* on an unmerged entry
/// today, `update::operate`), so until then the pane keeps its own keys,
/// minus the two that would be a lie.
fn changes_pane_list(unmerged: bool) -> Vec<Hint> {
    if unmerged {
        vec![
            planned("M", "mergetool"),
            hint("e", "edit"),
            planned("space", "resolved"),
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

/// Every hint in every context's table, planned ones included, deduped by
/// `(key, verb, state)` — what `graft`'s guard tests walk so that neither
/// half of the table can drift from what the app binds.
#[must_use]
pub fn every_hint() -> Vec<Hint> {
    let mut all = Vec::new();
    let contexts = [
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
        HintContext::ChangesPane { unmerged: false },
        HintContext::ChangesPane { unmerged: true },
        HintContext::Journal,
        HintContext::Stash,
        HintContext::Dragging,
    ];
    for context in contexts {
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
