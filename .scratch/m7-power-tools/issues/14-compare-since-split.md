# compare-since-split — `x` between two commits shows what one has since it left the other, in words

Status: ready
Blocked by: none
Crates: graft (the `x` compare state, the diff range), graft-ops (`compare_span`), graft-ui (the detail header words, the
`x base` badge).
Source: note Decision 2; "Pass 2a" R6 (Compare); journey 16 (`pr-contents`); `docs/research/branch-compare-submodules.md`
takeaways 1, 4, 5, 6.
Build with: Sonnet. A diff-range change and words on an existing compare. The scout decides whether today's `x` already
diffs live as the cursor moves.
Scout: `scout` first (ADR-0035).
Needs: none.

## Decided
- **Diff range:** with a compare base B marked by `x` and the cursor on C, the detail pane diffs `merge-base(B, C)..C`
  (three-dot). The note says "since the split", never dots.
- **Header words** (detail pane, top):
  - `changes on <C's branch or sha7> since it left <B's branch or sha7> · N commits · F files +a −d`, with N =
    `compare_span.ahead`.
  - C is an ancestor of B: `<C> is already in <B> · nothing of its own`.
  - C == B: `this is the compare base · x clears it`.
  - No common history: `<C> and <B> share no history · showing <B>..<C>`, which is two-dot, said in words.
- **Live:** once B is marked, the diff follows the cursor. No second `x` is needed (journey 16 = `x`, motion). If the
  scout finds today's `x` needs a second press to complete, this ticket makes it live, and a second `x` on another row
  keeps today's meaning (complete/clear). The journey's count assumes live.
- **Badge:** the base row shows `x base` in the badge column. Distinction: the word + position; colour unverified.
- **No new screen.** Two-dot is not offered as a toggle in M7.

## Read first
- Note Decision 2, "Pass 2a" R6, "Journey counts" `pr-contents`.
- `docs/design/02-keymap.md` §2.3 `x`.
- `crates/graft/MAP.md`: `compare_base`, the detail pane's diff source. `crates/graft-ops/MAP.md` `history_reads`
  (`merge_base`, `commits_between`). `crates/graft-ui/MAP.md` `detail_pane::header`, badges.

## Scout brief (≤ 1 page)
1. What `x` does today, step by step: mark, what moving the cursor does, what a second `x` does. Is the diff two-dot or
   three-dot? Give the lines.
2. Where the detail header's words for a compare are built.
3. Captures and walks that show a compare (`grep -rn compare` in the tests and `docs/shots`).

## Signatures
```rust
pub struct CompareSpan { pub split: Option<Oid>, pub ahead: usize, pub behind: usize }
pub fn compare_span(repo: &Repo, git: &Git, base: Oid, tip: Oid, stop: &dyn Stop) -> Result<CompareSpan, FactsError>;
```

## Acceptance
- [ ] **graft-ops:**
  - `compare_span_counts_commits_since_the_split`;
  - `an_ancestor_has_nothing_ahead`;
  - `unrelated_histories_have_no_split`;
  - `compare_span_honours_stop`.
- [ ] **App:**
  - `the_compare_diff_is_since_the_split` (red first if today is two-dot);
  - `the_diff_follows_the_cursor_after_x`;
  - `the_header_says_changes_since_it_left`;
  - `an_ancestor_says_nothing_of_its_own`;
  - `on_the_base_says_x_clears`.
- [ ] **Walk** `pr_contents` = `x` motion ≤ `noted_keys("pr-contents")`, starting with the cursor on main. The header
      reads `changes on feature-a since it left main · 3 commits`.
- [ ] **Capture** `docs/shots/m7/compare-since-split-187x50.txt`. **Right:** the header words above; `x base` on main's
      row; the diff lists only feature-a's files. Colour unverified.
- [ ] Hint-bar tests green. MAP.md for the three crates. TESTING.md row.

## Touches (check each, report how)
- **Existing compare walks/captures:** scout Q3. Each is re-taken and listed.
- **The diff pane (`w W v [ ] < > L`):** unchanged. Check: the diff-pane tests.
- **Keymap:** §2.3 `x` wording may need "diffs since the split". The conductor's row, named in NOTES.
- **Journal:** none (read only).

## Rules / Gates
As rewrite-engine.

## Size
Small-medium (~35k).
