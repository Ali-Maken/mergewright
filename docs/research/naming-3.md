# Naming, round 3 (2026-10-11): led by the problem

"Graft" is a codename (CLAUDE.md). All checks below are read-only lookups; nothing registered.

## Brief (user, 2026-10-11)

"think about what problem we are solving, gear towards that, no matter the language." Command 2–4 letters. Not re-proposed: everything rejected in `docs/naming-research.md:250-279` and `docs/research/naming-2.md:27-63`, plus Tributary, Fern, Banyan, Prong, Alder, Rowan, Osier. No `git`, no trail/marker words.

## The problem, grounded

- History's *shape* is invisible in the terminal: lazygit/tig/gitui "draw history as a thin line of ASCII characters, or leave it out" (`PROMPT.md:23`). "The graph is the product" (`PROMPT.md:25`).
- Fear of losing work: "Never lose work. Every destructive operation can be undone" (`PROMPT.md:34`); M5 promise "you see the result before you confirm" (CLAUDE.md, acceptance promises).
- Leaving the terminal: "Times I left" is "the only measurement of the promise" (`TESTING.md:127-131`).
- Chains of dependent branches (memory: works-in-a-team-on-dependent-branches) — not read in field notes this round; UNVERIFIED as a field-notes quote.

Themes mined: **seeing / sight**, **mirror / clear view**, **light that reveals**, **guide**, **manifest / made visible**, **safety**.

## Method

Same as `docs/research/naming-2.md:14-24`, run today by script: crates.io API, npm registry, PyPI JSON, formulae.brew.sh formula + cask (200 = taken, 404 = free); `command -v` on this Mac; GitHub repository search `q=<name>+git` sorted by stars (top 4 read); one web search per group for a Git/VCS tool.

**Debian contents: UNVERIFIED for every command.** The sanity check against `tig` (which must hit `/usr/bin/tig`) returned no hit, so the parser/site response was not trustworthy today; "no hit" is not reported as free.
**GitHub search hit the anonymous rate limit** after 10 names; Darshan and Zahir GitHub searches not done (UNVERIFIED). Trademark, domains, GitHub org: not checked (UNVERIFIED).
"Unfortunate meaning" checks below are from my own language knowledge, not a looked-up source — treat as UNVERIFIED until a native speaker confirms.

## Candidates

| # | Name | Cmd | Language, literal meaning | Problem it speaks to | Said aloud |
|---|---|---|---|---|---|
| 1 | **Ayna** | `ayn` | Persian/Urdu/Turkish *āyina/ayna*, "mirror" | Shows the repo back to you as it is — the shape made visible | "EYE-na" |
| 2 | **Nazar** | `nzr` | Arabic/Persian/Urdu/Turkish, "sight, gaze"; in Turkish also the blue evil-eye charm that *protects* | Seeing + protection (never lose work) | "na-ZAR" |
| 3 | **Roshan** | `rsn` | Persian/Urdu, "bright, lit, clear" | Light on what was dark: history you couldn't see | "ROH-shan" |
| 4 | **Rahbar** | `rah` | Persian/Urdu, "guide, one who shows the way" (*rah* = path) | Guides you through rebases/conflicts without leaving | "RAH-bar"; cmd "rah" |
| 5 | **Zahir** | `zhr` | Arabic *ẓāhir*, "manifest, apparent, visible" | History made manifest | "ZAH-heer" |
| 6 | **Amaan** | `amn` | Arabic/Urdu *amān*, "safety, protection, refuge" | Fear of Git: you cannot lose work | "a-MAAN" |
| 7 | **Drishti** | `dri` | Sanskrit *dṛṣṭi*, "sight, vision, focused gaze" | Seeing the shape | "DRISH-tee" — hard for many |
| 8 | **Miru** | `miru` | Japanese 見る, "to see / to look" | Seeing; name = command | "MEE-roo" |
| 9 | **Delos** | `dls` | Greek *dēlos*, "clear, manifest" (also the island) | Clear view | "DEE-los" |
| 10 | **Opsis** | `ops` | Greek, "sight, view" | Seeing | "OP-sis" — cmd reads as "ops" |

## Checks run today

| Name / cmd | crates.io | npm | PyPI | Brew F / C | `command -v` | GitHub `<x> git` | Git/VCS tool by this name? |
|---|---|---|---|---|---|---|---|
| ayna | **free** | **free** | **free** | free / free | none | 44 results, top 4 personal pages | None found ([web search](https://alternativeto.net/category/developer-tools/git-client/?p=3) returned no "ayna" Git tool) |
| `ayn` | **free** | taken | taken | free / free | none | — | — |
| nazar | taken | taken | taken | free / free | none | 534, top 4 personal pages | None found (web search: no Git client called nazar; [terminaltrove git clients](https://terminaltrove.com/categories/git-clients/)) |
| `nzr` | **free** | **free** | **free** | free / free | none | — | — |
| roshan | **free** | taken | **free** | free / free | none | 635, top 4 portfolios | None found (same web search as Ayna) |
| `rsn` | taken | taken | **free** | free / free | none | — | — |
| rahbar | **free** | **free** | **free** | free / free | none | 16, none tools | not web-searched — UNVERIFIED |
| `rah` | **free** | taken | **free** | free / free | none | — | — |
| zahir | **free** | **free** | **free** | free / free | none | rate-limited — UNVERIFIED | UNVERIFIED |
| `zhr` | **free** | taken | **free** | free / free | none | — | — |
| amaan | **free** | **free** | **free** | free / free | none | 184, one repo "Git handoff snapshot" (1★, not a tool name clash) | not web-searched — UNVERIFIED |
| `amn` | **free** | taken | taken | free / free | none | — | — |
| drishti | taken | taken | taken | free / free | none | 68, none tools | No Git tool; name used by a [volume-visualisation app](https://en.wikipedia.org/wiki/Drishti_(client)) and [drishti-io](https://pypi.org/project/drishti-io/0.4/) (HPC I/O analyser) |
| `dri` | crates.io no response (curl 000 twice) — UNVERIFIED | taken | free | free / free | none | — | — |
| miru | taken | taken | taken | free / free | none | 150; `thaunknown/miru` (Flathub, EOL media app); `debarchito/miru` (a Lisp) | No Git tool (web search: none) |
| delos | taken | taken | taken | free / free | none | 52, none tools | not web-searched — UNVERIFIED |
| `dls` | taken | taken | free | free / free | none | — | — |
| opsis | taken | free | taken | free / free | none | 12, none tools | not web-searched — UNVERIFIED |
| `ops` | taken | taken | taken | free / free | none | — | generic word; many `ops` scripts likely — avoid |
| darshan (screened) | taken | taken | taken | — | — | rate-limited | `darshan` is the HPC I/O profiler Drishti reads (per [drishti-io](https://pypi.org/project/drishti-io/0.4/)); `dar` is a Homebrew formula (taken) — **dropped** |
| hodoku (screened) | taken | taken | taken | — | — | 0 | Japanese 解く "untie"; HoDoKu is a known Sudoku app (from memory — UNVERIFIED); `hdk` taken everywhere — **dropped** |

### Meaning risks (UNVERIFIED — own knowledge, needs a native speaker)

- **Nazar**: in Turkish/Urdu "nazar lagna" = to be struck by the evil eye. The word itself is neutral-to-protective, but the association is with harm. Mixed.
- **Zahir**: also a common given name; Borges' story "The Zahir" is an obsession. Mild.
- **Ayna, Roshan, Rahbar, Amaan**: common given names / everyday words in Urdu/Persian; no negative meaning I know of. Roshan and Amaan are very common names — weak searchability.
- **Miru**: Japanese everyday verb; no negative meaning known.

## Ranked top 4

1. **Ayna — `ayn`.** "Mirror": the repo shown back to you as it is; name free on crates.io, npm and PyPI. Trade-off: `ayn` is taken on npm/PyPI (crate free), and English speakers may not know the word — has to be explained once.
2. **Rahbar — `rah`.** "Guide who shows the way" — speaks to fear and to never leaving; name free on all three registries, `rah` crate free. Trade-off: two syllables of Persian a non-Urdu speaker may misspell (rahbar/rahber); no VCS web search done yet.
3. **Nazar — `nzr`.** "Sight" plus a protective charm — seeing and safety in one word; `nzr` free on every registry checked. Trade-off: evil-eye association cuts both ways, and `nzr` is said letter by letter.
4. **Miru — `miru`.** Japanese "to see"; name = command, four letters, easy for any speaker. Trade-off: zero ownership (crate/npm/PyPI all taken) and an existing EOL media app named Miru.

## Open (UNVERIFIED)

Debian contents for every command; GitHub search for Zahir/Darshan; web VCS search for Rahbar, Amaan, Delos, Opsis; `dri` on crates.io; trademark, domains, GitHub orgs; native-speaker meaning checks.

## Sources

crates.io API, npm registry, PyPI JSON, formulae.brew.sh API, api.github.com repository search — queried 2026-10-11. Web: [terminaltrove git clients](https://terminaltrove.com/categories/git-clients/), [alternativeto git clients](https://alternativeto.net/category/developer-tools/git-client/?p=3), [Drishti (client)](https://en.wikipedia.org/wiki/Drishti_(client)), [drishti-io](https://pypi.org/project/drishti-io/0.4/).
