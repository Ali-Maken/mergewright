//! Search over history (`/`, m1-the-graph/26): the query a user typed and the
//! pure test of one commit against it, plus how a search job stands.
//!
//! Matching is a case-insensitive literal substring over the raw repository
//! bytes — subject, author name and email, the full message body — or a prefix
//! of the commit id. No regex and no query language. Nothing here draws: text
//! shown for a hit goes through `SanitizedText` like every other repo string.

use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::history::{Order, Walk};

use crate::repos::RepoStore;

/// A parsed search query: the literal text, folded for case-insensitive
/// matching, and whether it could also name a commit by id prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// What the user typed, verbatim (for the prompt and the status line).
    raw: String,
    /// `raw` lowercased; compared against lowercased haystacks.
    folded: String,
    /// `raw` as a lowercase hex prefix, when it is one.
    sha_prefix: Option<String>,
}

impl Query {
    /// Parse a typed query. `None` for an empty (or all-whitespace) query:
    /// there is nothing to search for.
    pub fn parse(raw: &str) -> Option<Query> {
        if raw.trim().is_empty() {
            return None;
        }
        let folded = raw.to_lowercase();
        let sha_prefix = folded
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
            .then(|| folded.clone());
        Some(Query {
            raw: raw.to_owned(),
            folded,
            sha_prefix,
        })
    }

    /// The query as typed.
    pub fn text(&self) -> &str {
        &self.raw
    }

    /// Does this commit match? `subject`, `author` (name and email, however the
    /// caller joins them) and `body` are raw repository bytes, not necessarily
    /// UTF-8.
    pub fn matches(&self, id: Oid, subject: &[u8], author: &[u8], body: &[u8]) -> bool {
        if let Some(prefix) = &self.sha_prefix
            && id.to_string().starts_with(prefix.as_str())
        {
            return true;
        }
        [subject, author, body]
            .into_iter()
            .any(|field| self.found_in(field))
    }

    fn found_in(&self, haystack: &[u8]) -> bool {
        let needle = self.folded.as_bytes();
        if self.folded.is_ascii() {
            // ASCII needle: fold bytes in place, no allocation, and still
            // correct over non-UTF-8 haystacks.
            return haystack
                .windows(needle.len())
                .any(|window| window.eq_ignore_ascii_case(needle));
        }
        match std::str::from_utf8(haystack) {
            Ok(text) => text.to_lowercase().contains(self.folded.as_str()),
            // A non-ASCII needle can only match valid UTF-8 text; a byte
            // comparison is the best a malformed haystack allows.
            Err(_) => haystack
                .windows(needle.len())
                .any(|window| window == needle),
        }
    }
}

/// Where a search job stands. Rendered by state: a running or finished search
/// with no hits is not an error, and only `Failed` reads as one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchState {
    /// Still reading history; `hits` so far, and more may come.
    Running {
        /// Matches found so far.
        hits: usize,
    },
    /// Read every commit; `hits` is final (zero is a result, not a failure).
    Done {
        /// Matches in all of history.
        hits: usize,
    },
    /// Stopped by the user before it finished; `hits` found by then.
    Cancelled {
        /// Matches found before the stop.
        hits: usize,
    },
    /// The walk failed; why.
    Failed(String),
}

impl SearchState {
    /// The status line's words for this state.
    pub fn describe(&self) -> String {
        match self {
            SearchState::Running { hits } => format!("{hits} {} so far", plural(*hits)),
            SearchState::Done { hits: 0 } => "no matches".to_owned(),
            SearchState::Done { hits } => format!("{hits} {}", plural(*hits)),
            SearchState::Cancelled { hits } => {
                format!("search cancelled, {hits} {} found", plural(*hits))
            }
            SearchState::Failed(why) => format!("search failed: {why}"),
        }
    }
}

fn plural(hits: usize) -> &'static str {
    if hits == 1 { "match" } else { "matches" }
}

/// How [`scan`] ended when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scanned {
    /// Every commit reachable from every ref was read.
    Finished,
    /// Stopped by `stop`, or `found` said nobody is listening.
    Stopped,
}

/// Reads history from every ref, `chunk` commits at a time, handing each
/// chunk's hits to `found` as soon as the chunk is read (a job's work). `found`
/// returns `false` when nobody is listening any more. `stop` is checked before
/// every chunk.
pub fn scan(
    store: &RepoStore,
    query: &Query,
    chunk: usize,
    stop: &dyn Fn() -> bool,
    found: &mut dyn FnMut(Arc<[Oid]>) -> bool,
) -> Result<Scanned, String> {
    walk_matching(store, query, chunk, stop, &mut |hits| {
        hits.is_empty() || found(hits.iter().map(|commit| commit.id).collect())
    })
}

/// One commit the palette's COMMITS group lists: its id and subject line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitHit {
    /// The commit.
    pub id: Oid,
    /// Its subject, raw repository bytes (sanitised when drawn).
    pub subject: Box<[u8]>,
}

/// How many commits [`first_hits`] reads per chunk: small, so a query that
/// hits early stops early, and still one `stop` check per chunk.
const FIRST_HITS_CHUNK: usize = 64;

/// The first `limit` commits [`scan`] would find, in the same walk order
/// (topological from every ref, newest first) — the palette's COMMITS group
/// (m8 12b). Stops reading as soon as `limit` are found; a query with fewer
/// hits walks all of history (a job's work, never the loop's).
///
/// # Errors
/// The walk's own failure, as words.
pub fn first_hits(
    store: &RepoStore,
    query: &Query,
    limit: usize,
    stop: &dyn Fn() -> bool,
) -> Result<Vec<CommitHit>, String> {
    let mut hits = Vec::new();
    if limit == 0 {
        return Ok(hits);
    }
    // `Stopped` and `Finished` both mean "this is all there is": the hits
    // in hand. A cancelled job's answer is dropped by its caller.
    walk_matching(store, query, FIRST_HITS_CHUNK, stop, &mut |found| {
        for commit in found.iter().take(limit - hits.len()) {
            hits.push(CommitHit {
                id: commit.id,
                subject: commit.subject().into(),
            });
        }
        hits.len() < limit
    })?;
    Ok(hits)
}

/// The walk both [`scan`] and [`first_hits`] share: each chunk's matching
/// commits (possibly none) go to `found`, which returns `false` to stop.
fn walk_matching(
    store: &RepoStore,
    query: &Query,
    chunk: usize,
    stop: &dyn Fn() -> bool,
    found: &mut dyn FnMut(&[&Commit]) -> bool,
) -> Result<Scanned, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let table = store.table().map_err(|error| error.to_string())?;
    let mut walk =
        Walk::all_refs(&handle, Order::Topological, table).map_err(|error| error.to_string())?;
    let mut author = Vec::new();
    loop {
        if stop() {
            return Ok(Scanned::Stopped);
        }
        let read = walk
            .next_chunk(chunk, stop)
            .map_err(|error| error.to_string())?;
        let hits: Vec<&Commit> = read
            .commits
            .iter()
            .filter(|commit| {
                // Name and email as `git log` shows them, reused per commit.
                author.clear();
                author.extend_from_slice(&commit.author.name);
                author.extend_from_slice(b" <");
                author.extend_from_slice(&commit.author.email);
                author.push(b'>');
                query.matches(commit.id, commit.subject(), &author, &commit.message)
            })
            .collect();
        if !found(&hits) {
            return Ok(Scanned::Stopped);
        }
        if read.stopped {
            return Ok(Scanned::Stopped);
        }
        if read.done {
            return Ok(Scanned::Finished);
        }
    }
}

/// The search on screen: its query, the hits found so far in walk order, which
/// one the selection is on, and where the selection was before it began.
///
/// Hits are kept as the batches the job sent, shared, because `AppState` is
/// cloned on every event: a broad query on a big repository would otherwise
/// copy every hit per key press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search {
    /// What is being searched for.
    pub query: Query,
    /// The job searching, once it has started and while it runs.
    pub job: Option<JobId>,
    /// Searches asked for whose job has not said it started yet. Only the
    /// last to start is this search's; earlier ones are superseded.
    pub unstarted: usize,
    /// Hits found, in batches, in walk order.
    pub batches: Vec<Arc<[Oid]>>,
    /// The hit the selection was sent to, if any.
    pub current: Option<usize>,
    /// Where the search stands.
    pub state: SearchState,
    /// Cursor row and viewport top before the search moved them.
    pub prior: (usize, usize),
    /// The commit a jump is still seeking, while it walks.
    pub seeking: Option<Oid>,
    /// The last hit sought is not a row in the view on screen (first-parent
    /// view, a folded merge, a hidden stash).
    pub missed: bool,
}

impl Search {
    /// A search for `query` just asked for, from a selection at `prior`.
    pub fn new(query: Query, unstarted: usize, prior: (usize, usize)) -> Self {
        Search {
            query,
            job: None,
            unstarted,
            batches: Vec::new(),
            current: None,
            state: SearchState::Running { hits: 0 },
            prior,
            seeking: None,
            missed: false,
        }
    }

    /// How many hits so far.
    pub fn hits(&self) -> usize {
        self.batches.iter().map(|batch| batch.len()).sum()
    }

    /// The `index`th hit, in walk order.
    pub fn hit(&self, index: usize) -> Option<Oid> {
        let mut index = index;
        for batch in &self.batches {
            match batch.get(index) {
                Some(id) => return Some(*id),
                None => index -= batch.len(),
            }
        }
        None
    }

    /// The hit `step` away from the current one (`n` is `+1`, `N` is `-1`),
    /// wrapping at either end; the first hit when none is current yet.
    pub fn step(&mut self, forward: bool) -> Option<Oid> {
        let hits = self.hits();
        if hits == 0 {
            return None;
        }
        let next = match (self.current, forward) {
            (None, _) => 0,
            (Some(at), true) => (at + 1) % hits,
            (Some(at), false) => (at + hits - 1) % hits,
        };
        self.current = Some(next);
        self.hit(next)
    }

    /// The footer's words: query, state, position, and the keys that apply,
    /// spelled from `keys` ([`keys_words`]).
    pub fn line(&self, keys: &graft_ui::keymap::Keymap) -> String {
        let position = match self.current {
            Some(at) => format!(" ({}/{})", at + 1, self.hits()),
            None => String::new(),
        };
        let missed = if self.missed {
            "  (not in this view)"
        } else {
            ""
        };
        let running = matches!(self.state, SearchState::Running { .. });
        let keys = keys_words(running, self.hits(), keys);
        format!(
            "/{}  {}{position}{missed}{keys}",
            self.query.text(),
            self.state.describe()
        )
    }
}

/// The search line's keys: `  n/N next/prev  esc close` (`esc cancel`
/// while running; no `n/N` with no hits). Spelled from the graph's rows —
/// `n`/`N` step on the translated key (`update/search.rs` `search_step`),
/// `esc` is the graph's `Esc` ladder. An unbound key's clause goes; with
/// one of `n`/`N` unbound the other keeps its own verb (`n next`).
#[must_use]
pub fn keys_words(running: bool, hits: usize, keys: &graft_ui::keymap::Keymap) -> String {
    use graft_ui::keymap::ActionId;
    let mut parts = Vec::new();
    if hits > 0 {
        let next = keys.spell_keys(&[ActionId("app.next_match")]);
        let prev = keys.spell_keys(&[ActionId("app.prev_match")]);
        match (next, prev) {
            (Some(next), Some(prev)) => parts.push(format!("{next}/{prev} next/prev")),
            (Some(next), None) => parts.push(format!("{next} next")),
            (None, Some(prev)) => parts.push(format!("{prev} prev")),
            (None, None) => {}
        }
    }
    let verb = if running { "cancel" } else { "close" };
    parts.extend(keys.key_verb(&[ActionId("app.back")], verb));
    let mut line = String::new();
    for part in parts {
        line.push_str("  ");
        line.push_str(&part);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(hex: &str) -> Oid {
        Oid::from_hex(hex).expect("valid hex")
    }

    const ID: &str = "abcdef0123456789abcdef0123456789abcdef01";

    #[test]
    fn empty_query_is_none() {
        assert_eq!(Query::parse(""), None);
        assert_eq!(Query::parse("   "), None);
    }

    #[test]
    fn matches_each_field_case_insensitively() {
        let q = Query::parse("Fix").unwrap();
        assert!(q.matches(id(ID), b"fix the thing", b"", b""));
        assert!(q.matches(id(ID), b"", b"Ann Fixer <ann@x>", b""));
        assert!(q.matches(id(ID), b"", b"", b"line one\nPREFIX\n"));
        assert!(!q.matches(id(ID), b"tidy", b"Bob <bob@x>", b"nothing"));
    }

    #[test]
    fn matches_author_email() {
        let q = Query::parse("ann@example").unwrap();
        assert!(q.matches(id(ID), b"s", b"Ann <ann@example.com>", b""));
    }

    #[test]
    fn sha_prefix_matches_only_as_prefix() {
        assert!(
            Query::parse("ABCDEF")
                .unwrap()
                .matches(id(ID), b"", b"", b"")
        );
        assert!(
            !Query::parse("cdef01")
                .unwrap()
                .matches(id(ID), b"", b"", b"")
        );
    }

    #[test]
    fn non_utf8_haystack_and_non_ascii_needle() {
        let q = Query::parse("bug").unwrap();
        assert!(q.matches(id(ID), b"\xff\xfeBUG", b"", b""));
        let q = Query::parse("ÉTÉ").unwrap();
        assert!(q.matches(id(ID), "un été".as_bytes(), b"", b""));
    }

    #[test]
    fn escape_sequence_subject_matches_literally() {
        let q = Query::parse("\u{1b}[31mred").unwrap();
        assert!(q.matches(id(ID), b"\x1b[31mred alert", b"", b""));
    }

    #[test]
    fn states_describe_without_error_words() {
        assert_eq!(
            SearchState::Running { hits: 12 }.describe(),
            "12 matches so far"
        );
        assert_eq!(SearchState::Done { hits: 1 }.describe(), "1 match");
        assert_eq!(SearchState::Done { hits: 0 }.describe(), "no matches");
        assert_eq!(
            SearchState::Cancelled { hits: 3 }.describe(),
            "search cancelled, 3 matches found"
        );
        assert!(
            SearchState::Failed("x".into())
                .describe()
                .contains("failed")
        );
    }

    use std::sync::atomic::{AtomicBool, Ordering};

    use graft_fixtures::Shape;

    /// Every commit a walk from every ref reaches, in walk order.
    fn walked(store: &RepoStore) -> Vec<Commit> {
        let repo = store.shared().expect("the fixture is a repository");
        let handle = repo.handle();
        let table = store.table().expect("the table opens");
        let mut walk = Walk::all_refs(&handle, Order::Topological, table).expect("walk starts");
        let mut commits = Vec::new();
        loop {
            let chunk = walk.next_chunk(500, &|| false).expect("walk reads");
            commits.extend(chunk.commits);
            if chunk.done {
                return commits;
            }
        }
    }

    /// Runs a scan two commits a chunk, collecting every batch.
    fn scanned(store: &RepoStore, raw: &str) -> (Scanned, Vec<Arc<[Oid]>>) {
        let query = Query::parse(raw).expect("a query");
        let mut batches = Vec::new();
        let ended = scan(store, &query, 2, &|| false, &mut |hits| {
            batches.push(hits);
            true
        })
        .expect("the scan reads");
        (ended, batches)
    }

    fn found(batches: &[Arc<[Oid]>], id: Oid) -> bool {
        batches.iter().any(|batch| batch.contains(&id))
    }

    /// A hit in each field on a real repository, for the last commit the walk
    /// reaches, so it lies well past the first two-commit chunk.
    #[test]
    fn a_scan_finds_the_last_commit_by_each_field() {
        let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
        let commits = walked(&store);
        assert!(commits.len() > 2, "the hit must lie past the first chunk");
        let last = commits.last().expect("commits");
        let subject = String::from_utf8_lossy(last.subject()).into_owned();
        let name = String::from_utf8_lossy(&last.author.name).into_owned();
        let email = String::from_utf8_lossy(&last.author.email).into_owned();
        let sha = last.id.to_string()[..10].to_uppercase();
        for raw in [
            subject.as_str(),
            name.as_str(),
            email.as_str(),
            sha.as_str(),
        ] {
            let (ended, batches) = scanned(&store, raw);
            assert_eq!(ended, Scanned::Finished, "{raw}");
            assert!(found(&batches, last.id), "{raw} finds the last commit");
        }
    }

    /// A hit in a message body, beyond the subject line: a commit with a body
    /// is added on top of a writable copy of the linear shape.
    #[test]
    fn a_scan_finds_a_commit_by_its_body() {
        let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
        graft_fixtures::git::run(
            scratch.path(),
            &[
                "-c",
                "user.name=Body Writer",
                "-c",
                "user.email=body@example.com",
                "commit",
                "--allow-empty",
                "-m",
                "a plain subject",
                "-m",
                "The body mentions a Quux-Frobnicator.",
            ],
        )
        .expect("commits");
        let store = RepoStore::new(scratch.path().to_path_buf(), None);
        let top = walked(&store)[0].id;

        let (_, batches) = scanned(&store, "quux-frobnicator");
        assert!(found(&batches, top), "the body line is searched");
        assert_eq!(batches.iter().map(|batch| batch.len()).sum::<usize>(), 1);
    }

    #[test]
    fn a_scan_with_no_hit_finishes_with_nothing() {
        let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
        let (ended, batches) = scanned(&store, "no commit says zq-xj-vk-7431");
        assert_eq!((ended, batches.len()), (Scanned::Finished, 0));
    }

    /// Cancelled after its first hit: the scan stops at the next chunk and
    /// hands over nothing more.
    #[test]
    fn a_cancelled_scan_stops_after_the_chunk_in_hand() {
        let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
        let email = String::from_utf8_lossy(&walked(&store)[0].author.email).into_owned();
        let query = Query::parse(&email).expect("a query");
        let cancelled = AtomicBool::new(false);
        let mut calls = 0;
        let ended = scan(
            &store,
            &query,
            1,
            &|| cancelled.load(Ordering::Relaxed),
            &mut |_| {
                calls += 1;
                cancelled.store(true, Ordering::Relaxed);
                true
            },
        )
        .expect("the scan reads");
        assert_eq!((ended, calls), (Scanned::Stopped, 1));
    }
    /// The palette's COMMITS are `/`'s first hits, same commits, same order,
    /// each with its subject.
    #[test]
    fn palette_commit_hits_match_slash_search() {
        let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
        let email = String::from_utf8_lossy(&walked(&store)[0].author.email).into_owned();
        let (_, batches) = scanned(&store, &email);
        let slash: Vec<Oid> = batches.iter().flat_map(|batch| batch.iter().copied()).collect();
        assert!(slash.len() > 9, "every commit matches its author");
        let query = Query::parse(&email).expect("a query");

        let hits = first_hits(&store, &query, 5, &|| false).expect("reads");

        assert_eq!(
            hits.iter().map(|hit| hit.id).collect::<Vec<_>>(),
            slash[..5].to_vec()
        );
        let commits = walked(&store);
        for hit in &hits {
            let commit = commits.iter().find(|c| c.id == hit.id).expect("walked");
            assert_eq!(&*hit.subject, commit.subject());
        }
    }

    /// Nine hits found → the walk ends there: it reads fewer chunks than
    /// a full walk (`stop` is asked once per chunk).
    #[test]
    fn commits_job_stops_after_nine_hits() {
        let store = RepoStore::new(graft_fixtures::borrow(Shape::Linear), None);
        let email = String::from_utf8_lossy(&walked(&store)[0].author.email).into_owned();
        let query = Query::parse(&email).expect("a query");
        let asked = std::cell::Cell::new(0_usize);
        let limit = crate::update::palette::COMMIT_HITS;

        let hits = first_hits(&store, &query, limit, &|| {
            asked.set(asked.get() + 1);
            false
        })
        .expect("reads");

        assert_eq!(hits.len(), 9);
        let full = std::cell::Cell::new(0_usize);
        let none = Query::parse("no commit says zq-xj-vk-7431").expect("a query");
        assert!(first_hits(&store, &none, limit, &|| {
            full.set(full.get() + 1);
            false
        })
        .expect("reads")
        .is_empty());
        assert!(
            asked.get() < full.get(),
            "stopped early: {} stop checks vs {} for a full walk",
            asked.get(),
            full.get()
        );
    }
}
