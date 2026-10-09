//! The groups before the query: `IN PROGRESS`, `HERE`, `EVERYWHERE` under
//! `?`; `ACTIONS` and recent runs under `⌃P`. Spelling and filter are
//! `listing.rs`'s.

#[allow(clippy::wildcard_imports)] // a split of one module: its parent is its prelude
use super::*;

/// Every group before the query: `YOUR CONFIG` first when there are
/// problems; then, under `?`, `IN PROGRESS`, `HERE`, `EVERYWHERE`; under
/// `⌃P`, one `ACTIONS` group and the app rows.
pub(super) fn groups(
    source: &Source<'_>,
    palette: &PaletteState,
) -> Vec<(&'static str, Vec<Entry>)> {
    let keymap = source.keymap;
    let at = palette.opened_from;
    let mut out = Vec::new();
    if !source.problems.is_empty() {
        let notes = source
            .problems
            .iter()
            .map(|words| Entry {
                row: PaletteRow {
                    label: sanitize_line(words),
                    key: None,
                    config: None,
                    kind: RowKind::App,
                },
                target: PaletteTarget::Note,
                search: &[],
            })
            .collect();
        out.push(("YOUR CONFIG", notes));
    }
    let in_progress: Vec<Entry> = if source.operation_live {
        // Spelled where the operation takes them: the banner's own keys.
        keymap
            .in_context(ContextId::Banner)
            .into_iter()
            .filter(|(b, _)| listed(b))
            // A merge has no step to skip: its banner answers `s` only with
            // a refusal, so the row is not offered.
            .filter(|(b, _)| source.skippable || b.action != ActionId("operation.skip"))
            .filter_map(|(b, _)| {
                key_in(keymap, ContextId::Banner, b).map(|key| action(b, key, ContextId::Banner))
            })
            .collect()
    } else {
        Vec::new()
    };
    let taken: Vec<ActionId> = in_progress.iter().filter_map(action_of).collect();
    let here = place_rows(keymap, at, false, &taken);
    let everywhere = place_rows(keymap, at, true, &taken);
    match palette.mode {
        PaletteMode::Here(_) => {
            out.push(("IN PROGRESS", in_progress));
            out.push(("HERE", here));
            out.push(("EVERYWHERE", everywhere));
        }
        PaletteMode::Everything => {
            let actions = everything(in_progress, here, everywhere);
            let recent = recent_rows(source.recent, &actions);
            // Empty box: recent first; typed: last (ADR-0061).
            if palette.query.text().trim().is_empty() {
                out.push((RECENT_HEADER, recent));
                out.push(("ACTIONS", actions));
            } else {
                out.push(("ACTIONS", actions));
                out.push((RECENT_HEADER, recent));
            }
        }
    }
    out
}

/// COMMANDS RUN RECENTLY: the action row as ACTIONS draws it, for each
/// recent run still listed (ADR-0061), newest first.
pub(super) fn recent_rows(recent: &RecentRuns, actions: &[Entry]) -> Vec<Entry> {
    recent
        .iter()
        .filter_map(|target| actions.iter().find(|e| same_command(e.target, target)))
        .map(|entry| Entry {
            row: PaletteRow {
                kind: RowKind::Recent,
                ..entry.row.clone()
            },
            ..entry.clone()
        })
        .collect()
}

/// The app row that starts the practice tour (m8/19b): `⌃P` "tour".
pub(super) const TOUR_LABEL: &str = "Practice tour";

/// `ACTIONS`: every row of the three groups once, then the app rows: the
/// themes, and `Practice tour` unless a key row already starts it (the
/// empty state's `empty_state.tour`, `t`).
pub(super) fn everything(
    in_progress: Vec<Entry>,
    here: Vec<Entry>,
    everywhere: Vec<Entry>,
) -> Vec<Entry> {
    let mut actions: Vec<Entry> = Vec::new();
    for entry in in_progress.into_iter().chain(here).chain(everywhere) {
        let id = action_of(&entry);
        if !actions.iter().any(|kept| action_of(kept) == id) {
            actions.push(entry);
        }
    }
    actions.extend(THEMES.iter().map(|(kind, label)| Entry {
        row: PaletteRow {
            label: sanitize_line(label),
            key: None,
            config: None,
            kind: RowKind::App,
        },
        target: PaletteTarget::App(AppAction::Theme(*kind)),
        search: &[],
    }));
    let tour_row = Some(ActionId("empty_state.tour"));
    if !actions.iter().any(|kept| action_of(kept) == tour_row) {
        actions.push(Entry {
            row: PaletteRow {
                label: sanitize_line(TOUR_LABEL),
                key: None,
                config: None,
                kind: RowKind::App,
            },
            target: PaletteTarget::App(AppAction::Tour),
            search: &[],
        });
    }
    actions
}

/// `HERE` (`global` false: rows not bound in `Global`) or `EVERYWHERE`
/// (`global` true): first each row a key reaches in `at`, in resolution
/// order, spelled with that key; then the place's own rows (`at`'s, or
/// `Global`'s) no key reaches there, spelled `—`. Less `taken` (the
/// `IN PROGRESS` rows).
pub(super) fn place_rows(
    keymap: &Keymap,
    at: ContextId,
    global: bool,
    taken: &[ActionId],
) -> Vec<Entry> {
    let wanted = |b: &Binding| {
        listed(b) && (b.context == ContextId::Global) == global && !taken.contains(&b.action)
    };
    let mut out: Vec<Entry> = keymap
        .reachable(at)
        .into_iter()
        .filter(|(b, _)| wanted(b))
        .filter_map(|(b, _)| key_in(keymap, at, b).map(|key| action(b, key, at)))
        .collect();
    let own = if global { ContextId::Global } else { at };
    for (b, _) in keymap.in_context(own) {
        if wanted(b) && key_in(keymap, at, b).is_none() {
            out.push(action(b, KeyLabel("—".to_owned()), at));
        }
    }
    out
}
