//! The team-shaped history at his scale (m7-power-tools/41). Heavy tier.
//!
//! Each spec is built fresh into a temp dir, so the `TEAM-MEASURE` lines it
//! prints are a real build's time and size (read by
//! `scripts/41-scroll-team-fixture.sh`), not a cache hit.

use std::error::Error;
use std::path::Path;
use std::time::Instant;

use graft_fixtures::team::{self, TeamSpec, TeamStats};

type Outcome<T> = Result<T, Box<dyn Error>>;

fn tree_bytes(dir: &Path) -> std::io::Result<u64> {
    let mut total = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            total += tree_bytes(&entry.path())?;
        } else if kind.is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

fn measured(name: &str, spec: &TeamSpec) -> Outcome<(tempfile::TempDir, TeamStats)> {
    let dir = tempfile::tempdir()?;
    let started = Instant::now();
    let stats = team::build(spec, dir.path())?;
    let millis = started.elapsed().as_millis();
    let kib = tree_bytes(dir.path())? / 1024;
    println!(
        "TEAM-MEASURE {name}: {}.{:02} s, {}.{:02} MiB, {stats:?}",
        millis / 1000,
        millis % 1000 / 10,
        kib / 1024,
        kib % 1024 * 100 / 1024
    );
    Ok((dir, stats))
}

#[test]
fn monorepo_is_his_scale() -> Outcome<()> {
    let (dir, built) = measured("MONOREPO", &TeamSpec::MONOREPO)?;
    let read = team::stats(dir.path())?;
    assert_eq!(built, read);
    assert!(read.commits >= 17_694, "{read:?}");
    assert!(read.refs >= 1_323, "{read:?}");
    assert!(read.merges >= 2_000, "{read:?}");
    assert!(read.most_open >= 16, "{read:?}");
    assert!(
        dir.path()
            .join(".git/objects/info/commit-graphs/commit-graph-chain")
            .is_file()
    );
    Ok(())
}

#[test]
fn small_build_is_measured() -> Outcome<()> {
    let (_dir, stats) = measured("SMALL", &TeamSpec::SMALL)?;
    assert_eq!(stats.commits, 400);
    Ok(())
}
