//! Fuzzes `mergewright_git::blame::parse_porcelain` with arbitrary bytes: it must
//! return `Ok` or `Err`, never panic, and every committed line it accepts
//! names a commit it also returns (m7-power-tools/18).
#![no_main]

use mergewright_git::blame::parse_porcelain;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok((lines, commits)) = parse_porcelain(data) {
        for line in &lines {
            if let Some(id) = line.commit {
                assert!(commits.contains_key(&id), "a line's commit is in the map");
            }
        }
    }
});
