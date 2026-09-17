//! Fuzzes `graft_git::diff::parse_file_list` with arbitrary bytes: it must
//! return `Ok` or `Err`, never panic, for any input at all.
//!
//! `parse_file_list` takes two byte slices (`--raw -z` and `--numstat -z`);
//! the first input byte picks where to split the rest of the fuzz data
//! between them.
#![no_main]

use graft_git::diff::parse_file_list;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&split, rest)) = data.split_first() else {
        return;
    };
    let at = (split as usize).min(rest.len());
    let (raw_z, numstat_z) = rest.split_at(at);
    let _ = parse_file_list(raw_z, numstat_z);
});
