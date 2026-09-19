# Naming research: the production name

**Checked 2026-09-19.** Registries move. Re-run every check the week the name is
announced. Everything below is either a check I ran (source given) or marked
**unverified** — no guesses are presented as checks.

**Recommendation: Tributary, command `trib`.** Jump to
[Recommendation](#recommendation) for the reason.

---

## 1. The landscape

### What the incumbents did

| Name | Pattern | Command | Read |
|---|---|---|---|
| **Sublime Merge** | brand extension + generic noun | `smerge` | Only works if you already own the brand. "Merge" alone is unownable. Command ≠ name, and nobody minds. |
| **Fork** | plain Git noun | `fork` | Confident, 4 letters, zero searchability. You cannot google it. Works because the product is GUI-first and found by word of mouth. |
| **Tower** | abstract noun, no Git meaning | — | The "strong generic word" play. Landed in 2010. Not repeatable: every 4–6 letter English noun is now squatted on crates.io, npm and PyPI. |
| **GitKraken** | `git` + noun | — | Pre-dates enforcement of the Git trademark policy. Copying it today is out (see §2). |
| **Sourcetree** | compound descriptive | — | Dated on arrival. Describes the category, not the product. |
| **lazygit / gitui / tig / gitk** | `git` + qualifier, or a backronym | `lazygit`, `gitui`, `tig` | The terminal convention. Exhausted *and* blocked. `tig` survives as a backronym ("text interface for git"); `git-delta` quietly dropped `git` and ships as `delta`. |
| **jj (Jujutsu)** | plain foreign noun, 2-char command | `jj` | The modern pattern: long evocative name, brutally short command. |
| **Sapling / Pijul / Fossil / Darcs** | plain nouns from outside computing | `sl`, `pijul`, `fossil`, `darcs` | Same pattern. No `git`, no description, concrete over abstract. |

### What the good ones share

- One or two syllables **spoken**, even when longer written.
- A **concrete noun**, not an abstraction. Fork, Tower, Sapling, Fossil.
- **No `git`.**
- The command is often a *different, shorter string* than the product name
  (Sublime Merge → `smerge`, Jujutsu → `jj`, Sapling → `sl`). This is normal and
  worth exploiting: the name can be evocative while the command stays 3–4 chars.

### Patterns that are exhausted

1. **`git`-compounds.** Forbidden (§2), and the good ones are gone anyway.
2. **`*-flow`, `*-kit`, `*-hub`, `*-ctl` suffixes.** Reads as 2016.
3. **Single common 4–5 letter English nouns.** I screened 130+. Almost every one
   is taken on crates.io, npm and PyPI — usually by a dormant 2015 package that
   will never be released to you.
4. **New in 2026: trail/marker/navigation words.** Cairn, Beacon, Compass,
   Waypoint, Lodestar have been consumed wholesale by the AI-agent tooling wave.
   Cairn alone has eight distinct dev tools (§5). Avoid this whole lexical field.

The live space is: **short concrete nouns from craft, water or rigging**, where
the metaphor does real work and the word is rare enough that crates.io is free.

---

## 2. The `git` constraint, confirmed

From [git-scm.com/about/trademark](https://git-scm.com/about/trademark), §2.3:

> "You may not use any of the Marks as a syllable in a new word or as part of a
> portmanteau (e.g., "Gitalicious", "Gitpedia") used as a mark for a third-party
> product or service."

§2.2 permits only the form **"[Product Name] for Git"**.

So `tergit`, `gitfoo`, `foogit`, `gitgraph`, `nugit` — all out, no exceptions.
This removes most of the obvious space, including every name that would have
carried its own category signal. The consequence: **the name cannot say what the
product is.** It has to earn meaning, like Fork and Tower did. Plan the tagline
to do the work the name can't ("a terminal Git client you cannot lose work in").

---

## 3. Finalists

Four. Each was checked against nine sources. A fifth, Cairn, was the strongest
metaphor in the whole search and is dead — see §5, it is worth reading so nobody
proposes it again.

### 3.1 Tributary — command `trib`

**Why it fits.** A tributary is a branch that leaves the main channel and flows
back into it. That is the picture the product is built around — PROMPT.md: "The
graph is the product." It is the only candidate whose metaphor survives being
*drawn*: the commit graph literally is a river system. It also carries the team
workflow honestly — many small streams, one mainline.

**Said aloud.** TRIB-yoo-tree. Four syllables written, but the thing you type
and say daily is `trib` — one syllable, unambiguous, no homophone. A non-native
speaker hearing "trib" spells it right first time. Hearing "tributary" they may
write *tributory*; the command protects against that.

**Against it.** It does not say *safe*. The #1 promise — you cannot lose work —
gets nothing from this name; it speaks to promise #3. Nine letters. "Tributary"
as a bare word is registered on all three registries (all dormant), so the
product word is not clean even though the command is.

| Check | Result | Source |
|---|---|---|
| crates.io `trib` | **FREE** | `api.github.com`-equivalent: `crates.io/api/v1/crates/trib` → `crate 'trib' does not exist` |
| crates.io `tributary` | Taken. v0.1.0, 1,556 downloads, "Data streams", `timkpaine/tributary-rs`. Dormant. | crates.io API |
| npm `trib` | **FREE** (404) | registry.npmjs.org |
| npm `tributary` | Taken. v1.1.0, published 2015-02-08, "Insert one stream into another". Dead. | registry.npmjs.org |
| PyPI `trib` | **FREE** (404) | pypi.org/pypi/trib/json |
| PyPI `tributary` | Taken. v0.2.1, "Streaming reactive and dataflow graphs in Python", `streamlet-dev`. Semi-active. | pypi.org |
| GitHub org/user | `tributary` taken (user, 2 repos). `trib` taken (user, 0 repos). **Neither org is obtainable.** | api.github.com/users/… |
| GitHub prominent repos | `enjalot/tributary` 593★ (d3 prototyping), `1kbgz/tributary` 466★ (Python dataflow), `jm2/tributary` 143★ (media manager), `Query-farm/tributary` 57★ (DuckDB/Kafka). **None in VCS, Git or terminal.** | GitHub search API, sorted by stars |
| Homebrew | No formula, no cask, for `tributary` or `trib` | formulae.brew.sh API → 404/404 |
| Unix binary | Not on macOS 27 PATH incl. Homebrew. No Debian stable package ships `bin/trib` or `bin/tributary`. | `command -v`; packages.debian.org contents search |
| Domains | `tributary.dev` registered, serving (Cloudflare NS, A 62.238.24.135). `tributary.io` **parked for sale** (Atom.com nameservers). `tributary.com` registered (Google Domains NS), **no A record — not in use**. `trib.dev` registered (Google NS), no A record. `trib.io` and `trib.com` both resolve. | `dig` A/NS |
| Trademark | **UNVERIFIED** — see §6 | — |
| VCS/software meaning | None. "Tributary" carries no loaded meaning in version control. | — |

### 3.2 Belay — command `belay`

**Why it fits.** The best metaphor found, by a distance. In climbing, to belay is
to hold the rope so a fall is arrested — *the whole point is that the fall does
not cost you anything.* And there is a second, exact meaning: at sea, **"belay
that" means cancel the order.** A tool whose headline key is Ctrl+Z and whose
first promise is "you cannot lose work" is named twice over by one word.

**Said aloud.** be-LAY. Two common English syllables. A non-native hearing it
might write *belai* or *belae* — moderate spelling risk, but both syllables are
high-frequency so it usually lands.

**Against it — and this is fatal.** crates.io `belay` is taken by a **Rust CLI in
the Git workflow space**: "Run your CI checks locally to git push with
confidence" (`JoshMcguigan/belay`, v0.5.0, 11,936 downloads). Same language,
same ecosystem, adjacent problem. `cargo install belay` already installs
somebody else's Git tool. Separately, the commercial software space is crowded:
Belay Software, Belay Technologies (DoD/commercial IT), a Belay quality-management
platform for audit firms, and BELAY Solutions — plus belay.io is a live software-
industry publication. This is exactly the collision the brief says to avoid.

| Check | Result | Source |
|---|---|---|
| crates.io | **TAKEN — disqualifying.** v0.5.0, 11,936 dl, "Run your CI checks locally to git push with confidence", `github.com/JoshMcguigan/belay` | crates.io API |
| npm | Taken. v1.5.0 (2018-09-27), "Belay react and sass component library". Dead. | registry.npmjs.org |
| PyPI | Taken. v0.30.1, `BrianPugh/belay` (MicroPython tooling). Active. | pypi.org |
| GitHub org/user | `belay` taken (user, 0 repos) | api.github.com |
| GitHub prominent repos | `Annex-Engineering/Belay` 300★ (3D-printer sensor), `BrianPugh/belay` 273★, `belay-labs/gpt-explorer` 207★. None in VCS. | GitHub search API |
| Homebrew | Free (404/404) | formulae.brew.sh |
| Unix binary | None on PATH; none in Debian stable | `command -v`; packages.debian.org |
| Domains | `belay.dev` registered (Cloudflare NS), no A record. `belay.io` **live** — "ground-up tactics for building software companies" (WordPress). `belay.com` **live** (Squarespace/NS1). | `dig`, HTTP title fetch |
| Trademark | **UNVERIFIED at USPTO.** But live commercial software brands confirmed by search: Belay Software (St Paul MN), Belay Technologies (SDVOSB, DoD + commercial IT), Belay QMS for audit firms, Belay rideshare-income app. Assume high risk in classes 9 and 42. | Crunchbase/PitchBook/ZoomInfo profiles via search |
| VCS/software meaning | None in VCS, but heavily used as a software *brand*. | — |

### 3.3 Batten — command `batten`

**Why it fits.** "Batten down the hatches" — make everything fast and secured
before the storm. That is the journal-plus-backup promise: nothing is loose,
nothing can be washed overboard. A batten is also the strip that holds a
tarpaulin down so the contents survive. It is the only finalist whose metaphor
points at promise #1.

**Said aloud.** BAT-'n. Two syllables, both trivial. A non-native hearing it once
may write *baton* or *batton* — real but mild risk.

**Against it.** The idiom is about *prevention*, not *recovery*; it doesn't say
"undo", it says "brace". The bare word's dominant search result is **Batten
disease**, a rare neurodegenerative disorder — bad neighbours in any search, and
a slightly grim association if someone looks it up. `dockersecuritytools/batten`
(69★, Docker hardening, dormant) is a dev tool of the same exact name. And the
command is 6 chars where `trib` is 4.

| Check | Result | Source |
|---|---|---|
| crates.io | **FREE** | crates.io API → `does not exist` |
| npm | Record exists but has **no published version** (empty dist-tags) — an unpublished/removed stub. Name is blocked but nothing ships under it. | registry.npmjs.org |
| PyPI | **FREE** (404) | pypi.org |
| GitHub org/user | `batten` taken (user, 0 repos) | api.github.com |
| GitHub prominent repos | `dockersecuritytools/batten` 69★ (Docker hardening, dormant). `Wedge-lab/battenberg` 101★ is a different word (genomics). **Nothing in VCS, Git or terminal.** | GitHub search API |
| Homebrew | Free (404/404) | formulae.brew.sh |
| Unix binary | None on PATH; none in Debian stable | `command -v`; packages.debian.org |
| Domains | `batten.dev` registered (Cloudflare NS) | `dig` |
| Trademark | **UNVERIFIED** | — |
| VCS/software meaning | None. But "Batten" alone is dominated by Batten disease in general search. | — |

### 3.4 Piton — command `piton`

**Why it fits.** A piton is the anchor a climber hammers into rock and clips
into, so that a fall stops at the last anchor placed. That is the undo journal
described exactly: every write drives another anchor; you never fall further than
the last one. Five letters, crates.io free.

**Said aloud.** PEE-ton / PIT-on. Phonetically transparent, easy to spell.

**Against it — and it's serious.** "Pitón" is Spanish for *python*, "питон" is
Russian for the same. Three of the five most-starred GitHub repos named `piton`
are Python tooling (`mendrugory/piton` 44★, "Run your Python algorithms in
parallel"; `fpantigny/piton`, a LaTeX package for typesetting Python). A Rust
terminal Git client called Piton will be quietly misfiled as a Python thing
forever. Princeton's **OpenPiton** manycore processor is also a real, cited
project in the same broad "systems" world.

| Check | Result | Source |
|---|---|---|
| crates.io | **FREE** | crates.io API → `does not exist` |
| npm | Taken. v2.1.0 (2018-07), dormant. | registry.npmjs.org |
| PyPI | Taken. v0.3.1, "A local python package manager". | pypi.org |
| GitHub org/user | `piton` taken (user, 0 repos) | api.github.com |
| GitHub prominent repos | `mendrugory/piton` 44★ (Python parallelism), `PrincetonUniversity/piton-linux` 36★ + `piton-sw` 19★ (OpenPiton), `fpantigny/piton` 17★ (LuaLaTeX Python typesetting), `Ironholds/piton` 16★ (PEG for Rcpp). Nothing in VCS. | GitHub search API |
| Homebrew | Free (404/404) | formulae.brew.sh |
| Unix binary | None on PATH; none in Debian stable | `command -v`; packages.debian.org |
| Domains | `piton.dev` registered (Namecheap NS) | `dig` |
| Trademark | **UNVERIFIED** | — |
| VCS/software meaning | None in VCS, but strongly loaded as "Python" in three languages. | — |

---

## 4. Recommendation

**Tributary. Command, crate and binary: `trib`.**

Four reasons, in order of weight:

1. **It is the only name whose command clears every registry.** `trib` is free on
   crates.io, npm, PyPI and Homebrew, ships as no binary on macOS or Debian
   stable, and collides with no Unix command. Nothing else in a 130-name screen
   cleared that bar. `cargo install trib`, `brew install trib` and `trib` typed
   forty times a day are all uncontested, today.
2. **Zero collision in the Git, VCS, build or terminal space.** The crowded uses
   of "tributary" — d3 prototyping, Python dataflow, a media manager, a DuckDB
   extension, capital management — are all in unrelated niches, and every one of
   them is dormant or small. Belay fails here (a Rust Git-workflow CLI holds the
   crate). Cairn fails here (an experimental Rust VCS holds the crate, plus eight
   live dev tools). Thicket fails here (an existing `git log` tool).
3. **The metaphor is the centrepiece feature.** PROMPT.md: "The graph is the
   product." A tributary is a branch that leaves the mainline and flows back into
   it. The name describes the picture on screen, which is the one thing about this
   product a new user sees in the first second.
4. **The 4-character command is the real ergonomics win**, and it follows the live
   convention (Jujutsu→`jj`, Sublime Merge→`smerge`, Sapling→`sl`). The evocative
   name and the fast command do not have to be the same string.

**What you are giving up by choosing it:** the name does not say *safe*. The
undo/can't-lose-work promise — identity #1 — gets nothing from "Tributary" and
must be carried entirely by the tagline and the first-run tour. If that promise
matters more than the graph, take **Batten** instead: it is the only finalist
that names promise #1, its slate is nearly as clean (crates.io and PyPI both
free), and its costs are a grimmer bare-word search and two extra characters.

**Do not take Belay**, however good it sounds. A Rust CLI for Git workflows
already owns the crate name with 12k downloads, and four live software companies
own the word commercially. That is precisely the failure mode the availability
check exists to catch.

Runner-up order: **Batten**, then **Piton**, then Belay (disqualified on
availability, listed only because the metaphor is the best one available and
someone will re-propose it).

---

## 5. Rejected, and why

Do not re-propose these.

### Prior work, re-checked

| Name | 2026-09-11 verdict | 2026-09-19 verdict |
|---|---|---|
| **Tributary** | passed clash check | **Holds up — promoted to recommendation.** The old check missed that the *crate* `tributary` is taken; the command `trib` is free, which is what matters. |
| **Thicket** | passed clash check | **Rejected.** The old check was wrong. `taylorthurlow/thicket` is "a better one-line git log" — a direct collision in the Git space. `thicket.dev` serves "OCaml Thicket", a live dev tool. crates.io `thicket` taken (splay trees, 898 dl). GitHub user `thicket` taken, 18 repos. Separately, the word names *the problem* (a tangle) rather than the promise; this product's job is to untangle. |

### Near-misses worth recording

| Name | Killed by |
|---|---|
| **Cairn** | Strongest metaphor in the search — stacked stones marking the way back, which is exactly the journal and the time machine. Dead on crowding: **eight distinct dev tools** named Cairn as of 2026 (`cairn-dev/cairn` 215★ background-agent system, `cairn-framework` on crates.io with "change tracking", `cairn-tool/cairn` CLI, `getcairns.dev`, plus AI-agent repos at 2,840★ and 2,135★). GitHub org `cairn` is taken by "Cairn Software". crates.io `cairn` is taken by an **experimental Rust version-control system** — a direct VCS-space hit. |
| **Cleft** | Survived availability (crates.io FREE, Homebrew free, no binary anywhere) and failed on meaning. "Cleft" means *split*, not *safe*; the dominant association is cleft palate; GitHub org `Cleft` is already squatted. Available ≠ right. |
| **Palimpsest** | The single best metaphor for an undo journal — a manuscript erased and rewritten where the old text stays recoverable. Ten letters, unspellable from hearing, and crates.io `palimpsest` is an annotation tool for **Pijul**, a VCS. |
| **Holdfast** | "Hold fast" as both a nautical order and the organ by which seaweed grips rock. crates.io holds a name-reservation stub (an MCP server). Eight letters — too long to type daily. |
| **Truss** | Good "holds the structure up" metaphor. **`truss` is a real syscall-tracing command** on Solaris, FreeBSD and AIX. Shadows a binary — automatic disqualification. |
| **Cleat** | crates.io near-free (28 downloads). Killed by PyPI: `cleat` is "a headless terminal layer: a persistent PTY" — a collision in the terminal space, the one place this product lives. |
| **Heartwood** | Radicle's peer-to-peer Git implementation. Straight into the VCS space. |
| **Rappel** | crates.io FREE, and "rappel" is French for *recall* — a lovely second meaning for an undo tool. But a rappel is a *descent*, which reads as downgrade/rollback-in-the-bad-sense, and PyPI `rappel` is active. |
| **Annals** | Free on crates.io, npm **and** PyPI — the only fully-free word found. Unusable when said aloud. |
| **Bat** | As a short command form of anything: `bat` is the widely installed `cat` clone. |
| **Bollard, Sluice, Skein, Quire** | crates.io taken with 51M, 16M, 373k and 87k downloads respectively. |
| **Oxbow, Weft, Twill, Strata, Trove, Grove, Arbor, Braid, Copse, Vestige, Bramble, Riffle, Runnel** | crates.io taken by live or semi-live crates. Several are in adjacent dev-tool space (`braid` = repo-local issue tracker; `vestige` = repo-pinned memory for coding agents). |
| **Riprap, Gunwale, Rabbet, Thwart, Sheave, Bight, Quire** | Free or near-free, all fail "spellable by a non-native hearing it once", or read as negative ("rip", "thwart"). |
| **Trunk, Warp, Shuttle, Tide, Anchor, Vault, Harbor, Ledger, Delta, Compass, Atlas, Facet, Sapling, Halyard, Keystone, Semaphore, Waypoint, Tripwire, Grommet, Fender** | All live developer-tool, infrastructure or major consumer brands. Several (`delta`, `sapling`, `trunk`) are in the Git space itself. |
| **Anything containing `git`** | Git trademark policy §2.3, quoted in §2 above. |

---

## 6. What a human must do before the name can be used

Ordered. Items 1 and 2 block announcement; 3 blocks by weeks.

1. **Trademark clearance — not done, and it must be.** Every public trademark
   source refused automated access: `tmsearch.uspto.gov` (API returns
   MethodNotAllowed), `trademarks.justia.com` (403, Cloudflare), `uspto.report`
   (403), `trademarkelite.com` (403). No browser was available in this session.
   **Treat every "Trademark" row in §3 as unverified.** A person must run the
   USPTO search at <https://tmsearch.uspto.gov/> for the chosen name in
   **class 9** (software) and **class 42** (SaaS / IT services), and for a name
   going on a public release, a trademark attorney should do it. This is not
   legal advice and nothing above is a clearance opinion.
2. **GitHub org.** Every finalist's org/username is already taken, including
   `tributary` (user, 2 repos) and `trib` (user, 0 repos). Pick and register a
   variant before the first public commit — `tributary-dev`, `trib-vcs`,
   `tribtool` — or file a GitHub name-release request against the inactive
   account and expect it to fail. Do not plan on getting `github.com/trib`.
3. **crates.io placeholder.** `trib` is free **today**. crates.io does not reserve
   names and does not transfer squatted ones. The day the name is agreed, publish
   a `0.0.0` placeholder crate under it. Same for the Homebrew formula name at M8.
4. **Domain purchase.** `trib.dev` and `tributary.dev` are both registered;
   `tributary.io` is parked for sale through Atom.com (price unverified);
   `tributary.com` is registered but serving nothing. Budget for a variant
   (`trib.tools`, `usetrib.dev`) rather than assuming a clean buy, and price the
   Atom.com listing before committing to the `.io`.
5. **Update `PROGRESS.md` line 93 and `PROMPT.md`** once the name is chosen —
   PROGRESS.md still records Tributary and Thicket as co-equal, and this file
   rejects Thicket. That edit is out of scope here; it belongs to the M8 naming
   ticket.
6. **Re-run §3's checks the week of announcement.** These are dated 2026-09-19.
   A dormant crate can be yanked and re-taken; a parked domain can be sold.

### Method, so the checks can be audited

crates.io via `crates.io/api/v1/crates/<name>`; npm via `registry.npmjs.org/<name>`;
PyPI via `pypi.org/pypi/<name>/json`; Homebrew via
`formulae.brew.sh/api/formula/<name>.json` and `/api/cask/<name>.json`; GitHub via
`api.github.com/users/<name>` and the repository search API sorted by stars;
domains via `dig` A and NS records plus an HTTP title fetch to tell parked from
in-use; binary collisions via `command -v` on macOS 27 with Homebrew on PATH and
an exact-filename contents search on packages.debian.org (stable, any arch).
130+ names were screened on the three package registries before these four.
