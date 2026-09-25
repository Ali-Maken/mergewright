//! The conflict resolver's frame: the resolver itself, the banner above it and
//! its own footer.

// One file of `frame`, split for size (m6-07 review, Major 4): it shares
// the parent's imports and siblings rather than restating them.
#[allow(clippy::wildcard_imports)]
use super::*;

/// The conflict resolver over the whole frame. Always `None`: it draws no
/// pixels, whatever the terminal can do.
///
/// **It keeps the banner word, and it is the only overlay that does.** Map,
/// journal, stash and the commit editor all replace the graph screen, and
/// the operation banner rides the graph screen's header — so opening one
/// mid-rebase takes `REBASING step 3 of 7` off the screen. That is a fair
/// trade for a branch list; it is not one for the screen a user opens
/// *because* they are mid-rebase and needs to know which side is which. So
/// the resolver stays an ordinary `GraphOverlay` — the frame's match, the
/// row budget and `banner::graph_screen_on_frame` all keep their one
/// answer — and pays for the banner out of its own rows, on the first line
/// of its own area. `update::resolver::banner_rows` is the one place that
/// row is counted, and the keys and the mouse count from the same function.
///
/// The last row is the resolver's own footer: a failure's line if there is
/// one, else whatever just happened, else the hint bar. The same ladder the
/// graph screen's footer uses, because the resolver covers the graph screen
/// and its footer with it (ADR-0039: a bar is never hidden).
pub(super) fn draw_resolver(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    let banner_rows = crate::update::resolver::banner_rows(state);
    let body = draw_resolver_banner(frame, state, area, &session.theme, banner_rows);
    // The footer's row is not the document's.
    let document = Rect {
        height: body.height.saturating_sub(1),
        ..body
    };
    // Where the list goes, and so the arrangement and the document's
    // width, from the one split the mouse uses too (m6-conflicts/14 S5):
    // `resolver_split` measures `state.size`, which is this frame's area
    // (seeded from the terminal at startup, updated on every resize), and
    // it splits the same rows `draw_resolver_banner` and the footer leave.
    let (_, placed, split) = crate::view::resolver::resolver_split(state, banner_rows);
    let doc = crate::view::resolver::resolver_view(state, split.doc.width)?;
    let laid_out = graft_ui::conflict_resolver::layout(
        &doc,
        crate::view::resolver::arrangement(state, placed),
        split.doc.width,
    );
    let tier = tier_in_force(session, state);
    frame.render_widget(
        graft_ui::conflict_resolver::ResolverScreen {
            doc: &doc,
            layout: &laid_out,
            top: state.resolver.as_ref().map_or(0, |open| open.top),
            placement: placed,
            theme: &session.theme,
            ascii: tier == RenderingTier::Ascii,
        },
        document,
    );
    draw_resolver_footer(frame, state, body, &session.theme, tier);
    None
}

/// The operation banner on the resolver's first row, and what is left for
/// the resolver itself.
pub(super) fn draw_resolver_banner(
    frame: &mut Frame,
    state: &AppState,
    area: Rect,
    theme: &Theme,
    rows: u16,
) -> Rect {
    if rows == 0 || area.height <= rows {
        return area;
    }
    if let Some(banner) = crate::update::banner::banner_view(state) {
        let line = graft_ui::graph_screen::banner_line(&banner);
        frame.buffer_mut().set_string(
            area.x,
            area.y,
            graft_ui::fit::fit(&line, usize::from(area.width)),
            Style::default().fg(color(theme.token(Token::Warning))),
        );
    }
    Rect {
        y: area.y + rows,
        height: area.height - rows,
        ..area
    }
}

/// The resolver's bottom row.
pub(super) fn draw_resolver_footer(
    frame: &mut Frame,
    state: &AppState,
    body: Rect,
    theme: &Theme,
    tier: RenderingTier,
) {
    if body.height == 0 || body.width == 0 {
        return;
    }
    let hints = || {
        graft_ui::graph_screen::hints_text(&graft_ui::graph_screen::hints_for(
            &crate::view::rows::resolver_context(state),
            graft_ui::graph_screen::Divider::Hidden,
            // Not `body.width`: the floor `hints_for` drops the resolver's
            // keys at is a *width* one, and the rows can be short enough to
            // put the widget below its floor at a perfectly wide screen.
            // Asking with 0 there gives the same one-key bar the narrow case
            // gets, so the bar never offers a key `resolver_key` is about to
            // refuse.
            if crate::update::resolver::fits(state) {
                body.width
            } else {
                0
            },
        ))
    };
    draw_status_or_hints(
        frame,
        state,
        tier,
        (body.x, body.bottom() - 1, body.width),
        theme,
        hints,
    );
}

/// The finish summary over the whole frame (m6-conflicts/19). Always
/// `None`: it draws no pixels.
///
/// Laid out like the resolver: the banner line on the first row, the
/// summary under it ([`crate::view::finish::summary_area`], which the keys
/// and the mouse measure from too), and its own bar on the last row —
/// `committing…` while the commit runs, else the summary's keys. **Never
/// the status line**: the summary says what went wrong in its own notice
/// row, and a status left over from before it opened is not about it.
pub(super) fn draw_finish(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    let banner_rows = crate::update::resolver::banner_rows(state);
    let body = draw_resolver_banner(frame, state, area, &session.theme, banner_rows);
    let view = crate::view::finish::finish_view(state)?;
    let summary = Rect {
        height: body.height.saturating_sub(1),
        ..body
    };
    let tier = tier_in_force(session, state);
    frame.render_widget(
        &graft_ui::finish_summary::FinishScreen {
            view: &view,
            theme: &session.theme,
            ascii: tier == RenderingTier::Ascii,
        },
        summary,
    );
    if body.height > 0 && body.width > 0 {
        frame.buffer_mut().set_string(
            body.x,
            body.bottom() - 1,
            graft_ui::fit::fit(
                &crate::view::finish::finish_bar(state),
                usize::from(body.width),
            ),
            Style::default().fg(color(session.theme.token(Token::TextMuted))),
        );
    }
    None
}
