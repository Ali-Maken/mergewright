//! A ghost's synthetic id: SHA-1 over a domain-separated subject.

#[allow(clippy::wildcard_imports)]
use super::*;

/// A ghost's synthetic id: SHA-1 over `"graft-ghost\0"`, the **subject** git
/// would give the commit, a NUL and `n`.
///
/// The subject, not the operation's description: a rebase draws one ghost per
/// replayed commit, and they must differ from one another. The doc said
/// "description" and no caller ever passed one (m5/07 review, Minor 2).
///
/// Deterministic on purpose — two previews of the same operation must draw
/// the same rows, or the graph's row caches would churn and the eye would
/// see the picture flicker. The domain-separating prefix is why this will
/// not collide with a real commit: git hashes `commit <len>\0…`, never
/// `graft-ghost\0…`.
#[must_use]
pub fn ghost_oid(subject: &str, n: usize) -> Oid {
    let mut message = b"graft-ghost\0".to_vec();
    message.extend_from_slice(subject.as_bytes());
    message.push(0);
    message.extend_from_slice(n.to_string().as_bytes());
    Oid::from_sha1(sha1(&message))
}

/// SHA-1 (FIPS 180-4), in safe Rust: the crate forbids `unsafe`, and adding
/// a hash dependency to `core` or `ops` for one synthetic id is not worth
/// the edge. Sixty lines, fixed by a standard against which the tests check
/// it.
pub(super) fn sha1(message: &[u8]) -> [u8; 20] {
    let mut state: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut padded = message.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    let bits = (message.len() as u64).wrapping_mul(8);
    padded.extend_from_slice(&bits.to_be_bytes());

    for block in padded.as_chunks::<64>().0 {
        let mut schedule = [0u32; 80];
        for (index, word) in block.as_chunks::<4>().0.iter().enumerate() {
            schedule[index] = u32::from_be_bytes(*word);
        }
        for index in 16..80 {
            schedule[index] = (schedule[index - 3]
                ^ schedule[index - 8]
                ^ schedule[index - 14]
                ^ schedule[index - 16])
                .rotate_left(1);
        }
        let [mut wa, mut wb, mut wc, mut wd, mut we] = state;
        for (index, word) in schedule.iter().enumerate() {
            let (mixed, constant) = match index {
                0..=19 => ((wb & wc) | ((!wb) & wd), 0x5A82_7999),
                20..=39 => (wb ^ wc ^ wd, 0x6ED9_EBA1),
                40..=59 => ((wb & wc) | (wb & wd) | (wc & wd), 0x8F1B_BCDC),
                _ => (wb ^ wc ^ wd, 0xCA62_C1D6),
            };
            let next = wa
                .rotate_left(5)
                .wrapping_add(mixed)
                .wrapping_add(we)
                .wrapping_add(constant)
                .wrapping_add(*word);
            we = wd;
            wd = wc;
            wc = wb.rotate_left(30);
            wb = wa;
            wa = next;
        }
        for (slot, added) in state.iter_mut().zip([wa, wb, wc, wd, we]) {
            *slot = slot.wrapping_add(added);
        }
    }

    let mut digest = [0u8; 20];
    for (slot, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *slot = word.to_be_bytes();
    }
    digest
}
