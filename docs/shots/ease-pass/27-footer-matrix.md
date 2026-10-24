# ease-pass/27: the conflict footer, re-shot

One merge stopped on conflicts (2 files), two origins, five sizes, three screens each. `shell` = merge begun in the
shell, app opened on it (`b-launch-to-first-conflict`); `app` = merge begun in the app (`a-stop-to-first-conflict`,
which lands in the resolver; `app-banner` is one Esc later, `app-pane-closed` two). `pane-focused` = Esc from the
shell landing, `pane-closed` = Esc Esc. Taken by `.scratch/ease-pass/pty-ease27.py` (pty, pyte, redacted by
`mergewright-fixtures capture`), unicode tier. Character grids only: colour is unverified (rule 5), and whether
the rows look right is the user's to judge (rule 6).

Rule under test (user 2026-10-24): when room is short the origin words go first; the count stays.
Right looks like: the bottom row never carries the origin without the count. Shell landing: 120 origin + count,
160 neither, 187 count only, 200 origin + count with `q quit` yielded, 80 keys only.

## 27-shell-landing-80x24

```
merging feature into main
Graph  [Detail]
UNCOMMITTED · 1 staged
diff vs HEAD a301516

CONFLICTS (2)
  gone.txt                                                       deleted by them
  notes.txt                                                        both modified
STAGED (1)
  M README.md                                                              +1 −0













 ⏎ resolve  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-shell-landing-120x40

```
 ◆ graft-trivial-0  /tmp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main      merging feature into main

▸ ◌                           Uncommitted changes (2 unmerged, 1 staged)
                              not committed yet · 3 files · on main
    ●    feature              feature's readme  not pushed yet  not merged                                   Graft   6y
    ●                         feature's notes                                                                Graft   6y
    ●                         feature drops gone                                                             Graft   6y
  ◆ ┃    main (you)           main's notes  not pushed yet                                                   Graft   6y
  ● ┃                         main changes gone                                                              Graft   6y
  ●━╯                         add the readme                                                                 Graft   6y
  ●                           add notes                                                                      Graft   6y
  ●                           add gone                                                                       Graft   6y
  ●                           Correct the note                                                               Graft   7y
  ●                           Add the first note                                                             Graft   7y
  ◇                           Add a readme                                                                   Graft   7y







────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 merge in progress · from the shell · 2 conflicted · ⏎ resolve  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
UNCOMMITTED · 1 staged
diff vs HEAD a301516
CONFLICTS (2)
  gone.txt    deleted by them
  notes.txt     both modified
STAGED (1)
  M README.md           +1 −0
```

## 27-shell-landing-160x48

```
 ◆ graft-trivial-0  …ch-to-first-conflict/graft-trivial-0 · main  merging feature into main  UNCOMMITTED · 1 staged
                                                                                             diff vs HEAD a301516
▸ ◌                   Uncommitted changes (2 unmerged, 1 staged)
                      not committed yet · 3 files · on main
    ●    feature      feature's readme  not pushed yet  not merged               Graft   6y
    ●                 feature's notes                                            Graft   6y  CONFLICTS (2)
    ●                 feature drops gone                                         Graft   6y    gone.txt                                          deleted by them
  ◆ ┃    main (you)   main's notes  not pushed yet                               Graft   6y    notes.txt                                           both modified
  ● ┃                 main changes gone                                          Graft   6y  STAGED (1)
  ●━╯                 add the readme                                             Graft   6y    M README.md                                                 +1 −0
  ●                   add notes                                                  Graft   6y
  ●                   add gone                                                   Graft   6y
  ●                   Correct the note                                           Graft   7y
  ●                   Add the first note                                         Graft   7y
  ◇                   Add a readme                                               Graft   7y































────────────────────────────────────────────────────────────────────────────────────────────
 merge in progress · ⏎ resolve  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-shell-landing-187x50

```
 ◆ graft-trivial-0  …t-capture/b-launch-to-first-conflict/graft-trivial-0 · main  merging feature into main  UNCOMMITTED · 1 staged
                                                                                                             diff vs HEAD a301516
▸ ◌                     Uncommitted changes (2 unmerged, 1 staged)
                        not committed yet · 3 files · on main
    ●    feature        feature's readme  not pushed yet  not merged                             Graft   6y
    ●                   feature's notes                                                          Graft   6y  CONFLICTS (2)
    ●                   feature drops gone                                                       Graft   6y    gone.txt                                                     deleted by them
  ◆ ┃    main (you)     main's notes  not pushed yet                                             Graft   6y    notes.txt                                                      both modified
  ● ┃                   main changes gone                                                        Graft   6y  STAGED (1)
  ●━╯                   add the readme                                                           Graft   6y    M README.md                                                            +1 −0
  ●                     add notes                                                                Graft   6y
  ●                     add gone                                                                 Graft   6y
  ●                     Correct the note                                                         Graft   7y
  ●                     Add the first note                                                       Graft   7y
  ◇                     Add a readme                                                             Graft   7y

































────────────────────────────────────────────────────────────────────────────────────────────────────────────
 merge in progress · 2 conflicted · ⏎ resolve  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-shell-landing-200x50

```
 ◆ graft-trivial-0  …mp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main  merging feature into main  UNCOMMITTED · 1 staged
                                                                                                                    diff vs HEAD a301516
▸ ◌                     Uncommitted changes (2 unmerged, 1 staged)
                        not committed yet · 3 files · on main
    ●    feature        feature's readme  not pushed yet  not merged                                    Graft   6y
    ●                   feature's notes                                                                 Graft   6y  CONFLICTS (2)
    ●                   feature drops gone                                                              Graft   6y    gone.txt                                                           deleted by them
  ◆ ┃    main (you)     main's notes  not pushed yet                                                    Graft   6y    notes.txt                                                            both modified
  ● ┃                   main changes gone                                                               Graft   6y  STAGED (1)
  ●━╯                   add the readme                                                                  Graft   6y    M README.md                                                                  +1 −0
  ●                     add notes                                                                       Graft   6y
  ●                     add gone                                                                        Graft   6y
  ●                     Correct the note                                                                Graft   7y
  ●                     Add the first note                                                              Graft   7y
  ◇                     Add a readme                                                                    Graft   7y

































───────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 merge in progress · from the shell · 2 conflicted · ⏎ resolve  c finish  a abort  M mergetool  e edit  ⇥ graph
```

## 27-shell-pane-focused-80x24

```
merging feature into main
Graph  [Detail]
UNCOMMITTED · 1 staged
diff vs HEAD a301516

CONFLICTS (2)
  gone.txt                                                       deleted by them
  notes.txt                                                        both modified
STAGED (1)
  M README.md                                                              +1 −0













 ⏎ resolve · M mergetool · e edit · space mark · c finish · ⇥ diff · ? keys
```

## 27-shell-pane-focused-120x40

```
 ◆ graft-trivial-0  /tmp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main      merging feature into main

▸ ◌                           Uncommitted changes (2 unmerged, 1 staged)
                              not committed yet · 3 files · on main
    ●    feature              feature's readme  not pushed yet  not merged                                   Graft   6y
    ●                         feature's notes                                                                Graft   6y
    ●                         feature drops gone                                                             Graft   6y
  ◆ ┃    main (you)           main's notes  not pushed yet                                                   Graft   6y
  ● ┃                         main changes gone                                                              Graft   6y
  ●━╯                         add the readme                                                                 Graft   6y
  ●                           add notes                                                                      Graft   6y
  ●                           add gone                                                                       Graft   6y
  ●                           Correct the note                                                               Graft   7y
  ●                           Add the first note                                                             Graft   7y
  ◇                           Add a readme                                                                   Graft   7y







────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⏎ resolve · M mergetool · e edit · space mark resolved · c finish · ⇥ diff · </> resize · . actions · ? keys
UNCOMMITTED · 1 staged
diff vs HEAD a301516
CONFLICTS (2)
  gone.txt    deleted by them
  notes.txt     both modified
STAGED (1)
  M README.md           +1 −0
```

## 27-shell-pane-focused-160x48

```
 ◆ graft-trivial-0  …ch-to-first-conflict/graft-trivial-0 · main  merging feature into main  UNCOMMITTED · 1 staged
                                                                                             diff vs HEAD a301516
▸ ◌                   Uncommitted changes (2 unmerged, 1 staged)
                      not committed yet · 3 files · on main
    ●    feature      feature's readme  not pushed yet  not merged               Graft   6y
    ●                 feature's notes                                            Graft   6y  CONFLICTS (2)
    ●                 feature drops gone                                         Graft   6y    gone.txt                                          deleted by them
  ◆ ┃    main (you)   main's notes  not pushed yet                               Graft   6y    notes.txt                                           both modified
  ● ┃                 main changes gone                                          Graft   6y  STAGED (1)
  ●━╯                 add the readme                                             Graft   6y    M README.md                                                 +1 −0
  ●                   add notes                                                  Graft   6y
  ●                   add gone                                                   Graft   6y
  ●                   Correct the note                                           Graft   7y
  ●                   Add the first note                                         Graft   7y
  ◇                   Add a readme                                               Graft   7y































────────────────────────────────────────────────────────────────────────────────────────────
 ⏎ resolve · M mergetool · e edit · space mark · c finish · ⇥ diff · </> resize · ? keys
```

## 27-shell-pane-focused-187x50

```
 ◆ graft-trivial-0  …t-capture/b-launch-to-first-conflict/graft-trivial-0 · main  merging feature into main  UNCOMMITTED · 1 staged
                                                                                                             diff vs HEAD a301516
▸ ◌                     Uncommitted changes (2 unmerged, 1 staged)
                        not committed yet · 3 files · on main
    ●    feature        feature's readme  not pushed yet  not merged                             Graft   6y
    ●                   feature's notes                                                          Graft   6y  CONFLICTS (2)
    ●                   feature drops gone                                                       Graft   6y    gone.txt                                                     deleted by them
  ◆ ┃    main (you)     main's notes  not pushed yet                                             Graft   6y    notes.txt                                                      both modified
  ● ┃                   main changes gone                                                        Graft   6y  STAGED (1)
  ●━╯                   add the readme                                                           Graft   6y    M README.md                                                            +1 −0
  ●                     add notes                                                                Graft   6y
  ●                     add gone                                                                 Graft   6y
  ●                     Correct the note                                                         Graft   7y
  ●                     Add the first note                                                       Graft   7y
  ◇                     Add a readme                                                             Graft   7y

































────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⏎ resolve · M mergetool · e edit · space mark · c finish · ⇥ diff · </> resize · . actions · ? keys
```

## 27-shell-pane-focused-200x50

```
 ◆ graft-trivial-0  …mp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main  merging feature into main  UNCOMMITTED · 1 staged
                                                                                                                    diff vs HEAD a301516
▸ ◌                     Uncommitted changes (2 unmerged, 1 staged)
                        not committed yet · 3 files · on main
    ●    feature        feature's readme  not pushed yet  not merged                                    Graft   6y
    ●                   feature's notes                                                                 Graft   6y  CONFLICTS (2)
    ●                   feature drops gone                                                              Graft   6y    gone.txt                                                           deleted by them
  ◆ ┃    main (you)     main's notes  not pushed yet                                                    Graft   6y    notes.txt                                                            both modified
  ● ┃                   main changes gone                                                               Graft   6y  STAGED (1)
  ●━╯                   add the readme                                                                  Graft   6y    M README.md                                                                  +1 −0
  ●                     add notes                                                                       Graft   6y
  ●                     add gone                                                                        Graft   6y
  ●                     Correct the note                                                                Graft   7y
  ●                     Add the first note                                                              Graft   7y
  ◇                     Add a readme                                                                    Graft   7y

































───────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⏎ resolve · M mergetool · e edit · space mark resolved · c finish · ⇥ diff · </> resize · . actions · ? keys
```

## 27-shell-pane-closed-80x24

```
 ◆ graft-trivial-0  …conflict/graft-trivial-0 · main  merging feature into main
▸ ◌                   Uncommitted changes (2 unmerged, 1 staged)
                      not committed yet · 3 files · on main
    ●    feature      feature's readme  not pushed yet  not merged   Graft   6y
    ●                 feature's notes                                Graft   6y
    ●                 feature drops gone                             Graft   6y
  ◆ ┃    main (you)   main's notes  not pushed yet                   Graft   6y
  ● ┃                 main changes gone                              Graft   6y
  ●━╯                 add the readme                                 Graft   6y
  ●                   add notes                                      Graft   6y
  ●                   add gone                                       Graft   6y
  ●                   Correct the note                               Graft   7y
  ●                   Add the first note                             Graft   7y
  ◇                   Add a readme                                   Graft   7y









 ⇥ banner · c finish · ⏎ changes · S stash · p pull · P push · f fetch · ? keys
```

## 27-shell-pane-closed-120x40

```
 ◆ graft-trivial-0  /tmp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main      merging feature into main

▸ ◌                           Uncommitted changes (2 unmerged, 1 stag…            │ feature
                              not committed yet · 3 files · on main               │ ! not pushed yet
    ●    feature              feature's readme  not pushed yet          Graft   6y│ · not merged
    ●                         feature's notes                           Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                        Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet              Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                         Graft   6y│
  ●━╯                         add the readme                            Graft   6y│ built on
  ●                           add notes                                 Graft   6y│   main
  ●                           add gone                                  Graft   6y│    ─◆ feature
  ●                           Correct the note                          Graft   7y│
  ●                           Add the first note                        Graft   7y│
  ◇                           Add a readme                              Graft   7y│
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ changes · S stash · p pull · P push · f fetch · J journal · / search · j/k move · ? keys
```

## 27-shell-pane-closed-160x48

```
 ◆ graft-trivial-0  /tmp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main                                              merging feature into main

▸ ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                  │ feature
                              not committed yet · 3 files · on main                                                       │ ! not pushed yet
    ●    feature              feature's readme  not pushed yet  not merged                                      Graft   6y│ · not merged
    ●                         feature's notes                                                                   Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                      Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                 Graft   6y│
  ●━╯                         add the readme                                                                    Graft   6y│ built on
  ●                           add notes                                                                         Graft   6y│   main
  ●                           add gone                                                                          Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                  Graft   7y│
  ●                           Add the first note                                                                Graft   7y│
  ◇                           Add a readme                                                                      Graft   7y│
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ changes · S stash · p pull · P push · f fetch · J journal · / search · j/k move · q quit · F2 stat · F3 rend · + lane · ? keys
```

## 27-shell-pane-closed-187x50

```
 ◆ graft-trivial-0  /tmp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main                                                                         merging feature into main

▸ ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                                             │ feature
                              not committed yet · 3 files · on main                                                                                  │ ! not pushed yet
    ●    feature              feature's readme  not pushed yet  not merged                                                                 Graft   6y│ · not merged
    ●                         feature's notes                                                                                              Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                                           Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                                                 Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                                            Graft   6y│
  ●━╯                         add the readme                                                                                               Graft   6y│ built on
  ●                           add notes                                                                                                    Graft   6y│   main
  ●                           add gone                                                                                                     Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                                             Graft   7y│
  ●                           Add the first note                                                                                           Graft   7y│
  ◇                           Add a readme                                                                                                 Graft   7y│
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ changes · S stash · p pull · P push · f fetch · J journal · / search · j/k move · q quit · F2 stat · F3 rend · + lane · 1 first · F focus · H hide · ? keys
```

## 27-shell-pane-closed-200x50

```
 ◆ graft-trivial-0  /tmp/graft-capture/b-launch-to-first-conflict/graft-trivial-0 · main                                                                                      merging feature into main

▸ ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                                                          │ feature
                              not committed yet · 3 files · on main                                                                                               │ ! not pushed yet
    ●    feature              feature's readme  not pushed yet  not merged                                                                              Graft   6y│ · not merged
    ●                         feature's notes                                                                                                           Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                                                        Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                                                              Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                                                         Graft   6y│
  ●━╯                         add the readme                                                                                                            Graft   6y│ built on
  ●                           add notes                                                                                                                 Graft   6y│   main
  ●                           add gone                                                                                                                  Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                                                          Graft   7y│
  ●                           Add the first note                                                                                                        Graft   7y│
  ◇                           Add a readme                                                                                                              Graft   7y│
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ changes · S stash · p pull · P push · f fetch · J journal · / search · j/k move · q quit · F2 stat · F3 rend · + lane · 1 first · F focus · H hide · M map · O over · ? keys
```

## 27-app-landing-80x24

```
merging feature into main
files   ▸ gone.txt keep?   notes.txt •
────────────────────────────────────────────────────────────────────────────────
▸ gone.txt · modified on yours · main, deleted on theirs · feature

deleted by them · o keeps the file  t deletes it
   1 gone, changed on main
















 o yours  t theirs  e edit  n next  M mergetool  esc close
```

## 27-app-landing-120x40

```
merging feature into main
Clashing files                │▸ gone.txt · modified on yours · main, deleted on theirs · feature
                              │
▏? gone.txt   keep or delete  │deleted by them · o keeps the file  t deletes it
   notes.txt  •               │   1 gone, changed on main
                              │
Combined by git (1)           │
✓ README.md                   │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
 o yours  t theirs  e edit  n next  M mergetool  esc close
```

## 27-app-landing-160x48

```
merging feature into main
Clashing files                │▸ gone.txt · modified on yours · main, deleted on theirs · feature
                              │
▏? gone.txt   keep or delete  │deleted by them · o keeps the file  t deletes it
   notes.txt  •               │   1 gone, changed on main
                              │
Combined by git (1)           │
✓ README.md                   │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
 o yours  t theirs  e edit  n next  M mergetool  esc close
```

## 27-app-landing-187x50

```
merging feature into main
Clashing files                │▸ gone.txt · modified on yours · main, deleted on theirs · feature
                              │
▏? gone.txt   keep or delete  │deleted by them · o keeps the file  t deletes it
   notes.txt  •               │   1 gone, changed on main
                              │
Combined by git (1)           │
✓ README.md                   │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
 o yours  t theirs  e edit  n next  M mergetool  esc close
```

## 27-app-landing-200x50

```
merging feature into main
Clashing files                │▸ gone.txt · modified on yours · main, deleted on theirs · feature
                              │
▏? gone.txt   keep or delete  │deleted by them · o keeps the file  t deletes it
   notes.txt  •               │   1 gone, changed on main
                              │
Combined by git (1)           │
✓ README.md                   │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
                              │
 o yours  t theirs  e edit  n next  M mergetool  esc close
```

## 27-app-banner-80x24

```
 ◆ graft-trivial-0  …conflict/graft-trivial-0 · main  merging feature into main
  ◌                   Uncommitted changes (2 unmerged, 1 staged)
▸   ●    feature      feature's readme  not pushed yet  not merged   Graft   6y
    ┃                 Graft · 6 years ago · made on feature · 1e78110
    ●                 feature's notes                                Graft   6y
    ●                 feature drops gone                             Graft   6y
  ◆ ┃    main (you)   main's notes  not pushed yet                   Graft   6y
  ● ┃                 main changes gone                              Graft   6y
  ●━╯                 add the readme                                 Graft   6y
  ●                   add notes                                      Graft   6y
  ●                   add gone                                       Graft   6y
  ●                   Correct the note                               Graft   7y
  ●                   Add the first note                             Graft   7y
  ◇                   Add a readme                                   Graft   7y









 ⏎ back to the clashes  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-app-banner-120x40

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main        merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 stag…            │ feature
▸   ●    feature              feature's readme  not pushed yet          Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110     │ · not merged
    ●                         feature's notes                           Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                        Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet              Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                         Graft   6y│
  ●━╯                         add the readme                            Graft   6y│ built on
  ●                           add notes                                 Graft   6y│   main
  ●                           add gone                                  Graft   6y│    ─◆ feature
  ●                           Correct the note                          Graft   7y│
  ●                           Add the first note                        Graft   7y│
  ◇                           Add a readme                              Graft   7y│
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 merge stopped · 2 files conflicted · ⏎ back to the clashes  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-app-banner-160x48

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main                                                merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                  │ feature
▸   ●    feature              feature's readme  not pushed yet  not merged                                      Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110                                             │ · not merged
    ●                         feature's notes                                                                   Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                      Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                 Graft   6y│
  ●━╯                         add the readme                                                                    Graft   6y│ built on
  ●                           add notes                                                                         Graft   6y│   main
  ●                           add gone                                                                          Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                  Graft   7y│
  ●                           Add the first note                                                                Graft   7y│
  ◇                           Add a readme                                                                      Graft   7y│
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 merge stopped · 2 files conflicted · ⏎ back to the clashes  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-app-banner-187x50

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main                                                                           merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                                             │ feature
▸   ●    feature              feature's readme  not pushed yet  not merged                                                                 Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110                                                                        │ · not merged
    ●                         feature's notes                                                                                              Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                                           Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                                                 Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                                            Graft   6y│
  ●━╯                         add the readme                                                                                               Graft   6y│ built on
  ●                           add notes                                                                                                    Graft   6y│   main
  ●                           add gone                                                                                                     Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                                             Graft   7y│
  ●                           Add the first note                                                                                           Graft   7y│
  ◇                           Add a readme                                                                                                 Graft   7y│
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 merge stopped · 2 files conflicted · ⏎ back to the clashes  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-app-banner-200x50

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main                                                                                        merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                                                          │ feature
▸   ●    feature              feature's readme  not pushed yet  not merged                                                                              Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110                                                                                     │ · not merged
    ●                         feature's notes                                                                                                           Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                                                        Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                                                              Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                                                         Graft   6y│
  ●━╯                         add the readme                                                                                                            Graft   6y│ built on
  ●                           add notes                                                                                                                 Graft   6y│   main
  ●                           add gone                                                                                                                  Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                                                          Graft   7y│
  ●                           Add the first note                                                                                                        Graft   7y│
  ◇                           Add a readme                                                                                                              Graft   7y│
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 merge stopped · 2 files conflicted · ⏎ back to the clashes  c finish  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-app-pane-closed-80x24

```
 ◆ graft-trivial-0  …conflict/graft-trivial-0 · main  merging feature into main
  ◌                   Uncommitted changes (2 unmerged, 1 staged)
▸   ●    feature      feature's readme  not pushed yet  not merged   Graft   6y
    ┃                 Graft · 6 years ago · made on feature · 1e78110
    ●                 feature's notes                                Graft   6y
    ●                 feature drops gone                             Graft   6y
  ◆ ┃    main (you)   main's notes  not pushed yet                   Graft   6y
  ● ┃                 main changes gone                              Graft   6y
  ●━╯                 add the readme                                 Graft   6y
  ●                   add notes                                      Graft   6y
  ●                   add gone                                       Graft   6y
  ●                   Correct the note                               Graft   7y
  ●                   Add the first note                             Graft   7y
  ◇                   Add a readme                                   Graft   7y









 ⇥ banner · c finish · ⏎ open · b branch · t tag · y sha · S stash · ? keys
```

## 27-app-pane-closed-120x40

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main        merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 stag…            │ feature
▸   ●    feature              feature's readme  not pushed yet          Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110     │ · not merged
    ●                         feature's notes                           Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                        Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet              Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                         Graft   6y│
  ●━╯                         add the readme                            Graft   6y│ built on
  ●                           add notes                                 Graft   6y│   main
  ●                           add gone                                  Graft   6y│    ─◆ feature
  ●                           Correct the note                          Graft   7y│
  ●                           Add the first note                        Graft   7y│
  ◇                           Add a readme                              Graft   7y│
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
                                                                                  │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ open · b branch · t tag · y sha · S stash · J journal · / search · q quit · F2 stat · ? keys
```

## 27-app-pane-closed-160x48

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main                                                merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                  │ feature
▸   ●    feature              feature's readme  not pushed yet  not merged                                      Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110                                             │ · not merged
    ●                         feature's notes                                                                   Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                      Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                 Graft   6y│
  ●━╯                         add the readme                                                                    Graft   6y│ built on
  ●                           add notes                                                                         Graft   6y│   main
  ●                           add gone                                                                          Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                  Graft   7y│
  ●                           Add the first note                                                                Graft   7y│
  ◇                           Add a readme                                                                      Graft   7y│
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
                                                                                                                          │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ open · b branch · t tag · y sha · S stash · J journal · / search · q quit · F2 stat · F3 rend · x compare · + lane · j/k move · ? keys
```

## 27-app-pane-closed-187x50

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main                                                                           merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                                             │ feature
▸   ●    feature              feature's readme  not pushed yet  not merged                                                                 Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110                                                                        │ · not merged
    ●                         feature's notes                                                                                              Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                                           Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                                                 Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                                            Graft   6y│
  ●━╯                         add the readme                                                                                               Graft   6y│ built on
  ●                           add notes                                                                                                    Graft   6y│   main
  ●                           add gone                                                                                                     Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                                             Graft   7y│
  ●                           Add the first note                                                                                           Graft   7y│
  ◇                           Add a readme                                                                                                 Graft   7y│
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
                                                                                                                                                     │
───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ open · b branch · t tag · y sha · S stash · J journal · / search · q quit · F2 stat · F3 rend · x compare · + lane · j/k move · space fold · 1 first · ? keys
```

## 27-app-pane-closed-200x50

```
 ◆ graft-trivial-0  /tmp/graft-capture/a-stop-to-first-conflict/graft-trivial-0 · main                                                                                        merging feature into main

  ◌                           Uncommitted changes (2 unmerged, 1 staged)                                                                                          │ feature
▸   ●    feature              feature's readme  not pushed yet  not merged                                                                              Graft   6y│ ! not pushed yet
    ┃                         Graft · 6 years ago · made on feature · 1e78110                                                                                     │ · not merged
    ●                         feature's notes                                                                                                           Graft   6y│ ! main has 2 new commits since it
    ●                         feature drops gone                                                                                                        Graft   6y│   left
  ◆ ┃    main (you)           main's notes  not pushed yet                                                                                              Graft   6y│ · Graft, last commit 6y ago
  ● ┃                         main changes gone                                                                                                         Graft   6y│
  ●━╯                         add the readme                                                                                                            Graft   6y│ built on
  ●                           add notes                                                                                                                 Graft   6y│   main
  ●                           add gone                                                                                                                  Graft   6y│    ─◆ feature
  ●                           Correct the note                                                                                                          Graft   7y│
  ●                           Add the first note                                                                                                        Graft   7y│
  ◇                           Add a readme                                                                                                              Graft   7y│
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
                                                                                                                                                                  │
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⇥ banner · c finish · ⏎ open · b branch · t tag · y sha · S stash · J journal · / search · q quit · F2 stat · F3 rend · x compare · + lane · j/k move · space fold · 1 first · F focus · ? keys
```

## 27-relaunch-rebase-80x24

```
REBASING  step 1 of 2 · pick c3b01ea feature writes
Graph  [Detail]
UNCOMMITTED
diff vs HEAD b0eb655

CONFLICTS (1)
  shared.txt                                                       both modified
▎keep 1
  3   3   keep 2
  4   4   keep 3
  5   5   keep 4
      6 + <<<<<<< HEAD
  6   7   main's bottom
      8 + =======
      9 + feature's bottom
     10 + >>>>>>> c3b01ea (feature writes)







 ⏎ resolve  c continue  s skip  a abort  M mergetool  e edit  ⇥ graph  q quit
```

## 27-relaunch-rebase-120x40

```
 ◆ graft-trivial-0  …n-relaunch/graft-trivial-0 · detached b0eb655  REBASING  step 1 of 2 · pick c3b01ea feature writes

╭ still to apply · not commits yet ────────────────────────────────────────────────────────────────────────────────────╮
│ pick 2e2d51c feature writes again                                                                                    │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
▸ ◌                           Uncommitted changes (1 unmerged)
                              not committed yet · 1 file
    ●    feature              feature writes again  not pushed yet                                           Graft   6y
    ●                         feature writes                                                                 Graft   6y
  ◆ ┊  ⓘ main                 main rewrites it  not pushed yet  HEAD is here                                 Graft   6y
  ●┄╯                         add the shared file                                                            Graft   6y
  ●                           Correct the note                                                               Graft   7y
  ●                           Add the first note                                                             Graft   7y
  ◇                           Add a readme                                                                   Graft   7y








────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 rebase still in progress · 1 conflicted · ⏎ resolve  c continue  s skip  a abort  M mergetool  e edit  ⇥ graph  q quit
UNCOMMITTED
diff vs HEAD b0eb655
CONFLICTS (1)                 ▎keep 1
  shared.txt    both modified   3   3   keep 2
                                4   4   keep 3
                                5   5   keep 4
                                    6 + <<<<<<< HEAD
                                6   7   main's bottom
                                    8 + =======
                                    9 + feature's bottom
                                   10 + >>>>>>> c3b01ea (feature writes)
```
