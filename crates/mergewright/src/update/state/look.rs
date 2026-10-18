//! How the graph is drawn this session, apart from what it shows (ease-pass
//! 07): colours on or off, and the route the lanes are coloured by.

/// The look of the frame: neither field changes what a row says.
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
}
