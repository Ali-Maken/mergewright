//! How a held-key run's terminal takes images: Kitty raw, Kitty deflated or iTerm2.

use mergewright_core::capabilities::{PixelProtocol, RenderingTier};

use crate::frames::run::capabilities;

impl Encoding {
    /// The harness's terminal at `tier`, sending images this way: a field
    /// set on the value, never a `Capabilities` literal (42 Decided 4).
    pub(super) fn capabilities(
        self,
        tier: RenderingTier,
    ) -> mergewright_core::capabilities::Capabilities {
        let mut caps = capabilities(tier);
        match self {
            Encoding::Raw => {}
            Encoding::Deflate => caps.kitty_deflate = true,
            Encoding::Iterm2 => caps.pixel_protocol = Some(PixelProtocol::Iterm2),
        }
        caps
    }
}

/// How the pixel tier's images go out (46 Decided 7): set on the
/// capabilities value by parameter, never by env (42 Decided 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Encoding {
    /// Kitty, raw RGBA: `kitty_deflate` off.
    Raw,
    /// Kitty, zlib (`o=z`): `kitty_deflate` on.
    Deflate,
    /// iTerm2 inline images: one composited band per strip.
    Iterm2,
}
