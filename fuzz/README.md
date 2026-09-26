# `graft-git` diff-parser fuzz targets

`cargo-fuzz` scaffold for `graft_git::diff`'s two pure parsers: `parse_unified`
and `parse_file_list`. Standalone crate (its own `[workspace]`, not a member
of the root workspace — see `fuzz/Cargo.toml`), so it never affects the main
build or `cargo test`.

## What it checks

Both parsers take arbitrary git output and must never panic — a malformed or
truncated byte stream is a `ParseError`, not a crash. Neither target spawns
`git`; the fixture round-trip test (`crates/graft-git/tests/it/diff_fixture.rs`)
is what proves the parsers agree with real git output. Fuzzing is for the
bytes real git would never produce.

## Running

Requires nightly Rust and `cargo-fuzz` (`cargo install cargo-fuzz`):

```sh
cd fuzz
cargo +nightly fuzz build                    # proves it compiles
cargo +nightly fuzz run parse_unified        # runs until stopped (Ctrl-C)
cargo +nightly fuzz run parse_file_list
cargo +nightly fuzz run conflict_parse       # never panics, and render(parse(b)) == b (m6-conflicts/01)
cargo +nightly fuzz run blame_porcelain      # blame --porcelain reader (m7-power-tools/18)
cargo +nightly fuzz run line_log             # git log -L reader
cargo +nightly fuzz run follow_log           # git log --follow --raw --numstat -z reader
```

Never run from CI, `cargo test`, or a builder's own gates (CLAUDE.md: "One
heavy job at a time" / "Tests must be laptop-safe"). A milestone review is
where an actual fuzzing run (not just the build) happens, per
`KNOWN_GAPS.md` row 127.

## Corpus

`cargo fuzz run <target>` writes interesting inputs to `fuzz/corpus/<target>/`
by default (gitignored). Seed it from the hand-written cases in
`crates/graft-git/tests/it/diff_cases.rs` if a review wants a head start
rather than starting from nothing.
