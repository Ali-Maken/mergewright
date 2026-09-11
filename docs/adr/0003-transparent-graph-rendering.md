# The pixel graph renders with a real alpha channel

Terminal graphics strips are composited by the terminal over whatever is behind
it, and many users — including this project's primary user — run a translucent,
blurred background. `OSC 11` reports an *opaque* colour and gives no way to read
the terminal's alpha, so a strip that bakes in the queried background paints
opaque rectangles through the user's translucency. We therefore rasterise the
graph with a genuine alpha channel, never paint a background into it, and treat
the `OSC 11` reply only as a light/dark hint for choosing a theme.

## Consequences

Anti-aliased edges must be premultiplied correctly for the encoders that expect
it, and lane colours have to stay legible over an unknown backdrop — which is
also why the lane palette is fixed and independent of the UI theme rather than
being restyled per theme.
