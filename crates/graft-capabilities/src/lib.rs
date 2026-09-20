//! Graft's feature database: parses `docs/capabilities/*.md`, checks its
//! citations against the repository on disk, and generates `CAPABILITIES.md`
//! and the browsable `docs/generated/capabilities.html`.
//!
//! Repo metadata tooling, not product code — see `docs/capabilities/README.md`
//! for the schema and workflow, and `ARCHITECTURE.md` ("`capabilities`:
//! repo metadata tooling, outside the table") for why it lives outside the
//! product crate graph. Nothing in `crates/graft` or `crates/graft-ops` may
//! depend on this crate.
//!
//! Three calls are the whole interface: [`check::check`] is what
//! `tests/it/checker.rs` runs under `cargo nextest run`; [`load`] and the
//! two renderers are what `src/bin/gen_capabilities.rs` calls to regenerate
//! the generated files.

#![forbid(unsafe_code)]

mod check;
mod model;
mod parse;
mod render_html;
mod render_md;

pub use check::{Finding, FindingKind, check};
pub use model::{Capability, Decision, DecisionPath, Status};
pub use parse::{LoadError, load};
pub use render_html::render as render_html;
pub use render_md::render as render_markdown;
