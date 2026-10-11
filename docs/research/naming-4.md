# Naming, round 4 (2026-10-11): foresight + watchful calm

"Graft" is a codename. Lookups below are read-only; nothing registered or claimed.

## Brief (user, 2026-10-11)

Foresight (Ghost Preview: the future drawn faded before you confirm) + watchful calm (Mistake Radar) + terminal + easy on the eye. Feel: "zen". Short, lowercase, pleasant typed. Command 2–4 letters, ideally = name. Any language; easy for an Urdu speaker. No `git`, no trail/marker words, no "sight/mirror/guide" generics, no existing Git/VCS or well-known terminal/dev tool names.

Not re-proposed: `docs/naming-research.md:250-279`, `docs/research/naming-2.md:27-63`, `docs/research/naming-3.md:28-65`, Tributary.

## Method (same as `docs/research/naming-2.md:14-22`), run today by script

- HTTP status: crates.io `api/v1/crates/<x>`, npm `registry.npmjs.org/<x>`, PyPI `pypi.org/pypi/<x>/json`, Homebrew `formulae.brew.sh/api/{formula,cask}/<x>.json`. 200 = taken, 404 = free, 000 = no response (UNVERIFIED).
- `command -v <x>` on this Mac.
- GitHub `api.github.com/search/repositories?q=<x>+git`, stars-sorted, top 3 read. Rate-limited after 10 names → UNVERIFIED for the rest.
- Web search for a Git/VCS/terminal tool: done for enso, mira, sukoon/sabr only. Others UNVERIFIED.
- Not done for any: Debian binaries, trademark, domains, GitHub orgs, native-speaker meaning checks.

## Candidates

| # | Name | Cmd | Language, meaning | Blend | Typed | Said |
|---|---|---|---|---|---|---|
| 1 | **Sukoon** | `skn` / `suk` | Urdu/Persian/Arabic *sukūn*, "calm, peace of mind" | Calm while it watches; his own language | `$ skn` | "su-KOON" |
| 2 | **Sabr** | `sabr` | Arabic/Urdu/Persian/Turkish, "patience, composure" | Calm restraint: look, then confirm | `$ sabr` | "SAB-r" |
| 3 | **Saki** | `saki` | Japanese 先 *saki*, "ahead, what comes next" | Foresight, short, zen-feeling | `$ saki` | "SAH-kee" |
| 4 | **Aram** | `aram` | Urdu/Persian *ārām*, "rest, ease"; Armenian given name | Easy on the eye, calm | `$ aram` | "AA-raam" |
| 5 | **Mira** | `mira` | Latin/Spanish "look!/behold"; Slavic *mir* "peace"; a star | Look ahead + peace | `$ mira` | "MEE-ra" |
| 6 | **Yume** | `yume` | Japanese 夢, "dream" (vision of what may be) | Ghost future, soft | `$ yume` | "YOO-meh" |
| 7 | **Lumi** | `lumi` | Finnish "snow" | Quiet, clean screen | `$ lumi` | "LOO-mee" |
| 8 | **Koan** | `koan` | Zen 公案, a riddle for insight | Zen, insight | `$ koan` | "KOH-an" |
| 9 | **Enso** | `enso` | Japanese 円相, zen brush circle | Purest "zen" | `$ enso` | "EN-so" |
| 10 | **Kiri** | `kiri` | Japanese 霧, "mist" | Faded ghost commits | `$ kiri` | "KEE-ree" |
| 11 | **Sora** | `sora` | Japanese 空, "sky" | Calm, open view | `$ sora` | "SO-ra" |
| 12 | **Kumo** | `kumo` | Japanese 雲, "cloud" (also 蜘蛛 "spider") | Calm, soft | `$ kumo` | "KOO-mo" |

Letter shapes: Saki, Mira, Lumi, Kiri, Sora, Aram, Enso are all open vowel–consonant alternation, no clusters. `sabr` ends in a cluster ("br"); `skn` is consonants only.

## Checks run today

| Name / cmd | crates | npm | PyPI | Brew F / C | `command -v` | GitHub `<x> git` top hits | Git/VCS/terminal tool? |
|---|---|---|---|---|---|---|---|
| sukoon | **free** | **free** | **free** | free / free | none | 14; websites only | None ([web search](https://lib.rs/crates/git-gud) results showed none) |
| `skn` | **free** | taken | **free** | free / free | none | — | — |
| `suk` | no response — UNVERIFIED | taken | taken | free / — | none | — | — |
| sabr (=cmd) | **free** | taken | taken | free / free | none | 871; personal sites | None (same web search) |
| `sbr` | taken | taken | **free** | free / — | none | — | — |
| saki (=cmd) | taken | taken | taken | free / free | none | 722; `duniter/sakia` (a currency client), blogs | not web-searched — UNVERIFIED |
| `sak` | **free** | taken | taken | free / — | none | — | — |
| aram (=cmd) | **free** | **free** | taken | free / free | none | rate-limited — UNVERIFIED | UNVERIFIED |
| mira (=cmd) | taken | taken | taken | free / free | none | 2279; none named mira | No Git client ([web search](https://terminaltrove.com/categories/git-clients/)); a Go terminal folder visualiser `mondial7/mira` ([pkg.go.dev](https://pkg.go.dev/github.com/mondial7/mira)) — minor |
| yume | taken | taken | **free** | free / free | none | 312; blogs | UNVERIFIED. **`yum` (short form) is the RHEL package manager — don't clip** |
| lumi | taken | taken | taken | free / free | none | 1022; none tools | UNVERIFIED |
| koan | taken | taken | taken | free / free | none | 70; Ruby Koans repos | UNVERIFIED; "koans" is a known programming-exercise genre — weak ownership |
| enso | taken | taken | taken | free / free | none | 86; none | **Enso is a macOS terminal app built on libghostty** (per web search, [awesome.ecosyste.ms](https://awesome.ecosyste.ms/projects/github.com%2Famanfromsolan%2Fenso)); also Enso the data language (own knowledge, UNVERIFIED) — **fails "no terminal tool names"** |
| kiri | taken (after one 000) | taken | taken | free / free | none | `leoheck/kiri` (710★): visual diff of KiCad schematics across git commits | **Git-adjacent visual-diff tool — fails** |
| sora | taken | taken | taken | free / free | none | 750; none tools | UNVERIFIED; OpenAI's Sora video model (own knowledge) — crowded |
| kumo | taken | taken | taken | **taken** / free | none | rate-limited | Homebrew formula exists — **fails** |

### Meaning risks (UNVERIFIED — own knowledge, needs native speakers)

- **Sukoon**: positive in Urdu/Hindi/Arabic. `suk` near "suck" in English, and Danish/Norwegian *suk* = "sigh" — prefer `skn`.
- **Sabr**: positive; religious overtone in Urdu/Arabic (patience as virtue). Mild.
- **Saki**: Japanese "ahead"; also *sake* (rice wine) server/girl's name, and H.H. Munro's pen name. Mild.
- **Aram**: Urdu "rest" but also Armenian name / ancient Aramaeans. "aaraam" can read as "lazy, idle". Mild.
- **Mira**: positive across Spanish, Slavic, Hindi (given name). None known.
- **Kumo**: 蜘蛛 also "spider". Mild.

## Ranked top 5

1. **Sukoon — `skn`.** Calm in his own language; free on crates, npm, PyPI, Brew. Trade-off: `skn` is unsayable as a word; name is 6 letters.
2. **Mira — `mira`.** Look ahead + peace, name = command, prettiest typed. Trade-off: zero registry ownership; small Go terminal folder viewer of the same name.
3. **Saki — `saki`.** Literally "ahead" — foresight with zen feel, name = command. Trade-off: all registries taken; Git-tool web check not done.
4. **Sabr — `sabr`.** Composure; crate free, name = command. Trade-off: "br" ending is the one awkward cluster; religious overtone.
5. **Aram — `aram`.** Ease/rest, soft letters, crate+npm free. Trade-off: can read "idle"; no GitHub/web clash check done (rate-limited).

## Open (UNVERIFIED)

Web Git-tool search for Saki, Aram, Yume, Lumi, Koan, Sora; GitHub search for Aram, Kumo; `suk` on crates.io; Debian binaries; trademark; domains; native-speaker checks.

## Sources

crates.io API, npm registry, PyPI JSON, formulae.brew.sh API, api.github.com search — queried 2026-10-11. Web: [terminaltrove git clients](https://terminaltrove.com/categories/git-clients/), [pkg.go.dev mondial7/mira](https://pkg.go.dev/github.com/mondial7/mira), [awesome.ecosyste.ms enso](https://awesome.ecosyste.ms/projects/github.com%2Famanfromsolan%2Fenso), [lib.rs git-gud](https://lib.rs/crates/git-gud) (search result page for sukoon/sabr; neither found).
