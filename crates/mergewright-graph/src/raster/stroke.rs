//! A lane's shape by role (ease-pass G1, G2, G14): the pixel tier's
//! counterpart of the Unicode tier's `┃ │ ║ ┊`. Shape, not colour, carries
//! the role when colours are off, so every role has its own stroke.
//!
//! The width still comes from one `stroke_width(cell_width)` per strip; a
//! role only scales it (ADR-0024 holds: emphasis never widens a stroke, a
//! role is not emphasis).

use mergewright_core::capabilities::Rgb;
use tiny_skia::{FillRule, Mask, Paint, Path, Pixmap, Stroke, StrokeDash, Transform};

use super::geometry::{snapped, stroke_placed};
use crate::identity::BranchIdentity;
use crate::role::LaneRole;

/// How a lane of one role is stroked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneStroke {
    /// The focused branch: one stroke, half as wide again.
    Thick,
    /// A branch the focused one was built on: one stroke.
    Thin,
    /// The trunk: two parallel strokes of the base width.
    Double,
    /// Anything else: one dashed stroke of the base width.
    Dotted,
}

impl LaneStroke {
    /// The stroke a lane of `role` draws with.
    #[must_use]
    pub fn of(role: LaneRole) -> LaneStroke {
        match role {
            LaneRole::This => LaneStroke::Thick,
            LaneRole::Under => LaneStroke::Thin,
            LaneRole::Trunk => LaneStroke::Double,
            LaneRole::Other => LaneStroke::Dotted,
        }
    }

    /// The width one stroke of this shape draws at, from the strip's base.
    pub(super) fn width(self, base: f32) -> f32 {
        match self {
            LaneStroke::Thick => base * 1.5,
            LaneStroke::Thin | LaneStroke::Double | LaneStroke::Dotted => base,
        }
    }
}

/// The gap between a [`LaneStroke::Double`]'s two strokes: at least a whole
/// pixel, so the two never blur into one.
pub(super) fn double_gap(width: f32) -> f32 {
    (width * 0.75).max(1.0)
}

/// How far each of a double's strokes sits from the lane's centre line.
pub(super) fn double_offset(width: f32) -> f32 {
    f32::midpoint(width, double_gap(width))
}

/// A dotted lane's dash: on and off the same length, a stroke and a half.
fn dash(width: f32) -> Option<StrokeDash> {
    StrokeDash::new(vec![width * 1.5, width * 1.5], 0.0)
}

/// The role `identity` has in `roles`; one absent from it is
/// [`LaneRole::Other`].
pub(super) fn role_in(roles: &[(BranchIdentity, LaneRole)], identity: BranchIdentity) -> LaneRole {
    roles
        .iter()
        .find(|&&(lane, _)| lane == identity)
        .map_or(LaneRole::Other, |&(_, role)| role)
}

/// The colour and stroke a lane draws in. Without roles: its identity's
/// palette slot, one plain stroke (the look before ease-pass). With roles:
/// the palette indexed by role (This, Under, Trunk, Other; a shorter
/// palette wraps, so one colour mutes every role) and the role's stroke.
pub(super) fn lane_look(
    roles: Option<&[(BranchIdentity, LaneRole)]>,
    palette: &[Rgb],
    muted: Rgb,
    identity: BranchIdentity,
) -> (Rgb, LaneStroke) {
    match roles {
        None => (
            palette
                .get(identity.slot(palette.len()))
                .copied()
                .unwrap_or(muted),
            LaneStroke::Thin,
        ),
        Some(roles) => {
            let role = role_in(roles, identity);
            let index = match role {
                LaneRole::This => 0,
                LaneRole::Under => 1,
                LaneRole::Trunk => 2,
                LaneRole::Other => 3,
            };
            let rgb = match palette.len() {
                0 => muted,
                len => palette[index % len],
            };
            (rgb, LaneStroke::of(role))
        }
    }
}

/// Stroke `path` as a shape that is not one plain line: dotted (dashed) or
/// double (two parallel lines, made as one wide stroke with its middle
/// masked out, so curves stay parallel). `width` is one stroke's.
pub(super) fn styled(
    pixmap: &mut Pixmap,
    path: &Path,
    paint: &Paint<'_>,
    width: f32,
    shape: LaneStroke,
    place: Transform,
) {
    let plain = Stroke {
        width,
        line_cap: tiny_skia::LineCap::Butt,
        ..Stroke::default()
    };
    match shape {
        LaneStroke::Thick | LaneStroke::Thin => {
            stroke_placed(pixmap, path, paint, &plain, place);
        }
        LaneStroke::Dotted => {
            let dotted = Stroke {
                dash: dash(width),
                ..plain
            };
            stroke_placed(pixmap, path, paint, &dotted, place);
        }
        LaneStroke::Double => double(pixmap, path, paint, width, place),
    }
}

/// Two parallel strokes of `width` along `path`, `double_gap` apart.
fn double(pixmap: &mut Pixmap, path: &Path, paint: &Paint<'_>, width: f32, place: Transform) {
    let gap = double_gap(width);
    let outer = Stroke {
        width: 2.0 * width + gap,
        line_cap: tiny_skia::LineCap::Butt,
        ..Stroke::default()
    };
    let inner = Stroke {
        width: gap,
        ..outer.clone()
    };
    // `stroke` is `None` only for an outline with no area: nothing to draw.
    let Some(outer) = path.stroke(&outer, 1.0).as_ref().and_then(snapped) else {
        return;
    };
    let Some(inner) = path.stroke(&inner, 1.0).as_ref().and_then(snapped) else {
        return;
    };
    // `new` is `None` only for an empty pixmap: nothing to draw into.
    let Some(mut mask) = Mask::new(pixmap.width(), pixmap.height()) else {
        return;
    };
    mask.fill_path(&inner, FillRule::Winding, true, place);
    mask.invert();
    pixmap.fill_path(&outer, paint, FillRule::Winding, place, Some(&mask));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_role_has_its_own_stroke() {
        assert_eq!(LaneStroke::of(LaneRole::This), LaneStroke::Thick);
        assert_eq!(LaneStroke::of(LaneRole::Under), LaneStroke::Thin);
        assert_eq!(LaneStroke::of(LaneRole::Trunk), LaneStroke::Double);
        assert_eq!(LaneStroke::of(LaneRole::Other), LaneStroke::Dotted);
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "Exact multiples of an exact base.")]
    fn only_the_focused_branch_is_wider() {
        assert_eq!(LaneStroke::Thick.width(2.0), 3.0);
        for shape in [LaneStroke::Thin, LaneStroke::Double, LaneStroke::Dotted] {
            assert_eq!(shape.width(2.0), 2.0, "{shape:?}");
        }
    }

    #[test]
    fn an_identity_absent_from_the_roles_is_other_and_one_colour_mutes_all() {
        let a = BranchIdentity::of_name(b"a");
        let b = BranchIdentity::of_name(b"b");
        let roles = [(a, LaneRole::This)];
        assert_eq!(role_in(&roles, b), LaneRole::Other);
        let grey = Rgb {
            red: 9,
            green: 9,
            blue: 9,
        };
        let palette = [
            Rgb {
                red: 1,
                green: 2,
                blue: 3,
            },
            grey,
            grey,
            grey,
        ];
        assert_eq!(
            lane_look(Some(&roles), &palette, grey, a),
            (palette[0], LaneStroke::Thick)
        );
        assert_eq!(
            lane_look(Some(&roles), &palette, grey, b),
            (grey, LaneStroke::Dotted)
        );
        assert_eq!(
            lane_look(Some(&roles), &[grey], grey, a),
            (grey, LaneStroke::Thick)
        );
    }
}
