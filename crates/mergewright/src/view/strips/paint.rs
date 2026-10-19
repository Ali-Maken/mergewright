//! What strips are painted with: the session's cell and colours, and each
//! branch's role on the frame's route.

use mergewright_core::capabilities::Rgb;
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::{LaneRole, Route};

use super::StripRows;
use crate::session::Session;
use crate::update::AppState;

/// What every strip this session draws shares: cell size and colours.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripPaint {
    /// Cell width in pixels.
    pub cell_width: u32,
    /// Cell height in pixels.
    pub cell_height: u32,
    /// Role hues, This, Under, Trunk, Other (`Theme::raster_roles`).
    pub palette: Vec<Rgb>,
    /// The rail's hue.
    pub muted: Rgb,
}

impl StripPaint {
    /// The session's measured cell and its theme's truecolor role hues
    /// (`Theme::raster_roles`): strips colour lanes by role.
    pub fn of(session: &Session) -> StripPaint {
        StripPaint {
            cell_width: session.rendering.cell_size.width,
            cell_height: session.rendering.cell_size.height,
            palette: session.theme.raster_roles(),
            muted: session.theme.raster_muted(),
        }
    }

    /// The paint this frame's strips draw in: the session's, or with `#`
    /// pressed (ease-pass G14) its colourless theme's, as the text tiers'.
    pub fn for_frame(session: &Session, state: &AppState) -> StripPaint {
        if state.look.colours_off {
            StripPaint::of(&session.colours_off())
        } else {
            StripPaint::of(session)
        }
    }
}

/// Every branch `strips` draw (lanes of each row and the row above, and
/// stubs), once each, with its role on `route`.
pub(super) fn roles_of(strips: &[StripRows], route: &Route) -> Vec<(BranchIdentity, LaneRole)> {
    let mut seen = std::collections::HashSet::new();
    let rows = strips.iter().flat_map(|strip| {
        strip
            .rows
            .iter()
            .flat_map(|painted| {
                let lanes = painted.row.lanes.iter().flatten().copied();
                lanes.chain(painted.stubs.iter().map(|stub| stub.identity))
            })
            .chain(
                strip
                    .above
                    .iter()
                    .flat_map(|row| row.lanes.iter().flatten().copied()),
            )
    });
    rows.filter(|&identity| seen.insert(identity))
        .map(|identity| (identity, route.role(identity)))
        .collect()
}
