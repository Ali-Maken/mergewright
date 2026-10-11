//! The listing: the rows a palette shows, filtered by its query.

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;
use graft_ui::conflict_resolver::SideRoles;

pub(super) fn source_of(state: &AppState) -> Source<'_> {
    Source {
        keymap: &state.keymap,
        problems: &state.config_problems,
        operation_live: crate::update::banner::live(state).is_some(),
        skippable: crate::update::banner::live(state)
            .is_some_and(|live| live.kind != graft_git::state::InProgress::Merge),
        recent: &state.palette_recent,
        finds: true,
        tags_read: state.refs.is_some(),
        sides: state
            .resolver
            .is_some()
            .then(|| crate::update::resolver::side_names(state).roles),
    }
}

/// What the empty state lists from: no repository, so no finds and no
/// recent runs (it has no `AppState` to keep them in).
pub(crate) fn outside_source<'a>(keymap: &'a Keymap, problems: &'a [String]) -> Source<'a> {
    Source {
        keymap,
        problems,
        operation_live: false,
        skippable: false,
        recent: &NO_RECENT,
        finds: false,
        tags_read: false,
        sides: None,
    }
}

/// The palette's rows as drawn: groups in order, filtered by the query.
pub(crate) fn rows(state: &AppState, palette: &PaletteState) -> Vec<Group> {
    listing(&source_of(state), palette)
}

/// [`rows`] from a [`Source`]: the groups of decision 2, filtered and
/// ordered by decision 4.
/// Under `⌃P` with something typed, the find groups come after ACTIONS and
/// before COMMANDS RUN RECENTLY (§2.12, ticket 12).
pub(crate) fn listing(source: &Source<'_>, palette: &PaletteState) -> Vec<Group> {
    let mut groups = groups(source, palette);
    if let Some(roles) = source.sides {
        for (_, entries) in &mut groups {
            for entry in entries.iter_mut() {
                if let Some(words) = action_of(entry).and_then(|id| side_label(id, roles)) {
                    entry.row.label = sanitize_line(&words);
                }
            }
        }
    }
    let mut out = filter(
        groups,
        palette.query.text(),
        source.keymap,
        palette.opened_from,
    );
    let query = palette.query.text().trim();
    if palette.mode == PaletteMode::Everything && source.finds && !query.is_empty() {
        let at = out
            .iter()
            .position(|(header, _)| header.as_str() == RECENT_HEADER)
            .unwrap_or(out.len());
        let finds = find_groups(&palette.find, query, source.tags_read, source.keymap);
        out.splice(at..at, finds);
    }
    out
}

/// The resolver's side keys named as its bar names them: `o` is `Take
/// upstream` in a rebase, `Take yours` in a merge (ADR-0043 §4, review
/// m8-usability B). `None` for every other action: its table label stands.
fn side_label(action: ActionId, roles: SideRoles) -> Option<String> {
    let (ours, theirs) = (roles.ours(), roles.theirs());
    Some(match action.0 {
        "resolver.take_ours" => format!("Take {ours}"),
        "resolver.take_theirs" => format!("Take {theirs}"),
        "resolver.all_ours" => format!("{} in every block", capital(ours)),
        "resolver.all_theirs" => format!("{} in every block", capital(theirs)),
        "resolver.file_ours" => format!("{} for the whole file, done", capital(ours)),
        "resolver.file_theirs" => format!("{} for the whole file, done", capital(theirs)),
        _ => return None,
    })
}

/// `word` with its first letter upper-case (the role words are ASCII).
fn capital(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// The `more` row under [`COMMIT_HITS`] commits: `⏎` hands the query to `/`.
/// Names the find key as his keymap spells it; unbound, no key.
pub(crate) fn commits_more(keymap: &Keymap) -> String {
    match keymap.spell_keys(&[ActionId("app.find")]) {
        Some(keys) => format!("more · {keys} searches all of history"),
        None => "more · search all of history".to_owned(),
    }
}

/// COMMITS for `query`: the current answer's hits, then [`commits_more`]
/// when the job stopped at [`COMMIT_HITS`]; its failure as one row; while
/// the query's job runs, the last answer's hits still matching by sha or
/// subject, then `searching…`.
pub(super) fn commit_rows(
    commits: &Asked<Result<Arc<[CommitHit]>, String>>,
    query: &str,
    keymap: &Keymap,
) -> Rows {
    if query.is_empty() {
        return Vec::new();
    }
    match &commits.answer {
        Some((asked, Ok(hits))) if asked == query => {
            let mut rows: Vec<_> = hits.iter().map(commit_row).collect();
            if hits.len() >= COMMIT_HITS {
                let (row, _) = placeholder(&commits_more(keymap));
                rows.push((row, PaletteTarget::More(FindGroup::Commits)));
            }
            rows
        }
        Some((asked, Err(why))) if asked == query => {
            vec![placeholder(&format!(
                "commits: search failed: {}",
                one_line(why)
            ))]
        }
        stale => {
            let parsed = crate::search::Query::parse(query);
            let mut rows: Vec<_> = match (stale, parsed) {
                (Some((_, Ok(hits))), Some(parsed)) => hits
                    .iter()
                    .filter(|hit| parsed.matches(hit.id, &hit.subject, b"", b""))
                    .map(commit_row)
                    .collect(),
                _ => Vec::new(),
            };
            rows.push(placeholder("searching…"));
            rows
        }
    }
}

/// `a1b2c3d  subject`: the widget mutes the sha.
pub(super) fn commit_row(hit: &CommitHit) -> (PaletteRow, PaletteTarget) {
    let sha = hit.id.to_string();
    let short = sha.get(..7).unwrap_or(&sha);
    (
        PaletteRow {
            label: sanitize_line(&format!(
                "{short}  {}",
                String::from_utf8_lossy(&hit.subject)
            )),
            key: None,
            config: None,
            kind: RowKind::Commit,
        },
        PaletteTarget::Seek(hit.id),
    )
}

/// A muted row `⏎` does nothing on: [`RowKind::More`], a note.
pub(super) fn placeholder(words: &str) -> (PaletteRow, PaletteTarget) {
    (
        PaletteRow {
            label: sanitize_line(words),
            key: None,
            config: None,
            kind: RowKind::More,
        },
        PaletteTarget::Note,
    )
}

/// A placeholder row: muted, `⏎` does nothing.
pub(super) fn is_placeholder(row: &PaletteRow, target: PaletteTarget) -> bool {
    row.kind == RowKind::More && target == PaletteTarget::Note
}

/// Whether a row can be listed at all: something can run it, and it is not
/// a text field's or the confirm box's.
pub(super) fn listed(binding: &Binding) -> bool {
    !binding.default.is_empty() && !NEVER_LISTED.contains(&binding.context)
}

/// The key that does `binding` when pressed in `at`, his lines applied:
/// the first of its chords that acts as it there and is not a key nobody
/// answers. `None` when no chord of his does it there.
pub(super) fn key_in(keymap: &Keymap, at: ContextId, binding: &Binding) -> Option<KeyLabel> {
    keymap
        .chords(&binding.action)
        .iter()
        .find(|chord| {
            keymap.acting(at, **chord) == Some(binding.action)
                && keymap.translate(at, **chord) != Translated::Unbound
        })
        .map(KeyChord::label)
}

/// A row for `binding`, spelled `key` in `ctx`, where `⏎` runs it.
pub(super) fn action(binding: &Binding, key: KeyLabel, ctx: ContextId) -> Entry {
    Entry {
        row: PaletteRow {
            label: sanitize_line(binding.label),
            key: Some(key),
            config: Some(binding.action),
            kind: RowKind::Action,
        },
        target: PaletteTarget::Action {
            id: binding.action,
            ctx,
        },
        search: binding.search,
    }
}

pub(super) fn action_of(entry: &Entry) -> Option<ActionId> {
    match entry.target {
        PaletteTarget::Action { id, .. } => Some(id),
        PaletteTarget::App(_)
        | PaletteTarget::Note
        | PaletteTarget::Seek(_)
        | PaletteTarget::File(_)
        | PaletteTarget::More(_) => None,
    }
}

/// Decision 4. Empty query: every group with rows. Typed: each row scored
/// by its label and search words (`graft_ops::matcher`), groups in their
/// order, rows by score then table order, empty groups dropped. A query
/// that is a chord bound in `at` puts that row first, its group first.
pub(super) fn filter(
    groups: Vec<(&'static str, Vec<Entry>)>,
    query: &str,
    keymap: &Keymap,
    at: ContextId,
) -> Vec<Group> {
    let typed = query.trim();
    let chord_row = KeyChord::parse(typed)
        .ok()
        .filter(|chord| keymap.translate(at, *chord) != Translated::Unbound)
        .and_then(|chord| keymap.acting(at, chord));
    let mut out: Vec<(bool, Group)> = Vec::new();
    for (header, entries) in groups {
        let mut kept: Vec<(u32, usize, Entry)> = entries
            .into_iter()
            .enumerate()
            .filter_map(|(order, entry)| {
                if typed.is_empty() {
                    return Some((0, order, entry));
                }
                let hit = chord_row.is_some() && action_of(&entry) == chord_row;
                let score = std::iter::once(entry.row.label.as_str())
                    .chain(entry.search.iter().copied())
                    .filter_map(|text| fuzzy_score(typed, text))
                    .min();
                match (hit, score) {
                    // The chord's own row heads its group.
                    (true, _) => Some((0, 0, entry)),
                    (false, Some(score)) => Some((score, order + 1, entry)),
                    (false, None) => None,
                }
            })
            .collect();
        if kept.is_empty() {
            continue;
        }
        kept.sort_by_key(|(score, order, _)| (*score, *order));
        let has_chord = !typed.is_empty()
            && chord_row.is_some()
            && kept
                .first()
                .is_some_and(|(_, _, entry)| action_of(entry) == chord_row);
        let rows = kept
            .into_iter()
            .map(|(_, _, entry)| (entry.row, entry.target))
            .collect();
        out.push((has_chord, (sanitize_line(header), rows)));
    }
    // The chord's group first; the rest keep their order (a stable sort).
    out.sort_by_key(|(has_chord, _)| !has_chord);
    out.into_iter().map(|(_, group)| group).collect()
}

/// One shown row, flattened: its group's header, the row, what it runs.
pub(super) type Flat = (String, PaletteRow, PaletteTarget);

pub(super) fn flat(groups: Vec<Group>) -> Vec<Flat> {
    groups
        .into_iter()
        .flat_map(|(header, rows)| {
            let header = header.as_str().to_owned();
            rows.into_iter()
                .map(move |(row, target)| (header.clone(), row, target))
        })
        .collect()
}
