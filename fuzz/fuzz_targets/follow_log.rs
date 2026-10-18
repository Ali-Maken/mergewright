//! Fuzzes `mergewright_git::trace::parse_follow_log` with arbitrary bytes: it must
//! return `Ok` or `Err`, never panic, and no row it accepts carries a line
//! range (`--follow` has none) (m7-power-tools/18).
#![no_main]

use mergewright_git::trace::parse_follow_log;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(rows) = parse_follow_log(data) {
        assert!(rows.iter().all(|row| row.range_at.is_none()));
    }
});
