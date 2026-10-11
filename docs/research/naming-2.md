# Naming, round 2 (2026-10-11)

Fresh shortlist for the product's real name. "Graft" is a codename (CLAUDE.md); nothing here was registered, published or claimed — every check is a read-only lookup.

## Brief (user, 2026-10-11)

- Drop **Tributary** and anything that sounds like it.
- Evoke **branches and the graph**, and/or simply **a good word** (Fork, Tower). Not safety, not speed.
- Command **2–4 letters**, typed all day.
- Non-native friendly: say it, spell it after hearing it once.

Carried from `docs/naming-research.md`: no `git` (trademark §2.3, `docs/naming-research.md:52-66`); no trail/marker words or `-flow`/`-kit` (`:36-45`); concrete nouns (`:27-34`); name and command may differ (`:32-34`); do not re-propose its rejected list (`:250-279` — that list already kills Twig-adjacent Grove, Arbor, Braid, Copse, Bramble, Trunk, Sapling, Delta, Thicket).

## Method (same sources as `docs/naming-research.md:316-325`)

Run today, 2026-10-11, by script:
- crates.io `api/v1/crates/<x>`, npm `registry.npmjs.org/<x>`, PyPI `pypi.org/pypi/<x>/json`, Homebrew `formulae.brew.sh/api/{formula,cask}/<x>.json` — HTTP 200 = taken, 404 = free.
- `command -v <cmd>` on this Mac (macOS, Homebrew on PATH).
- packages.debian.org contents search, exact filename, stable, any arch. Parser sanity-checked against `tig` (returns a `/usr/bin/` hit); "no hit" below means the same parser found nothing.
- GitHub repository search `q=<name> git`, sorted by stars (top 4 read).
- One web search per name for an existing Git/VCS tool.

**Not done → UNVERIFIED for every candidate:** trademark (USPTO refused automation last round, `docs/naming-research.md:287-295`), GitHub org/user availability, domains.

~45 names and commands screened. As in round 1, almost every plain English word is taken as a **crate** — so, as with `trib`, what must be free is the **command**.

## Candidates

| # | Name | Command | Why it fits | Said aloud |
|---|---|---|---|---|
| 1 | **Fern** | `fern` | A frond is a graph: one stem, branches off branches. Short good word like Fork. | "fern" (one syllable) |
| 2 | **Prong** | `prg` | A prong is one tine of a fork — a branch, Fork's own family. | "prong"; command "p-r-g" |
| 3 | **Banyan** | `ban` | Banyan branches drop roots and rejoin the ground — branches that merge back. | "BAN-yan"; command "ban" |
| 4 | **Alder** | `ald` | A tree; plain, easy word. Graph meaning weak. | "AWL-der" |
| 5 | **Rowan** | `rwn` | A tree; soft, common given name. Graph meaning weak. | "ROH-an"; command unsayable |
| 6 | **Osier** | `osr` | Willow withy — the branch used for weaving. | "OH-zher" — hard to spell from hearing |

### Checks run today

| Name / cmd | crates.io | npm | PyPI | Homebrew F / C | `command -v` | Debian bin | Git/VCS tool by this name? |
|---|---|---|---|---|---|---|---|
| fern (name **and** cmd) | taken | taken | taken | free / free | none | no hit | **No VCS tool.** Adjacent: `lambdalisue/vim-fern-git-status` (90★) is a git-status badge for the fern.vim file explorer; `fern` crate is a known Rust logging library. |
| prong | taken | taken | **free** | free / free | none | — | None found (GitHub search 21 results, none Git; web search empty) |
| `prg` | **free** | taken | taken | free / free | none | no hit | — |
| banyan | taken (tree database crate) | taken | taken | free / free | none | — | None found (web search: "didn't turn up any tool called Banyan as a Git client") |
| `ban` | **free** | taken | taken | free / free | none | no hit | — |
| alder | taken | taken | taken | free / free | none | — | No VCS tool; **`alder` is an existing `tree`-command clone CLI** (aweary/alder, AUR package) and `orbistry/alder`, an archived toolchain CLI |
| `ald` | **free** | taken | **free** | free / free | none | no hit | — |
| rowan | taken (rust-analyzer's syntax-tree crate, prominent) | taken | taken | free / free | none | — | None found |
| `rwn` | **free** | taken | **free** | free / free | none | no hit | — |
| osier | **free** | taken | taken | free / free | none | — | not searched — UNVERIFIED |
| `osr` | **free** | taken | taken | free / free | none | no hit | — |

### Disqualified today (Git/VCS clash)

| Name | Killed by |
|---|---|
| **Sprig** | Two Git clients: `billdenney/sprig` ("a git interface for MacOS and Windows") and `zhiyu547/sprig` ("lightweight native macOS Git client") — GitHub search. |
| **Kelp** | `Hobo-Ware/kelp`: "a small, fast, low-power git client for macOS, written in Rust" — GitHub search. |
| **Knot** | Tangled's self-hosted Git server is called a "knot" ([tangled.org knot](https://tangled.org/aesthetic.computer/core/tree/main/at/knot)); Homebrew formula `knot` also taken (DNS server). |
| Hazel | Homebrew cask `hazel` (Noodlesoft's Mac app) — major brand. |
| Spray, `lt`, `fd` | Shadow an installed command on this Mac. |
| Coral, Willow, Bough, Limb, Spur, Tendril, Larch, Frond, Burl, Lattice, Vine | No VCS clash found, but every short command form tried was taken on crates.io or a pronunciation dead end; not taken further. |

## Ranked top 3

1. **Fern — `fern`.** Name = command, 4 letters, one syllable, every speaker spells it. Trade-off: zero ownership — crate/npm/PyPI all taken and a Vim file explorer already uses the word with a git-status plugin; Fork-style "found by word of mouth", unsearchable.
2. **Prong — `prg`.** Closest to Fork in meaning (a fork's tine), crate `prg` free. Trade-off: command isn't the word, `prg` is said letter by letter, and "prong" is less familiar to non-native speakers.
3. **Banyan — `ban`.** Best graph metaphor (branches that root and rejoin = merge), `ban` crate free, three-letter command says itself. Trade-off: three syllables, and the command reads as the English word "ban".

## Open — must be done before use (UNVERIFIED)

- Trademark (class 9 and 42) for whichever is chosen — not attempted.
- GitHub org/user and domains — not checked this round.
- Re-run registry checks the week of announcement; free today is not reserved.

## Sources

- crates.io API, npm registry, PyPI JSON, formulae.brew.sh API, packages.debian.org contents search, api.github.com repository search — queried 2026-10-11 by the URLs in Method.
- [Tangled knot server](https://tangled.org/aesthetic.computer/core/tree/main/at/knot)
- [alder tree CLI (LinuxLinks)](https://www.linuxlinks.com/alder-minimal-implementation-tree/), [AUR alder](https://aur.archlinux.org/packages/alder), [orbistry/alder](https://github.com/orbistry/alder/releases)
- [github.com/billdenney/sprig](https://github.com/billdenney/sprig), [github.com/zhiyu547/sprig](https://github.com/zhiyu547/sprig), [github.com/Hobo-Ware/kelp](https://github.com/Hobo-Ware/kelp), [github.com/lambdalisue/vim-fern-git-status](https://github.com/lambdalisue/vim-fern-git-status)
- Git trademark policy: [git-scm.com/about/trademark](https://git-scm.com/about/trademark) (via `docs/naming-research.md:54`)
