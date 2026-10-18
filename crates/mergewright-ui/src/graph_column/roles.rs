//! A lane drawn by its role (ease-pass G1, G2): what it is to the branch
//! the cursor row was made on, in shape and in colour, so three colours and
//! grey serve any number of branches, and the shapes still say it with
//! colour off (`#`, `NO_COLOR`).

use mergewright_graph::LaneRole;

use crate::theme::Token;

/// A lane's straight run: `┃` this, `│` built on, `║` trunk, `┊` other.
/// ASCII: `|` for the first three, `:` for other.
#[must_use]
pub fn lane_glyph(role: LaneRole, ascii: bool) -> char {
    match (role, ascii) {
        (LaneRole::This, false) => '┃',
        (LaneRole::Under, false) => '│',
        (LaneRole::Trunk, false) => '║',
        (LaneRole::Other, false) => '┊',
        (LaneRole::This | LaneRole::Under | LaneRole::Trunk, true) => '|',
        (LaneRole::Other, true) => ':',
    }
}

/// A connector's run, in the joining branch's role: `━ ─ ═ ┄`.
/// ASCII: `-` for the first three, `.` for other.
#[must_use]
pub fn join_glyph(role: LaneRole, ascii: bool) -> char {
    match (role, ascii) {
        (LaneRole::This, false) => '━',
        (LaneRole::Under, false) => '─',
        (LaneRole::Trunk, false) => '═',
        (LaneRole::Other, false) => '┄',
        (LaneRole::This | LaneRole::Under | LaneRole::Trunk, true) => '-',
        (LaneRole::Other, true) => '.',
    }
}

/// The colour a role draws in.
#[must_use]
pub fn role_token(role: LaneRole) -> Token {
    match role {
        LaneRole::This => Token::RoleThis,
        LaneRole::Under => Token::RoleUnder,
        LaneRole::Trunk => Token::RoleTrunk,
        LaneRole::Other => Token::RoleOther,
    }
}

/// A geometry glyph redrawn in `role`: straight runs take the role's
/// shape; elbows, junctions and nodes keep theirs. Unicode only — the ASCII
/// table (`style::to_ascii`) maps the role shapes onward.
pub(super) fn in_role(glyph: char, role: LaneRole) -> char {
    match glyph {
        '│' => lane_glyph(role, false),
        '─' => join_glyph(role, false),
        other => other,
    }
}

#[cfg(test)]
mod roles_tests;
