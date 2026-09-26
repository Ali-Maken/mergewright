# Scout: rm --cached dropped by a set-aside switch (M7/rmcached, review m7-28 Q1)

## Which step drops it
The put-back. `crates/graft-ops/src/apply/set_aside.rs:493-519` (`put_back`): for every operation except an absorb with an `index_base`, it runs plain `git stash apply S` (lines 514-518), **without `--index`**. Only absorb's rebuilt stash gets `--index` (508-513). Plain `stash apply` restores the working tree only; the index comes from HEAD, so `.env` goes back to being tracked and shows ` M`.

Set-aside itself (`create` :60-73 `stash create`, `put_aside` :104-118 `stash store` + `reset --hard`) records the deletion correctly: `S^2` (index commit) lacks `.env`. Checkout is not the cause.

Side fact from the probe: `reset --hard` (:118) overwrites the ignored `.env` on disk with HEAD's `base` until the put-back writes `MINE` back. If the put-back fails, `MINE` lives only in `S`.

## Probe (temp repo a throwaway temp repo (removed from the tree))
Same state: `main` tracks `.env`, `.gitignore` has `.env`, `git rm --cached .env`, `.env`=`MINE`, `other` tracks `.env`.
- **Plain `git switch other`: carries it.** Exits 0, prints `D .env`; after it: `D  .env` (plus `?? .env`, since `other` has no `.gitignore`), content `MINE`.
- Graft's sequence (`stash create`/`store`, `reset --hard`, `checkout other`, `stash apply S`): ` M .env`, `MINE`. Reproduces Q1.
- `stash apply --index S` (back on `main`) gives `D  .env`. Not tried on `other`: could not confirm that `--index` applies cleanly there.

## Contradictions with the question
None. The question's step 3 matches the probe.
