//! His `[keys]` remaps from a real config file, through the real binary
//! (m8-polish-and-ship/09b): startup reads the file, builds the keymap and
//! says how many problems it had on the bottom row, until the first key.

use crate::pty::{Isolated, Pty};

/// `[keys.graph] merge = "M"` beside `suspend_on_ctrl_z`: `M` was the Map's,
/// so the file has one problem. The first repository screen says so on its
/// bottom row; one key later the line is gone. What `M` does is the unit
/// tests' (`update::tests::keymap_edge`): this checks the startup wiring.
#[test]
fn a_keys_table_in_the_config_file_reaches_the_first_screen() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
    let repo = scratch.path().canonicalize().expect("canonical");
    let isolated = Isolated::new();
    let dir = isolated.config().join("tributary");
    std::fs::create_dir_all(&dir).expect("config dir");
    std::fs::write(
        dir.join("config.toml"),
        "[keys]\nsuspend_on_ctrl_z = true\n\n[keys.graph]\nmerge = \"M\"\n",
    )
    .expect("write the config");

    let mut pty = Pty::spawn(&isolated, &repo, &repo, (120, 36));
    let screen = pty.wait_for("the config notice", |s| {
        s.has("config: 1 problem \u{b7} ? shows them")
    });
    assert!(!screen.has("unknown config key"), "{}", screen.text());

    pty.send(b"j");
    pty.wait_for("the notice gone at the first key", |s| !s.has("config:"));

    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}
