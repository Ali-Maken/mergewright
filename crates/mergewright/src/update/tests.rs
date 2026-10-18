use crate::spelled::Spelled as _;
use mergewright_core::capabilities::RenderingTier;
use mergewright_core::event::{
    Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind,
};
use mergewright_core::job::{JobFailure, JobId};

use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::oid::Oid;
use mergewright_git::history::Order;
use mergewright_git::version::SystemGit;
use mergewright_graph::{Follow, Row};

use super::geometry::HEADER_ROWS;
use super::mouse::WHEEL_LINES;
use super::*;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use crate::history::{
    History, HistoryUpdate, Phase, Promotion, RowRequest, View, WalkFacts, Wanted, Window,
};
use crate::repository::{Head, RepositoryFacts};
use crate::search::{Search, SearchState};
use mergewright_ui::graph_screen::{
    CommitGraphLine, Divider, Hint, HintContext, HintKey, OrderNotice,
};
use mergewright_ui::keymap::{ActionId, KeyChord, Keymap};

use super::detail::Loading as EvilLoading;
use super::detail::{DetailSpec, DetailState, LOCAL_JOB_BIT, Loading, LocalJobs};
use crate::search::Query;
use mergewright_git::diff::{DiffOptions, FileEntry, FileList, FileStatus};
use mergewright_ops::mistake_radar::probes::EvilVerdict;
use mergewright_term::sanitize::sanitize_line;
use mergewright_ui::detail::{
    DetailFocus, DiffView, FileListView, FileRow, FileRowKind, FileStatusView,
};
use std::sync::Arc as DetailArc;

mod fixtures;
use fixtures::facts;
pub(crate) use fixtures::reveal;
pub(crate) use fixtures::{
    asked_view, awaiting, commit_pane, core, key, merge_on_screen, operation_live_as, order,
    progress, request, resolver_card_on_screen, resolver_on_screen, rows, rows_for, sized,
    sorted_order, sorting,
};

mod pane_space;

mod first_launch;
mod folded_merge;
mod graph_mode;
mod graph_mode_help;
mod graph_mode_reask;
mod head_keep;
mod place_footers;

mod keymap_edge;

mod keymap_rules;

mod keymap_words;

mod palette_live;

mod pixel_preview;

mod commit_graph_offer;

/// Critical 1 of the m3/06 review: a global binding must never fire while a
/// text field has the keyboard. Typing `update` as a stash message launched
/// an undo at the `u` — a journaled write nobody asked for, from inside a
/// field the same milestone had just added.
///
/// One test per text-entry surface in the app, each driven through
/// [`update`] so the whole key path runs, because the defect was in the
/// *order* of that path and a handler called directly cannot see it.
mod a_text_field_keeps_its_own_letters;

// Helpers more than one child uses, defined beside their first users
// (m8/21b-5 split this file by topic); the children reach them through here.
use dead_keys::{bar_hints, dead_keys_on};
use detail_pane::{diff_of, list_commands, two_files};
use doorway::pinned_harness;
use prelude::working_tree_read;
use resolver_screens::{clash_for, resolver_for};
use screens::walk_started;
use search::{found, on_hand, typed};
use sweep::{
    ALSO_INERT, Press, SWEPT_WIDTHS, actions_of, answered, did_something, nothing_happened,
    outcome, presses_for, settled, states_for,
};
use sweep_boxes::{
    EVERY_OVERLAY, Shown, box_over_the_graph, finish_screen_for, overlay_over, picker_over,
    two_branch_map,
};
use sweep_entries::{badge_in_the_air, journal_entry, stash_entry, status_entry};
use sweep_screens::{changes_pane_screen, history_screens, screen_for};
use sweep_states::{
    MERGE_ROW, changes_pane_open, head_spec, holds_search, holds_stash_row, long_diff,
    on_the_message, one_stash, pane_files, search_standing, show_pane, stash_under_cursor,
    stopped_operation,
};
use walk::{from, started};
use working_tree_row::dirty_on_screen;

mod bar_keys;
mod bar_over_overlay;
mod carve_geometry;
mod copy_and_compare;
mod cursor_drawn;
mod dead_keys;
mod detail_pane;
mod divider_and_wheel;
mod doorway;
mod evil_probe_retry;
mod graph_toggles;
mod hint_coverage;
mod job_ids;
mod jobs_and_refresh;
mod opening;
mod pointer;
mod prelude;
mod prelude_reads;
mod purity;
mod quit_focus_resize;
mod resolver_screens;
mod screens;
mod search;
mod sweep;
mod sweep_boxes;
mod sweep_entries;
mod sweep_screens;
mod sweep_states;
mod tab_strip_geometry;
mod walk;
mod walk_sort;
mod working_tree_row;
