//! Fuzzes `graft_git::conflict::parse` with arbitrary bytes: it must return
//! `Ok` or `Err`, never panic, and whatever it accepts must render back to
//! exactly the input — the byte-exactness every take and write-back rests on
//! (m6-conflicts/01).
#![no_main]

use graft_git::conflict::parse;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(file) = parse(data) {
        assert_eq!(file.render(), data, "render(parse(b)) must equal b");
    }
});
