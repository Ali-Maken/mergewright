//! The rows at the frame's edges: the footer (status or hints), the bar under a
//! live box, the tab layout's strip, banner and hint row, and what the footer
//! and banner say.

// One file of `frame`, split for size (m6-07 review, Major 4): it shares
// the parent's imports and siblings rather than restating them.
#[allow(clippy::wildcard_imports)]
use super::*;

/// A bottom row that stands in for the graph screen's footer: what went
/// wrong, else what is happening or just happened, else `hints` — the
/// graph footer's own ladder (`graft_ui::graph_screen::footer`), so a
/// status is never hidden by the screen that happens to own the row
/// (ADR-0039: a bar is never hidden).
///
/// Three rows use it: the resolver's footer, and in the sub-100-column tab
/// layout the Detail tab's hint row and the focused banner's key row. The
/// last two drew their keys only, so on the Detail tab at 80 columns the
/// relaunch's `rebase still in progress from a previous run` and the
/// resolver's `all conflicts resolved · c continue` were on
/// `state.status` and on no cell (m6-07 F9). `(x, y, width)` is the row.
pub(super) fn draw_status_or_hints(
    frame: &mut Frame,
    state: &AppState,
    tier: RenderingTier,
    (x, y, width): (u16, u16, u16),
    theme: &Theme,
    hints: impl FnOnce() -> String,
) {
    // A status is indented one space, as the graph footer indents it, so
    // it starts in the column the hints do (`hints_text` leads with the
    // space itself) — rulings 07b, Q3.
    // While a modal is open the only problem is its own refusal, and the
    // notice is `None` (`footer_notice`): the modal owns the row
    // (m6-conflicts/69, Decided 1).
    let (text, token) = match crate::view::status::bottom_row_problem(state) {
        Some(problem) => (
            format!(
                " {}",
                graft_ui::graph_screen::fit_status_line(
                    problem.reason.as_str(),
                    problem.keys.as_ref().map(SanitizedText::as_str),
                    width.saturating_sub(1),
                )
            ),
            Token::Warning,
        ),
        None => match footer_notice(state, tier) {
            Some(notice) => (format!(" {}", notice.as_str()), Token::TextMuted),
            None => (hints(), Token::TextMuted),
        },
    };
    frame.buffer_mut().set_string(
        x,
        y,
        graft_ui::fit::fit(&text, usize::from(width)),
        Style::default().fg(color(theme.token(token))),
    );
}

/// **Every bar on the frame, under a live box, says the box's keys**
/// (rulings 07b, Q1) — or what went wrong, or what is running, first, as
/// every bar does (`draw_status_or_hints`). One call, after whichever
/// screen drew and before the boxes go on top, for the reason
/// `draw_modal_boxes` is one call: the resolver, the Map view, the journal,
/// the stash list and the commit editor each draw their own bar, and a
/// per-screen answer is the one a sixth screen forgets. The graph's footer
/// and the detail pane already asked `rows::box_context` for their context;
/// writing them again here costs nothing and leaves no screen out.
pub(super) fn draw_box_bars(session: &Session, state: &AppState, frame: &mut Frame, area: Rect) {
    let Some(context) = crate::view::rows::box_context(state) else {
        return;
    };
    let tier = tier_in_force(session, state);
    for bar in bar_rows(state, area) {
        frame.render_widget(Clear, bar);
        draw_status_or_hints(
            frame,
            state,
            tier,
            (bar.x, bar.y, bar.width),
            &session.theme,
            || {
                graft_ui::graph_screen::hints_text(&graft_ui::graph_screen::hints_for(
                    &context,
                    graft_ui::graph_screen::Divider::Hidden,
                    bar.width,
                ))
            },
        );
    }
}

/// The rows this frame drew a bar of keys on: the bottom row of the screen
/// that stands where the graph does — the graph, an overlay, or with the
/// sub-100-column Detail tab up, the tab's own hint row at the foot of the
/// frame. The resolver covers the detail pane too, so its bar runs the
/// whole width. The status screen has none: its `q  quit` is a line of the
/// page, not a bar.
pub(super) fn bar_rows(state: &AppState, area: Rect) -> Vec<Rect> {
    let bottom_of = |rect: Rect| Rect {
        y: rect.bottom().saturating_sub(1),
        height: rect.height.min(1),
        ..rect
    };
    match state.screen {
        Screen::Status => Vec::new(),
        Screen::Graph if state.resolver.is_some() || state.finish.is_some() => {
            vec![bottom_of(area)]
        }
        Screen::Graph => vec![bottom_of(primary_pane_area(state, area).unwrap_or(area))],
    }
}

/// The detail pane's own hint bar, on the bottom row of the sub-100-column
/// tab layout (`update::detail::tab_hint_rows`,
/// m5-history-and-ghost-preview/29). `left` is what the operation banner
/// left, so the row is that area's last.
///
/// Everywhere else the keys ride the graph screen's footer. On the Detail
/// tab the graph screen is not drawn at all, so the bar went with it and
/// every key was hidden at 80 columns — the one outcome ADR-0037 §4 rules
/// out. The pane that owns the screen owns the keys, so it draws its own
/// `HintContext` here, through the same `hints_for` ladder and floor every
/// other bar uses (ADR-0039; m5/24, m5/28).
///
/// `Divider::Hidden`: the tab layout has no divider to drag, so `<>` is a
/// key that would do nothing — the same reason the focused banner's own
/// hints hide it.
///
/// The row is the graph footer's in every other way too: a problem or a
/// notice takes it before the keys do ([`draw_status_or_hints`], m6-07 F9).
pub(super) fn draw_tab_hints(
    frame: &mut Frame,
    state: &AppState,
    left: Rect,
    context: &HintContext,
    theme: &Theme,
    tier: RenderingTier,
) {
    let rows = crate::update::detail::tab_hint_rows(state).min(left.height);
    if rows == 0 || left.width == 0 {
        return;
    }
    draw_status_or_hints(
        frame,
        state,
        tier,
        (left.x, left.bottom() - 1, left.width),
        theme,
        || {
            graft_ui::graph_screen::hints_text(&graft_ui::graph_screen::hints_for(
                context,
                graft_ui::graph_screen::Divider::Hidden,
                left.width,
            ))
        },
    );
}

/// The operation banner in the sub-100-column tab layout: its own row above
/// the `[Graph] Detail` strip, and — while it has focus — its key hints on
/// the bottom row. Returns what is left of `area` for the strip and the pane.
///
/// Everywhere else the banner rides the graph screen's header. Below 100
/// columns the layout switches to tabs (DESIGN §1.1), the shown pane takes
/// the whole screen, the graph screen is not drawn — and the banner went
/// with it. A conflict then stopped the operation, the banner took focus,
/// and it was **modal**: `c`, `s` and `a` were live with nothing on screen
/// to find them by, at 80 columns (m5/16, review C1).
///
/// The rule this settles: *a modal overlay must be visible wherever it has
/// focus.* It is not a pane's content, so it does not travel with one — it
/// draws outside, above the tab line. ADR-0039 guarantees the small,
/// always-needed marker rather than choosing between everything and
/// nothing; this is the same principle for the thing ADR-0039 did not
/// cover.
///
/// The cost, knowingly accepted: **one** row of pane content while an
/// operation is live, **two** while its banner has the keyboard, and none
/// at all the rest of the time.
pub(super) fn draw_tab_banner(
    frame: &mut Frame,
    state: &AppState,
    area: Rect,
    theme: &Theme,
    tier: RenderingTier,
) -> Rect {
    // The row budget is `update`'s, not this function's: `geometry` and
    // `graph_rows` count from the same number, and a second opinion here is
    // exactly how a click comes to land one row off (review M1). `0` also
    // covers the Graph tab, whose graph header already carries the banner
    // (review M2).
    let left = tab_banner_area(state, area);
    if left == area {
        return area;
    }
    let Some(banner) = crate::update::banner::banner_view(state) else {
        return area;
    };
    let width = usize::from(area.width);
    let mut style = Style::default().fg(color(theme.token(Token::Warning)));
    if banner.focused {
        style = style.add_modifier(Modifier::REVERSED);
    }
    let line = graft_ui::graph_screen::banner_line(&banner);
    frame
        .buffer_mut()
        .set_string(area.x, area.y, graft_ui::fit::fit(&line, width), style);
    if left.bottom() == area.bottom() {
        return left;
    }
    // The banner's own keys, on the bottom row — the row `tab_banner_area`
    // just took off the end — unless a problem or a notice has it, as it
    // would the graph footer (m6-07 F9: the relaunch's Sticky names the
    // banner's keys itself). `Divider::Hidden`: the tab layout has no
    // divider to drag (`update::detail`'s angle keys are a no-op here), and
    // the modal would swallow `<>` in any case.
    draw_status_or_hints(
        frame,
        state,
        tier,
        (left.x, area.bottom() - 1, area.width),
        theme,
        || {
            graft_ui::graph_screen::hints_text(&graft_ui::graph_screen::hints_for(
                &crate::view::rows::banner_context(state, &banner),
                graft_ui::graph_screen::Divider::Hidden,
                area.width,
            ))
        },
    );
    left
}

/// The narrow tier's tab strip: `[Graph] Detail` or `Graph [Detail]`
/// (`[Map] Detail` / `Map [Detail]` when `state.map.shown`), bracketing
/// whichever side is on screen.
pub(super) fn draw_tab_strip(
    frame: &mut Frame,
    area: Rect,
    detail_shown: bool,
    primary_label: &str,
    theme: &graft_ui::theme::Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let text = if detail_shown {
        format!("{primary_label}  [Detail]")
    } else {
        format!("[{primary_label}]  Detail")
    };
    let style = Style::default().fg(graft_ui::paint::color(
        theme.token(graft_ui::theme::Token::TextMuted),
    ));
    frame.buffer_mut().set_string(area.x, area.y, text, style);
}

/// The footer's notice: what is happening or just happened, else `F3`'s
/// renderer line while an override is on. `None` otherwise, so the key
/// hints show.
///
/// `Status::Failed` is not here: a failure is the graph screen's `problem`
/// line, which is louder and does not vanish on the next key.
///
/// `None` while a modal is open: the modal owns the bottom row, and a
/// notice from before it (a `Busy`, a `Done`) is hidden, not cleared
/// (`AppState::modal_owns_bottom_row`, m6-conflicts/69). Reads
/// `AppState::shown_status`: an unseen failure stands in for the status
/// under it, and is the `problem` line, so no notice (m6-conflicts/78).
pub(super) fn footer_notice(state: &AppState, tier: RenderingTier) -> Option<SanitizedText> {
    if state.modal_owns_bottom_row() {
        return None;
    }
    match state.shown_status() {
        // A write is running. The only acknowledgement `Space` gives, and the
        // only thing that says why the other write keys do nothing meanwhile
        // (m3/04 review M3).
        Status::Busy(description) if description.ends_with(ESC_HINT) => {
            return Some(sanitize_line(description));
        }
        Status::Busy(description) => {
            return Some(sanitize_line(&format!("{description}…")));
        }
        // It finished. Lasts until the next key, like the copy line.
        Status::Done { what, keys } => {
            return Some(sanitize_line(&crate::view::status::done_line(
                what,
                keys.as_deref(),
            )));
        }
        // `y`'s notice, worded as what was sent: whether the terminal honoured
        // it is unknowable.
        Status::Copied { short } => {
            return Some(sanitize_line(&format!("{short}… → clipboard (osc 52)")));
        }
        // A key waiting on a read in flight: a notice, not a failure.
        Status::Waiting(words) => return Some(sanitize_line(words)),
        Status::Ready | Status::Failed(_) | Status::Sticky { .. } => {}
    }
    // An operation a previous run left half-done. Louder than the renderer
    // line and quieter than a failure — nothing is broken, but there is a
    // backup waiting and the user should know before they press `u`
    // (m3-working-tree-undo-and-stash/06).
    if let Some(notice) = crate::update::undo::interrupted_notice(state) {
        return Some(sanitize_line(&notice));
    }
    // Only below the terminal's own tier: `F3` clears the override when it
    // cycles back to the top (m1-the-graph/33).
    state
        .renderer_override
        .map(|_| sanitize_line(&format!("renderer: {} (F3 cycles)", tier_name(tier))))
}

/// The harness banner as drawn: its title with ` · <tier>` appended for the
/// tier in force (the harness cannot know it; the probe decides), then its
/// lines. Empty without a harness banner, so the shipped binary draws
/// nothing extra.
pub(in crate::view) fn banner_lines(state: &AppState) -> Vec<SanitizedText> {
    let Some(banner) = &state.harness.banner else {
        return Vec::new();
    };
    std::iter::once(format!("{} · {}", banner.title, tier_name(state.tier())))
        .chain(banner.lines.iter().cloned())
        .map(|line| sanitize_line(&line))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wait's words show in the footer as a notice — the problem line no
    /// longer carries them (review 47-49, m1).
    #[test]
    fn a_wait_is_the_footer_notice() {
        let words = "still reading the refs \u{2014} try again in a moment";
        let state = AppState {
            status: Status::Waiting(words.to_owned()),
            ..AppState::default()
        };
        let notice = footer_notice(&state, RenderingTier::Unicode).expect("a notice");
        assert_eq!(notice.as_str(), words);
    }
}
