//! Where the message column's pieces land: the message, a folding merge's
//! tail, the row tag and the labels after them (ease-pass G6, G7) — and
//! which ref each label and the branch column's name stand for, the cells a
//! drag picks a branch up from (§2.15).
//!
//! **One packing, read by both sides.** [`MessageLayout::of`] is what
//! `GraphScreen` draws from and what [`ref_spans`] reports, so the mouse
//! hit-test (`mergewright::view::hit`) and the drawn cells cannot drift.
//! Pure arithmetic over text widths: no buffer, no theme.

use std::borrow::Cow;
use std::ops::Range;

use unicode_width::UnicodeWidthStr;

use super::{Badge, BadgeKind, Columns, GraphEntry, Icons, TAG_GAP, TAG_MIN_SUBJECT, Tracking};
use crate::keymap::{ActionId, Keymap};

/// Cells between the subject and a folding merge's tail: the mock's two.
pub(super) const FOLD_GAP: u16 = 2;

/// Cells before each label: the mock's two.
pub(super) const LABEL_GAP: u16 = 2;

/// One label as drawn: its cells, the badge it reads, and its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlacedLabel {
    pub(super) cells: Range<u16>,
    pub(super) badge: usize,
    pub(super) words: String,
}

/// The message column of one row, packed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MessageLayout {
    /// Cells the subject is drawn into (cut with `…` when it is narrower
    /// than the subject).
    pub(super) subject: Range<u16>,
    /// The subject needed a `…` to fit.
    pub(super) truncated: bool,
    /// A folding merge's tail and where its words start (its gap before).
    pub(super) fold: Option<(u16, String)>,
    /// The row tag and where its words start (its gap before).
    pub(super) tag: Option<(u16, String)>,
    /// The labels, left to right.
    pub(super) labels: Vec<PlacedLabel>,
}

impl MessageLayout {
    /// Packs `entry`'s message column. **The subject gives way last**
    /// (ease-pass 09 ruling W, the main mock's one-cell order): first the
    /// labels drop, right-most first and always whole; then the fold tail
    /// sheds ` from <branch>`, then its count, then drops; only then is the
    /// subject cut with `…`. The row tag keeps its own rule (m7/09): the
    /// subject is cut for it, and it is left off below [`TAG_MIN_SUBJECT`]
    /// subject cells. Everything after the subject follows its last
    /// character rather than a fixed column.
    pub(super) fn of(
        entry: &GraphEntry<'_>,
        (icons, keys): (Icons, &Keymap),
        ascii: bool,
        columns: &Columns,
    ) -> Self {
        let x = columns.subject_x;
        let width = columns.subject_width;
        let subject_width =
            u16::try_from(super::walk_text::text_width(entry.subject.as_str())).unwrap_or(u16::MAX);
        let words = labels(entry, icons, keys);
        let tag = drawn_tag(entry, ascii, width);
        let tag_width = tag.as_deref().map_or(0, |tag| cells(tag) + TAG_GAP);
        let whole = subject_width.saturating_add(tag_width);
        let (count, fold) = keep(whole, width, &words, fold_forms(entry, ascii));
        let fold_width = fold.as_deref().map_or(0, |mark| cells(mark) + FOLD_GAP);
        let room = width.saturating_sub(fold_width + tag_width);
        let used = subject_width.min(room);

        let mut at = x + used;
        let fold = fold.map(|mark| {
            let start = at + FOLD_GAP;
            at += fold_width;
            (start, mark)
        });
        let tag = tag.map(|tag| {
            let start = at + TAG_GAP;
            at += tag_width;
            (start, tag.into_owned())
        });
        let labels = words
            .into_iter()
            .take(count)
            .map(|(badge, text)| {
                let start = at + LABEL_GAP;
                at = start + cells(&text);
                PlacedLabel {
                    cells: start..at,
                    badge,
                    words: text,
                }
            })
            .collect();
        MessageLayout {
            subject: x..x + used,
            truncated: subject_width > room,
            fold,
            tag,
            labels,
        }
    }
}

/// How many labels (a prefix, in drawing order) and which fold-tail form
/// fit beside `whole` cells of subject and tag in `width`: labels drop
/// first with the whole tail kept, then the tail shortens form by form.
/// `(0, None)` when even the subject alone does not fit — it is cut then.
fn keep(
    whole: u16,
    width: u16,
    words: &[(usize, String)],
    forms: Vec<String>,
) -> (usize, Option<String>) {
    let tail = |form: Option<&String>| form.map_or(0, |mark| cells(mark) + FOLD_GAP);
    let full = whole.saturating_add(tail(forms.first()));
    let mut wanted = full;
    let mut fit = (full <= width).then_some(0);
    for (n, (_, text)) in words.iter().enumerate() {
        wanted = wanted.saturating_add(cells(text).saturating_add(LABEL_GAP));
        if wanted <= width {
            fit = Some(n + 1);
        }
    }
    let mut forms = forms.into_iter();
    let first = forms.next();
    if let Some(n) = fit {
        return (n, first);
    }
    let fold = forms.find(|form| whole.saturating_add(tail(Some(form))) <= width);
    (0, fold)
}

/// The screen columns each ref drawn on `entry`'s row occupies: one
/// `(range, index)` per label that had room and for the branch column's
/// name when that branch's own ref sits on this row, `index` naming its
/// place in [`GraphEntry::badges`]. What a drag starts from (§2.15) and
/// what the renderer draws the labels at — the same [`MessageLayout`].
#[must_use]
pub fn ref_spans(
    entry: &GraphEntry<'_>,
    columns: &Columns,
    icons: Icons,
    keys: &Keymap,
    ascii: bool,
) -> Vec<(Range<u16>, usize)> {
    let mut spans: Vec<(Range<u16>, usize)> = Vec::new();
    if let Some((cells, badge)) = branch_span(entry, columns) {
        spans.push((cells, badge));
    }
    spans.extend(
        MessageLayout::of(entry, (icons, keys), ascii, columns)
            .labels
            .into_iter()
            .map(|label| (label.cells, label.badge)),
    );
    spans
}

/// The branch column's name and the badge of the branch it names, when
/// that branch's ref is on this row. Not on a row inside an opened merge:
/// its name is a merged branch's, which no ref may still carry.
fn branch_span(entry: &GraphEntry<'_>, columns: &Columns) -> Option<(Range<u16>, usize)> {
    let cell = entry.branch.as_ref().filter(|cell| !cell.dim)?;
    let badge = entry.badges.iter().position(|badge| {
        matches!(badge.kind, BadgeKind::Branch { .. }) && badge.name == cell.name
    })?;
    let width = cells(&branch_words(cell, columns.branch_width)).min(columns.branch_width);
    (width > 0).then(|| (columns.branch_x..columns.branch_x + width, badge))
}

/// The branch column's words in `width` cells: the name, and ` (you)` on
/// `HEAD`'s. A name too long is cut, never ` (you)`: whose branch it is
/// matters more than the name's tail.
pub(super) fn branch_words(cell: &super::BranchCell, width: u16) -> Cow<'_, str> {
    const YOU: &str = " (you)";
    if !cell.you {
        return Cow::Borrowed(cell.name.as_str());
    }
    let room = usize::from(width).saturating_sub(YOU.len());
    // `fit` pads a short name to `room`: only a name too long goes through it.
    if room == 0 || cell.name.as_str().width() <= room {
        return Cow::Owned(format!("{}{YOU}", cell.name.as_str()));
    }
    Cow::Owned(format!(
        "{}{YOU}",
        crate::fit::fit(cell.name.as_str(), room)
    ))
}

/// Each badge's label words, in drawing order: the compare base first, then
/// the push words of the branch the branch column names on this row, then
/// the rest as `app` listed them. A badge with nothing to say (the column's
/// branch, level with its upstream) is left out.
fn labels(entry: &GraphEntry<'_>, icons: Icons, keys: &Keymap) -> Vec<(usize, String)> {
    let named = entry
        .branch
        .as_ref()
        .filter(|cell| !cell.dim)
        .map(|cell| cell.name.as_str());
    let mut words: Vec<(u8, usize, String)> = entry
        .badges
        .iter()
        .enumerate()
        .filter_map(|(index, badge)| {
            let in_column = named == Some(badge.name.as_str())
                && matches!(badge.kind, BadgeKind::Branch { .. });
            let rank = match badge.kind {
                BadgeKind::CompareBase => 0,
                _ if in_column => 1,
                _ => 2,
            };
            label_words(badge, icons, keys, in_column).map(|text| (rank, index, text))
        })
        .collect();
    words.sort_by_key(|(rank, index, _)| (*rank, *index));
    words
        .into_iter()
        .map(|(_, index, text)| (index, text))
        .collect()
}

/// One ref's label (ease-pass G7): `<branch> is here`, a local branch's
/// push state after its name (`↑2 to push`, `not pushed yet`), `tag v1.2`.
/// `in_column`: the branch column already names this branch on this row,
/// so only its push state is said — `None` when it has none to say. With
/// [`Icons::Nerd`] a ref's label leads with its kind's icon.
#[must_use]
pub fn label_words(badge: &Badge, icons: Icons, keys: &Keymap, in_column: bool) -> Option<String> {
    let words = plain_words(badge, keys, in_column)?;
    Some(match icons {
        Icons::Nerd if !in_column => format!("{} {words}", badge.nerd_icon()),
        _ => words,
    })
}

fn plain_words(badge: &Badge, keys: &Keymap, in_column: bool) -> Option<String> {
    let name = badge.name.as_str();
    Some(match &badge.kind {
        BadgeKind::Branch { tracking, .. } => {
            let push = match tracking {
                Tracking::Untracked => Some("not pushed yet".to_owned()),
                Tracking::Apart { ahead, .. } if *ahead > 0 => {
                    Some(format!("\u{2191}{ahead} to push"))
                }
                Tracking::Apart { .. } | Tracking::InSync { .. } => None,
            };
            match (in_column, push) {
                (true, push) => return push,
                (false, Some(push)) => format!("{name} is here  {push}"),
                (false, None) => format!("{name} is here"),
            }
        }
        BadgeKind::Remote { .. } => format!("{name} is here"),
        BadgeKind::Tag => format!("tag {name}"),
        BadgeKind::Head => "HEAD is here".to_owned(),
        BadgeKind::Stash => name.to_owned(),
        BadgeKind::CompareBase => keys
            .key_verb(&[ActionId("graph.compare")], "base")
            .unwrap_or_else(|| "base".to_owned()),
        BadgeKind::OnlyInStash(n) => format!("only in stash@{{{n}}}"),
    })
}

/// The most badges a row can draw on a screen `columns` wide. A row
/// builder may stop at this many and [`ref_spans`] returns the same spans,
/// so the same cells are drawn, whatever the commit carries (m8/12c:
/// 16 000 refs on one commit cost 16 000 badges a row a frame).
///
/// Why this many is enough: labels are drawn whole or not at all, inside a
/// message column narrower than the screen, and each takes at least its
/// two-cell gap and one cell, so fewer than `columns / 2` ever fit. Pinned by `tests::many_refs_row_draws_like_its_first_badges`.
#[must_use]
pub fn badges_that_fit(columns: u16) -> usize {
    usize::from(columns / 2) + 1
}

/// Whether `entry`'s subject needed a `…` to fit the message column it is
/// drawn into, labels, fold tail and tag taken first. What the footer
/// reveal ([`GraphView::reveal`](super::GraphView)) keys off.
pub(super) fn subject_truncated(
    entry: &GraphEntry<'_>,
    look: (Icons, &Keymap),
    ascii: bool,
    columns: &Columns,
) -> bool {
    MessageLayout::of(entry, look, ascii, columns).truncated
}

/// The row tag's words in a column of `width` cells: `None` for an
/// untagged row, or when fewer than [`TAG_MIN_SUBJECT`] cells would be left
/// for the subject.
fn drawn_tag<'e>(entry: &'e GraphEntry<'_>, ascii: bool, width: u16) -> Option<Cow<'e, str>> {
    let tag = entry.tag.as_ref()?.words(ascii);
    (width.saturating_sub(cells(&tag).saturating_add(TAG_GAP)) >= TAG_MIN_SUBJECT).then_some(tag)
}

/// A folding merge's tail forms, longest first, each said once: the whole
/// words, `▸ 4 commits`, `▸` alone (06r: it shortens before it drops and
/// never cuts a word). Empty off a folding merge.
fn fold_forms(entry: &GraphEntry<'_>, ascii: bool) -> Vec<String> {
    let mut forms = Vec::with_capacity(3);
    for form in entry.fold.iter().flat_map(|mark| mark.forms(ascii)) {
        if !forms.contains(&form) {
            forms.push(form);
        }
    }
    forms
}

fn cells(text: &str) -> u16 {
    u16::try_from(text.width()).unwrap_or(u16::MAX)
}
