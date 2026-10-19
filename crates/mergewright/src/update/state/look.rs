//! How the graph is drawn this session, apart from what it shows (ease-pass
//! 07): colours on or off, the route the lanes are coloured by, and the
//! branch reads (09) the route and the branch pane come from.

/// The look of the frame: no field changes what a row says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Look {
    /// `#` turned colours off (ease-pass G14, Q3 a): every screen draws
    /// through `Theme::colourless` until `#` again or quit. Shapes, bold and
    /// dim stay. Not saved.
    pub(crate) colours_off: bool,
    /// The route last read for a branch (ease-pass G1): its trunk and the
    /// bases it was built on. `None` until 09's read lands; the graph then
    /// draws by `Route::unread`, the cursor row's branch and the trunk
    /// only. `Option` because `AppState` is `Default` and a route is not.
    pub(crate) route: Option<mergewright_graph::Route>,
    /// The branch pane's reads this refs generation (ease-pass 09): where
    /// `route` comes from. Kept here, beside the route, rather than as one
    /// more `AppState` field (its size row).
    /// Boxed: inline, the map tips `AppState` over 8 KiB (clippy `large_stack_arrays` in tests).
    pub(crate) reads: Box<crate::update::branch_pane::Reads>,
}
