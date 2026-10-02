//! The credential type: never printed, never cloned, zeroed on drop.

use std::fmt;

/// A credential in memory: never printed, never cloned, zeroed on drop.
///
/// Zeroing is best effort: it covers this value's own allocation (all of
/// its capacity), not copies made before it was built — build it straight
/// from the typed bytes.
pub struct Secret(pub(super) Vec<u8>);

impl Secret {
    /// Takes ownership of `bytes`.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Secret {
        Secret(bytes)
    }

    /// The bytes, for the one place that must send them.
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}

/// Overwrites the whole allocation with zeros, spare capacity included.
///
/// The crate forbids `unsafe`, so this is not `ptr::write_volatile`; instead
/// `black_box` makes the zeroed buffer observable, which stops the compiler
/// from dropping the writes as dead stores before the free.
pub(super) fn wipe(bytes: &mut Vec<u8>) {
    let capacity = bytes.capacity();
    bytes.clear();
    // `resize` within capacity never reallocates, so this writes zeros over
    // every byte the allocation holds.
    bytes.resize(capacity, 0);
    std::hint::black_box(&bytes[..]);
}
