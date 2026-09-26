# branch-picker — `C` opens a picker of his branches and the remote's; type, `⏎` switches; `C ⏎` goes back

Status: ready — the switch rule (clean switch instant for `.` Checkout and `b ⇥` too) awaits the user's yes
Blocked by: 03-land-on-result
Crates: graft (`update/picker.rs` new, `GraphOverlay::Picker`, `text_field_open`, the switch rule for `.` Checkout and
`b ⇥`), graft-ui (`picker.rs` new), graft-ops (`switch_list`; `CheckoutTarget` for a remote-only branch if missing).
Source: note "Pass 2a" R5; Decisions 4–5; field notes :67, :73 ("C and o are Ok"); KNOWN_GAPS :772 (confirm decided
three ways), :854; `docs/research/checkout-cherry-pick.md` §1, §4, §7; `docs/shots/m7-proto-4-picker.txt`.
Build with: Opus. A new overlay on the `GraphOverlay` pattern; a read; one switch rule.
Scout: `scout` first (ADR-0035).
Needs: land-on-result (the landing after a switch). Doc rows: the conductor applies the keymap diff (§2.1 picker, §2.3
`C`, new §2.16) first. "Where I'd differ" 3 (the one switch rule for `.` Checkout / `b ⇥`) must be **approved**
before the build. If it isn't, the picker alone follows Decision 4, and `.`/`b ⇥` stay as they are.

## Decided (note R5)
- **`C` on the graph** opens `GraphOverlay::Picker`. `switch_list` runs as a job, and the rows fill when it lands.
  While reading: `reading branches…`.
- **Rows, order, filter, words, glyphs:** exactly note R5.
  - `↩ before` first when nothing is typed; then `● local` newest first; then `◌ remote` with no local copy.
  - `· here` last.
  - The right column: `tracks …`, `built on …`, or `no copy of yours yet`.
  - Segment-prefix matches rank above substring matches.
- **Keys:** a text field: every printable key is typed.
  - `Backspace`; `↑/↓` move.
  - `⏎` switches.
  - `Tab` closes and puts the graph cursor on that tip. It writes nothing; if the tip is outside the loaded history,
    it seeks as the Map's `Enter` does.
  - `Esc` closes.
  - Mouse: a click selects; a double-click switches.
- **The switch rule** (one rule):
  - **Instant:** a clean tree onto a local branch. A remote-only branch is instant too: `git switch <name>` creates the
    tracking branch, journaled as Checkout.
  - **Previewed, then `y`:** unsaved edits (the set-aside line, ADR-0045), or a detached target.
  - `⏎` on `· here`: `you're on <b>`, nothing written.
  - With approval of "Where I'd differ" 3: `.` → Checkout on a branch row and `b ⇥` follow the same rule.
- **Lands:** the graph, cursor on the new HEAD (`land-on-result`). The status line:
  - `✓ on <b> · u goes back`;
  - `✓ on <b> (new, tracks <remote>) · u goes back`;
  - with a set-aside: `✓ on <b> · your N edited files put back · u goes back`.

  A clash lands on the resolver (ADR-0045).
- **Box:** width = min(cols − 4, 150), over the lower 60% of the graph. ≥120 columns: kind, name, who, age, subject,
  relation. <120: kind glyph, name, age. At 24 rows: 10 rows, scrolling.
- **Empty filter result:** `no branch matches "<q>" · esc closes`.

## Read first
- Note "Pass 2a" R5 (the mock, rows, rule, words), "Journey counts" (switch-*), "Keymap: proposed diff" (§2.1, §2.3,
  §2.16), "Where I'd differ" 3.
- `docs/design/02-keymap.md` §2.1 (text fields; the `.` rules), §2.3, §2.3b.
- `docs/adr/0045-…` §4 (set-aside for Checkout).
- `crates/graft/MAP.md`: `GraphOverlay`, `text_field_open`, `update::context_menu` (Checkout slot 1), `update::name_prompt`
  (`b ⇥`), `update::map` (seek on `Enter`), `update::operate`. `crates/graft-ops/MAP.md`: `CheckoutParams`,
  `CheckoutTarget`, `history::previous_tips`, refs reads.

## Scout brief (≤ 1 page)
1. `CheckoutTarget`'s variants; how `.` Checkout and `b ⇥` launch (previewed or not); the line where each decides.
2. Is `@{-1}` read anywhere? Is there a refs read listing local + remote branches with tip, subject, author and date?
3. How the Map's `Enter` seeks the graph to a tip outside the loaded window.
4. What `C` does on the graph today (grep the key route).
5. Every walk/capture of `.` Checkout and `b ⇥` (they change if "Where I'd differ" 3 is approved).

## Signatures
```rust
// graft-ops
pub enum SwitchKind { Local, RemoteOnly }
pub struct SwitchRow { pub name: RefName, pub kind: SwitchKind, pub tip: Oid, pub subject: String,
                       pub author: String, pub when: i64, pub tracks: Option<RefName>, pub current: bool }
pub struct SwitchList { pub previous: Option<SwitchRow>, pub rows: Vec<SwitchRow> }
pub fn switch_list(repo: &Repo, git: &Git, stop: &dyn Stop) -> Result<SwitchList, FactsError>;
// graft
pub fn open_picker(state: &mut AppState) -> Vec<Command>;
pub fn picker_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
// graft-ui
pub fn draw_picker(frame: &mut Frame, area: Rect, view: &PickerView);
```

## Acceptance
- [ ] **graft-ops:**
  - `switch_list_puts_the_previous_branch_first`;
  - `a_remote_with_a_local_copy_shows_once`;
  - `remote_only_branches_are_listed`;
  - `switch_list_honours_stop`.
- [ ] **App:**
  - `c_opens_the_picker` (red first);
  - `typing_filters_segment_prefix_first`;
  - `enter_on_a_clean_tree_switches_at_once`;
  - `enter_with_edits_opens_the_set_aside_preview`;
  - `enter_on_here_writes_nothing`;
  - `tab_puts_the_cursor_on_the_tip_and_writes_nothing`;
  - `esc_closes`;
  - `the_picker_owns_every_letter` (`j`, `k`, `q` are typed);
  - `a_remote_only_branch_creates_its_tracking_branch`.
- [ ] **Walks** (`m7_walk`):
  - `switch_named_branch` = `C login ⏎`;
  - `switch_back` = `C ⏎`;
  - `switch_with_edits` = `C login ⏎ y`.

  Each ≤ its `noted_keys`. After each, HEAD is on the target and the cursor on its row; after `switch_with_edits` the
  edits are back. Then `u` goes back.
- [ ] **Captures** (colour unverified):
  - `docs/shots/m7/picker-empty-query-187x50.txt`. **Right:** `↩ before  feature-a` first; `● local` / `◌ remote` /
    `· here` columns; the bar `type to find  ⏎ switch to it  tab show it on the graph  ↑↓ row  esc close`.
  - `docs/shots/m7/picker-filtered-100x30.txt`. **Right:** the narrow columns (glyph, name, age).
  - `docs/shots/m7/switch-landed-120x40.txt`. **Right:** `▸` on the new HEAD; `✓ on login-page · u goes back`.
- [ ] Hint-bar tests green (`C` on the graph bar's `more` list at least, per the §2.1 ranking). MAP.md for the three
      crates. TESTING.md section "Switching" + captions + contact sheet.

## Touches (check each, report how)
- **`.` Checkout and `b ⇥`:** only with approval (above). If changed: their walks and captures re-taken and listed. If
  not: unchanged, checked byte-identical.
- **Set-aside for Checkout (ADR-0045):** reused. Check: the switch-dirty journey tests green.
- **Other overlays:** a new variant. Check: every match site; the overlay tests.
- **Graph key `C`:** scout Q4; the keymap diff gives it.
- **The sidebar's `Enter` checkout (§2.3):** unchanged. Check: its tests.
- **Journal/undo:** Checkout entries as today. Check: `u` in the walks.

## Rules / Gates
As rewrite-engine.

## Size
Large (~70k). Split if needed:
- **-a:** `switch_list` + the overlay + keys + the switch rule;
- **-b:** the ui draw, walks, captures.
