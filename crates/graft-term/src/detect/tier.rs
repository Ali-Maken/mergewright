//! Picking the rendering tier: DESIGN.md §4.2's table, plus §4.3's tmux rule.

use graft_core::capabilities::{PixelProtocol, RenderingTier, Supported};

use super::Environment;
use super::probe;

/// What the terminal can draw, once tmux has had its say.
pub(super) struct Graphics {
    pub(super) kitty: Supported,
    /// The terminal answered `OK` to the `o=z` query in a fenced round, and
    /// supports Kitty graphics at all.
    pub(super) kitty_deflate: bool,
    pub(super) sixel: Supported,
    /// Whether image-protocol bytes reach a terminal that can act on them at
    /// all. False inside a tmux that will not pass them through, which is the
    /// whole point of §4.3: no image tier is attempted on a hope.
    pub(super) reaches_the_terminal: bool,
}

impl Graphics {
    /// Graphics as a direct probe round found them.
    pub(super) fn from(replies: &probe::ProbeReplies) -> Self {
        let kitty = super::supported(replies.fenced().then_some(replies.kitty_graphics));
        Self {
            kitty,
            kitty_deflate: kitty == Supported::Yes && replies.kitty_deflate,
            sixel: super::supported(replies.sixel()),
            reaches_the_terminal: true,
        }
    }

    /// Graphics ruled out because nothing gets through to the terminal.
    pub(super) fn blocked() -> Self {
        Self {
            kitty: Supported::Silent,
            kitty_deflate: false,
            sixel: Supported::Silent,
            reaches_the_terminal: false,
        }
    }
}

/// The tier tables of DESIGN.md §4.2, plus §4.3's tmux rule.
///
/// Two answers, decided separately because they are two questions. The
/// *protocol* is the best image transport this terminal can be reached
/// through — Kitty, then iTerm2, then Sixel — and is reported whatever tier
/// is chosen, because it is a fact about the terminal and because
/// `graph.renderer = "pixel"` needs to know what to force.
///
/// The *tier* follows ADR-0016's order: **Kitty → Unicode → iTerm2 → Sixel →
/// ASCII**. Unicode is not a fallback here; it is the default for every
/// terminal without the Kitty protocol, because the terminals this graph is
/// read in most days have no image protocol at all. iTerm2 takes the tier
/// only where the alternative is ASCII, which it plainly beats; moving it
/// above Unicode needs a `TESTING.md` checklist row saying it beats that too,
/// and a new ADR. Sixel never takes the tier: there is no encoder behind it
/// (ADR-0016), so a Sixel-only terminal draws text like one with no protocol.
pub(super) fn tier(
    environment: &Environment,
    graphics: &Graphics,
    iterm2_images: bool,
) -> (RenderingTier, Option<PixelProtocol>) {
    let protocol = if graphics.kitty == Supported::Yes {
        Some(PixelProtocol::Kitty)
    } else if iterm2_images {
        Some(PixelProtocol::Iterm2)
    } else if graphics.sixel == Supported::Yes {
        Some(PixelProtocol::Sixel)
    } else {
        None
    };

    let tier = match protocol {
        Some(PixelProtocol::Kitty) => RenderingTier::Pixel,
        // §4.3 is explicit that a tmux without passthrough settles on the
        // Unicode tier, so it does not drop further on a missing `COLORTERM`:
        // tmux draws box-drawing characters whatever its colour depth.
        _ if environment.tmux.is_some() || environment.has_truecolor() => RenderingTier::Unicode,
        // No truecolor and no tmux: the choice is between pixels and `|`/`*`,
        // and an image protocol wins that one — when Graft has an encoder
        // for it.
        Some(PixelProtocol::Iterm2) => RenderingTier::Pixel,
        // Sixel is reported but never drawn: ADR-0016 keeps no Sixel encoder,
        // so a pixel tier here would be a tier with nothing behind it. With
        // no truecolor, the text tier the table leaves is ASCII.
        Some(PixelProtocol::Sixel) | None => RenderingTier::Ascii,
    };
    (tier, protocol)
}
