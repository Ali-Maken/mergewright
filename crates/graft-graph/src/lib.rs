//! The commit graph: the pure layout engine that turns a rev-walk into lanes
//! and rows, and the pixel, Unicode and ASCII renderers that draw them. Layout
//! is data in, data out — no terminal, no filesystem, no clock — so it can be
//! tested and benchmarked on its own. The pixel renderer's output is a
//! `RasterFrame`; encoding that into a terminal image protocol is `term`'s job,
//! deliberately a different crate. Depends only on `core`; it never calls into
//! `git`, `ops` or `term`.
